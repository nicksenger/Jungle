use clap::Parser;
use directories_next::BaseDirs;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, JourneyStatus, JungleClient, Server};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    ApplyDspPatch, BeginIteration, BuildPrompt, MockingBirdCompilePending, MockingBirdLoopForever,
    RenderSample, RenderSpectrogram, RequestDspPatch, ScoreSpectrogram, SeedMockingBirdState,
    SelectDspBranch, SubmitDspBranch,
};
use crate::tokens::Tool;

const DEFAULT_WORKERS: usize = 3;
const DEFAULT_TREE_DEPTH: usize = 8;
const DEFAULT_LOG_FILTER: &str = "warn,mockingbird=info";
pub(crate) const MOCKINGBIRD_DURATION_SECS: f64 = 4.0;
pub(crate) const MOCKINGBIRD_DSP_TOOL_NAME: &str = "replace_electric_guitar_dsp";
pub(crate) const RELATIVE_TARGET_SPECTROGRAM_PATH: &str =
    "jungle-examples/examples/mockingbird/assets/guitar_intro_4s.png";
pub(crate) const RELATIVE_ELECTRIC_GUITAR_DSP_PATH: &str =
    "jungle-examples/examples/welcome/audio/src/dsp/electric_guitar.rs";
pub(crate) const MOCKINGBIRD_SCORE_SPEC: &str =
    "electric-guitar(sustained):[350,58,96],[350,58,96],[446,58,96],[542,58,96],[542,58,96],[638,56,96],[638,56,96],[734,56,96],[830,56,96],[830,56,96],[926,53,96],[926,53,96],[1022,53,96],[1118,53,96],[1118,53,96],[1214,51,96],[1214,51,96],[1310,51,96],[1406,51,96],[1406,51,96],[1502,49,96],[1502,49,96],[1598,49,96],[1694,46,96],[1694,49,96],[1694,46,96],[1790,49,96],[1790,46,96],[1886,58,96],[1886,58,96],[1982,58,96],[2078,58,96],[2078,58,96],[2174,56,96],[2174,56,96],[2270,56,96],[2366,56,96],[2366,56,96],[2462,53,96],[2462,53,96],[2558,53,96],[2654,53,96],[2654,53,96],[2750,51,96],[2750,51,96],[2846,51,96]";

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

pub struct MockingBirdMctsTag;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState {
    pub output_root: String,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
    pub selected_branch: Vec<DspCode>,
    pub iteration: u64,
    pub iteration_id: String,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub last_similarity: f32,
    pub compile_ready: bool,
    pub prompt_attempt: u32,
    pub last_retry_reason: Option<String>,
    pub latest_generated_code: Option<DspCode>,
    pub latest_generated_sample_path: Option<String>,
    pub latest_generated_spectrogram_path: Option<String>,
    pub latest_generated_similarity: Option<f32>,
    pub best_generated_sample_path: Option<String>,
    pub best_generated_spectrogram_path: Option<String>,
    pub best_similarity: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdSeed {
    pub output_root: String,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub initial_dsp_code: DspCode,
}

impl From<MockingBirdSeed> for MockingBirdState {
    fn from(seed: MockingBirdSeed) -> Self {
        Self {
            output_root: seed.output_root,
            target_spectrogram_path: seed.target_spectrogram_path,
            dsp_source_path: seed.dsp_source_path,
            initial_dsp_code: seed.initial_dsp_code.clone(),
            selected_branch: vec![seed.initial_dsp_code],
            ..Self::default()
        }
    }
}

#[derive(Flow)]
pub struct MockingBirdCompileLoop(
    Step<BuildPrompt>,
    Step<RequestDspPatch>,
    Step<ApplyDspPatch>,
);

#[derive(Flow)]
pub struct MockingBirdIteration(
    Step<BeginIteration>,
    Step<SelectDspBranch>,
    While<MockingBirdCompilePending, MockingBirdCompileLoop>,
    Step<RenderSample>,
    Step<RenderSpectrogram>,
    Step<ScoreSpectrogram>,
    Step<SubmitDspBranch>,
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
    tokens_model: String,
    tokens_url: Url,
    tools: Vec<Tool>,
    initial_dsp_code: DspCode,
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
            tokens_model: Self::tokens_model_from_env(),
            tokens_url,
            tools: Vec::new(),
            initial_dsp_code: DspCode::placeholder_initial(),
            max_tree_depth: DEFAULT_TREE_DEPTH,
        })
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_mcts_config(mut self, initial_dsp_code: DspCode, max_tree_depth: usize) -> Self {
        self.initial_dsp_code = initial_dsp_code;
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
    let dsp_source_path = workspace_root.join(RELATIVE_ELECTRIC_GUITAR_DSP_PATH);
    let target_spectrogram_path = workspace_root.join(RELATIVE_TARGET_SPECTROGRAM_PATH);
    let initial_dsp_code = effect::capture_current_dsp_code_snapshot(
        "initial",
        &output_root,
        &target_spectrogram_path,
        &dsp_source_path,
    )
    .await
    .map_err(PulseCodeParadiseError::Bootstrap)?;

    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?
        .with_tools(vec![replace_electric_guitar_tool()])
        .with_mcts_config(initial_dsp_code.clone(), cli.tree_depth);
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

    let seed = build_seed(&workspace_root, &output_root, initial_dsp_code);
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

    Ok(())
}

fn replace_electric_guitar_tool() -> Tool {
    Tool {
        name: MOCKINGBIRD_DSP_TOOL_NAME.to_owned(),
        description: format!(
            "Replace the full contents of `{}` with updated Rust source.",
            RELATIVE_ELECTRIC_GUITAR_DSP_PATH
        ),
        parameters: json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "The complete replacement Rust source for electric_guitar.rs."
                }
            },
            "required": ["source"],
            "additionalProperties": false
        }),
    }
}

fn build_seed(
    workspace_root: &Path,
    output_root: &Path,
    initial_dsp_code: DspCode,
) -> MockingBirdSeed {
    MockingBirdSeed {
        output_root: output_root.display().to_string(),
        target_spectrogram_path: workspace_root
            .join(RELATIVE_TARGET_SPECTROGRAM_PATH)
            .display()
            .to_string(),
        dsp_source_path: workspace_root
            .join(RELATIVE_ELECTRIC_GUITAR_DSP_PATH)
            .display()
            .to_string(),
        initial_dsp_code,
    }
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

        match err {
            PulseCodeParadiseError::InvalidTokensUrl { reason, .. } => {
                assert!(reason.contains("chat completions endpoint"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_query_parameters_in_tokens_url() {
        let url = Url::parse("https://api.openai.com/v1?model=gpt-5").unwrap();
        let err = validate_openai_api_base_url(&url).unwrap_err();

        match err {
            PulseCodeParadiseError::InvalidTokensUrl { reason, .. } => {
                assert!(reason.contains("query parameters"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
