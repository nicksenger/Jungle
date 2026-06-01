use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use jungle_sdk::prelude::*;
use jungle_sdk::{Client, JourneyHandle};
use serde::{Deserialize, Serialize};
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
    /// Run server and worker together in one process.
    Daemon(DaemonArgs),
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

#[derive(Debug, Args)]
struct DaemonArgs {
    #[arg(long, default_value = DEFAULT_SERVER_ADDR)]
    listen: SocketAddr,
    #[arg(long)]
    redb_path: Option<PathBuf>,
    #[arg(long, default_value = DEFAULT_SERVER_NAME)]
    server_name: String,
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
        Command::Daemon(args) => run_daemon(args).await?,
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

async fn run_daemon(args: DaemonArgs) -> Result<(), Box<dyn std::error::Error>> {
    let redb_path = args.redb_path.unwrap_or(default_redb_path()?);
    ensure_parent_dir_exists(&redb_path)?;

    let server_builder = jungle_sdk::server::ServerBuilder::new()
        .listen(args.listen)
        .redb_path(redb_path);
    let mut server_task = tokio::spawn(async move { server_builder.run().await });

    let connection = ConnectionArgs {
        server_addr: args.listen,
        server_name: args.server_name,
    };
    let worker_client = wait_for_client(&connection).await?;
    let worker = JungleWorker::new(PaleozoicEcosystem, worker_client);

    tokio::select! {
        server_outcome = &mut server_task => {
            match server_outcome {
                Ok(Ok(())) => Ok(()),
                Ok(Err(err)) => Err(Box::new(err)),
                Err(err) => Err(Box::new(err)),
            }
        }
        worker_outcome = worker.spawn() => {
            server_task.abort();
            worker_outcome.map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
        }
    }
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

    if args.follow {
        stream_updates(&journey, None).await?;
    }

    Ok(())
}

async fn run_job_list(args: JobListArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args.connection).await?;
    let journeys = client
        .list_journeys(PaleozoicEcosystem::NAME.to_string())
        .await?;
    let cronfish_animal_id = <<Cronfish as Animal>::Id as AnimalIdValue>::U32;
    let mut printed = 0usize;
    for record in journeys {
        if record.animal_id != cronfish_animal_id {
            continue;
        }
        let status = record.status;
        if !args.all && is_terminal(status) {
            continue;
        }
        let seed = postcard::from_bytes::<CronState>(&record.seed).ok();
        let expr = seed
            .as_ref()
            .map(|value| value.expr.as_str())
            .unwrap_or("<seed decode failed>");
        let cmd = seed
            .as_ref()
            .map(|value| value.cmd.as_str())
            .unwrap_or("<seed decode failed>");
        println!("{}\t{:?}\t{}\t{}", record.journey_id, status, expr, cmd);
        printed = printed.saturating_add(1);
    }
    if printed == 0 {
        println!("no jobs found");
    }
    Ok(())
}

async fn run_job_status(args: JobStatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let client = connect_client(&args.connection).await?;
    let status = client
        .journey_details(args.journey_id)
        .await
        .map_err(|err| {
            std::io::Error::other(format!(
                "failed to fetch job status for {}: {}",
                args.journey_id, err
            ))
        })?;
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

async fn wait_for_client(args: &ConnectionArgs) -> Result<Client, Box<dyn std::error::Error>> {
    let mut attempts = 0_u32;
    loop {
        match connect_client(args).await {
            Ok(client) => return Ok(client),
            Err(err) => {
                attempts += 1;
                if attempts >= 50 {
                    return Err(err);
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
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

fn ensure_parent_dir_exists(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
