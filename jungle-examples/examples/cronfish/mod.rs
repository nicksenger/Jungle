use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{JungleClient, FusedClient};
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
    const NAME: &'static str = "cronfish-ecosystem";
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

    let journey_id = client
        .spawn::<Cronfish>(&seed)
        .await
        .expect("cronfish journey should start");
    println!("cronfish journey started: {journey_id}");

    await_journey_completion(&client, journey_id).await;
}

async fn await_journey_completion(client: &FusedClient, journey_id: uuid::Uuid) {
    loop {
        let status = client
            .journey_details(journey_id)
            .await
            .expect("cronfish journey details should be available");
        match status {
            JourneyStatus::Completed => break,
            JourneyStatus::Dead | JourneyStatus::Stopped => {
                panic!("cronfish journey reached terminal non-complete status: {status:?}");
            }
            JourneyStatus::Created | JourneyStatus::Alive => {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        }
    }
}
