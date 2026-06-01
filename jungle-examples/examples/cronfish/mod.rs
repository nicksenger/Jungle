use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{JungleClient, FusedClient};
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
struct Args {
    expr: CronExpr,
    #[arg(long, default_value = "echo \"cronfish fired!\"")]
    cmd: String,
}

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = CronState;
    type Seed = CronState;
    type Journey = CronJob;
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
    let args = Args::parse();
    let seed = CronState {
        expr: args.expr,
        cmd: args.cmd,
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

    let final_status = journey
        .await_completion()
        .await
        .expect("cronfish journey completion wait should succeed");
    if !matches!(final_status, JourneyStatus::Completed) {
        panic!("cronfish journey reached terminal non-complete status: {final_status:?}");
    }
}
