use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use jungle_sdk::prelude::*;
use jungle_sdk::{Client, JourneyHandle};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use uuid::Uuid;

mod action;
mod effect;
mod flow;

use flow::CronJob;

const DEFAULT_SERVER_ADDR: &str = "[::1]:4433";
const DEFAULT_SERVER_NAME: &str = "localhost";
const DEFAULT_CRONFISH_DIR: &str = ".cronfish";
const DEFAULT_REDB_FILENAME: &str = "db.redb";
const DEFAULT_JOBS_FILENAME: &str = "jobs.toml";

pub(crate) type CronExpr = String;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CronState {
    pub expr: CronExpr,
    pub cmd: String,
}

#[derive(Debug, Parser)]
#[command(name = "cronfish")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run a long-lived transport server.
    Server(ServerArgs),
    /// Run a long-lived worker.
    Worker(ConnectionArgs),
    /// One-off job management commands.
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    /// Stream updates for an existing job.
    Monitor(MonitorArgs),
}

#[derive(Debug, Args)]
struct ServerArgs {
    #[arg(long, default_value = DEFAULT_SERVER_ADDR)]
    listen: SocketAddr,
    #[arg(long)]
    redb_path: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
struct ConnectionArgs {
    #[arg(long, default_value = DEFAULT_SERVER_ADDR)]
    server_addr: SocketAddr,
    #[arg(long, default_value = DEFAULT_SERVER_NAME)]
    server_name: String,
}

#[derive(Debug, Subcommand)]
enum JobCommand {
    /// Start a new cronfish job.
    Start(JobStartArgs),
    /// List known cronfish jobs and their statuses.
    List(JobListArgs),
    /// Show status for a specific job.
    Status(JobStatusArgs),
    /// Mark a job dead.
    Kill(JobKillArgs),
}

#[derive(Debug, Args)]
struct JobStartArgs {
    expr: CronExpr,
    #[arg(long, default_value = "echo \"cronfish jumped!\"")]
    cmd: String,
    #[arg(long)]
    follow: bool,
    #[command(flatten)]
    connection: ConnectionArgs,
}

#[derive(Debug, Args)]
struct JobListArgs {
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    connection: ConnectionArgs,
}

#[derive(Debug, Args)]
struct JobStatusArgs {
    journey_id: Uuid,
    #[command(flatten)]
    connection: ConnectionArgs,
}

#[derive(Debug, Args)]
struct JobKillArgs {
    journey_id: Uuid,
    #[command(flatten)]
    connection: ConnectionArgs,
}

#[derive(Debug, Args)]
struct MonitorArgs {
    journey_id: Uuid,
    #[arg(long)]
    after_sequence_id: Option<u64>,
    #[command(flatten)]
    connection: ConnectionArgs,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct JobRegistry {
    jobs: BTreeMap<String, JobRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobRecord {
    journey_id: Uuid,
    expr: String,
    cmd: String,
}

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = CronState;
    type Seed = CronState;
    type Flow = CronJob;
}

#[derive(Animals)]
pub struct PaleozoicAnimals(Cronfish);

pub struct PaleozoicEcosystem;
impl Ecosystem for PaleozoicEcosystem {
    const NAME: &'static str = "paleozoic-ecosystem";
    type Animals = PaleozoicAnimals;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Server(args) => run_server(args).await?,
        Command::Worker(args) => run_worker(args).await?,
        Command::Job { command } => run_job(command).await?,
        Command::Monitor(args) => run_monitor(args).await?,
    }
    Ok(())
}

async fn run_server(args: ServerArgs) -> Result<(), Box<dyn std::error::Error>> {
    let redb_path = args.redb_path.unwrap_or(default_redb_path()?);
    ensure_parent_dir_exists(&redb_path)?;
    jungle_sdk::server::ServerBuilder::new()
        .listen(args.listen)
        .redb_path(redb_path)
        .run()
        .await?;
    Ok(())
}

async fn run_worker(args: ConnectionArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args).await?;
    let worker = JungleWorker::new(PaleozoicEcosystem, client);
    worker.spawn().await?;
    Ok(())
}

