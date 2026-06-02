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
use uuid::Uuid;

mod action;
mod effect;
pub mod mcts;
pub mod tokens;

use crate::action::{
    ApplyDspPatch, BeginIteration, BuildPrompt, MockingBirdCompilePending,
    MockingBirdLoopForever, RenderSample, RenderSpectrogram, RequestDspPatch,
    ScoreSpectrogram, SeedMockingBirdState,
};
use crate::tokens::Tool;

const DEFAULT_WORKERS: usize = 3;
pub(crate) const MOCKINGBIRD_DURATION_SECS: f64 = 4.0;
pub(crate) const MOCKINGBIRD_DSP_TOOL_NAME: &str = "replace_electric_guitar_dsp";
pub(crate) const RELATIVE_TARGET_SPECTROGRAM_PATH: &str =
    "jungle-examples/examples/mockingbird/assets/guitar_intro_4s.png";
pub(crate) const RELATIVE_ELECTRIC_GUITAR_DSP_PATH: &str =
    "jungle-examples/examples/welcome/audio/src/dsp/electric_guitar.rs";
pub(crate) const MOCKINGBIRD_SCORE_SPEC: &str =
    "electric-guitar(sustained):[350,58,96],[350,58,96],[446,58,96],[542,58,96],[542,58,96],[638,56,96],[638,56,96],[734,56,96],[830,56,96],[830,56,96],[926,53,96],[926,53,96],[1022,53,96],[1118,53,96],[1118,53,96],[1214,51,96],[1214,51,96],[1310,51,96],[1406,51,96],[1406,51,96],[1502,49,96],[1502,49,96],[1598,49,96],[1694,46,96],[1694,49,96],[1694,46,96],[1790,49,96],[1790,46,96],[1886,58,96],[1886,58,96],[1982,58,96],[2078,58,96],[2078,58,96],[2174,56,96],[2174,56,96],[2270,56,96],[2366,56,96],[2366,56,96],[2462,53,96],[2462,53,96],[2558,53,96],[2654,53,96],[2654,53,96],[2750,51,96],[2750,51,96],[2846,51,96]";

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState {
    pub output_root: String,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
    pub iteration: u64,
    pub iteration_id: String,
    pub sample_path: String,
    pub spectrogram_path: String,
    pub last_similarity: f32,
    pub compile_ready: bool,
    pub prompt_attempt: u32,
    pub last_retry_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdSeed {
    pub output_root: String,
    pub target_spectrogram_path: String,
    pub dsp_source_path: String,
}

impl From<MockingBirdSeed> for MockingBirdState {
    fn from(seed: MockingBirdSeed) -> Self {
        Self {
            output_root: seed.output_root,
            target_spectrogram_path: seed.target_spectrogram_path,
            dsp_source_path: seed.dsp_source_path,
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
    Step<RenderSample>,
    Step<RenderSpectrogram>,
    Step<ScoreSpectrogram>,
    While<MockingBirdCompilePending, MockingBirdCompileLoop>,
);

#[derive(Flow)]
pub struct MockingBirdJourney(
    Step<SeedMockingBirdState>,
    While<MockingBirdLoopForever, MockingBirdIteration>,
);

pub struct MockingBird;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for MockingBird {
    type State = MockingBirdState;
    type Seed = MockingBirdSeed;
    type Flow = MockingBirdJourney;
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
        })
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
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
}

#[derive(Debug, Parser)]
#[command(name = "mockingbird")]
struct Cli {
    #[arg(long = "tokens-url")]
    tokens_url: Url,
    #[arg(long = "tokens-token")]
    tokens_token: Option<String>,
    #[arg(long = "db-path")]
    db_path: Option<PathBuf>,
    #[arg(long = "jungle-redb-path")]
    jungle_redb_path: Option<PathBuf>,
    #[arg(long = "workers", default_value_t = DEFAULT_WORKERS, value_parser = parse_workers)]
    workers: usize,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let workspace_root = std::env::current_dir()?;
    let output_root = default_mockingbird_root()?;
    let jungle_redb_path = cli
        .jungle_redb_path
        .unwrap_or_else(|| output_root.join("jungle.redb"));
    ensure_parent_dir_exists(&jungle_redb_path)?;

    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?
        .with_tools(vec![replace_electric_guitar_tool()]);

    let backend = Server::builder().redb_path(&jungle_redb_path).build().await?;
    let client = FusedClient::builder()
        .namespace(PulseCodeParadise::NAME)
        .backend(backend)
        .build()
        .await?;

    let mut worker_handles = Vec::with_capacity(cli.workers);
    for worker_index in 0..cli.workers {
        let ecosystem = ecosystem.clone();
        let worker_client = client.clone();
        worker_handles.push(tokio::spawn(async move {
            let worker = JungleWorker::new(ecosystem, worker_client);
            if let Err(err) = worker.spawn().await {
                eprintln!("mockingbird worker {worker_index} exited: {err}");
            }
        }));
    }

    let seed = build_seed(&workspace_root, &output_root);
    let journey_id = ensure_mockingbird_running(&client, &seed).await?;

    eprintln!(
        "mockingbird active on journey {} using jungle redb {} and mcts redb {}",
        journey_id,
        jungle_redb_path.display(),
        ecosystem.db_path.display()
    );

    tokio::signal::ctrl_c().await?;

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

fn build_seed(workspace_root: &Path, output_root: &Path) -> MockingBirdSeed {
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

    if let Some(existing) = journeys.into_iter().find(|record| {
        record.animal_id == mockingbird_animal_id && !is_terminal(record.status)
    }) {
        return Ok(existing.journey_id);
    }

    Ok(client.spawn::<MockingBird>(seed).await?.journey_id)
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

fn parse_workers(value: &str) -> Result<usize, String> {
    let workers = value
        .parse::<usize>()
        .map_err(|_| format!("invalid workers argument: {value}"))?;
    if workers == 0 {
        return Err("workers must be at least 1".to_owned());
    }
    Ok(workers)
}
