use clap::Parser;
use directories_next::BaseDirs;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, JourneyStatus, JungleClient, Server};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    BeginIteration, BuildPrompt, CompilePreparedDspPatch, CurrentInstrumentCompileReady,
    FinalizeIterationRender, FlattenEitherUnit, FlattenJoinedUnit, InstrumentMarker,
    LyrebirdLoopForever, PrepareDspPatch, RequestDspPatch, ScoreSpectrogram, SeedLyrebirdState,
    SelectDspBranch, SetCurrentInstrument, SkipInstrumentIteration, SubmitDspBranch,
};
use crate::tokens::Tool;

const DEFAULT_WORKERS: usize = 3;
const DEFAULT_TREE_DEPTH: usize = 8;
const DEFAULT_LOG_FILTER: &str = "warn,lyrebird=info";
pub(crate) const LYREBIRD_DURATION_SECS: f64 = 4.0;
pub(crate) const GUITAR_INTRO_SCORE_SPEC: &str =
    "electric-guitar(rhythm-sustained):[350,58,96],[350,58,96],[446,58,96],[542,58,96],[542,58,96],[638,56,96],[638,56,96],[734,56,96],[830,56,96],[830,56,96],[926,53,96],[926,53,96],[1022,53,96],[1118,53,96],[1118,53,96],[1214,51,96],[1214,51,96],[1310,51,96],[1406,51,96],[1406,51,96],[1502,49,96],[1502,49,96],[1598,49,96],[1694,46,96],[1694,49,96],[1694,46,96],[1790,49,96],[1790,46,96],[1886,58,96],[1886,58,96],[1982,58,96],[2078,58,96],[2078,58,96],[2174,56,96],[2174,56,96],[2270,56,96],[2366,56,96],[2366,56,96],[2462,53,96],[2462,53,96],[2558,53,96],[2654,53,96],[2654,53,96],[2750,51,96],[2750,51,96],[2846,51,96]";
pub(crate) const VOCALS_SCORE_SPEC: &str = "vocals(formant):[250,66,96,'wel'],[346,68,288,'come'],[634,68,96,'to'],[730,66,96,'the'],[826,71,384,'jun'],[1210,68,192,'gol'],[1786,66,96,'weve'],[1882,68,288,'got'],[2170,68,96,'fun'],[2266,66,192,'and'],[2458,68,288,'games']";
pub(crate) const BACKUP_VOCALS_SCORE_SPEC: &str = "vocals(group-harmony):[150,71,384],[534,70,384],[918,68,384],[1302,66,384],[1686,73,384],[2070,72,384],[2454,70,384],[2838,68,384]";
pub(crate) const GUITAR_SOLO_SCORE_SPEC: &str = "electric-guitar(sustained):[240,60,192],[432,72,128],[560,75,129],[689,82,896],[1585,82,128],[1713,81,129],[1842,80,704],[2546,78,96],[2642,79,96],[2738,73,672],[3410,73,224]";
pub(crate) const BASS_SCORE_SPEC: &str = "bass:[150,32,192],[342,32,192],[534,30,192],[726,27,96],[822,32,192],[1014,27,96],[1110,30,192],[1302,29,192],[1494,27,192],[1686,32,192],[1878,32,192],[2070,30,192],[2262,27,96],[2358,32,192],[2550,27,96],[2646,42,96],[2838,42,96],[3030,42,96]";

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DspCode {
    pub iteration_id: String,
    pub source: String,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub similarity: Option<f32>,
}

