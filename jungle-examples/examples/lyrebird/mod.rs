use clap::{Parser, ValueEnum};
use directories_next::BaseDirs;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, JourneyStatus, JungleClient, Server};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(feature = "viewer")]
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod action;
mod effect;
pub mod mcts;
pub mod tokens;
#[cfg(feature = "viewer")]
mod ui;

use crate::action::{
    BeginIteration, BuildOptimizationPromptFocused, CompareIterationCandidateMels, FlattenEither,
    FlattenLyrebirdPromptPhase, GenerateIterationCandidateAudio, GenerateIterationCandidateMels,
    InstrumentEnabled, InstrumentEnabledFocused, LogIterationTiming, LyrebirdLoopForever,
    PreparePromptCandidatesFocused, RequestPromptCandidatesFocused, SeedLyrebirdState,
    SelectDspBranchFocused, SetCurrentInstrument, SkipInstrumentPromptFocused,
    SkipInstrumentSubmit, SubmitDspBranch,
};
use crate::tokens::{Prompt, Tool};

const DEFAULT_WORKERS: usize = 3;
const DEFAULT_TREE_DEPTH: usize = 64;
const DEFAULT_INSTRUMENT_PARALLELISM: usize = 1;
const DEFAULT_LOG_FILTER: &str = "warn,lyrebird=info";
pub(crate) const LYREBIRD_DURATION_SECS: f64 = 4.0;
pub(crate) const GUITAR_INTRO_SCORE_SPEC: &str =
    "electric-guitar(rhythm-sustained):[350,58,96],[350,58,96],[446,58,96],[542,58,96],[542,58,96],[638,56,96],[638,56,96],[734,56,96],[830,56,96],[830,56,96],[926,53,96],[926,53,96],[1022,53,96],[1118,53,96],[1118,53,96],[1214,51,96],[1214,51,96],[1310,51,96],[1406,51,96],[1406,51,96],[1502,49,96],[1502,49,96],[1598,49,96],[1694,46,96],[1694,49,96],[1694,46,96],[1790,49,96],[1790,46,96],[1886,58,96],[1886,58,96],[1982,58,96],[2078,58,96],[2078,58,96],[2174,56,96],[2174,56,96],[2270,56,96],[2366,56,96],[2366,56,96],[2462,53,96],[2462,53,96],[2558,53,96],[2654,53,96],[2654,53,96],[2750,51,96],[2750,51,96],[2846,51,96]";
pub(crate) const VOCALS_SCORE_SPEC: &str = "vocals(formant):[250,66,96,'wel'],[346,68,288,'come'],[634,68,96,'to'],[730,66,96,'the'],[826,71,384,'jun'],[1210,68,192,'gol'],[1786,66,96,'weve'],[1882,68,288,'got'],[2170,68,96,'fun'],[2266,66,192,'and'],[2458,68,288,'games']";
pub(crate) const BACKUP_VOCALS_SCORE_SPEC: &str = "vocals(group-harmony):[150,71,384],[534,70,384],[918,68,384],[1302,66,384],[1686,73,384],[2070,72,384],[2454,70,384],[2838,68,384]";
pub(crate) const GUITAR_SOLO_SCORE_SPEC: &str = "electric-guitar(sustained):[240,60,192],[432,72,128],[560,75,129],[689,82,896],[1585,82,128],[1713,81,129],[1842,80,704],[2546,78,96],[2642,79,96],[2738,73,672],[3410,73,224]";
pub(crate) const BASS_SCORE_SPEC: &str = "bass:[150,32,192],[342,32,192],[534,30,192],[726,27,96],[822,32,192],[1014,27,96],[1110,30,192],[1302,29,192],[1494,27,192],[1686,32,192],[1878,32,192],[2070,30,192],[2262,27,96],[2358,32,192],[2550,27,96],[2646,42,96],[2838,42,96],[3030,42,96]";
pub(crate) const DRUMS_CYMBAL_SCORE_SPEC: &str =
    "cymbal:[150,57,192],[438,49,192],[726,57,192],[1686,57,192]";
pub(crate) const DRUMS_HIHAT_SCORE_SPEC: &str =
    "hihat:[1878,46,192],[2070,46,192],[2262,46,192],[2454,46,192],[2646,46,192],[2838,46,192],[3030,46,192],[3222,46,192]";
pub(crate) const DRUMS_KICK_DRUM_SCORE_SPEC: &str =
    "kick-drum:[150,36,192],[438,36,192],[726,36,192],[1110,36,192],[1686,36,48],[1686,36,192],[2454,36,192],[3030,36,192],[3222,36,192]";
pub(crate) const DRUMS_SNARE_DRUM_SCORE_SPEC: &str =
    "snare-drum:[1302,38,48],[1350,38,192],[2070,38,192],[2838,38,192],[3606,38,192]";

const RHYTHM_GUITAR_SCORE_SPECS: [&str; 1] = [GUITAR_INTRO_SCORE_SPEC];
const VOCALS_SCORE_SPECS: [&str; 1] = [VOCALS_SCORE_SPEC];
const BACKUP_VOCALS_SCORE_SPECS: [&str; 1] = [BACKUP_VOCALS_SCORE_SPEC];
const BASS_SCORE_SPECS: [&str; 1] = [BASS_SCORE_SPEC];
const GUITAR_SOLO_SCORE_SPECS: [&str; 1] = [GUITAR_SOLO_SCORE_SPEC];
const DRUMS_SCORE_SPECS: [&str; 4] = [
    DRUMS_CYMBAL_SCORE_SPEC,
    DRUMS_HIHAT_SCORE_SPEC,
    DRUMS_KICK_DRUM_SCORE_SPEC,
    DRUMS_SNARE_DRUM_SCORE_SPEC,
];

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DspCode {
    pub iteration_id: String,
    pub source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
    #[serde(default, alias = "similarity")]
    pub mel_similarity: Option<f32>,
    #[serde(default)]
    pub score: Option<f32>,
    #[serde(default)]
    pub audio_metrics: Option<LyrebirdAudioMetrics>,
    #[serde(default)]
    pub audio_metric_errors: Option<LyrebirdAudioMetricErrors>,
}

impl DspCode {
    fn placeholder_initial() -> Self {
        Self {
            iteration_id: "initial".to_owned(),
            ..Self::default()
        }
    }

    pub fn mel_similarity(&self) -> Option<f32> {
        self.mel_similarity.or(self.score)
    }

