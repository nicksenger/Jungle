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
    MockingBirdLoopForever, PrepareDspPatch, RequestDspPatch, ScoreSpectrogram,
    SeedMockingBirdState, SelectDspBranch, SetCurrentInstrument, SkipInstrumentIteration,
    SubmitDspBranch,
};
use crate::tokens::Tool;

const DEFAULT_WORKERS: usize = 3;
const DEFAULT_TREE_DEPTH: usize = 8;
const DEFAULT_LOG_FILTER: &str = "warn,mockingbird=info";
pub(crate) const MOCKINGBIRD_DURATION_SECS: f64 = 4.0;
pub(crate) const MOCKINGBIRD_SCORE_SPEC: &str =
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

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum MockingBirdInstrument {
    #[default]
    RhythmGuitar,
    Vocals,
    BackupVocals,
    Bass,
    GuitarSolo,
}

impl MockingBirdInstrument {
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
            Self::RhythmGuitar => MOCKINGBIRD_SCORE_SPEC,
            Self::Vocals => VOCALS_SCORE_SPEC,
            Self::BackupVocals => BACKUP_VOCALS_SCORE_SPEC,
            Self::Bass => BASS_SCORE_SPEC,
            Self::GuitarSolo => GUITAR_SOLO_SCORE_SPEC,
        }
    }

    pub fn relative_target_spectrogram_path(self) -> &'static str {
        match self {
            Self::RhythmGuitar => "jungle-examples/examples/mockingbird/assets/guitar_intro_4s.png",
            Self::Vocals => "jungle-examples/examples/mockingbird/assets/vocals_4s.png",
            Self::BackupVocals => {
                "jungle-examples/examples/mockingbird/assets/backup_vocals_4s.png"
            }
            Self::Bass => "jungle-examples/examples/mockingbird/assets/bass_4s.png",
            Self::GuitarSolo => "jungle-examples/examples/mockingbird/assets/guitar_solo_4s.png",
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
pub struct MockingBirdInstrumentState {
    pub instrument: MockingBirdInstrument,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
    pub selected_branch: Vec<DspCode>,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub last_similarity: f32,
    pub compile_ready: bool,
    pub prompt_attempt: u32,
    pub skipped_this_iteration: bool,
    pub last_retry_reason: Option<String>,
    pub pending_generated_source: Option<String>,
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

impl MockingBirdInstrumentState {
    fn from_seed(seed: MockingBirdInstrumentSeed) -> Self {
        Self {
            instrument: seed.instrument,
            target_spectrogram_path: seed.target_spectrogram_path,
            dsp_source_path: seed.dsp_source_path,
            initial_dsp_code: seed.initial_dsp_code.clone(),
            selected_branch: vec![seed.initial_dsp_code],
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
        self.pending_generated_source = None;
        self.last_similarity = 0.0;
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState {
    pub output_root: String,
    pub current_instrument: MockingBirdInstrument,
    pub instruments: Vec<MockingBirdInstrumentState>,
    pub iteration: u64,
    pub iteration_id: String,
}

impl MockingBirdState {
    pub fn instrument_state(
        &self,
        instrument: MockingBirdInstrument,
    ) -> &MockingBirdInstrumentState {
        self.instruments
            .iter()
            .find(|state| state.instrument == instrument)
            .expect("mockingbird instrument state missing")
    }

    pub fn instrument_state_mut(
        &mut self,
        instrument: MockingBirdInstrument,
    ) -> &mut MockingBirdInstrumentState {
        self.instruments
            .iter_mut()
            .find(|state| state.instrument == instrument)
            .expect("mockingbird instrument state missing")
    }

    pub fn current_state(&self) -> &MockingBirdInstrumentState {
        self.instrument_state(self.current_instrument)
    }

    pub fn current_state_mut(&mut self) -> &mut MockingBirdInstrumentState {
        self.instrument_state_mut(self.current_instrument)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdInstrumentSeed {
    pub instrument: MockingBirdInstrument,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdSeed {
    pub output_root: String,
    pub instruments: Vec<MockingBirdInstrumentSeed>,
}

impl From<MockingBirdSeed> for MockingBirdState {
    fn from(seed: MockingBirdSeed) -> Self {
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
                .map(MockingBirdInstrumentState::from_seed)
                .collect(),
            ..Self::default()
        }
    }
}

#[derive(Flow)]
pub struct MockingBirdInstrumentPrompt<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Step<SelectDspBranch>,
    Step<BuildPrompt>,
    Step<RequestDspPatch>,
    Step<PrepareDspPatch>,
);

#[derive(Flow)]
pub struct MockingBirdInstrumentScoringBody(Step<ScoreSpectrogram>, Step<SubmitDspBranch>);

#[derive(Flow)]
pub struct MockingBirdInstrumentScoring<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Conditional<
        CurrentInstrumentCompileReady,
        MockingBirdInstrumentScoringBody,
        Step<SkipInstrumentIteration>,
    >,
    Step<FlattenEitherUnit<MockingBirdState>>,
);

pub struct RhythmGuitarMarker;
impl InstrumentMarker for RhythmGuitarMarker {
    const INSTRUMENT: MockingBirdInstrument = MockingBirdInstrument::RhythmGuitar;
}

pub struct VocalsMarker;
impl InstrumentMarker for VocalsMarker {
    const INSTRUMENT: MockingBirdInstrument = MockingBirdInstrument::Vocals;
}

pub struct BackupVocalsMarker;
impl InstrumentMarker for BackupVocalsMarker {
    const INSTRUMENT: MockingBirdInstrument = MockingBirdInstrument::BackupVocals;
}

pub struct BassMarker;
impl InstrumentMarker for BassMarker {
    const INSTRUMENT: MockingBirdInstrument = MockingBirdInstrument::Bass;
}

pub struct GuitarSoloMarker;
impl InstrumentMarker for GuitarSoloMarker {
    const INSTRUMENT: MockingBirdInstrument = MockingBirdInstrument::GuitarSolo;
}

#[derive(Flow)]
pub struct MockingBirdPromptLeft(
    Join<
        MockingBirdInstrumentPrompt<RhythmGuitarMarker>,
        MockingBirdInstrumentPrompt<VocalsMarker>,
    >,
    Step<FlattenJoinedUnit<MockingBirdState>>,
);

#[derive(Flow)]
pub struct MockingBirdPromptRightPair(
    Join<MockingBirdInstrumentPrompt<BackupVocalsMarker>, MockingBirdInstrumentPrompt<BassMarker>>,
    Step<FlattenJoinedUnit<MockingBirdState>>,
);

#[derive(Flow)]
pub struct MockingBirdPromptRight(
    Join<MockingBirdPromptRightPair, MockingBirdInstrumentPrompt<GuitarSoloMarker>>,
    Step<FlattenJoinedUnit<MockingBirdState>>,
);

#[derive(Flow)]
pub struct MockingBirdPromptPhase(
    Join<MockingBirdPromptLeft, MockingBirdPromptRight>,
    Step<FlattenJoinedUnit<MockingBirdState>>,
);

#[derive(Flow)]
pub struct MockingBirdInstrumentCompilation<Marker: InstrumentMarker>(
    Step<SetCurrentInstrument<Marker>>,
    Step<CompilePreparedDspPatch>,
);

#[derive(Flow)]
pub struct MockingBirdIteration(
    Step<BeginIteration>,
    MockingBirdPromptPhase,
    MockingBirdInstrumentCompilation<RhythmGuitarMarker>,
    MockingBirdInstrumentCompilation<VocalsMarker>,
    MockingBirdInstrumentCompilation<BackupVocalsMarker>,
    MockingBirdInstrumentCompilation<BassMarker>,
    MockingBirdInstrumentCompilation<GuitarSoloMarker>,
    Step<FinalizeIterationRender>,
    MockingBirdInstrumentScoring<RhythmGuitarMarker>,
    MockingBirdInstrumentScoring<VocalsMarker>,
    MockingBirdInstrumentScoring<BackupVocalsMarker>,
    MockingBirdInstrumentScoring<BassMarker>,
    MockingBirdInstrumentScoring<GuitarSoloMarker>,
);

#[derive(Flow)]
pub struct MockingBirdJourney(
    Step<SeedMockingBirdState>,
    While<MockingBirdLoopForever, MockingBirdIteration>,
);

pub struct MockingBird;
#[jungle::animal(id = 0, generation = 0, observe)]
impl Animal for MockingBird {
    type State = MockingBirdState;
    type Seed = MockingBirdSeed;
    type Flow = MockingBirdJourney;
}

impl Observe for MockingBird {
    type Appearance = MockingBirdState;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.clone()
    }
}

#[derive(Animals)]
pub struct PulseCodeParadiseAnimals(MockingBird);

#[derive(Clone)]
pub struct PulseCodeParadise {
    client: reqwest::Client,
    db: Arc<redb::Database>,
    db_path: PathBuf,
    runtime_session_id: String,
    tokens_model: String,
    tokens_url: Url,
    tools: Vec<Tool>,
    initial_dsp_codes: BTreeMap<MockingBirdInstrument, DspCode>,
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
            initial_dsp_codes: MockingBirdInstrument::ALL
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
        initial_dsp_codes: impl IntoIterator<Item = (MockingBirdInstrument, DspCode)>,
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
    #[error("failed to build initial mockingbird dsp baseline: {0}")]
    Bootstrap(String),
}

#[derive(Debug, Parser)]
#[command(name = "mockingbird")]
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
    let output_root = default_mockingbird_root()?;
    let jungle_redb_path = cli
        .jungle_redb_path
        .unwrap_or_else(|| output_root.join("jungle.redb"));
    ensure_parent_dir_exists(&jungle_redb_path)?;

    let instrument_seeds = build_instrument_seeds(&workspace_root, &output_root).await?;
    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?
        .with_tools(
            MockingBirdInstrument::ALL
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
        "starting mockingbird runtime"
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
        info!(worker_index, "spawning mockingbird worker");
        worker_handles.push(tokio::spawn(async move {
            let worker = JungleWorker::new(ecosystem, worker_client);
            if let Err(err) = worker.spawn().await {
                warn!(worker_index, error = %err, "mockingbird worker exited");
            }
        }));
    }

    let seed = build_seed(&output_root, &instrument_seeds);
    let journey_id = ensure_mockingbird_running(&client, &seed).await?;

    info!(
        %journey_id,
        jungle_redb_path = %jungle_redb_path.display(),
        mcts_redb_path = %ecosystem.db_path.display(),
        "mockingbird active"
    );

    #[cfg(feature = "viewer")]
    {
        tokio::task::block_in_place(|| ui::run_ui(client.clone(), journey_id))?;
    }

    #[cfg(not(feature = "viewer"))]
    tokio::signal::ctrl_c().await?;
    #[cfg(not(feature = "viewer"))]
    info!("received ctrl-c; shutting down mockingbird workers");
    #[cfg(feature = "viewer")]
    info!("mockingbird viewer closed; shutting down mockingbird workers");

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
) -> Result<Vec<MockingBirdInstrumentSeed>, PulseCodeParadiseError> {
    let mut seeds = Vec::with_capacity(MockingBirdInstrument::ALL.len());
    for instrument in MockingBirdInstrument::ALL {
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
        seeds.push(MockingBirdInstrumentSeed {
            instrument,
            target_spectrogram_path: target_spectrogram_path.display().to_string(),
            dsp_source_path: dsp_source_path.display().to_string(),
            initial_dsp_code,
        });
    }
    Ok(seeds)
}

fn build_seed(output_root: &Path, instruments: &[MockingBirdInstrumentSeed]) -> MockingBirdSeed {
    MockingBirdSeed {
        output_root: output_root.display().to_string(),
        instruments: instruments.to_vec(),
    }
}

fn restore_instrument_sources(
    instruments: &[MockingBirdInstrumentSeed],
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

async fn ensure_mockingbird_running(
    client: &FusedClient,
    seed: &MockingBirdSeed,
) -> Result<Uuid, jungle_sdk::ExecutorError> {
    let journeys = client
        .list_journeys(PulseCodeParadise::NAME.to_owned())
        .await?;
    let mockingbird_animal_id = <<MockingBird as Animal>::Id as AnimalIdValue>::U32;

    if let Some(existing) = journeys
        .into_iter()
        .find(|record| record.animal_id == mockingbird_animal_id && !is_terminal(record.status))
    {
        info!(journey_id = %existing.journey_id, "reusing existing mockingbird journey");
        return Ok(existing.journey_id);
    }

    let journey_id = client.spawn::<MockingBird>(seed).await?.journey_id;
    info!(%journey_id, "spawned new mockingbird journey");
    Ok(journey_id)
}

fn is_terminal(status: JourneyStatus) -> bool {
    matches!(status, JourneyStatus::Completed | JourneyStatus::Dead)
}

fn default_mockingbird_root() -> Result<PathBuf, PulseCodeParadiseError> {
    let base_dirs = BaseDirs::new().ok_or(PulseCodeParadiseError::HomeDirUnavailable)?;
    Ok(base_dirs.home_dir().join(".jungle").join("mockingbird"))
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
    debug!("mockingbird tracing initialized");
}

pub(crate) fn build_replace_tool(instrument: MockingBirdInstrument) -> Tool {
    Tool {
        name: instrument.tool_name().to_owned(),
        description: format!(
            "Replace the full contents of `{}` with updated Rust source.",
            instrument.relative_dsp_path()
        ),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": format!(
                        "The complete replacement Rust source for {}.",
                        Path::new(instrument.relative_dsp_path())
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("the current DSP file")
                    )
                }
            },
            "required": ["source"],
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
    fn mockingbird_instrument_metadata_covers_all_five_targets() {
        let instruments = MockingBirdInstrument::ALL;

        assert_eq!(instruments.len(), 5);
        assert_eq!(
            instruments
                .iter()
                .map(|instrument| instrument.relative_target_spectrogram_path())
                .collect::<Vec<_>>(),
            vec![
                "jungle-examples/examples/mockingbird/assets/guitar_intro_4s.png",
                "jungle-examples/examples/mockingbird/assets/vocals_4s.png",
                "jungle-examples/examples/mockingbird/assets/backup_vocals_4s.png",
                "jungle-examples/examples/mockingbird/assets/bass_4s.png",
                "jungle-examples/examples/mockingbird/assets/guitar_solo_4s.png",
            ]
        );
    }

    #[test]
    fn begin_iteration_preserves_previous_most_recent_outputs() {
        let mut state = MockingBirdInstrumentState {
            instrument: MockingBirdInstrument::Bass,
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
            ..MockingBirdInstrumentState::default()
        };

        state.begin_iteration("/tmp/mockingbird", "00000008");

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
}