async fn run_job(command: JobCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        JobCommand::Start(args) => run_job_start(args).await,
        JobCommand::List(args) => run_job_list(args).await,
        JobCommand::Status(args) => run_job_status(args).await,
        JobCommand::Kill(args) => run_job_kill(args).await,
    }
}

async fn run_job_start(args: JobStartArgs) -> Result<(), Box<dyn std::error::Error>> {
    let seed = CronState {
        expr: args.expr,
        cmd: args.cmd,
    };
    let client = connect_client(&args.connection).await?;
    let journey = client.spawn::<Cronfish>(&seed).await?;
    println!("started job {}", journey.journey_id);

    let mut registry = load_registry().unwrap_or_default();
    registry.jobs.insert(
        journey.journey_id.to_string(),
        JobRecord {
            journey_id: journey.journey_id,
            expr: seed.expr,
            cmd: seed.cmd,
        },
    );
    save_registry(&registry)?;

    if args.follow {
        stream_updates(&journey, None).await?;
    }

    Ok(())
}

async fn run_job_list(args: JobListArgs) -> Result<(), Box<dyn std::error::Error>> {
    let registry = load_registry().unwrap_or_default();
    if registry.jobs.is_empty() {
        println!("no jobs recorded");
        return Ok(());
    }

    let client = connect_client(&args.connection).await?;
    for record in registry.jobs.values() {
        let status = client.journey_details(record.journey_id).await?;
        if !args.all && is_terminal(status) {
            continue;
        }
        println!(
            "{}\t{:?}\t{}\t{}",
            record.journey_id, status, record.expr, record.cmd
        );
    }
    Ok(())
}

async fn run_job_status(args: JobStatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args.connection).await?;
    let status = client.journey_details(args.journey_id).await?;
    println!("{}\t{:?}", args.journey_id, status);
    Ok(())
}

async fn run_job_kill(args: JobKillArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args.connection).await?;
    client.dead_journey(args.journey_id).await?;
    println!("killed job {}", args.journey_id);
    Ok(())
}

async fn run_monitor(args: MonitorArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args.connection).await?;
    let journey = JourneyHandle::new(args.journey_id, Box::new(client));
    stream_updates(&journey, args.after_sequence_id).await
}

async fn stream_updates(
    journey: &JourneyHandle,
    after_sequence_id: Option<u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut updates = journey.subscribe_step_updates(after_sequence_id).await?;
    while let Some(update) = updates.next().await {
        let update = update?;
        println!("{}\t{:?}", update.sequence_id, update.event);
    }
    let final_status = journey.journey_details().await?;
    println!("final status: {:?}", final_status);
    Ok(())
}

async fn connect_client(args: &ConnectionArgs) -> Result<Client, Box<dyn std::error::Error>> {
    let client = Client::builder()
        .namespace(PaleozoicEcosystem::NAME)
        .remote(args.server_addr)
        .server_name(args.server_name.clone())
        .build()
        .await?;
    Ok(client)
}

fn is_terminal(status: JourneyStatus) -> bool {
    matches!(
        status,
        JourneyStatus::Completed | JourneyStatus::Dead | JourneyStatus::Stopped
    )
}

fn cronfish_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").map_err(|_| io::Error::other("HOME is not set"))?;
    Ok(PathBuf::from(home).join(DEFAULT_CRONFISH_DIR))
}

fn default_redb_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(cronfish_dir()?.join(DEFAULT_REDB_FILENAME))
}

fn default_jobs_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(cronfish_dir()?.join(DEFAULT_JOBS_FILENAME))
}

fn ensure_parent_dir_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn load_registry() -> Result<JobRegistry, Box<dyn std::error::Error>> {
    let path = default_jobs_path()?;
    match fs::read_to_string(&path) {
        Ok(content) => Ok(toml::from_str(&content)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(JobRegistry::default()),
        Err(err) => Err(Box::new(err)),
    }
}

fn save_registry(registry: &JobRegistry) -> Result<(), Box<dyn std::error::Error>> {
    let path = default_jobs_path()?;
    ensure_parent_dir_exists(&path)?;
    let out = toml::to_string_pretty(registry)?;
    fs::write(path, out)?;
    Ok(())
}