impl DspCode {
    fn placeholder_initial() -> Self {
        Self {
            iteration_id: "initial".to_owned(),
            ..Self::default()
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LyrebirdPatch {
    pub search: String,
    pub replacement: String,
    pub note: String,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LyrebirdBranchNode {
    pub code: DspCode,
    pub patch: Option<LyrebirdPatch>,
    pub mel_spectrogram_path: String,
}

impl LyrebirdBranchNode {
    pub fn similarity(&self) -> Option<f32> {
        self.code.similarity
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
}

impl LyrebirdInstrument {
    pub const ALL: [Self; 5] = [
        Self::RhythmGuitar,
        Self::Vocals,
        Self::BackupVocals,
        Self::Bass,
        Self::GuitarSolo,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "rhythm-guitar",
            Self::Vocals => "vocals",
            Self::BackupVocals => "backup-vocals",
            Self::Bass => "bass",
            Self::GuitarSolo => "guitar-solo",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "Electric Guitar (Intro)",
            Self::Vocals => "Vocals (Formant)",
            Self::BackupVocals => "Vocals (Group Harmony)",
            Self::Bass => "Bass",
            Self::GuitarSolo => "Electric Guitar (Solo)",
        }
    }

    pub fn score_spec(self) -> &'static str {
        match self {
            Self::RhythmGuitar => GUITAR_INTRO_SCORE_SPEC,
            Self::Vocals => VOCALS_SCORE_SPEC,
            Self::BackupVocals => BACKUP_VOCALS_SCORE_SPEC,
            Self::Bass => BASS_SCORE_SPEC,
            Self::GuitarSolo => GUITAR_SOLO_SCORE_SPEC,
        }
    }

    pub fn relative_target_spectrogram_path(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "jungle-examples/examples/lyrebird/assets/guitar_intro_4s.png",
            Self::Vocals => "jungle-examples/examples/lyrebird/assets/vocals_4s.png",
            Self::BackupVocals => "jungle-examples/examples/lyrebird/assets/backup_vocals_4s.png",
            Self::Bass => "jungle-examples/examples/lyrebird/assets/bass_4s.png",
            Self::GuitarSolo => "jungle-examples/examples/lyrebird/assets/guitar_solo_4s.png",
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
        }
    }

    pub fn output_stem(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "guitar_intro_4s",
            Self::Vocals => "vocals_4s",
            Self::BackupVocals => "backup_vocals_4s",
            Self::Bass => "bass_4s",
            Self::GuitarSolo => "guitar_solo_4s",
        }
    }

    pub fn tool_name(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "replace_rhythm_guitar_dsp",
            Self::Vocals => "replace_vocals_formant_dsp",
            Self::BackupVocals => "replace_backup_vocals_dsp",
            Self::Bass => "replace_bass_dsp",
            Self::GuitarSolo => "replace_guitar_solo_dsp",
        }
    }

    pub fn render_subject(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "electric-guitar(rhythm-sustained)",
            Self::Vocals => "vocals(formant)",
            Self::BackupVocals => "vocals(group-harmony)",
            Self::Bass => "bass",
            Self::GuitarSolo => "electric-guitar(sustained)",
        }
    }

    pub fn tree_tag(self) -> &'static str {
        self.slug()
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdInstrumentState {
    pub instrument: LyrebirdInstrument,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
    pub selected_branch: Vec<LyrebirdBranchNode>,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub last_similarity: f32,
    pub compile_ready: bool,
    pub prompt_attempt: u32,
    pub skipped_this_iteration: bool,
    pub last_retry_reason: Option<String>,
    pub pending_generated_patch: Option<LyrebirdPatch>,
    pub pending_generated_source: Option<String>,
    pub latest_generated_patch: Option<LyrebirdPatch>,
    pub latest_generated_code: Option<DspCode>,
    pub latest_rendered_code: Option<DspCode>,
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
            target_spectrogram_path: instrument.relative_target_spectrogram_path().to_owned(),
            dsp_source_path: instrument.relative_dsp_path().to_owned(),
            initial_dsp_code: DspCode::placeholder_initial(),
            selected_branch: vec![DspCode::placeholder_initial().into()],
            ..Self::default()
        }
    }

    fn begin_iteration(&mut self, output_root: &str, iteration_id: &str) {
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
        self.prompt_attempt = 0;
        self.skipped_this_iteration = false;
        self.last_retry_reason = None;
        self.pending_generated_patch = None;
        self.pending_generated_source = None;
        self.latest_generated_patch = None;
        self.last_similarity = 0.0;
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdState {
    pub output_root: String,
    pub current_instrument: LyrebirdInstrument,
    pub instruments: Vec<LyrebirdInstrumentState>,
    pub iteration: u64,
    pub iteration_id: String,
}

impl LyrebirdState {
    pub fn has_all_instrument_states(&self) -> bool {
        LyrebirdInstrument::ALL.into_iter().all(|instrument| {
            self.instruments
                .iter()
                .any(|state| state.instrument == instrument)
        })
    }

    pub fn normalized_for_observation(&self) -> Self {
        let mut normalized = self.clone();
        let mut instruments = Vec::with_capacity(LyrebirdInstrument::ALL.len());
        for instrument in LyrebirdInstrument::ALL {
            instruments.push(
                normalized
                    .instruments
                    .iter()
                    .find(|state| state.instrument == instrument)
                    .cloned()
                    .unwrap_or_else(|| {
                        LyrebirdInstrumentState::observation_placeholder(instrument)
                    }),
            );
        }
        normalized.instruments = instruments;
        if !normalized
            .instruments
            .iter()
            .any(|state| state.instrument == normalized.current_instrument)
        {
            normalized.current_instrument = LyrebirdInstrument::ALL[0];
        }
        normalized
    }

    pub fn instrument_state(&self, instrument: LyrebirdInstrument) -> &LyrebirdInstrumentState {
        self.instruments
            .iter()
            .find(|state| state.instrument == instrument)
            .expect("lyrebird instrument state missing")
    }

    pub fn instrument_state_mut(
        &mut self,
        instrument: LyrebirdInstrument,
    ) -> &mut LyrebirdInstrumentState {
        self.instruments
            .iter_mut()
            .find(|state| state.instrument == instrument)
            .expect("lyrebird instrument state missing")
    }

    pub fn current_state(&self) -> &LyrebirdInstrumentState {
        self.instrument_state(self.current_instrument)
    }

    pub fn current_state_mut(&mut self) -> &mut LyrebirdInstrumentState {
        self.instrument_state_mut(self.current_instrument)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdInstrumentSeed {
    pub instrument: LyrebirdInstrument,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LyrebirdSeed {
    pub output_root: String,
    pub instruments: Vec<LyrebirdInstrumentSeed>,
}

impl From<LyrebirdSeed> for LyrebirdState {
    fn from(seed: LyrebirdSeed) -> Self {
        let current_instrument = seed
            .instruments
            .first()
            .map(|instrument| instrument.instrument)
            .unwrap_or_default();

        Self {
            output_root: seed.output_root,
            current_instrument,
            instruments: seed
                .instruments
                .into_iter()
                .map(LyrebirdInstrumentState::from_seed)
                .collect(),
            ..Self::default()
        }
    }
}

#[derive(Flow)]
pub struct LyrebirdInstrumentPrompt<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Step<SelectDspBranch>,
    Step<BuildPrompt>,
    Step<RequestDspPatch>,
    Step<PrepareDspPatch>,
);

#[derive(Flow)]
pub struct LyrebirdInstrumentScoringBody(Step<ScoreSpectrogram>, Step<SubmitDspBranch>);

#[derive(Flow)]
pub struct LyrebirdInstrumentScoring<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Conditional<
        CurrentInstrumentCompileReady,
        LyrebirdInstrumentScoringBody,
        Step<SkipInstrumentIteration>,
    >,
    Step<FlattenEitherUnit<LyrebirdState>>,
);

pub struct RhythmGuitarMarker;
impl InstrumentMarker for RhythmGuitarMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::RhythmGuitar;
}

pub struct VocalsMarker;
impl InstrumentMarker for VocalsMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::Vocals;
}

