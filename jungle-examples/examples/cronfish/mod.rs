use chrono::Utc;
use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::effect;
use jungle_sdk::prelude::*;
use jungle_sdk::{JungleClient, LocalClient};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;

type CronExpr = String;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct CronState {
    pub expr: CronExpr,
    pub script: String,
}

#[derive(Parser, Debug)]
#[command(name = "cronfish")]
struct Args {
    expr: CronExpr,
    #[arg(long, default_value = "echo \"cronfish fired!\"")]
    script: String,
}

pub struct ApplyCronfishSeed;
#[jungle::action(carry = CronState)]
impl Action for ApplyCronfishSeed {
    type Effect = Noop;
    type Input = CronState;
    type Output = ();

    fn emit(
        _state: &CronState,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, CronState) {
        ((), input)
    }

    fn absorb(
        state: &mut CronState,
        _output: EffectCompletion<Self::Effect>,
        seed: CronState,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = (|| {
            *state = seed;
        })();
        Ok(__absorb_out_1)
    }
}

pub struct CronfishUntilNextFireEffect;
#[effect(id = 0)]
impl<J> Effect<J> for CronfishUntilNextFireEffect {
    type In = CronExpr;
    type Out = Duration;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let result = (|| {
            let schedule = cron::Schedule::from_str(&input)
                .map_err(|err| format!("cron parse failed: {err}"))?;
            let now = Utc::now();
            let next = schedule
                .after(&now)
                .next()
                .ok_or_else(|| "cron schedule has no future fire time".to_string())?;
            let remaining = next.signed_duration_since(now);
            remaining
                .to_std()
                .map_err(|err| format!("invalid fire duration: {err}"))
        })();
        std::future::ready(result)
    }
}

pub struct CronfishUntilNextFire;
#[jungle::action]
impl Action for CronfishUntilNextFire {
    type Effect = CronfishUntilNextFireEffect;
    type Input = ();
    type Output = Duration;

    fn emit(state: &CronState, _input: Self::Input) -> CronExpr {
        state.expr.clone()
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = (|| output.expect("cron duration step should complete"))();
        Ok(__absorb_out_2)
    }
}

pub struct CronfishSleep;
#[jungle::action]
impl Action for CronfishSleep {
    type Effect = Sleep;
    type Input = Duration;
    type Output = ();

    fn emit(_state: &CronState, input: Self::Input) -> Duration {
        input
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = (|| {
            output.expect("cron sleep step should complete");
        })();
        Ok(__absorb_out_3)
    }
}

pub struct CronfishFiredEffect;
#[effect(id = 1)]
impl<J> Effect<J> for CronfishFiredEffect {
    type In = String;
    type Out = ();
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        let result = (|| {
            let status = std::process::Command::new("bash")
                .arg("-lc")
                .arg(&input)
                .status()
                .map_err(|err| format!("failed to run fired script: {err}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("fired script failed with status: {status}"))
            }
        })();
        std::future::ready(result)
    }
}

pub struct CronfishFire;
#[jungle::action]
impl Action for CronfishFire {
    type Effect = CronfishFiredEffect;
    type Input = ();
    type Output = ();

    fn emit(state: &CronState, _input: Self::Input) -> String {
        state.script.clone()
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = (|| {
            output.expect("cron fire script step should complete");
        })();
        Ok(__absorb_out_4)
    }
}

#[derive(Flow)]
pub struct CronfishLoopBody(
    Step<CronfishUntilNextFire>,
    Step<CronfishSleep>,
    Step<CronfishFire>,
);

pub struct CronfishLoopForever;
impl Predicate<(&CronState, &())> for CronfishLoopForever {
    fn eval((_state, _): &(&CronState, &())) -> bool {
        true
    }
}

#[derive(Flow)]
pub struct CronJob(
    Step<ApplyCronfishSeed>,
    While<CronfishLoopForever, CronfishLoopBody>,
);

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = CronState;
    type Seed = CronState;
    type Journey = CronJob;
}

#[derive(Animals)]
pub struct CronfishAnimals(Cronfish);

pub struct PaleozoicEcosystem;
impl Ecosystem for PaleozoicEcosystem {
    const NAME: &'static str = "cronfish-ecosystem";
    type Animals = CronfishAnimals;
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let seed = CronState {
        expr: args.expr,
        script: args.script,
    };

    let client = LocalClient::builder()
        .build()
        .await
        .expect("cronfish local client should build");
    let worker = JungleWorker::new(PaleozoicEcosystem, client.clone());
    tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = postcard::to_allocvec(&seed).expect("cronfish seed should serialize");
    let journey_id = client
        .start_journey::<Cronfish>(seed)
        .await
        .expect("cronfish journey should start");
    println!("cronfish journey started: {journey_id}");

    await_journey_completion(&client, journey_id).await;
}

async fn await_journey_completion(client: &LocalClient, journey_id: uuid::Uuid) {
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