    pub fn score(&self) -> Option<f32> {
        self.score.or(self.mel_similarity)
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyrebirdPatch {
    pub search: String,
    pub replacement: String,
    pub note: String,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdAudioMetrics {
    pub zero_crossing_rate: f32,
    pub crest_factor: f32,
    pub spectral_centroid: f32,
    pub spectral_flatness: f32,
    pub spectral_rolloff: f32,
}

impl LyrebirdAudioMetrics {
    pub fn relative_errors(self, target: Self) -> LyrebirdAudioMetricErrors {
        LyrebirdAudioMetricErrors {
            zero_crossing_rate: normalized_relative_error(
                target.zero_crossing_rate,
                self.zero_crossing_rate,
            ),
            crest_factor: normalized_relative_error(target.crest_factor, self.crest_factor),
            spectral_centroid: normalized_relative_error(
                target.spectral_centroid,
                self.spectral_centroid,
            ),
            spectral_flatness: normalized_relative_error(
                target.spectral_flatness,
                self.spectral_flatness,
            ),
            spectral_rolloff: normalized_relative_error(
                target.spectral_rolloff,
                self.spectral_rolloff,
            ),
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdAudioMetricErrors {
    pub zero_crossing_rate: f32,
    pub crest_factor: f32,
    pub spectral_centroid: f32,
    pub spectral_flatness: f32,
    pub spectral_rolloff: f32,
}

impl LyrebirdAudioMetricErrors {
    pub fn average_match_score(self) -> f32 {
        let metric_matches = [
            1.0 - self.zero_crossing_rate,
            1.0 - self.crest_factor,
            1.0 - self.spectral_centroid,
            1.0 - self.spectral_flatness,
            1.0 - self.spectral_rolloff,
        ];

        metric_matches
            .into_iter()
            .map(|value| value.clamp(0.0, 1.0))
            .sum::<f32>()
            / metric_matches.len() as f32
    }
}

pub(crate) fn aggregate_sample_score(
    mel_similarity: f32,
    metric_errors: LyrebirdAudioMetricErrors,
) -> f32 {
    (mel_similarity.clamp(0.0, 1.0) * 5.0 + metric_errors.average_match_score() * 5.0) / 10.0
}

fn normalized_relative_error(target: f32, generated: f32) -> f32 {
    let scale = target.abs().max(generated.abs()).max(1e-6);
    ((generated - target).abs() / scale).clamp(0.0, 1.0)
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdBranchNode {
    pub code: DspCode,
    pub patch: Option<LyrebirdPatch>,
    pub mel_spectrogram_path: String,
}

impl LyrebirdBranchNode {
    pub fn score(&self) -> Option<f32> {
        self.code.score()
    }

    pub fn mel_similarity(&self) -> Option<f32> {
        self.code.mel_similarity()
    }

    pub fn similarity(&self) -> Option<f32> {
        self.score()
    }

    pub fn from_generated(code: DspCode, patch: LyrebirdPatch) -> Self {
        Self {
            mel_spectrogram_path: code.spectrogram_path.clone(),
            code,
            patch: Some(patch),
        }
    }
}

impl From<DspCode> for LyrebirdBranchNode {
    fn from(code: DspCode) -> Self {
        Self {
            mel_spectrogram_path: code.spectrogram_path.clone(),
            code,
            patch: None,
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdGeneratedCandidate {
    pub patch: LyrebirdPatch,
    pub code: DspCode,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdPreparedCandidate {
    pub patch: LyrebirdPatch,
    pub source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum LyrebirdInstrument {
    #[default]
    RhythmGuitar,
    Vocals,
    BackupVocals,
    Bass,
    GuitarSolo,
    Drums,
}

impl LyrebirdInstrument {
    pub const ALL: [Self; 6] = [
        Self::RhythmGuitar,
        Self::Vocals,
        Self::BackupVocals,
        Self::Bass,
        Self::GuitarSolo,
        Self::Drums,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "rhythm-guitar",
            Self::Vocals => "vocals",
            Self::BackupVocals => "backup-vocals",
            Self::Bass => "bass",
            Self::GuitarSolo => "guitar-solo",
            Self::Drums => "drums",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "Electric Guitar (Intro)",
            Self::Vocals => "Vocals (Formant)",
            Self::BackupVocals => "Vocals (Group Harmony)",
            Self::Bass => "Bass",
            Self::GuitarSolo => "Electric Guitar (Solo)",
            Self::Drums => "Drums",
        }
    }

    pub fn score_specs(self) -> &'static [&'static str] {
        match self {
            Self::RhythmGuitar => &RHYTHM_GUITAR_SCORE_SPECS,
            Self::Vocals => &VOCALS_SCORE_SPECS,
            Self::BackupVocals => &BACKUP_VOCALS_SCORE_SPECS,
            Self::Bass => &BASS_SCORE_SPECS,
            Self::GuitarSolo => &GUITAR_SOLO_SCORE_SPECS,
            Self::Drums => &DRUMS_SCORE_SPECS,
        }
    }

    pub fn relative_target_sample_path(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "jungle-examples/examples/lyrebird/assets/guitar_intro_4s.wav",
            Self::Vocals => "jungle-examples/examples/lyrebird/assets/vocals_4s.wav",
            Self::BackupVocals => "jungle-examples/examples/lyrebird/assets/backup_vocals_4s.wav",
            Self::Bass => "jungle-examples/examples/lyrebird/assets/bass_4s.wav",
            Self::GuitarSolo => "jungle-examples/examples/lyrebird/assets/guitar_solo_4s.wav",
            Self::Drums => "jungle-examples/examples/lyrebird/assets/drums_4s.wav",
        }
    }

    pub fn relative_dsp_path(self) -> &'static str {
        match self {
            Self::RhythmGuitar => {
                "jungle-examples/examples/welcome/audio/src/dsp/electric_guitar/rhythm.rs"
            }
            Self::Vocals => {
                "jungle-examples/examples/welcome/audio/src/dsp/vocals/formant/speech_synthesis/singer.rs"
            }
            Self::BackupVocals => {
                "jungle-examples/examples/welcome/audio/src/dsp/vocals/backup.rs"
            }
            Self::Bass => "jungle-examples/examples/welcome/audio/src/dsp/bass.rs",
            Self::GuitarSolo => {
                "jungle-examples/examples/welcome/audio/src/dsp/electric_guitar/lead.rs"
            }
            Self::Drums => "jungle-examples/examples/welcome/audio/src/dsp/drums.rs",
        }
    }

    pub fn output_stem(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "guitar_intro_4s",
            Self::Vocals => "vocals_4s",
            Self::BackupVocals => "backup_vocals_4s",
            Self::Bass => "bass_4s",
            Self::GuitarSolo => "guitar_solo_4s",
            Self::Drums => "drums_4s",
        }
    }

    pub fn tool_name(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "replace_rhythm_guitar_dsp",
            Self::Vocals => "replace_vocals_formant_dsp",
            Self::BackupVocals => "replace_backup_vocals_dsp",
            Self::Bass => "replace_bass_dsp",
            Self::GuitarSolo => "replace_guitar_solo_dsp",
            Self::Drums => "replace_drums_dsp",
        }
    }

    pub fn render_subject(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "electric-guitar(rhythm-sustained)",
            Self::Vocals => "vocals(formant)",
            Self::BackupVocals => "vocals(group-harmony)",
            Self::Bass => "bass",
            Self::GuitarSolo => "electric-guitar(sustained)",
            Self::Drums => "drum-kit",
        }
    }

    pub fn tree_tag(self) -> &'static str {
        self.slug()
    }

    pub fn parse_cli_selection(value: &str) -> Result<Self, String> {
        let normalized = value
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .flat_map(|ch| ch.to_lowercase())
            .collect::<String>();

        match normalized.as_str() {
            "rhythmguitar" | "introguitar" | "guitarintro" | "intro" => {
                Ok(Self::RhythmGuitar)
            }
            "vocals" | "leadvocals" | "leadvocal" | "vocal" => Ok(Self::Vocals),
            "backupvocals" | "backingvocals" | "harmonyvocals" | "groupharmony" => {
                Ok(Self::BackupVocals)
            }
            "bass" => Ok(Self::Bass),
            "guitarsolo" | "sologuitar" | "solo" => Ok(Self::GuitarSolo),
            "drums" | "drumkit" | "kit" => Ok(Self::Drums),
            _ => Err(format!(
                "invalid instrument argument: {value}. Expected a comma-delimited list drawn from introguitar,vocals,backupvocals,bass,sologuitar,drums"
            )),
        }
    }
}

pub trait LyrebirdInstrumentTag {
    const INSTRUMENT: LyrebirdInstrument;
}

#[derive(Optic, Default, Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdInstrumentState {
    pub instrument: LyrebirdInstrument,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub target_sample_path: String,
    #[serde(default)]
    pub target_audio_metrics: LyrebirdAudioMetrics,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
    pub selected_branch: Vec<LyrebirdBranchNode>,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub last_similarity: f32,
    pub compile_ready: bool,
    #[serde(default)]
    pub instrument_parallelism: usize,
    pub prompt_attempt: u32,
    #[serde(default)]
    pub iteration_id: String,
    #[serde(default)]
    pub pending_prompt: Option<Prompt>,
    #[serde(default)]
    pub pending_prompt_candidates: Vec<crate::effect::PromptCandidateResponse>,
    pub skipped_this_iteration: bool,
    pub last_retry_reason: Option<String>,
    pub pending_generated_patch: Option<LyrebirdPatch>,
    pub pending_generated_source: Option<String>,
    pub latest_generated_patch: Option<LyrebirdPatch>,
    pub latest_generated_code: Option<DspCode>,
    pub latest_rendered_code: Option<DspCode>,
    #[serde(default)]
    pub pending_candidates: Vec<LyrebirdPreparedCandidate>,
    #[serde(default)]
    pub iteration_candidates: Vec<LyrebirdGeneratedCandidate>,
    pub latest_generated_sample_path: Option<String>,
    pub latest_generated_spectrogram_path: Option<String>,
    pub latest_generated_similarity: Option<f32>,
    pub best_generated_code: Option<DspCode>,
    pub best_generated_sample_path: Option<String>,
    pub best_generated_spectrogram_path: Option<String>,
    pub best_similarity: Option<f32>,
}

impl LyrebirdInstrumentState {
    fn from_seed(seed: LyrebirdInstrumentSeed) -> Self {
        Self {
            instrument: seed.instrument,
            disabled: seed.disabled,
            target_sample_path: seed.target_sample_path,
            target_audio_metrics: seed.target_audio_metrics,
            target_spectrogram_path: seed.target_spectrogram_path,
            dsp_source_path: seed.dsp_source_path,
            initial_dsp_code: seed.initial_dsp_code.clone(),
            selected_branch: vec![seed.initial_dsp_code.into()],
            ..Self::default()
        }
    }

    fn observation_placeholder(instrument: LyrebirdInstrument) -> Self {
        Self {
            instrument,
            target_sample_path: instrument.relative_target_sample_path().to_owned(),
            target_audio_metrics: LyrebirdAudioMetrics::default(),
            target_spectrogram_path: String::new(),
            dsp_source_path: instrument.relative_dsp_path().to_owned(),
            initial_dsp_code: DspCode::placeholder_initial(),
            selected_branch: vec![DspCode::placeholder_initial().into()],
            ..Self::default()
        }
    }

    fn begin_iteration(
        &mut self,
        output_root: &str,
        iteration_id: &str,
        instrument_parallelism: usize,
    ) {
        let iteration_dir = PathBuf::from(output_root).join(iteration_id);
        let sample_stem = self.instrument.output_stem();
        self.sample_path = iteration_dir
            .join(format!("{sample_stem}.wav"))
            .display()
            .to_string();
        self.spectrogram_path = iteration_dir
            .join(format!("{sample_stem}.png"))
            .display()
            .to_string();
        self.compile_ready = false;
        self.instrument_parallelism = instrument_parallelism;
        self.prompt_attempt = 0;
        self.iteration_id = iteration_id.to_owned();
        self.pending_prompt = None;
        self.pending_prompt_candidates.clear();
        self.skipped_this_iteration = false;
        self.last_retry_reason = None;
        self.pending_generated_patch = None;
        self.pending_generated_source = None;
        self.latest_generated_patch = None;
        self.pending_candidates.clear();
        self.iteration_candidates.clear();
        self.last_similarity = 0.0;
    }
}

#[derive(Optic, Clone, Debug, Serialize, Deserialize)]
pub struct PromptInstrumentState<Marker> {
    pub state: LyrebirdInstrumentState,
    #[serde(skip)]
    marker: std::marker::PhantomData<fn() -> Marker>,
}

impl<Marker> PromptInstrumentState<Marker> {
    fn new(state: LyrebirdInstrumentState) -> Self {
        Self {
            state,
            marker: std::marker::PhantomData,
        }
    }
}

impl<Marker> Default for PromptInstrumentState<Marker>
where
    Marker: LyrebirdInstrumentTag,
{
    fn default() -> Self {
        Self::new(LyrebirdInstrumentState::observation_placeholder(
            Marker::INSTRUMENT,
        ))
    }
}

pub type RhythmGuitarPromptState = PromptInstrumentState<RhythmGuitarMarker>;
pub type VocalsPromptState = PromptInstrumentState<VocalsMarker>;
pub type BackupVocalsPromptState = PromptInstrumentState<BackupVocalsMarker>;
pub type BassPromptState = PromptInstrumentState<BassMarker>;
pub type GuitarSoloPromptState = PromptInstrumentState<GuitarSoloMarker>;
pub type DrumsPromptState = PromptInstrumentState<DrumsMarker>;

#[derive(Optic, Default, Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdState {
    pub output_root: String,
    pub current_instrument: LyrebirdInstrument,
    #[jungle(focus)]
    pub rhythm_guitar: RhythmGuitarPromptState,
    #[jungle(focus)]
    pub vocals: VocalsPromptState,
    #[jungle(focus)]
    pub backup_vocals: BackupVocalsPromptState,
    #[jungle(focus)]
    pub bass: BassPromptState,
    #[jungle(focus)]
    pub guitar_solo: GuitarSoloPromptState,
    #[jungle(focus)]
    pub drums: DrumsPromptState,
    #[serde(default = "default_instrument_parallelism")]
    pub instrument_parallelism: usize,
    #[serde(default)]
    pub iteration_start_time_ms: Option<u64>,
    pub iteration: u64,
    pub iteration_id: String,
}

impl LyrebirdState {
    pub fn enabled_instrument_count(&self) -> u64 {
        LyrebirdInstrument::ALL
            .into_iter()
            .filter(|instrument| !self.instrument_state(*instrument).disabled)
            .count() as u64
    }

    pub fn generation_count(&self) -> u64 {
        self.iteration
            .saturating_mul(self.instrument_parallelism as u64)
            .saturating_mul(self.enabled_instrument_count())
    }

    pub fn has_all_instrument_states(&self) -> bool {
        self.rhythm_guitar.state.instrument == LyrebirdInstrument::RhythmGuitar
            && self.vocals.state.instrument == LyrebirdInstrument::Vocals
            && self.backup_vocals.state.instrument == LyrebirdInstrument::BackupVocals
            && self.bass.state.instrument == LyrebirdInstrument::Bass
            && self.guitar_solo.state.instrument == LyrebirdInstrument::GuitarSolo
            && self.drums.state.instrument == LyrebirdInstrument::Drums
    }

    pub fn normalized_for_observation(&self) -> Self {
        self.clone()
    }

    pub fn instrument_state(&self, instrument: LyrebirdInstrument) -> &LyrebirdInstrumentState {
        match instrument {
            LyrebirdInstrument::RhythmGuitar => &self.rhythm_guitar.state,
            LyrebirdInstrument::Vocals => &self.vocals.state,
            LyrebirdInstrument::BackupVocals => &self.backup_vocals.state,
            LyrebirdInstrument::Bass => &self.bass.state,
            LyrebirdInstrument::GuitarSolo => &self.guitar_solo.state,
            LyrebirdInstrument::Drums => &self.drums.state,
        }
    }

    pub fn instrument_state_mut(
        &mut self,
        instrument: LyrebirdInstrument,
    ) -> &mut LyrebirdInstrumentState {
        match instrument {
            LyrebirdInstrument::RhythmGuitar => &mut self.rhythm_guitar.state,
            LyrebirdInstrument::Vocals => &mut self.vocals.state,
            LyrebirdInstrument::BackupVocals => &mut self.backup_vocals.state,
            LyrebirdInstrument::Bass => &mut self.bass.state,
            LyrebirdInstrument::GuitarSolo => &mut self.guitar_solo.state,
            LyrebirdInstrument::Drums => &mut self.drums.state,
        }
    }

    pub fn current_state(&self) -> &LyrebirdInstrumentState {
        self.instrument_state(self.current_instrument)
    }

    pub fn current_state_mut(&mut self) -> &mut LyrebirdInstrumentState {
        self.instrument_state_mut(self.current_instrument)
    }

    pub fn matches_seed_instrument_selection(&self, seed: &LyrebirdSeed) -> bool {
        LyrebirdInstrument::ALL.into_iter().all(|instrument| {
            let state = self.instrument_state(instrument);
            seed.instruments
                .iter()
                .find(|seed| seed.instrument == instrument)
                .map(|seed_state| {
                    state.disabled == seed_state.disabled
                        && state.target_audio_metrics == seed_state.target_audio_metrics
                })
                .unwrap_or(false)
        })
    }
}

fn default_instrument_parallelism() -> usize {
    DEFAULT_INSTRUMENT_PARALLELISM
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdInstrumentSeed {
    pub instrument: LyrebirdInstrument,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub target_sample_path: String,
    #[serde(default)]
    pub target_audio_metrics: LyrebirdAudioMetrics,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdSeed {
    pub output_root: String,
    pub instruments: Vec<LyrebirdInstrumentSeed>,
    #[serde(default = "default_instrument_parallelism")]
    pub instrument_parallelism: usize,
}

impl From<LyrebirdSeed> for LyrebirdState {
    fn from(seed: LyrebirdSeed) -> Self {
        let LyrebirdSeed {
            output_root,
            instruments,
            instrument_parallelism,
        } = seed;
        let current_instrument = instruments
            .first()
            .map(|instrument| instrument.instrument)
            .unwrap_or_default();

        let instrument_states = instruments
            .into_iter()
            .map(|seed| {
                let instrument = seed.instrument;
                let mut state = LyrebirdInstrumentState::from_seed(seed);
                state.instrument_parallelism = instrument_parallelism;
                (instrument, state)
            })
            .collect::<BTreeMap<_, _>>();

        Self {
            output_root,
            current_instrument,
            rhythm_guitar: RhythmGuitarPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::RhythmGuitar)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(
                            LyrebirdInstrument::RhythmGuitar,
                        )
                    }),
            ),
            vocals: VocalsPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::Vocals)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(LyrebirdInstrument::Vocals)
                    }),
            ),
            backup_vocals: BackupVocalsPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::BackupVocals)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(
                            LyrebirdInstrument::BackupVocals,
                        )
                    }),
            ),
            bass: BassPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::Bass)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(LyrebirdInstrument::Bass)
                    }),
            ),
            guitar_solo: GuitarSoloPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::GuitarSolo)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(
                            LyrebirdInstrument::GuitarSolo,
                        )
                    }),
            ),
            drums: DrumsPromptState::new(
                instrument_states
                    .get(&LyrebirdInstrument::Drums)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(LyrebirdInstrument::Drums)
                    }),
            ),
            instrument_parallelism,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RhythmGuitarMarker;
impl LyrebirdInstrumentTag for RhythmGuitarMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::RhythmGuitar;
}

#[derive(Clone, Copy, Debug)]
pub struct VocalsMarker;
impl LyrebirdInstrumentTag for VocalsMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::Vocals;
}

#[derive(Clone, Copy, Debug)]
pub struct BackupVocalsMarker;
impl LyrebirdInstrumentTag for BackupVocalsMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::BackupVocals;
}

#[derive(Clone, Copy, Debug)]
pub struct BassMarker;
impl LyrebirdInstrumentTag for BassMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::Bass;
}

#[derive(Clone, Copy, Debug)]
pub struct GuitarSoloMarker;
impl LyrebirdInstrumentTag for GuitarSoloMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::GuitarSolo;
}

#[derive(Clone, Copy, Debug)]
pub struct DrumsMarker;
impl LyrebirdInstrumentTag for DrumsMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::Drums;
}

macro_rules! lyrebird_prompt_flow {
    ($enabled:ident, $disabled:ident, $prompt:ident, $marker:ty, $focus:ty) => {
        #[derive(Flow)]
        pub struct $enabled(
            Step<SelectDspBranchFocused<$marker, $focus>>,
            Step<BuildOptimizationPromptFocused<$marker, $focus>>,
            Step<RequestPromptCandidatesFocused<$marker, $focus>>,
            Step<PreparePromptCandidatesFocused<$marker, $focus>>,
        );

        #[derive(Flow)]
        pub struct $disabled(Step<SkipInstrumentPromptFocused<$marker, $focus>>);

        #[derive(Flow)]
        #[jungle(focus = $focus)]
        pub struct $prompt(
            Conditional<
                FocusedCondition<InstrumentEnabledFocused<$marker, $focus>, $focus>,
                $enabled,
                $disabled,
            >,
            Step<FlattenEither<(), $focus>>,
        );
    };
}

lyrebird_prompt_flow!(
    RhythmGuitarPromptEnabled,
    RhythmGuitarPromptDisabled,
    RhythmGuitarPrompt,
    RhythmGuitarMarker,
    RhythmGuitarPromptState
);
lyrebird_prompt_flow!(
    VocalsPromptEnabled,
    VocalsPromptDisabled,
    VocalsPrompt,
    VocalsMarker,
    VocalsPromptState
);
lyrebird_prompt_flow!(
    BackupVocalsPromptEnabled,
    BackupVocalsPromptDisabled,
    BackupVocalsPrompt,
    BackupVocalsMarker,
    BackupVocalsPromptState
);
lyrebird_prompt_flow!(
    BassPromptEnabled,
    BassPromptDisabled,
    BassPrompt,
    BassMarker,
    BassPromptState
);
lyrebird_prompt_flow!(
    GuitarSoloPromptEnabled,
    GuitarSoloPromptDisabled,
    GuitarSoloPrompt,
    GuitarSoloMarker,
    GuitarSoloPromptState
);
lyrebird_prompt_flow!(
    DrumsPromptEnabled,
    DrumsPromptDisabled,
    DrumsPrompt,
    DrumsMarker,
    DrumsPromptState
);