pub struct BackupVocalsMarker;
impl InstrumentMarker for BackupVocalsMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::BackupVocals;
}

pub struct BassMarker;
impl InstrumentMarker for BassMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::Bass;
}

pub struct GuitarSoloMarker;
impl InstrumentMarker for GuitarSoloMarker {
    const INSTRUMENT: LyrebirdInstrument = LyrebirdInstrument::GuitarSolo;
}

#[derive(Flow)]
pub struct LyrebirdPromptLeft(
    Join<LyrebirdInstrumentPrompt<RhythmGuitarMarker>, LyrebirdInstrumentPrompt<VocalsMarker>>,
    Step<FlattenJoinedUnit<LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdPromptRightPair(
    Join<LyrebirdInstrumentPrompt<BackupVocalsMarker>, LyrebirdInstrumentPrompt<BassMarker>>,
    Step<FlattenJoinedUnit<LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdPromptRight(
    Join<LyrebirdPromptRightPair, LyrebirdInstrumentPrompt<GuitarSoloMarker>>,
    Step<FlattenJoinedUnit<LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdPromptPhase(
    Join<LyrebirdPromptLeft, LyrebirdPromptRight>,
    Step<FlattenJoinedUnit<LyrebirdState>>,
);

#[derive(Flow)]
pub struct LyrebirdInstrumentCompilation<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Step<CompilePreparedDspPatch>,
);

#[derive(Flow)]
pub struct LyrebirdIteration(
    Step<BeginIteration>,
    LyrebirdPromptPhase,
    LyrebirdInstrumentCompilation<RhythmGuitarMarker>,
    LyrebirdInstrumentCompilation<VocalsMarker>,
    LyrebirdInstrumentCompilation<BackupVocalsMarker>,
    LyrebirdInstrumentCompilation<BassMarker>,
    LyrebirdInstrumentCompilation<GuitarSoloMarker>,
    Step<FinalizeIterationRender>,
    LyrebirdInstrumentScoring<RhythmGuitarMarker>,
    LyrebirdInstrumentScoring<VocalsMarker>,
    LyrebirdInstrumentScoring<BackupVocalsMarker>,
    LyrebirdInstrumentScoring<BassMarker>,
    LyrebirdInstrumentScoring<GuitarSoloMarker>,
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
pub struct PulseCodeParadiseAnimals(Lyrebird);

#[derive(Clone)]
pub struct PulseCodeParadise {
    client: reqwest::Client,
    db: Arc<redb::Database>,
    db_path: PathBuf,
    runtime_session_id: String,
    tokens_model: String,
    tokens_url: Url,
    tools: Vec<Tool>,
    initial_dsp_codes: BTreeMap<LyrebirdInstrument, DspCode>,
    max_tree_depth: usize,
}

impl PulseCodeParadise {
    pub fn new(
        tokens_url: Url,
        tokens_token: Option<String>,
        db_path: Option<PathBuf>,
    ) -> Result<Self, PulseCodeParadiseError> {
        let client = Self::build_tokens_client(tokens_token.as_deref())?;
        let (db, db_path) = mcts::open_mcts_db(db_path)?;

        Ok(Self {
            client,
            db,
            db_path,
            runtime_session_id: Uuid::new_v4().to_string(),
            tokens_model: Self::tokens_model_from_env(),
            tokens_url,
            tools: Vec::new(),
            initial_dsp_codes: LyrebirdInstrument::ALL
                .into_iter()
                .map(|instrument| (instrument, DspCode::placeholder_initial()))
                .collect(),
            max_tree_depth: DEFAULT_TREE_DEPTH,
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
}

impl Ecosystem for PulseCodeParadise {
    const NAME: &'static str = "pulse-code-paradise";
    type Animals = PulseCodeParadiseAnimals;
}

#[derive(Debug, Error)]
pub enum PulseCodeParadiseError {
    #[error("failed to construct tokens client: {0}")]
    Client(#[from] reqwest::Error),
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
    #[error("failed to build initial lyrebird dsp baseline: {0}")]
    Bootstrap(String),
}

#[derive(Debug, Parser)]
#[command(name = "lyrebird")]
struct Cli {
    #[arg(
        long = "tokens-url",
        help = "OpenAI-compatible API base URL, for example https://api.openai.com/v1 or http://localhost:11434/v1"
    )]
    tokens_url: Url,
    #[arg(long = "tokens-token")]
    tokens_token: Option<String>,
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
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    validate_openai_api_base_url(&cli.tokens_url)?;
    let workspace_root = std::env::current_dir()?;
    let output_root = default_lyrebird_root()?;
    let jungle_redb_path = cli
        .jungle_redb_path
        .unwrap_or_else(|| output_root.join("jungle.redb"));
    ensure_parent_dir_exists(&jungle_redb_path)?;

    let instrument_seeds = build_instrument_seeds(&workspace_root, &output_root).await?;
    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?
        .with_tools(
            LyrebirdInstrument::ALL
                .into_iter()
                .map(build_replace_tool)
                .collect(),
        )
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
        jungle_redb_path = %jungle_redb_path.display(),
        mcts_redb_path = %ecosystem.db_path.display(),
        "starting lyrebird runtime"
    );

    let backend = Server::builder()
        .redb_path(&jungle_redb_path)
        .build()
        .await?;
    let client = FusedClient::builder()
        .namespace(PulseCodeParadise::NAME)
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

    let seed = build_seed(&output_root, &instrument_seeds);
    let journey_id = ensure_lyrebird_running(&client, &seed).await?;

    info!(
        %journey_id,
        jungle_redb_path = %jungle_redb_path.display(),
        mcts_redb_path = %ecosystem.db_path.display(),
        "lyrebird active"
    );

    #[cfg(feature = "viewer")]
    {
        tokio::task::block_in_place(|| ui::run_ui(client.clone(), journey_id))?;
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
) -> Result<Vec<LyrebirdInstrumentSeed>, PulseCodeParadiseError> {
    let mut seeds = Vec::with_capacity(LyrebirdInstrument::ALL.len());
    for instrument in LyrebirdInstrument::ALL {
        let dsp_source_path = workspace_root.join(instrument.relative_dsp_path());
        let target_spectrogram_path =
            workspace_root.join(instrument.relative_target_spectrogram_path());
        let initial_dsp_code = effect::capture_current_dsp_code_snapshot(
            "initial",
            output_root,
            instrument,
            &target_spectrogram_path,
            &dsp_source_path,
        )
        .await
        .map_err(PulseCodeParadiseError::Bootstrap)?;
        seeds.push(LyrebirdInstrumentSeed {
            instrument,
            target_spectrogram_path: target_spectrogram_path.display().to_string(),
            dsp_source_path: dsp_source_path.display().to_string(),
            initial_dsp_code,
        });
    }
    Ok(seeds)
}

fn build_seed(output_root: &Path, instruments: &[LyrebirdInstrumentSeed]) -> LyrebirdSeed {
    LyrebirdSeed {
        output_root: output_root.display().to_string(),
        instruments: instruments.to_vec(),
    }
}

fn restore_instrument_sources(
    instruments: &[LyrebirdInstrumentSeed],
) -> Result<(), PulseCodeParadiseError> {
    for instrument in instruments {
        let path = PathBuf::from(&instrument.dsp_source_path);
        std::fs::write(&path, &instrument.initial_dsp_code.source).map_err(|source| {
            PulseCodeParadiseError::RestoreDspSource {
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
        .list_journeys(PulseCodeParadise::NAME.to_owned())
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
            info!(
                journey_id = %existing.journey_id,
                "reusing existing lyrebird journey"
            );
            return Ok(existing.journey_id);
        }
        warn!(
            journey_id = %existing.journey_id,
            instrument_count = state.instruments.len(),
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

fn default_lyrebird_root() -> Result<PathBuf, PulseCodeParadiseError> {
    let base_dirs = BaseDirs::new().ok_or(PulseCodeParadiseError::HomeDirUnavailable)?;
    Ok(base_dirs.home_dir().join(".jungle").join("lyrebird"))
}

fn ensure_parent_dir_exists(path: &Path) -> Result<(), PulseCodeParadiseError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| PulseCodeParadiseError::CreateDbDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

fn validate_openai_api_base_url(tokens_url: &Url) -> Result<(), PulseCodeParadiseError> {
    match tokens_url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(PulseCodeParadiseError::InvalidTokensUrl {
                url: tokens_url.to_string(),
                reason: format!("unsupported URL scheme `{scheme}`"),
            });
        }
    }

    if tokens_url.query().is_some() {
        return Err(PulseCodeParadiseError::InvalidTokensUrl {
            url: tokens_url.to_string(),
            reason: "query parameters are not allowed".to_owned(),
        });
    }

    if tokens_url.fragment().is_some() {
        return Err(PulseCodeParadiseError::InvalidTokensUrl {
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
        return Err(PulseCodeParadiseError::InvalidTokensUrl {
            url: tokens_url.to_string(),
            reason: "received the chat completions endpoint instead of the API base URL".to_owned(),
        });
    }

    Ok(())
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
            PulseCodeParadiseError::InvalidTokensUrl { .. }
        ));
        assert!(err.to_string().contains("chat completions endpoint"));
    }

    #[test]
    fn lyrebird_instrument_metadata_covers_all_five_targets() {
        let instruments = LyrebirdInstrument::ALL;

        assert_eq!(instruments.len(), 5);
        assert_eq!(
            instruments
                .iter()
                .map(|instrument| instrument.relative_target_spectrogram_path())
                .collect::<Vec<_>>(),
            vec![
                "jungle-examples/examples/lyrebird/assets/guitar_intro_4s.png",
                "jungle-examples/examples/lyrebird/assets/vocals_4s.png",
                "jungle-examples/examples/lyrebird/assets/backup_vocals_4s.png",
                "jungle-examples/examples/lyrebird/assets/bass_4s.png",
                "jungle-examples/examples/lyrebird/assets/guitar_solo_4s.png",
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
                similarity: Some(0.8),
            }),
            latest_rendered_code: Some(DspCode {
                iteration_id: "00000007".to_owned(),
                source: "fn bass() {}".to_owned(),
                sample_path: "/tmp/old.wav".to_owned(),
                spectrogram_path: "/tmp/old.png".to_owned(),
                similarity: Some(0.8),
            }),
            latest_generated_sample_path: Some("/tmp/old.wav".to_owned()),
            latest_generated_spectrogram_path: Some("/tmp/old.png".to_owned()),
            latest_generated_similarity: Some(0.8),
            compile_ready: true,
            prompt_attempt: 3,
            skipped_this_iteration: true,
            last_retry_reason: Some("oops".to_owned()),
            last_similarity: 0.8,
            ..LyrebirdInstrumentState::default()
        };

        state.begin_iteration("/tmp/lyrebird", "00000008");

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

        assert_eq!(appearance.instruments.len(), LyrebirdInstrument::ALL.len());
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
                similarity: Some(0.9),
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
}
