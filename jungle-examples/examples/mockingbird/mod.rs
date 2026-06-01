use clap::Parser;
use jungle_sdk::prelude::*;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

mod effect;
pub mod mcts;
pub mod tokens;

use crate::tokens::Tool;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState;

pub type MockingBirdSeed = MockingBirdState;

pub struct MockingBirdIdle;
#[jungle::action]
impl Action for MockingBirdIdle {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &MockingBirdState, _input: Self::Input) -> Duration {
        Duration::from_millis(0)
    }

    fn absorb(
        _state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("mockingbird idle stub should succeed"))?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct MockingBirdJourney(Step<MockingBirdIdle>);

pub struct MockingBird;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for MockingBird {
    type State = MockingBirdState;
    type Seed = MockingBirdSeed;
    type Flow = MockingBirdJourney;
}

#[derive(Animals)]
pub struct PulseCodeParadiseAnimals(MockingBird);

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
}

fn main() -> Result<(), PulseCodeParadiseError> {
    let cli = Cli::parse();
    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?;

    eprintln!(
        "mockingbird example stub configured for {} with mcts db {}",
        ecosystem.chat_completions_endpoint(),
        ecosystem.db_path.display()
    );

    Ok(())
}