#[derive(Flow)]
pub struct LyrebirdPromptLeft(Join<RhythmGuitarPrompt, VocalsPrompt>);

#[derive(Flow)]
pub struct LyrebirdPromptMid(Join<BackupVocalsPrompt, BassPrompt>);

#[derive(Flow)]
pub struct LyrebirdPromptRight(Join<GuitarSoloPrompt, DrumsPrompt>);

#[derive(Flow)]
pub struct LyrebirdPromptPairs(Join<LyrebirdPromptLeft, LyrebirdPromptMid>);

#[derive(Flow)]
pub struct LyrebirdPromptPhase(
    Join<LyrebirdPromptPairs, LyrebirdPromptRight>,
    Step<FlattenLyrebirdPromptPhase<LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdInstrumentSubmitEnabled<Marker: LyrebirdInstrumentTag + Send + Sync + 'static>(
    Step<SetCurrentInstrument<Marker>>,
    Step<GenerateIterationCandidateAudio<Marker>>,
    Step<GenerateIterationCandidateMels<Marker>>,
    Step<CompareIterationCandidateMels<Marker>>,
    Step<SubmitDspBranch<Marker>>,
);

#[derive(Flow)]
pub struct LyrebirdInstrumentSubmitDisabled<Marker: LyrebirdInstrumentTag + Send + Sync + 'static>(
    Step<SetCurrentInstrument<Marker>>,
    Step<SkipInstrumentSubmit<Marker>>,
);

#[derive(Flow)]
pub struct LyrebirdInstrumentSubmit<Marker: LyrebirdInstrumentTag + Send + Sync + 'static>(
    Conditional<
        InstrumentEnabled<Marker>,
        LyrebirdInstrumentSubmitEnabled<Marker>,
        LyrebirdInstrumentSubmitDisabled<Marker>,
    >,
    Step<FlattenEither<(), LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdIteration(
    Step<LogIterationTiming>,
    Step<BeginIteration>,
    LyrebirdPromptPhase,
    LyrebirdInstrumentSubmit<RhythmGuitarMarker>,
    LyrebirdInstrumentSubmit<VocalsMarker>,
    LyrebirdInstrumentSubmit<BackupVocalsMarker>,
    LyrebirdInstrumentSubmit<BassMarker>,
    LyrebirdInstrumentSubmit<GuitarSoloMarker>,
    LyrebirdInstrumentSubmit<DrumsMarker>,
);

#[derive(Flow)]
pub struct LyrebirdJourney(
    Step<SeedLyrebirdState>,
    While<LyrebirdLoopForever, LyrebirdIteration>,
);

pub struct Lyrebird;
#[jungle::animal(id = 0, generation = 0, observe)]
impl Animal for Lyrebird {
    type State = LyrebirdState;
    type Seed = LyrebirdSeed;
    type Flow = LyrebirdJourney;
}

impl Observe for Lyrebird {
    type Appearance = LyrebirdState;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.normalized_for_observation()
    }
}

#[derive(Animals)]
pub struct PulseCodePurgatoryAnimals(Lyrebird);

#[derive(Clone)]
struct TokensApiTarget {
    base_url: Url,
    client: reqwest::Client,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokensApiConfig {
    routes: BTreeMap<LyrebirdInstrument, Url>,
    fallback: Option<Url>,
}

impl From<Url> for TokensApiConfig {
    fn from(value: Url) -> Self {
        Self {
            routes: BTreeMap::new(),
            fallback: Some(value),
        }
    }
}

impl TokensApiConfig {
    fn parse(value: &str) -> Result<Self, PulseCodePurgatoryError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(PulseCodePurgatoryError::InvalidTokensApi {
                value: value.to_owned(),
                reason: "expected at least one endpoint".to_owned(),
            });
        }

        let mut routes = BTreeMap::new();
        let mut fallback = None;
        for entry in value.split(',').map(str::trim) {
            if entry.is_empty() {
                return Err(PulseCodePurgatoryError::InvalidTokensApi {
                    value: value.to_owned(),
                    reason: "empty entries are not allowed".to_owned(),
                });
            }

            let (instrument, raw_url) = match entry.split_once(':') {
                Some((candidate, remainder)) => {
                    match LyrebirdInstrument::parse_cli_selection(candidate) {
                        Ok(instrument) => (Some(instrument), remainder.trim()),
                        Err(_) => (None, entry),
                    }
                }
                None => (None, entry),
            };

            if raw_url.is_empty() {
                return Err(PulseCodePurgatoryError::InvalidTokensApi {
                    value: value.to_owned(),
                    reason: format!("missing endpoint for `{entry}`"),
                });
            }

            let tokens_url = parse_tokens_api_base_url(raw_url)?;
            match instrument {
                Some(instrument) => {
                    if routes.insert(instrument, tokens_url).is_some() {
                        return Err(PulseCodePurgatoryError::InvalidTokensApi {
                            value: value.to_owned(),
                            reason: format!(
                                "duplicate endpoint mapping for instrument `{}`",
                                instrument.slug()
                            ),
                        });
                    }
                }
                None => {
                    if fallback.replace(tokens_url).is_some() {
                        return Err(PulseCodePurgatoryError::InvalidTokensApi {
                            value: value.to_owned(),
                            reason: "multiple fallback endpoints are not allowed".to_owned(),
                        });
                    }
                }
            }
        }

        if fallback.is_none() && routes.len() != LyrebirdInstrument::ALL.len() {
            let missing = LyrebirdInstrument::ALL
                .into_iter()
                .filter(|instrument| !routes.contains_key(instrument))
                .map(|instrument| instrument.slug())
                .collect::<Vec<_>>()
                .join(",");
            return Err(PulseCodePurgatoryError::InvalidTokensApi {
                value: value.to_owned(),
                reason: format!(
                    "routing must either cover all instruments or include a fallback endpoint; missing `{missing}`"
                ),
            });
        }

        Ok(Self { routes, fallback })
    }

    fn unique_urls(&self) -> BTreeMap<String, Url> {
        self.routes
            .values()
            .chain(self.fallback.iter())
            .map(|url| (url.to_string(), url.clone()))
            .collect()
    }
}

#[derive(Clone)]
pub struct PulseCodePurgatory {
    db: Arc<redb::Database>,
    db_path: PathBuf,
    runtime_session_id: String,
    tokens_model: String,
    tokens_clients: BTreeMap<String, TokensApiTarget>,
    tokens_routes: BTreeMap<LyrebirdInstrument, String>,
    tokens_fallback_server: Option<String>,
    tools: Vec<Tool>,
    initial_dsp_codes: BTreeMap<LyrebirdInstrument, DspCode>,
    max_tree_depth: usize,
    instrument_parallelism: usize,
}

impl PulseCodePurgatory {
    pub fn new(
        tokens_api: impl Into<TokensApiConfig>,
        tokens_token: Option<String>,
        db_path: Option<PathBuf>,
    ) -> Result<Self, PulseCodePurgatoryError> {
        let tokens_api = tokens_api.into();
        let tokens_clients = tokens_api
            .unique_urls()
            .into_iter()
            .map(|(server, base_url)| {
                Ok((
                    server,
                    TokensApiTarget {
                        base_url,
                        client: Self::build_tokens_client(tokens_token.as_deref())?,
                    },
                ))
            })
            .collect::<Result<BTreeMap<_, _>, PulseCodePurgatoryError>>()?;
        let (db, db_path) = mcts::open_mcts_db(db_path)?;

        Ok(Self {
            db,
            db_path,
            runtime_session_id: Uuid::new_v4().to_string(),
            tokens_model: Self::tokens_model_from_env(),
            tokens_clients,
            tokens_routes: tokens_api
                .routes
                .iter()
                .map(|(instrument, url)| (*instrument, url.to_string()))
                .collect(),
            tokens_fallback_server: tokens_api.fallback.map(|url| url.to_string()),
            tools: Vec::new(),
            initial_dsp_codes: LyrebirdInstrument::ALL
                .into_iter()
                .map(|instrument| (instrument, DspCode::placeholder_initial()))
                .collect(),
            max_tree_depth: DEFAULT_TREE_DEPTH,
            instrument_parallelism: DEFAULT_INSTRUMENT_PARALLELISM,
        })
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_mcts_config(
        mut self,
        initial_dsp_codes: impl IntoIterator<Item = (LyrebirdInstrument, DspCode)>,
        max_tree_depth: usize,
    ) -> Self {
        self.initial_dsp_codes = initial_dsp_codes.into_iter().collect();
        self.max_tree_depth = max_tree_depth;
        self
    }

    pub fn with_instrument_parallelism(mut self, instrument_parallelism: usize) -> Self {
        self.instrument_parallelism = instrument_parallelism;
        self
    }

    pub fn with_tokens_model(mut self, tokens_model: impl Into<String>) -> Self {
        self.tokens_model = tokens_model.into();
        self
    }
}

impl Ecosystem for PulseCodePurgatory {
    const NAME: &'static str = "pulse-code-purgatory";
    type Animals = PulseCodePurgatoryAnimals;
}

#[derive(Debug, Error)]
pub enum PulseCodePurgatoryError {
    #[error("failed to construct tokens client: {0}")]
    Client(#[from] reqwest::Error),
    #[error("invalid tokens API mapping `{value}`: {reason}")]
    InvalidTokensApi { value: String, reason: String },
    #[error(
        "invalid OpenAI API base URL `{url}`: {reason}. Expected an HTTP(S) base URL like `https://api.openai.com/v1` or `http://localhost:11434/v1`, not a full endpoint such as `.../chat/completions`"
    )]
    InvalidTokensUrl { url: String, reason: String },
    #[error("invalid bearer token header: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("failed to read image content from {path}: {source}")]
    ReadImage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create database directory {path}: {source}")]
    CreateDbDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to restore dsp source {path}: {source}")]
    RestoreDspSource {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("home directory is unavailable")]
    HomeDirUnavailable,
    #[error("mcts protocol error: {0}")]
    MctsProtocol(String),
    #[error("mcts persistence error: {0}")]
    Persistence(String),
    #[error("failed to parse tool-call arguments: {0}")]
    ToolArguments(#[from] serde_json::Error),
    #[error("tokens API routing requires a fallback endpoint when prompt metadata is absent")]
    MissingTokensMeta,
    #[error(
        "no tokens API endpoint configured for instrument `{instrument}` and no fallback endpoint is available"
    )]
    MissingTokensRoute { instrument: String },
    #[error("internal tokens client missing for server `{server}`")]
    MissingTokensClient { server: String },
    #[error("failed to build initial lyrebird dsp baseline: {0}")]
    Bootstrap(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ImageDumpPanel {
    Spectrograms,
    Graph,
}

#[cfg(feature = "viewer")]
impl From<ImageDumpPanel> for ui::ImageDumpPanel {
    fn from(value: ImageDumpPanel) -> Self {
        match value {
            ImageDumpPanel::Spectrograms => Self::Spectrograms,
            ImageDumpPanel::Graph => Self::Graph,
        }
    }
}

#[derive(Debug, Parser)]
#[command(name = "lyrebird")]
struct Cli {
    #[arg(
        long = "tokens-url",
        visible_aliases = ["tokens-api", "tokens-endpoint", "tokens-server"],
        value_parser = parse_tokens_api_config,
        help = "OpenAI-compatible API base URL or per-instrument mapping, for example https://api.openai.com/v1 or sologuitar:http://localhost:4567,bass:http://localhost:6789,http://localhost:9876"
    )]
    tokens_api: TokensApiConfig,
    #[arg(long = "tokens-token")]
    tokens_token: Option<String>,
    #[arg(
        long = "tokens-model",
        help = "OpenAI-compatible model string for chat completions requests; defaults to LYREBIRD_TOKENS_MODEL or codemonkey-d-luffy"
    )]
    tokens_model: Option<String>,
    #[arg(long = "db-path")]
    db_path: Option<PathBuf>,
    #[arg(long = "jungle-redb-path")]
    jungle_redb_path: Option<PathBuf>,
    #[arg(long = "workers", default_value_t = DEFAULT_WORKERS, value_parser = parse_workers)]
    workers: usize,
    #[arg(
        long = "tree-depth",
        default_value_t = DEFAULT_TREE_DEPTH,
        value_parser = parse_tree_depth
    )]
    tree_depth: usize,
    #[arg(
        long = "instrument-parallelism",
        default_value_t = DEFAULT_INSTRUMENT_PARALLELISM,
        value_parser = parse_instrument_parallelism
    )]
    instrument_parallelism: usize,
    #[arg(
        long = "instruments",
        value_delimiter = ',',
        value_parser = LyrebirdInstrument::parse_cli_selection
    )]
    instruments: Option<Vec<LyrebirdInstrument>>,
    #[arg(
        long = "img-dump",
        help = "Capture the lyrebird UI to this PNG path and then exit"
    )]
    img_dump: Option<PathBuf>,
    #[arg(
        long = "img-dump-time-secs",
        requires = "img_dump",
        value_parser = parse_img_dump_time_secs,
        help = "Seconds to wait after the UI starts before capturing --img-dump"
    )]
    img_dump_time_secs: Option<f64>,
    #[arg(
        long = "img-dump-panel",
        requires = "img_dump",
        value_enum,
        help = "Restrict --img-dump to either the spectrogram panel or the graph panel"
    )]
    img_dump_panel: Option<ImageDumpPanel>,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    let workspace_root = std::env::current_dir()?;
    let output_root = default_lyrebird_root()?;
    let jungle_redb_path = cli
        .jungle_redb_path
        .unwrap_or_else(|| output_root.join("jungle.redb"));
    ensure_parent_dir_exists(&jungle_redb_path)?;
    let selected_instruments = normalize_instrument_selection(cli.instruments.as_deref());

    let instrument_seeds = build_instrument_seeds(&workspace_root, &output_root).await?;
    let ecosystem = PulseCodePurgatory::new(cli.tokens_api, cli.tokens_token, cli.db_path)?
        .with_tokens_model(
            cli.tokens_model
                .unwrap_or_else(PulseCodePurgatory::tokens_model_from_env),
        )
        .with_tools(
            LyrebirdInstrument::ALL
                .into_iter()
                .map(build_replace_tool)
                .collect(),
        )
        .with_instrument_parallelism(cli.instrument_parallelism)
        .with_mcts_config(
            instrument_seeds
                .iter()
                .cloned()
                .map(|seed| (seed.instrument, seed.initial_dsp_code)),
            cli.tree_depth,
        );
    info!(
        workers = cli.workers,
        tree_depth = cli.tree_depth,
        instrument_parallelism = cli.instrument_parallelism,
        tokens_model = %ecosystem.tokens_model,
        instruments = %selected_instruments
            .iter()
            .map(|instrument| instrument.slug())
            .collect::<Vec<_>>()
            .join(","),
        jungle_redb_path = %jungle_redb_path.display(),
        mcts_redb_path = %ecosystem.db_path.display(),
        "starting lyrebird runtime"
    );

    let backend = Server::builder()
        .redb_path(&jungle_redb_path)
        .build()
        .await?;
    let client = FusedClient::builder()
        .namespace(PulseCodePurgatory::NAME)
        .backend(backend)
        .build()
        .await?;

    let mut worker_handles = Vec::with_capacity(cli.workers);
    for worker_index in 0..cli.workers {
        let ecosystem = ecosystem.clone();
        let worker_client = client.clone();
        info!(worker_index, "spawning lyrebird worker");
        worker_handles.push(tokio::spawn(async move {
            let worker = JungleWorker::new(ecosystem, worker_client);
            if let Err(err) = worker.spawn().await {
                warn!(worker_index, error = %err, "lyrebird worker exited");
            }
        }));
    }

    let seed = build_seed(
        &output_root,
        &instrument_seeds,
        cli.instrument_parallelism,
        &selected_instruments,
    );
    let journey_id = ensure_lyrebird_running(&client, &seed).await?;

    info!(
        %journey_id,
        jungle_redb_path = %jungle_redb_path.display(),
        mcts_redb_path = %ecosystem.db_path.display(),
        "lyrebird active"
    );

    #[cfg(feature = "viewer")]
    {
        let img_dump = cli.img_dump.map(|output_path| {
            ui::ImageDumpConfig::new(
                output_path,
                Duration::from_secs_f64(cli.img_dump_time_secs.unwrap_or(0.0)),
                cli.img_dump_panel.map(ui::ImageDumpPanel::from),
            )
        });
        tokio::task::block_in_place(|| ui::run_ui(client.clone(), journey_id, img_dump))?;
    }

    #[cfg(not(feature = "viewer"))]
    if cli.img_dump.is_some() {
        warn!("--img-dump was ignored because lyrebird was built without the `viewer` feature");
    }
    #[cfg(not(feature = "viewer"))]
    tokio::signal::ctrl_c().await?;
    #[cfg(not(feature = "viewer"))]
    info!("received ctrl-c; shutting down lyrebird workers");
    #[cfg(feature = "viewer")]
    info!("lyrebird viewer closed; shutting down lyrebird workers");

    for worker_handle in worker_handles.drain(..) {
        worker_handle.abort();
        let _ = worker_handle.await;
    }

    restore_instrument_sources(&instrument_seeds)?;

    Ok(())
}

