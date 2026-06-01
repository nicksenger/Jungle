use clap::{Parser, Subcommand};
use futures::StreamExt;
use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};

mod action;
mod effect;
mod flow;

use flow::CronJob;

pub(crate) type CronExpr = String;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CronState {
    pub expr: CronExpr,
    pub cmd: String,
}

#[derive(Parser, Debug)]
#[command(name = "cronfish")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Job(JobArgs),
    Monitor(JobArgs),
}

#[derive(Parser, Debug)]
struct JobArgs {
    expr: CronExpr,
    #[arg(long, default_value = "echo \"cronfish jumped!\"")]
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
async fn main() {
    let args = Cli::parse();
    let job = match &args.command {
        Command::Job(job) | Command::Monitor(job) => job,
    };
    let seed = CronState {
        expr: job.expr.clone(),
        cmd: job.cmd.clone(),
    };

    let client = FusedClient::builder()
        .build()
        .await
        .expect("cronfish local client should build");
    let worker = JungleWorker::new(PaleozoicEcosystem, client.clone());
    tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let journey = client
        .spawn::<Cronfish>(&seed)
        .await
        .expect("cronfish journey should start");
    println!("cronfish journey started: {}", journey.journey_id);

    if matches!(args.command, Command::Monitor(_)) {
        let mut updates = journey
            .subscribe_step_updates(None)
            .await
            .expect("cronfish monitor subscription should start");
        while let Some(update) = updates.next().await {
            let update = update.expect("cronfish monitor update should decode");
            println!(
                "cronfish update {}: {:?}",
                update.sequence_id, update.event
            );
        }
    }
}