async fn build_instrument_seeds(
    workspace_root: &Path,
    output_root: &Path,
) -> Result<Vec<LyrebirdInstrumentSeed>, PulseCodePurgatoryError> {
    let mut seeds = Vec::with_capacity(LyrebirdInstrument::ALL.len());
    for instrument in LyrebirdInstrument::ALL {
        let dsp_source_path = workspace_root.join(instrument.relative_dsp_path());
        let target_sample_path = workspace_root.join(instrument.relative_target_sample_path());
        let target_spectrogram_path = output_root
            .join("target-mels")
            .join(format!("{}.png", instrument.output_stem()));
        let target_audio_metrics = effect::analyze_audio_file(
            &target_sample_path.display().to_string(),
        )
        .map_err(|err| {
            PulseCodePurgatoryError::Bootstrap(format!(
                "failed to analyze target sample for {}: {err}",
                instrument.slug()
            ))
        })?;
        effect::generate_mel_spectrogram(
            &target_sample_path.display().to_string(),
            &target_spectrogram_path.display().to_string(),
        )
        .map_err(|err| {
            PulseCodePurgatoryError::Bootstrap(format!(
                "failed to generate target mel spectrogram for {}: {err}",
                instrument.slug()
            ))
        })?;
        let initial_dsp_code = effect::capture_current_dsp_code_snapshot(
            "initial",
            output_root,
            instrument,
            &target_spectrogram_path,
            &target_audio_metrics,
            &dsp_source_path,
        )
        .await
        .map_err(PulseCodePurgatoryError::Bootstrap)?;
        seeds.push(LyrebirdInstrumentSeed {
            instrument,
            disabled: false,
            target_sample_path: target_sample_path.display().to_string(),
            target_audio_metrics,
            target_spectrogram_path: target_spectrogram_path.display().to_string(),
            dsp_source_path: dsp_source_path.display().to_string(),
            initial_dsp_code,
        });
    }
    Ok(seeds)
}

fn build_seed(
    output_root: &Path,
    instruments: &[LyrebirdInstrumentSeed],
    instrument_parallelism: usize,
    selected_instruments: &[LyrebirdInstrument],
) -> LyrebirdSeed {
    let selected_lookup = selected_instruments
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();

    LyrebirdSeed {
        output_root: output_root.display().to_string(),
        instruments: instruments
            .iter()
            .cloned()
            .map(|mut instrument| {
                instrument.disabled = !selected_lookup.contains(&instrument.instrument);
                instrument
            })
            .collect(),
        instrument_parallelism,
    }
}

fn restore_instrument_sources(
    instruments: &[LyrebirdInstrumentSeed],
) -> Result<(), PulseCodePurgatoryError> {
    for instrument in instruments {
        let path = PathBuf::from(&instrument.dsp_source_path);
        std::fs::write(&path, &instrument.initial_dsp_code.source).map_err(|source| {
            PulseCodePurgatoryError::RestoreDspSource {
                path: path.clone(),
                source,
            }
        })?;
    }
    Ok(())
}

async fn ensure_lyrebird_running(
    client: &FusedClient,
    seed: &LyrebirdSeed,
) -> Result<Uuid, jungle_sdk::ExecutorError> {
    let journeys = client
        .list_journeys(PulseCodePurgatory::NAME.to_owned())
        .await?;
    let lyrebird_animal_id = <<Lyrebird as Animal>::Id as AnimalIdValue>::U32;

    for existing in journeys
        .into_iter()
        .filter(|record| record.animal_id == lyrebird_animal_id && !is_terminal(record.status))
    {
        let appearance = match client.animal_appearance(existing.journey_id).await {
            Ok(appearance) => appearance,
            Err(err) => {
                warn!(
                    journey_id = %existing.journey_id,
                    error = %err,
                    "failed to inspect existing lyrebird journey appearance; spawning a new journey instead"
                );
                continue;
            }
        };
        let Some(appearance) = appearance else {
            warn!(
                journey_id = %existing.journey_id,
                "existing lyrebird journey has no appearance yet; spawning a new journey instead"
            );
            continue;
        };
        let state = match postcard::from_bytes::<LyrebirdState>(&appearance) {
            Ok(state) => state,
            Err(err) => {
                warn!(
                    journey_id = %existing.journey_id,
                    error = %err,
                    "failed to decode existing lyrebird journey appearance; spawning a new journey instead"
                );
                continue;
            }
        };
        if state.has_all_instrument_states() {
            if !state.matches_seed_instrument_selection(seed) {
                warn!(
                    journey_id = %existing.journey_id,
                    "existing lyrebird journey instrument selection differs from requested configuration; spawning a new journey instead"
                );
                continue;
            }
            info!(
                journey_id = %existing.journey_id,
                "reusing existing lyrebird journey"
            );
            return Ok(existing.journey_id);
        }
        warn!(
            journey_id = %existing.journey_id,
            instrument_count = LyrebirdInstrument::ALL.len(),
            "existing lyrebird journey is missing instrument state; spawning a new journey instead"
        );
    }

    let journey_id = client.spawn::<Lyrebird>(seed).await?.journey_id;
    info!(%journey_id, "spawned new lyrebird journey");
    Ok(journey_id)
}

fn is_terminal(status: JourneyStatus) -> bool {
    matches!(status, JourneyStatus::Completed | JourneyStatus::Dead)
}

fn default_lyrebird_root() -> Result<PathBuf, PulseCodePurgatoryError> {
    let base_dirs = BaseDirs::new().ok_or(PulseCodePurgatoryError::HomeDirUnavailable)?;
    Ok(base_dirs.home_dir().join(".jungle").join("lyrebird"))
}

fn ensure_parent_dir_exists(path: &Path) -> Result<(), PulseCodePurgatoryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| PulseCodePurgatoryError::CreateDbDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

fn validate_openai_api_base_url(tokens_url: &Url) -> Result<(), PulseCodePurgatoryError> {
    match tokens_url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(PulseCodePurgatoryError::InvalidTokensUrl {
                url: tokens_url.to_string(),
                reason: format!("unsupported URL scheme `{scheme}`"),
            });
        }
    }

    if tokens_url.query().is_some() {
        return Err(PulseCodePurgatoryError::InvalidTokensUrl {
            url: tokens_url.to_string(),
            reason: "query parameters are not allowed".to_owned(),
        });
    }

    if tokens_url.fragment().is_some() {
        return Err(PulseCodePurgatoryError::InvalidTokensUrl {
            url: tokens_url.to_string(),
            reason: "fragments are not allowed".to_owned(),
        });
    }

    let path_segments: Vec<_> = tokens_url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if path_segments.ends_with(&["chat", "completions"]) {
        return Err(PulseCodePurgatoryError::InvalidTokensUrl {
            url: tokens_url.to_string(),
            reason: "received the chat completions endpoint instead of the API base URL".to_owned(),
        });
    }

    Ok(())
}

fn parse_tokens_api_base_url(value: &str) -> Result<Url, PulseCodePurgatoryError> {
    let value = value.trim();
    let candidate = if value.contains("://") {
        value.to_owned()
    } else {
        format!("http://{value}")
    };
    let tokens_url =
        Url::parse(&candidate).map_err(|_| PulseCodePurgatoryError::InvalidTokensUrl {
            url: value.to_owned(),
            reason: "failed to parse as an HTTP(S) URL or host[:port] address".to_owned(),
        })?;
    validate_openai_api_base_url(&tokens_url)?;
    Ok(tokens_url)
}

fn parse_tokens_api_config(value: &str) -> Result<TokensApiConfig, String> {
    TokensApiConfig::parse(value).map_err(|err| err.to_string())
}

fn parse_workers(value: &str) -> Result<usize, String> {
    let workers = value
        .parse::<usize>()
        .map_err(|_| format!("invalid workers argument: {value}"))?;
    if workers == 0 {
        return Err("workers must be at least 1".to_owned());
    }
    Ok(workers)
}

fn parse_tree_depth(value: &str) -> Result<usize, String> {
    let tree_depth = value
        .parse::<usize>()
        .map_err(|_| format!("invalid tree depth argument: {value}"))?;
    if tree_depth == 0 {
        return Err("tree depth must be at least 1".to_owned());
    }
    Ok(tree_depth)
}

fn parse_instrument_parallelism(value: &str) -> Result<usize, String> {
    let instrument_parallelism = value
        .parse::<usize>()
        .map_err(|_| format!("invalid instrument parallelism argument: {value}"))?;
    if instrument_parallelism == 0 {
        return Err("instrument parallelism must be at least 1".to_owned());
    }
    Ok(instrument_parallelism)
}

fn parse_img_dump_time_secs(value: &str) -> Result<f64, String> {
    let secs = value
        .parse::<f64>()
        .map_err(|_| format!("invalid image dump time argument: {value}"))?;
    if !secs.is_finite() || secs < 0.0 {
        return Err("image dump time must be a finite value greater than or equal to 0".to_owned());
    }
    Ok(secs)
}

fn normalize_instrument_selection(
    selected: Option<&[LyrebirdInstrument]>,
) -> Vec<LyrebirdInstrument> {
    LyrebirdInstrument::ALL
        .into_iter()
        .filter(|instrument| selected.is_none_or(|selected| selected.contains(instrument)))
        .collect()
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
    debug!("lyrebird tracing initialized");
}

pub(crate) fn build_replace_tool(instrument: LyrebirdInstrument) -> Tool {
    Tool {
        name: instrument.tool_name().to_owned(),
        description: format!(
            "Apply one small search/replace patch to `{}` and keep the file compiling.",
            instrument.relative_dsp_path()
        ),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "search": {
                    "type": "string",
                    "description": format!(
                        "Exact text to replace in {}. It must match the current file content exactly once.",
                        Path::new(instrument.relative_dsp_path())
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("the current DSP file")
                    )
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement text for the matched search block."
                },
                "note": {
                    "type": "string",
                    "maxLength": 100,
                    "description": "Brief purpose of the change, 100 characters maximum."
                }
            },
            "required": ["search", "replacement", "note"],
            "additionalProperties": false
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::Duration;

    #[test]
    fn parses_non_negative_img_dump_time_secs() {
        assert_eq!(parse_img_dump_time_secs("0").unwrap(), 0.0);
        assert_eq!(parse_img_dump_time_secs("30.5").unwrap(), 30.5);
    }

    #[test]
    fn rejects_negative_img_dump_time_secs() {
        assert!(parse_img_dump_time_secs("-1").is_err());
    }

    struct PromptJoinConcurrentRuntime {
        barrier: Mutex<Arc<tokio::sync::Barrier>>,
        active: AtomicUsize,
        max_active: AtomicUsize,
    }

    impl PromptJoinConcurrentRuntime {
        fn reset(&self, parties: usize) {
            self.active.store(0, Ordering::SeqCst);
            self.max_active.store(0, Ordering::SeqCst);
            *self
                .barrier
                .lock()
                .expect("prompt join barrier lock should not be poisoned") =
                Arc::new(tokio::sync::Barrier::new(parties));
        }
    }

    fn prompt_join_concurrent_runtime() -> Arc<PromptJoinConcurrentRuntime> {
        static RUNTIME: OnceLock<Arc<PromptJoinConcurrentRuntime>> = OnceLock::new();
        RUNTIME
            .get_or_init(|| {
                Arc::new(PromptJoinConcurrentRuntime {
                    barrier: Mutex::new(Arc::new(tokio::sync::Barrier::new(2))),
                    active: AtomicUsize::new(0),
                    max_active: AtomicUsize::new(0),
                })
            })
            .clone()
    }

    fn prompt_join_test_lock() -> &'static tokio::sync::Mutex<()> {
        static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    struct PromptJoinConcurrentGuard(Arc<PromptJoinConcurrentRuntime>);

    impl Drop for PromptJoinConcurrentGuard {
        fn drop(&mut self) {
            self.0.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub struct PromptJoinConcurrentEffect;
    #[jungle::effect(id = 142)]
    impl<J> Effect<J> for PromptJoinConcurrentEffect {
        type In = i32;
        type Out = i32;
        type Err = ();

        fn effect(
            _jungle: &J,
            input: Self::In,
        ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
            async move {
                let runtime = prompt_join_concurrent_runtime();
                let active = runtime.active.fetch_add(1, Ordering::SeqCst) + 1;
                runtime.max_active.fetch_max(active, Ordering::SeqCst);
                let _guard = PromptJoinConcurrentGuard(runtime.clone());
                let barrier = runtime
                    .barrier
                    .lock()
                    .expect("prompt join barrier lock should not be poisoned")
                    .clone();
                barrier.wait().await;
                Ok(input)
            }
        }
    }

    macro_rules! concurrent_prompt_flow {
        ($spec:ident, $flow:ident, $state:ty, $scale:expr) => {
            struct $spec;
            #[jungle::action]
            impl Action for $spec {
                type Effect = PromptJoinConcurrentEffect;
                type Input = i32;
                type Output = i32;

                fn emit(state: &$state, input: Self::Input) -> i32 {
                    state.state.prompt_attempt as i32 + input * $scale
                }

                fn absorb(
                    state: &mut $state,
                    output: EffectCompletion<Self::Effect>,
                ) -> Result<Self::Output, Failure> {
                    let out = output.map_err(|_err| {
                        Failure::from(concat!(
                            stringify!($spec),
                            " concurrent prompt should succeed"
                        ))
                    })?;
                    state.state.prompt_attempt = out as u32;
                    Ok(out)
                }
            }

            #[derive(Flow)]
            #[jungle(focus = $state)]
            struct $flow(Step<$spec>);
        };
    }

    concurrent_prompt_flow!(
        RhythmGuitarConcurrentPromptSpec,
        RhythmGuitarConcurrentPromptFlow,
        RhythmGuitarPromptState,
        1
    );
    concurrent_prompt_flow!(
        VocalsConcurrentPromptSpec,
        VocalsConcurrentPromptFlow,
        VocalsPromptState,
        10
    );
    #[derive(Flow)]
    struct ConcurrentLyrebirdPromptJoin(
        Join<RhythmGuitarConcurrentPromptFlow, VocalsConcurrentPromptFlow>,
    );

    struct ConcurrentLyrebirdPromptAnimal;
    #[jungle::animal(id = 90, generation = 0)]
    impl Animal for ConcurrentLyrebirdPromptAnimal {
        type State = LyrebirdState;
        type Seed = i32;
        type Flow = ConcurrentLyrebirdPromptJoin;
    }

    pub struct PromptJoinLiveHistoryDelayEffect;
    #[jungle::effect(id = 143)]
    impl<J> Effect<J> for PromptJoinLiveHistoryDelayEffect {
        type In = i32;
        type Out = i32;
        type Err = ();

        fn effect(
            _jungle: &J,
            input: Self::In,
        ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
            async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(input)
            }
        }
    }

    struct HiddenJoinUntakenNoop1;
    #[jungle::action]
    impl Action for HiddenJoinUntakenNoop1 {
        type Effect = Noop;
        type Input = ();
        type Output = ();

        fn emit(_state: &i32, _input: Self::Input) {}

        fn absorb(
            _state: &mut i32,
            _output: EffectCompletion<Self::Effect>,
        ) -> Result<Self::Output, Failure> {
            Ok(())
        }
    }

    struct HiddenJoinUntakenNoop2;
    #[jungle::action]
    impl Action for HiddenJoinUntakenNoop2 {
        type Effect = Noop;
        type Input = ();
        type Output = ();

        fn emit(_state: &i32, _input: Self::Input) {}

        fn absorb(
            _state: &mut i32,
            _output: EffectCompletion<Self::Effect>,
        ) -> Result<Self::Output, Failure> {
            Ok(())
        }
    }

    struct HiddenJoinTakenNoop;
    #[jungle::action]
    impl Action for HiddenJoinTakenNoop {
        type Effect = Noop;
        type Input = ();
        type Output = ();

        fn emit(_state: &i32, _input: Self::Input) {}

        fn absorb(
            _state: &mut i32,
            _output: EffectCompletion<Self::Effect>,
        ) -> Result<Self::Output, Failure> {
            Ok(())
        }
    }

    struct HiddenJoinDelayedPrompt;
    #[jungle::action]
    impl Action for HiddenJoinDelayedPrompt {
        type Effect = PromptJoinLiveHistoryDelayEffect;
        type Input = ();
        type Output = i32;

        fn emit(state: &i32, _input: Self::Input) -> i32 {
            *state
        }

        fn absorb(
            state: &mut i32,
            output: EffectCompletion<Self::Effect>,
        ) -> Result<Self::Output, Failure> {
            let out = output
                .map_err(|_err| Failure::from("hidden join delayed prompt should succeed"))?;
            *state = out;
            Ok(out)
        }
    }

    struct HiddenJoinAlwaysFalse;
    impl Predicate<(i32, ())> for HiddenJoinAlwaysFalse {
        fn eval((_state, _): &(i32, ())) -> bool {
            false
        }
    }

    #[derive(Flow)]
    struct HiddenJoinUntakenNoopFlow(Step<HiddenJoinUntakenNoop1>, Step<HiddenJoinUntakenNoop2>);

    #[derive(Flow)]
    struct HiddenJoinTakenNoopFlow(Step<HiddenJoinTakenNoop>);

    #[derive(Flow)]
    struct HiddenJoinConditionalNoopFlow(
        Conditional<HiddenJoinAlwaysFalse, HiddenJoinUntakenNoopFlow, HiddenJoinTakenNoopFlow>,
        Step<FlattenEither<(), i32>>,
    );

    #[derive(Flow)]
    struct HiddenJoinConditionalNoopJoin(
        Join<HiddenJoinConditionalNoopFlow, Step<HiddenJoinDelayedPrompt>>,
    );

    struct HiddenJoinConditionalNoopAnimal;
    #[jungle::animal(id = 91, generation = 0)]
    impl Animal for HiddenJoinConditionalNoopAnimal {
        type State = i32;
        type Seed = ();
        type Flow = HiddenJoinConditionalNoopJoin;
    }

    #[derive(Animals)]
    struct HiddenJoinConditionalNoopZoo(HiddenJoinConditionalNoopAnimal);

    struct HiddenJoinConditionalNoopEcosystem;
    impl Ecosystem for HiddenJoinConditionalNoopEcosystem {
        const NAME: &'static str = "lyrebird-hidden-join-noop-zoo";
        type Animals = HiddenJoinConditionalNoopZoo;
    }

    macro_rules! nested_prompt_branch {
        (
            $pred:ident,
            $selected:expr,
            $select_effect:ident,
            $select_effect_id:literal,
            $optimize_effect:ident,
            $optimize_effect_id:literal,
            $skip_effect:ident,
            $skip_effect_id:literal,
            $select:ident,
            $optimize:ident,
            $skip:ident,
            $enabled:ident,
            $disabled:ident,
            $flow:ident
        ) => {
            struct $pred;
            impl Predicate<(i32, ())> for $pred {
                fn eval((_state, _): &(i32, ())) -> bool {
                    $selected
                }
            }

            pub struct $select_effect;
            #[jungle::effect(id = $select_effect_id)]
            impl<J> Effect<J> for $select_effect {
                type In = ();
                type Out = ();
                type Err = ();

                fn effect(
                    _jungle: &J,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    async { Ok(()) }
                }
            }

            pub struct $optimize_effect;
            #[jungle::effect(id = $optimize_effect_id)]
            impl<J> Effect<J> for $optimize_effect {
                type In = ();
                type Out = ();
                type Err = ();

                fn effect(
                    _jungle: &J,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    async { Ok(()) }
                }
            }

            pub struct $skip_effect;
            #[jungle::effect(id = $skip_effect_id)]
            impl<J> Effect<J> for $skip_effect {
                type In = ();
                type Out = ();
                type Err = ();

                fn effect(
                    _jungle: &J,
                    _input: Self::In,
                ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
                    async { Ok(()) }
                }
            }

            struct $select;
            #[jungle::action]
            impl Action for $select {
                type Effect = $select_effect;
                type Input = ();
                type Output = ();

                fn emit(_state: &i32, _input: Self::Input) {}

                fn absorb(
                    _state: &mut i32,
                    _output: EffectCompletion<Self::Effect>,
                ) -> Result<Self::Output, Failure> {
                    Ok(())
                }
            }

            struct $optimize;
            #[jungle::action]
            impl Action for $optimize {
                type Effect = $optimize_effect;
                type Input = ();
                type Output = ();

                fn emit(_state: &i32, _input: Self::Input) {}

                fn absorb(
                    _state: &mut i32,
                    _output: EffectCompletion<Self::Effect>,
                ) -> Result<Self::Output, Failure> {
                    Ok(())
                }
            }

            struct $skip;
            #[jungle::action]
            impl Action for $skip {
                type Effect = $skip_effect;
                type Input = ();
                type Output = ();

                fn emit(_state: &i32, _input: Self::Input) {}

                fn absorb(
                    _state: &mut i32,
                    _output: EffectCompletion<Self::Effect>,
                ) -> Result<Self::Output, Failure> {
                    Ok(())
                }
            }

            #[derive(Flow)]
            struct $enabled(Step<$select>, Step<$optimize>);

            #[derive(Flow)]
            struct $disabled(Step<$skip>);

            #[derive(Flow)]
            struct $flow(
                Conditional<$pred, $enabled, $disabled>,
                Step<FlattenEither<(), i32>>,
            );
        };
    }

    nested_prompt_branch!(
        NestedBranch1Pred,
        false,
        Branch1SelectEffect,
        200,
        Branch1OptimizeEffect,
        201,
        Branch1SkipEffect,
        202,
        Branch1Select,
        Branch1Optimize,
        Branch1Skip,
        NestedBranch1Enabled,
        NestedBranch1Disabled,
        NestedBranch1Flow
    );
    nested_prompt_branch!(
        NestedBranch2Pred,
        true,
        Branch2SelectEffect,
        203,
        Branch2OptimizeEffect,
        204,
        Branch2SkipEffect,
        205,
        Branch2Select,
        Branch2Optimize,
        Branch2Skip,
        NestedBranch2Enabled,
        NestedBranch2Disabled,
        NestedBranch2Flow
    );
    nested_prompt_branch!(
        NestedBranch3Pred,
        false,
        Branch3SelectEffect,
        206,
        Branch3OptimizeEffect,
        207,
        Branch3SkipEffect,
        208,
        Branch3Select,
        Branch3Optimize,
        Branch3Skip,
        NestedBranch3Enabled,
        NestedBranch3Disabled,
        NestedBranch3Flow
    );
    nested_prompt_branch!(
        NestedBranch4Pred,
        true,
        Branch4SelectEffect,
        209,
        Branch4OptimizeEffect,
        210,
        Branch4SkipEffect,
        211,
        Branch4Select,
        Branch4Optimize,
        Branch4Skip,
        NestedBranch4Enabled,
        NestedBranch4Disabled,
        NestedBranch4Flow
    );
    nested_prompt_branch!(
        NestedBranch5Pred,
        false,
        Branch5SelectEffect,
        212,
        Branch5OptimizeEffect,
        213,
        Branch5SkipEffect,
        214,
        Branch5Select,
        Branch5Optimize,
        Branch5Skip,
        NestedBranch5Enabled,
        NestedBranch5Disabled,
        NestedBranch5Flow
    );

    #[derive(Flow)]
    struct NestedPromptLeft(Join<NestedBranch1Flow, NestedBranch2Flow>);

    #[derive(Flow)]
    struct NestedPromptRightPair(Join<NestedBranch3Flow, NestedBranch4Flow>);

    #[derive(Flow)]
    struct NestedPromptRight(Join<NestedPromptRightPair, NestedBranch5Flow>);

    #[derive(Flow)]
    struct NestedFiveWayPromptFlow(Join<NestedPromptLeft, NestedPromptRight>);

    struct NestedFiveWayPromptAnimal;
    #[jungle::animal(id = 92, generation = 0)]
    impl Animal for NestedFiveWayPromptAnimal {
        type State = i32;
        type Seed = ();
        type Flow = NestedFiveWayPromptFlow;
    }

    #[test]
    fn accepts_openai_compatible_api_base_url() {
        let url = Url::parse("http://localhost:11434/v1").unwrap();
        validate_openai_api_base_url(&url).unwrap();
    }

    #[test]
    fn rejects_chat_completions_endpoint_as_tokens_url() {
        let url = Url::parse("https://api.openai.com/v1/chat/completions").unwrap();
        let err = validate_openai_api_base_url(&url).unwrap_err();

        assert!(matches!(
            err,
            PulseCodePurgatoryError::InvalidTokensUrl { .. }
        ));
        assert!(err.to_string().contains("chat completions endpoint"));
    }

    #[test]
    fn parses_tokens_api_mapping_with_fallback() {
        let config =
            TokensApiConfig::parse("sologuitar:localhost:4567,bass:localhost:6789,localhost:9876")
                .unwrap();

        assert_eq!(
            config.routes.get(&LyrebirdInstrument::GuitarSolo),
            Some(&Url::parse("http://localhost:4567").unwrap())
        );
        assert_eq!(
            config.routes.get(&LyrebirdInstrument::Bass),
            Some(&Url::parse("http://localhost:6789").unwrap())
        );
        assert_eq!(
            config.fallback,
            Some(Url::parse("http://localhost:9876").unwrap())
        );
    }

    #[test]
    fn tokens_api_mapping_requires_full_coverage_without_fallback() {
        let err = TokensApiConfig::parse("bass:localhost:6789").unwrap_err();

        assert!(matches!(
            err,
            PulseCodePurgatoryError::InvalidTokensApi { .. }
        ));
        assert!(err.to_string().contains("cover all instruments"));
    }

    #[test]
    fn tokens_api_mapping_accepts_full_per_instrument_coverage_without_fallback() {
        TokensApiConfig::parse(
            "introguitar:localhost:4561,vocals:localhost:4562,backupvocals:localhost:4563,bass:localhost:4564,sologuitar:localhost:4565,drums:localhost:4566",
        )
        .unwrap();
    }

    #[test]
    fn lyrebird_instrument_metadata_covers_all_target_samples() {
        let instruments = LyrebirdInstrument::ALL;

        assert_eq!(instruments.len(), 6);
        assert_eq!(
            instruments
                .iter()
                .map(|instrument| instrument.relative_target_sample_path())
                .collect::<Vec<_>>(),
            vec![
                "jungle-examples/examples/lyrebird/assets/guitar_intro_4s.wav",
                "jungle-examples/examples/lyrebird/assets/vocals_4s.wav",
                "jungle-examples/examples/lyrebird/assets/backup_vocals_4s.wav",
                "jungle-examples/examples/lyrebird/assets/bass_4s.wav",
                "jungle-examples/examples/lyrebird/assets/guitar_solo_4s.wav",
                "jungle-examples/examples/lyrebird/assets/drums_4s.wav",
            ]
        );
    }

    #[test]
    fn begin_iteration_preserves_previous_most_recent_outputs() {
        let mut state = LyrebirdInstrumentState {
            instrument: LyrebirdInstrument::Bass,
            latest_generated_code: Some(DspCode {
                iteration_id: "00000007".to_owned(),
                source: "fn bass() {}".to_owned(),
                sample_path: "/tmp/old.wav".to_owned(),
                spectrogram_path: "/tmp/old.png".to_owned(),
                mel_similarity: Some(0.75),
                score: Some(0.8),
                audio_metrics: None,
                audio_metric_errors: None,
            }),
            latest_rendered_code: Some(DspCode {
                iteration_id: "00000007".to_owned(),
                source: "fn bass() {}".to_owned(),
                sample_path: "/tmp/old.wav".to_owned(),
                spectrogram_path: "/tmp/old.png".to_owned(),
                mel_similarity: Some(0.75),
                score: Some(0.8),
                audio_metrics: None,
                audio_metric_errors: None,
            }),
            latest_generated_sample_path: Some("/tmp/old.wav".to_owned()),
            latest_generated_spectrogram_path: Some("/tmp/old.png".to_owned()),
            latest_generated_similarity: Some(0.8),
            compile_ready: true,
            prompt_attempt: 3,
            pending_prompt: Some(Prompt {
                messages: vec![crate::tokens::Message {
                    role: "system".to_owned(),
                    contents: vec![crate::tokens::Content::Text("pending".to_owned())],
                }],
                tools: Vec::new(),
            }),
            pending_prompt_candidates: vec![crate::effect::PromptCandidateResponse {
                candidate_index: 0,
                tool_calls: None,
                retry_reason: Some("retry".to_owned()),
            }],
            skipped_this_iteration: true,
            last_retry_reason: Some("oops".to_owned()),
            last_similarity: 0.8,
            ..LyrebirdInstrumentState::default()
        };

        state.begin_iteration("/tmp/lyrebird", "00000008", 3);

        assert_eq!(
            state.latest_generated_sample_path.as_deref(),
            Some("/tmp/old.wav")
        );
        assert_eq!(
            state.latest_generated_spectrogram_path.as_deref(),
            Some("/tmp/old.png")
        );
        assert_eq!(state.latest_generated_similarity, Some(0.8));
        assert_eq!(
            state
                .latest_generated_code
                .as_ref()
                .map(|code| code.iteration_id.as_str()),
            Some("00000007")
        );
        assert_eq!(
            state
                .latest_rendered_code
                .as_ref()
                .map(|code| code.iteration_id.as_str()),
            Some("00000007")
        );
        assert!(!state.compile_ready);
        assert_eq!(state.prompt_attempt, 0);
        assert_eq!(state.pending_prompt, None);
        assert!(state.pending_prompt_candidates.is_empty());
        assert!(!state.skipped_this_iteration);
        assert_eq!(state.last_retry_reason, None);
        assert_eq!(state.last_similarity, 0.0);
        assert!(state.sample_path.ends_with("00000008/bass_4s.wav"));
        assert!(state.spectrogram_path.ends_with("00000008/bass_4s.png"));
    }

    #[test]
    fn observe_normalizes_missing_instrument_states() {
        let state = LyrebirdState::default();

        let appearance = <Lyrebird as Observe>::observe(&state);

        for instrument in LyrebirdInstrument::ALL {
            assert_eq!(
                appearance.instrument_state(instrument).instrument,
                instrument
            );
        }
        assert_eq!(
            appearance.current_instrument,
            LyrebirdInstrument::RhythmGuitar
        );
        for instrument in LyrebirdInstrument::ALL {
            assert!(!appearance.instrument_state(instrument).disabled);
        }
    }

    #[test]
    fn parse_instrument_selection_accepts_aliases_and_canonical_names() {
        assert_eq!(
            LyrebirdInstrument::parse_cli_selection("introguitar").unwrap(),
            LyrebirdInstrument::RhythmGuitar
        );
        assert_eq!(
            LyrebirdInstrument::parse_cli_selection("backup-vocals").unwrap(),
            LyrebirdInstrument::BackupVocals
        );
        assert_eq!(
            LyrebirdInstrument::parse_cli_selection("sologuitar").unwrap(),
            LyrebirdInstrument::GuitarSolo
        );
        assert_eq!(
            LyrebirdInstrument::parse_cli_selection("drums").unwrap(),
            LyrebirdInstrument::Drums
        );
    }

    #[test]
    fn parse_instrument_selection_rejects_unknown_values() {
        let err = LyrebirdInstrument::parse_cli_selection("kazoo").unwrap_err();

        assert!(err.contains("invalid instrument argument"));
    }

    #[test]
    fn build_seed_disables_unselected_instruments() {
        let seeds = LyrebirdInstrument::ALL
            .into_iter()
            .map(|instrument| LyrebirdInstrumentSeed {
                instrument,
                disabled: false,
                target_sample_path: format!("/tmp/{}.wav", instrument.output_stem()),
                target_audio_metrics: LyrebirdAudioMetrics::default(),
                target_spectrogram_path: format!("/tmp/{}.png", instrument.output_stem()),
                dsp_source_path: format!("/tmp/{}.rs", instrument.output_stem()),
                initial_dsp_code: DspCode::placeholder_initial(),
            })
            .collect::<Vec<_>>();

        let seed = build_seed(
            Path::new("/tmp/lyrebird"),
            &seeds,
            DEFAULT_INSTRUMENT_PARALLELISM,
            &[LyrebirdInstrument::Vocals, LyrebirdInstrument::GuitarSolo],
        );

        let disabled_by_instrument = seed
            .instruments
            .iter()
            .map(|instrument| (instrument.instrument, instrument.disabled))
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::Vocals),
            Some(&false)
        );
        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::GuitarSolo),
            Some(&false)
        );
        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::RhythmGuitar),
            Some(&true)
        );
        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::BackupVocals),
            Some(&true)
        );
        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::Bass),
            Some(&true)
        );
        assert_eq!(
            disabled_by_instrument.get(&LyrebirdInstrument::Drums),
            Some(&true)
        );
    }

    #[test]
    fn generation_count_uses_iteration_parallelism_and_enabled_instruments() {
        let mut state = LyrebirdState {
            instrument_parallelism: 5,
            iteration: 2,
            ..LyrebirdState::default()
        };
        state.rhythm_guitar.state.disabled = true;
        state.backup_vocals.state.disabled = true;
        state.guitar_solo.state.disabled = true;
        state.drums.state.disabled = true;

        assert_eq!(state.enabled_instrument_count(), 2);
        assert_eq!(state.generation_count(), 20);
    }

    #[tokio::test]
    async fn lyrebird_prompt_focus_branches_run_concurrently() {
        let _guard = prompt_join_test_lock().lock().await;
        let runtime = prompt_join_concurrent_runtime();
        runtime.reset(2);

        let mut state = LyrebirdState::default();
        state.rhythm_guitar.state.prompt_attempt = 1;
        state.vocals.state.prompt_attempt = 2;

        let mut executor = Executor::<ConcurrentLyrebirdPromptAnimal>::new(state);
        let request = executor
            .next_executable_request(3)
            .expect("focused lyrebird prompt join should produce an executable request");
        let completion = tokio::time::timeout(Duration::from_millis(250), request.run())
            .await
            .expect("focused lyrebird prompt branches should rendezvous without deadlock")
            .expect("focused lyrebird prompt runner should succeed");
        let emitted = executor
            .complete_serialized(completion)
            .expect("focused lyrebird prompt completion should apply cleanly");
        let final_emitted: (i32, i32) =
            postcard::from_bytes(&emitted).expect("focused prompt join output should deserialize");

        assert_eq!(runtime.max_active.load(Ordering::SeqCst), 2);
        assert_eq!(final_emitted, (4, 32));
        assert_eq!(executor.state().rhythm_guitar.state.prompt_attempt, 4);
        assert_eq!(executor.state().vocals.state.prompt_attempt, 32);
    }

    #[tokio::test]
    async fn lyrebird_prompt_focus_join_streams_child_history_live() {
        let _guard = prompt_join_test_lock().lock().await;
        let runtime = prompt_join_concurrent_runtime();
        runtime.reset(2);

        let mut state = LyrebirdState::default();
        state.rhythm_guitar.state.prompt_attempt = 1;
        state.vocals.state.prompt_attempt = 2;

        let mut executor = Executor::<ConcurrentLyrebirdPromptAnimal>::new(state);
        executor.set_journey_id(Uuid::new_v4());
        let mut request = executor
            .next_executable_request(3)
            .expect("focused lyrebird prompt join should produce an executable request");
        let join_node_id = request.node_id();
        let mut live_history = request
            .take_live_history()
            .expect("journey-bound prompt join should expose live child history");
        let run = tokio::spawn(async move { request.run().await });

        let deadline = tokio::time::sleep(Duration::from_millis(250));
        tokio::pin!(deadline);
        let mut saw_child_lifecycle = false;
        let mut saw_child_effect_input = false;
        loop {
            tokio::select! {
                maybe_event = live_history.next() => {
                    let event = maybe_event.expect("live prompt join history should produce child events before completion");
                    match event {
                        jungle_sdk::RunnerOut::NodeLifecycle(node) if node.node_id != join_node_id => {
                            saw_child_lifecycle = true;
                        }
                        jungle_sdk::RunnerOut::EffectInput { node_id, .. } if node_id != join_node_id => {
                            saw_child_effect_input = true;
                        }
                        _ => {}
                    }
                    if saw_child_lifecycle && saw_child_effect_input {
                        break;
                    }
                }
                _ = &mut deadline => {
                    panic!("timed out waiting for prompt join child history before completion");
                }
            }
        }

        let completion = run
            .await
            .expect("prompt join runner task should join")
            .expect("prompt join runner should succeed");
        let _ = executor
            .complete_serialized(completion)
            .expect("prompt join completion should still apply cleanly");
        let replayed_updates = executor.take_node_lifecycle_updates();
        assert!(
            replayed_updates
                .iter()
                .all(|update| update.node_id == join_node_id),
            "child lifecycle updates should not be replayed after live emission: {replayed_updates:?}"
        );
    }

    #[tokio::test]
    async fn hidden_join_streams_taken_noop_conditional_branch_lifecycle_live() {
        let mut executor = Executor::<HiddenJoinConditionalNoopAnimal>::new(7);
        executor.set_journey_id(Uuid::new_v4());
        let mut request = executor
            .next_executable_request(())
            .expect("journey-bound hidden join should produce an executable request");
        let join_node_id = request.node_id();
        let mut live_history = request
            .take_live_history()
            .expect("journey-bound hidden join should expose live child history");
        let run = tokio::spawn(async move { request.run().await });

        let deadline = tokio::time::sleep(Duration::from_millis(250));
        tokio::pin!(deadline);
        let mut seen_lifecycle_ids = std::collections::BTreeSet::new();
        loop {
            tokio::select! {
                maybe_event = live_history.next() => {
                    match maybe_event {
                        Some(jungle_sdk::RunnerOut::NodeLifecycle(node)) => {
                            if node.node_id != join_node_id {
                                seen_lifecycle_ids.insert(node.node_id);
                            }
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
                _ = &mut deadline => {
                    panic!("timed out waiting for hidden join live history; saw lifecycle ids {seen_lifecycle_ids:?}");
                }
            }
        }

        let completion = run
            .await
            .expect("hidden join runner task should join")
            .expect("hidden join runner should succeed");
        let _ = executor
            .complete_serialized(completion)
            .expect("hidden join completion should still apply cleanly");
        assert!(
            seen_lifecycle_ids.len() >= 3,
            "expected hidden join live history to include conditional/noop child lifecycles before completion, saw {seen_lifecycle_ids:?}"
        );
    }

    #[tokio::test]
    async fn hidden_join_taken_noop_branch_lifecycle_reaches_step_update_subscription() {
        let client = jungle_sdk::FusedClient::builder()
            .namespace("lyrebird-hidden-join-noop")
            .build()
            .await
            .expect("local client should build");

        let worker =
            jungle_sdk::core::JungleWorker::new(HiddenJoinConditionalNoopEcosystem, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let journey_id = client
            .spawn::<HiddenJoinConditionalNoopAnimal>(&())
            .await
            .expect("journey should start")
            .journey_id;
        let mut subscription = client
            .subscribe_step_updates(journey_id, None)
            .await
            .expect("subscribe_step_updates should succeed");

        let seen_lifecycle_ids = tokio::time::timeout(Duration::from_secs(8), async {
            let mut ids = std::collections::BTreeSet::new();
            while let Some(next) = subscription.next().await {
                let update = next.expect("streamed journey update should succeed");
                if let RunnerUpdateOut::NodeLifecycle(node) = update.event {
                    ids.insert(node.node_id);
                }
            }
            ids
        })
        .await
        .expect("journey update stream should finish before timeout");

        assert!(
            seen_lifecycle_ids.len() >= 4,
            "expected subscription to include hidden-join conditional and taken Noop lifecycle nodes, saw {seen_lifecycle_ids:?}"
        );

        worker_handle.abort();
    }

    #[ignore = "diagnostic layout dump"]
    #[test]
    fn debug_print_lyrebird_prompt_layout_order() {
        let plain = jungle_vision::debug_plain_layout_for_animal::<Lyrebird>();
        println!("{plain}");
    }

    #[tokio::test]
    async fn nested_five_way_prompt_join_live_history_keeps_branch_two_and_four_runtime_ids() {
        let mut executor = Executor::<NestedFiveWayPromptAnimal>::new(0);
        executor.set_journey_id(Uuid::new_v4());
        let mut request = executor
            .next_executable_request(())
            .expect("nested five-way prompt join should produce an executable request");
        let mut live_history = request
            .take_live_history()
            .expect("journey-bound nested prompt join should expose live child history");
        let label_by_runtime = jungle_vision::debug_render_states_for_animal::<
            NestedFiveWayPromptAnimal,
        >(std::iter::empty())
        .into_iter()
        .filter_map(|node| node.runtime_id.map(|runtime_id| (runtime_id, node.label)))
        .collect::<std::collections::HashMap<_, _>>();

        let run = tokio::spawn(async move { request.run().await });
        let mut seen_labels = std::collections::BTreeSet::new();
        while let Some(event) = live_history.next().await {
            match event {
                RunnerOut::NodeLifecycle(node) => {
                    if let Some(label) = label_by_runtime.get(&node.node_id) {
                        seen_labels.insert(label.clone());
                    }
                }
                RunnerOut::EffectInput { node_id, .. }
                | RunnerOut::EffectSuccessOutput { node_id, .. }
                | RunnerOut::EffectFailureOutput { node_id, .. } => {
                    if let Some(label) = label_by_runtime.get(&node_id) {
                        seen_labels.insert(label.clone());
                    }
                }
                RunnerOut::SleepScheduled { .. }
                | RunnerOut::SleepFired { .. }
                | RunnerOut::Appearance { .. } => {}
            }
        }

        let completion = run
            .await
            .expect("nested five-way prompt runner task should join")
            .expect("nested five-way prompt runner should succeed");
        let _ = executor
            .complete_serialized(completion)
            .expect("nested five-way prompt completion should still apply cleanly");

        assert!(
            seen_labels.contains("Branch2SelectEffect"),
            "{seen_labels:?}"
        );
        assert!(
            seen_labels.contains("Branch2OptimizeEffect"),
            "{seen_labels:?}"
        );
        assert!(
            seen_labels.contains("Branch4SelectEffect"),
            "{seen_labels:?}"
        );
        assert!(
            seen_labels.contains("Branch4OptimizeEffect"),
            "{seen_labels:?}"
        );
        assert!(seen_labels.contains("Branch1SkipEffect"), "{seen_labels:?}");
        assert!(seen_labels.contains("Branch3SkipEffect"), "{seen_labels:?}");
        assert!(seen_labels.contains("Branch5SkipEffect"), "{seen_labels:?}");
        assert!(
            !seen_labels.contains("Branch3SelectEffect"),
            "{seen_labels:?}"
        );
        assert!(
            !seen_labels.contains("Branch3OptimizeEffect"),
            "{seen_labels:?}"
        );
    }

    #[ignore = "diagnostic prompt-join trace"]
    #[tokio::test]
    async fn lyrebird_prompt_join_live_history_uses_bass_and_vocals_selected_branch_labels() {
        let root = std::env::temp_dir().join(format!("lyrebird-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("lyrebird test root should be created");
        let seed = LyrebirdSeed {
            output_root: root.display().to_string(),
            instruments: LyrebirdInstrument::ALL
                .into_iter()
                .map(|instrument| {
                    let sample_path = root.join(format!("{}.wav", instrument.output_stem()));
                    let spectrogram_path = root.join(format!("{}.png", instrument.output_stem()));
                    let dsp_source_path = root.join(format!("{}.rs", instrument.output_stem()));
                    LyrebirdInstrumentSeed {
                        instrument,
                        disabled: !matches!(
                            instrument,
                            LyrebirdInstrument::Vocals | LyrebirdInstrument::Bass
                        ),
                        target_sample_path: sample_path.display().to_string(),
                        target_audio_metrics: LyrebirdAudioMetrics::default(),
                        target_spectrogram_path: spectrogram_path.display().to_string(),
                        dsp_source_path: dsp_source_path.display().to_string(),
                        initial_dsp_code: DspCode {
                            iteration_id: "initial".to_owned(),
                            source: format!("// {}\nfn main() {{}}\n", instrument.slug()),
                            sample_path: sample_path.display().to_string(),
                            spectrogram_path: spectrogram_path.display().to_string(),
                            mel_similarity: Some(0.0),
                            score: Some(0.0),
                            audio_metrics: None,
                            audio_metric_errors: None,
                        },
                    }
                })
                .collect(),
            instrument_parallelism: 0,
        };

        let ecosystem = Arc::new(
            PulseCodePurgatory::new(
                Url::parse("http://localhost:1/v1").expect("lyrebird test tokens URL should parse"),
                None,
                Some(root.join("mcts.redb")),
            )
            .expect("lyrebird test ecosystem should build")
            .with_mcts_config(
                seed.instruments
                    .iter()
                    .cloned()
                    .map(|instrument| (instrument.instrument, instrument.initial_dsp_code)),
                DEFAULT_TREE_DEPTH,
            )
            .with_instrument_parallelism(0),
        );
        let mut executor = ContextExecutor::<PulseCodePurgatory, Lyrebird>::new(
            ecosystem,
            LyrebirdState::default(),
        );
        executor.set_journey_id(Uuid::new_v4());
        let label_by_runtime =
            jungle_vision::debug_render_states_for_animal::<Lyrebird>(std::iter::empty())
                .into_iter()
                .filter_map(|node| node.runtime_id.map(|runtime_id| (runtime_id, node.label)))
                .collect::<std::collections::HashMap<_, _>>();

        let mut prompt_labels = std::collections::BTreeSet::new();
        let mut inspected_prompt_join = false;

        for _ in 0..16 {
            if executor.is_complete() {
                break;
            }
            let mut request = executor
                .next_executable_request(seed.clone())
                .expect("lyrebird executor should keep producing executable requests");
            let maybe_live_history = request.take_live_history();
            let completion = if let Some(mut live_history) = maybe_live_history {
                inspected_prompt_join = true;
                let run = tokio::spawn(async move { request.run().await });
                while let Some(event) = live_history.next().await {
                    match event {
                        RunnerOut::NodeLifecycle(node) => {
                            if let Some(label) = label_by_runtime.get(&node.node_id) {
                                prompt_labels.insert(label.clone());
                            }
                        }
                        RunnerOut::EffectInput { node_id, .. }
                        | RunnerOut::EffectSuccessOutput { node_id, .. }
                        | RunnerOut::EffectFailureOutput { node_id, .. } => {
                            if let Some(label) = label_by_runtime.get(&node_id) {
                                prompt_labels.insert(label.clone());
                            }
                        }
                        RunnerOut::SleepScheduled { .. }
                        | RunnerOut::SleepFired { .. }
                        | RunnerOut::Appearance { .. } => {}
                    }
                }
                match run
                    .await
                    .expect("lyrebird prompt join runner task should join")
                {
                    Ok(completion) => Some(completion),
                    Err(_err) => None,
                }
            } else {
                Some(
                    request
                        .run()
                        .await
                        .expect("lyrebird setup request should serialize completion"),
                )
            };

            if let Some(completion) = completion {
                let _ = executor.complete_serialized(completion);
            }
            if inspected_prompt_join {
                break;
            }
        }

        assert!(inspected_prompt_join, "did not reach lyrebird prompt join");
        assert!(
            prompt_labels.contains("VocalsMarker>>"),
            "{prompt_labels:?}"
        );
        assert!(prompt_labels.contains("VocalsMarker>"), "{prompt_labels:?}");
        assert!(prompt_labels.contains("BassMarker>>"), "{prompt_labels:?}");
        assert!(prompt_labels.contains("BassMarker>"), "{prompt_labels:?}");
        assert!(
            !prompt_labels.contains("BackupVocalsMarker>"),
            "{prompt_labels:?}"
        );
        assert!(
            !prompt_labels.contains("BackupVocalsMarker>>"),
            "{prompt_labels:?}"
        );
        assert!(
            !prompt_labels.contains("RhythmGuitarMarker>"),
            "{prompt_labels:?}"
        );
        assert!(
            !prompt_labels.contains("GuitarSoloMarker>"),
            "{prompt_labels:?}"
        );
    }

    #[ignore = "diagnostic renderer trace"]
    #[tokio::test]
    async fn debug_trace_lyrebird_bass_vocals_step_updates_across_iterations() {
        let root = std::env::temp_dir().join(format!("lyrebird-debug-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("debug root should be created");
        let seed = LyrebirdSeed {
            output_root: root.display().to_string(),
            instruments: LyrebirdInstrument::ALL
                .into_iter()
                .map(|instrument| {
                    let sample_path = root.join(format!("{}.wav", instrument.output_stem()));
                    let spectrogram_path = root.join(format!("{}.png", instrument.output_stem()));
                    let dsp_source_path = root.join(format!("{}.rs", instrument.output_stem()));
                    LyrebirdInstrumentSeed {
                        instrument,
                        disabled: !matches!(
                            instrument,
                            LyrebirdInstrument::Vocals | LyrebirdInstrument::Bass
                        ),
                        target_sample_path: sample_path.display().to_string(),
                        target_audio_metrics: LyrebirdAudioMetrics::default(),
                        target_spectrogram_path: spectrogram_path.display().to_string(),
                        dsp_source_path: dsp_source_path.display().to_string(),
                        initial_dsp_code: DspCode {
                            iteration_id: "initial".to_owned(),
                            source: format!("// {}\nfn main() {{}}\n", instrument.slug()),
                            sample_path: sample_path.display().to_string(),
                            spectrogram_path: spectrogram_path.display().to_string(),
                            mel_similarity: Some(0.0),
                            score: Some(0.0),
                            audio_metrics: None,
                            audio_metric_errors: None,
                        },
                    }
                })
                .collect(),
            instrument_parallelism: 0,
        };

        let ecosystem = Arc::new(
            PulseCodePurgatory::new(
                Url::parse("http://localhost:1/v1").expect("debug tokens URL should parse"),
                None,
                Some(root.join("mcts.redb")),
            )
            .expect("debug lyrebird ecosystem should build")
            .with_mcts_config(
                seed.instruments
                    .iter()
                    .cloned()
                    .map(|instrument| (instrument.instrument, instrument.initial_dsp_code)),
                DEFAULT_TREE_DEPTH,
            )
            .with_instrument_parallelism(0),
        );
        let mut executor = ContextExecutor::<PulseCodePurgatory, Lyrebird>::new(
            ecosystem,
            LyrebirdState::default(),
        );
        let journey_id = Uuid::new_v4();
        executor.set_journey_id(journey_id);
        let label_by_runtime =
            jungle_vision::debug_render_states_for_animal::<Lyrebird>(std::iter::empty())
                .into_iter()
                .filter_map(|node| node.runtime_id.map(|runtime_id| (runtime_id, node.label)))
                .collect::<std::collections::HashMap<_, _>>();

        let mut seen_begin_iteration_enters = 0_usize;
        let mut updates = Vec::new();
        let mut log_lines = Vec::new();
        let mut sequence_id = 1_u64;
        for _ in 0..32 {
            if executor.is_complete() {
                break;
            }
            let mut request = executor
                .next_executable_request(seed.clone())
                .expect("debug lyrebird executor should produce a request");
            let request_node_id = request.node_id();
            for lifecycle in executor.take_node_lifecycle_updates() {
                if let Some(label) = label_by_runtime.get(&lifecycle.node_id) {
                    if label == "BeginIteration"
                        && matches!(lifecycle.phase, NodeLifecyclePhase::Entered)
                    {
                        seen_begin_iteration_enters = seen_begin_iteration_enters.saturating_add(1);
                    }
                    let line = format!(
                        "seq={} node={} label={} lifecycle={:?} path={:?}",
                        sequence_id,
                        lifecycle.node_id,
                        label,
                        lifecycle.phase,
                        lifecycle.activation_path
                    );
                    if line.contains("BeginIteration")
                        || line.contains("FlattenLyrebirdPromptPhase")
                        || line.contains("RhythmGuitarMarker")
                        || line.contains("VocalsMarker")
                        || line.contains("BackupVocalsMarker")
                        || line.contains("BassMarker")
                        || line.contains("GuitarSoloMarker")
                    {
                        log_lines.push(line);
                    }
                }
                updates.push(JourneyUpdateEvent {
                    sequence_id,
                    event_unix_ms: sequence_id as i64,
                    event: RunnerUpdateOut::NodeLifecycle(lifecycle),
                });
                sequence_id = sequence_id.saturating_add(1);
            }
            updates.push(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: sequence_id as i64,
                event: RunnerUpdateOut::EffectInput {
                    node_id: request_node_id,
                    uuid: journey_id,
                },
            });
            sequence_id = sequence_id.saturating_add(1);
            if let Some(mut live_history) = request.take_live_history() {
                let run = tokio::spawn(async move { request.run().await });
                while let Some(event) = live_history.next().await {
                    let update = match event {
                        RunnerOut::NodeLifecycle(node) => JourneyUpdateEvent {
                            sequence_id,
                            event_unix_ms: sequence_id as i64,
                            event: RunnerUpdateOut::NodeLifecycle(node),
                        },
                        RunnerOut::EffectInput { node_id, uuid, .. } => JourneyUpdateEvent {
                            sequence_id,
                            event_unix_ms: sequence_id as i64,
                            event: RunnerUpdateOut::EffectInput { node_id, uuid },
                        },
                        RunnerOut::EffectSuccessOutput { node_id, uuid, .. } => {
                            JourneyUpdateEvent {
                                sequence_id,
                                event_unix_ms: sequence_id as i64,
                                event: RunnerUpdateOut::EffectSuccessOutput { node_id, uuid },
                            }
                        }
                        RunnerOut::EffectFailureOutput { node_id, uuid, .. } => {
                            JourneyUpdateEvent {
                                sequence_id,
                                event_unix_ms: sequence_id as i64,
                                event: RunnerUpdateOut::EffectFailureOutput { node_id, uuid },
                            }
                        }
                        RunnerOut::SleepScheduled {
                            uuid,
                            timer_id,
                            wake_at_unix_ms,
                        } => JourneyUpdateEvent {
                            sequence_id,
                            event_unix_ms: sequence_id as i64,
                            event: RunnerUpdateOut::SleepScheduled {
                                uuid,
                                timer_id,
                                wake_at_unix_ms,
                            },
                        },
                        RunnerOut::SleepFired {
                            uuid,
                            timer_id,
                            fired_at_unix_ms,
                        } => JourneyUpdateEvent {
                            sequence_id,
                            event_unix_ms: sequence_id as i64,
                            event: RunnerUpdateOut::SleepFired {
                                uuid,
                                timer_id,
                                fired_at_unix_ms,
                            },
                        },
                        RunnerOut::Appearance { .. } => {
                            sequence_id = sequence_id.saturating_add(1);
                            continue;
                        }
                    };
                    sequence_id = sequence_id.saturating_add(1);
                    if let Some(line) = match &update.event {
                        RunnerUpdateOut::NodeLifecycle(node) => {
                            label_by_runtime.get(&node.node_id).cloned().map(|label| {
                                if label == "BeginIteration"
                                    && matches!(node.phase, NodeLifecyclePhase::Entered)
                                {
                                    seen_begin_iteration_enters =
                                        seen_begin_iteration_enters.saturating_add(1);
                                }
                                format!(
                                    "seq={} node={} label={} lifecycle={:?} path={:?}",
                                    update.sequence_id,
                                    node.node_id,
                                    label,
                                    node.phase,
                                    node.activation_path
                                )
                            })
                        }
                        RunnerUpdateOut::EffectInput { node_id, .. } => {
                            label_by_runtime.get(node_id).cloned().map(|label| {
                                format!(
                                    "seq={} node={} label={} effect=input",
                                    update.sequence_id, node_id, label
                                )
                            })
                        }
                        RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => {
                            label_by_runtime.get(node_id).cloned().map(|label| {
                                format!(
                                    "seq={} node={} label={} effect=success",
                                    update.sequence_id, node_id, label
                                )
                            })
                        }
                        RunnerUpdateOut::EffectFailureOutput { node_id, .. } => {
                            label_by_runtime.get(node_id).cloned().map(|label| {
                                format!(
                                    "seq={} node={} label={} effect=failure",
                                    update.sequence_id, node_id, label
                                )
                            })
                        }
                        RunnerUpdateOut::SleepScheduled { .. }
                        | RunnerUpdateOut::SleepFired { .. } => None,
                    }
                    .filter(|line| {
                        line.contains("BeginIteration")
                            || line.contains("FlattenLyrebirdPromptPhase")
                            || line.contains("RhythmGuitarMarker")
                            || line.contains("VocalsMarker")
                            || line.contains("BackupVocalsMarker")
                            || line.contains("BassMarker")
                            || line.contains("GuitarSoloMarker")
                    }) {
                        log_lines.push(line);
                    }
                    updates.push(update);
                }
                let completion = run
                    .await
                    .expect("debug lyrebird runner should join")
                    .expect("debug lyrebird request should complete");
                updates.push(JourneyUpdateEvent {
                    sequence_id,
                    event_unix_ms: sequence_id as i64,
                    event: RunnerUpdateOut::EffectSuccessOutput {
                        node_id: request_node_id,
                        uuid: journey_id,
                    },
                });
                sequence_id = sequence_id.saturating_add(1);
                let _ = executor
                    .complete_serialized(completion)
                    .expect("debug lyrebird completion should apply");
            } else {
                let completion = request
                    .run()
                    .await
                    .expect("debug lyrebird request should complete");
                updates.push(JourneyUpdateEvent {
                    sequence_id,
                    event_unix_ms: sequence_id as i64,
                    event: RunnerUpdateOut::EffectSuccessOutput {
                        node_id: request_node_id,
                        uuid: journey_id,
                    },
                });
                sequence_id = sequence_id.saturating_add(1);
                let _ = executor
                    .complete_serialized(completion)
                    .expect("debug lyrebird completion should apply");
            }
            for lifecycle in executor.take_node_lifecycle_updates() {
                if let Some(label) = label_by_runtime.get(&lifecycle.node_id) {
                    if label == "BeginIteration"
                        && matches!(lifecycle.phase, NodeLifecyclePhase::Entered)
                    {
                        seen_begin_iteration_enters = seen_begin_iteration_enters.saturating_add(1);
                    }
                    let line = format!(
                        "seq={} node={} label={} lifecycle={:?} path={:?}",
                        sequence_id,
                        lifecycle.node_id,
                        label,
                        lifecycle.phase,
                        lifecycle.activation_path
                    );
                    if line.contains("BeginIteration")
                        || line.contains("FlattenLyrebirdPromptPhase")
                        || line.contains("RhythmGuitarMarker")
                        || line.contains("VocalsMarker")
                        || line.contains("BackupVocalsMarker")
                        || line.contains("BassMarker")
                        || line.contains("GuitarSoloMarker")
                    {
                        log_lines.push(line);
                    }
                }
                updates.push(JourneyUpdateEvent {
                    sequence_id,
                    event_unix_ms: sequence_id as i64,
                    event: RunnerUpdateOut::NodeLifecycle(lifecycle),
                });
                sequence_id = sequence_id.saturating_add(1);
            }

            if seen_begin_iteration_enters >= 3 && log_lines.len() >= 40 {
                break;
            }
        }

        let rendered = jungle_vision::debug_render_states_for_animal::<Lyrebird>(updates.clone())
            .into_iter()
            .filter(|node| {
                node.label.contains("RhythmGuitarMarker")
                    || node.label.contains("VocalsMarker")
                    || node.label.contains("BackupVocalsMarker")
                    || node.label.contains("BassMarker")
                    || node.label.contains("GuitarSoloMarker")
            })
            .map(|node| format!("{} => {:?}", node.label, node.state))
            .collect::<Vec<_>>();
        let first_iteration_rendered = jungle_vision::debug_render_states_for_animal::<Lyrebird>(
            updates
                .iter()
                .filter(|update| update.sequence_id < 122)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .filter(|node| {
            node.label.contains("RhythmGuitarMarker")
                || node.label.contains("VocalsMarker")
                || node.label.contains("BackupVocalsMarker")
                || node.label.contains("BassMarker")
                || node.label.contains("GuitarSoloMarker")
        })
        .map(|node| format!("{} => {:?}", node.label, node.state))
        .collect::<Vec<_>>();
        let mut vocals_transitions = Vec::new();
        for cutoff in 1..122_u64 {
            let states = jungle_vision::debug_render_states_for_animal::<Lyrebird>(
                updates
                    .iter()
                    .filter(|update| update.sequence_id <= cutoff)
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            let mut interesting = states
                .into_iter()
                .filter(|node| {
                    matches!(
                        node.label.as_str(),
                        "VocalsMarker>>" | "VocalsMarker>" | "BassMarker>>" | "BassMarker>"
                    )
                })
                .map(|node| format!("{}={:?}", node.label, node.state))
                .collect::<Vec<_>>();
            interesting.sort();
            if interesting.iter().any(|entry| !entry.ends_with("Pending")) {
                vocals_transitions.push(format!("cutoff={cutoff} {}", interesting.join(" ")));
            }
        }
        let debug_events = updates
            .iter()
            .filter(|update| {
                (28..=32).contains(&update.sequence_id) || (52..=56).contains(&update.sequence_id)
            })
            .map(|update| match &update.event {
                RunnerUpdateOut::NodeLifecycle(node) => format!(
                    "seq={} lifecycle node={} label={} phase={:?} path={:?}",
                    update.sequence_id,
                    node.node_id,
                    label_by_runtime
                        .get(&node.node_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<{}>", node.node_id)),
                    node.phase,
                    node.activation_path
                ),
                RunnerUpdateOut::EffectInput { node_id, .. } => format!(
                    "seq={} effect_input node={} label={}",
                    update.sequence_id,
                    node_id,
                    label_by_runtime
                        .get(node_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<{}>", node_id))
                ),
                RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => format!(
                    "seq={} effect_success node={} label={}",
                    update.sequence_id,
                    node_id,
                    label_by_runtime
                        .get(node_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<{}>", node_id))
                ),
                RunnerUpdateOut::EffectFailureOutput { node_id, .. } => format!(
                    "seq={} effect_failure node={} label={}",
                    update.sequence_id,
                    node_id,
                    label_by_runtime
                        .get(node_id)
                        .cloned()
                        .unwrap_or_else(|| format!("<{}>", node_id))
                ),
                RunnerUpdateOut::SleepScheduled { timer_id, .. } => {
                    format!(
                        "seq={} sleep_scheduled timer={timer_id}",
                        update.sequence_id
                    )
                }
                RunnerUpdateOut::SleepFired { timer_id, .. } => {
                    format!("seq={} sleep_fired timer={timer_id}", update.sequence_id)
                }
            })
            .collect::<Vec<_>>();
        let cutoff_29_decisions = jungle_vision::debug_runtime_decisions_for_animal::<Lyrebird>(
            updates
                .iter()
                .filter(|update| update.sequence_id <= 29)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .filter(|node| matches!(node.label.as_str(), "VocalsMarker>" | "VocalsMarker>>"))
        .map(|node| {
            format!(
                "cutoff=29 label={} runtime={:?} state={:?} seq={:?} floor={:?} path={:?} prefix={:?}",
                node.label,
                node.runtime_id,
                node.state,
                node.sequence,
                node.floor,
                node.activation_path,
                node.required_prefix
            )
        })
        .collect::<Vec<_>>();
        let cutoff_30_decisions = jungle_vision::debug_runtime_decisions_for_animal::<Lyrebird>(
            updates
                .iter()
                .filter(|update| update.sequence_id <= 30)
                .cloned()
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .filter(|node| matches!(node.label.as_str(), "VocalsMarker>" | "VocalsMarker>>"))
        .map(|node| {
            format!(
                "cutoff=30 label={} runtime={:?} state={:?} seq={:?} floor={:?} path={:?} prefix={:?}",
                node.label,
                node.runtime_id,
                node.state,
                node.sequence,
                node.floor,
                node.activation_path,
                node.required_prefix
            )
        })
        .collect::<Vec<_>>();

        println!("trace:\n{}", log_lines.join("\n"));
        println!("debug-events:\n{}", debug_events.join("\n"));
        println!("cutoff-29-decisions:\n{}", cutoff_29_decisions.join("\n"));
        println!("cutoff-30-decisions:\n{}", cutoff_30_decisions.join("\n"));
        println!("transitions:\n{}", vocals_transitions.join("\n"));
        println!(
            "first-iteration-rendered:\n{}",
            first_iteration_rendered.join("\n")
        );
        println!("rendered:\n{}", rendered.join("\n"));
    }

    #[test]
    fn aggregate_sample_score_weights_mel_similarity_five_times() {
        let score = aggregate_sample_score(
            0.8,
            LyrebirdAudioMetricErrors {
                zero_crossing_rate: 0.1,
                crest_factor: 0.2,
                spectral_centroid: 0.3,
                spectral_flatness: 0.4,
                spectral_rolloff: 0.5,
            },
        );

        let expected = ((0.8 * 5.0) + 0.9 + 0.8 + 0.7 + 0.6 + 0.5) / 10.0;
        assert!((score - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn branch_node_deserializes_structured_json_shape() {
        let node = serde_json::from_value::<LyrebirdBranchNode>(serde_json::json!({
            "code": {
                "iteration_id": "00000002",
                "source": "fn bass() { updated(); }",
                "sample_path": "/tmp/updated.wav",
                "spectrogram_path": "/tmp/updated.png",
                "similarity": 0.8
            },
            "patch": {
                "search": "old();",
                "replacement": "new();",
                "note": "narrow resonance"
            },
            "mel_spectrogram_path": "/tmp/updated-mel.png"
        }))
        .unwrap();

        assert_eq!(node.code.iteration_id, "00000002");
        assert_eq!(node.mel_spectrogram_path, "/tmp/updated-mel.png");
        assert_eq!(
            node.patch.as_ref().map(|patch| patch.note.as_str()),
            Some("narrow resonance")
        );
        assert_eq!(node.mel_similarity(), Some(0.8));
        assert_eq!(node.score(), Some(0.8));
        assert_eq!(node.similarity(), Some(0.8));
    }

    #[test]
    fn branch_node_round_trips_through_postcard() {
        let node = LyrebirdBranchNode::from_generated(
            DspCode {
                iteration_id: "00000004".to_owned(),
                source: "fn bass() { postcard(); }".to_owned(),
                sample_path: "/tmp/postcard.wav".to_owned(),
                spectrogram_path: "/tmp/postcard.png".to_owned(),
                mel_similarity: Some(0.85),
                score: Some(0.9),
                audio_metrics: Some(LyrebirdAudioMetrics {
                    zero_crossing_rate: 0.1,
                    crest_factor: 2.0,
                    spectral_centroid: 500.0,
                    spectral_flatness: 0.2,
                    spectral_rolloff: 1_000.0,
                }),
                audio_metric_errors: Some(LyrebirdAudioMetricErrors {
                    zero_crossing_rate: 0.05,
                    crest_factor: 0.1,
                    spectral_centroid: 0.2,
                    spectral_flatness: 0.15,
                    spectral_rolloff: 0.25,
                }),
            },
            LyrebirdPatch {
                search: "postcard();".to_owned(),
                replacement: "postcard_v2();".to_owned(),
                note: "shift postcard path".to_owned(),
            },
        );

        let bytes = postcard::to_allocvec(&node).unwrap();
        let decoded = postcard::from_bytes::<LyrebirdBranchNode>(&bytes).unwrap();

        assert_eq!(decoded, node);
    }

    #[test]
    fn cli_accepts_tokens_model_override() {
        let cli = Cli::try_parse_from([
            "lyrebird",
            "--tokens-url",
            "https://api.openai.com/v1",
            "--tokens-model",
            "gpt-5.4-mini",
        ])
        .unwrap();

        assert_eq!(cli.tokens_model.as_deref(), Some("gpt-5.4-mini"));
    }
}
