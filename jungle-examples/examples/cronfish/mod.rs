use jungle_sdk::prelude::*;
use jungle_sdk::effect;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::{JungleClient, LocalClient};
use std::str::FromStr;
use std::time::Duration;
use chrono::Utc;

type CronExpr = String;

pub struct ApplyCronfishSeed;
#[jungle::action(carry = CronExpr)]
impl Action for ApplyCronfishSeed {
    type Effect = Noop;
    type Input = CronExpr;
    type Output = ();

    fn emit(
        _state: &CronExpr,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, CronExpr) {
        ((), input)
    }

    fn absorb(
        state: &mut CronExpr,
        _output: EffectCompletion<Self::Effect>,
        seed: CronExpr,
    ) -> Self::Output {
        *state = seed;
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

    fn emit(state: &CronExpr, _input: Self::Input) -> CronExpr {
        state.clone()
    }

    fn absorb(_state: &mut CronExpr, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cron duration step should complete")
    }
}

pub struct CronfishSleep;
#[jungle::action]
impl Action for CronfishSleep {
    type Effect = Sleep;
    type Input = Duration;
    type Output = ();

    fn emit(_state: &CronExpr, input: Self::Input) -> Duration {
        input
    }

    fn absorb(_state: &mut CronExpr, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cron sleep step should complete");
    }
}

pub struct CronfishFire;
#[jungle::action]
impl Action for CronfishFire {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &CronExpr, _input: Self::Input) -> () {}

    fn absorb(_state: &mut CronExpr, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cron fire print step should complete");
        println!("fired!");
    }
}

#[derive(Flow)]
pub struct CronfishLoopBody(
    Step<CronfishUntilNextFire>,
    Step<CronfishSleep>,
    Step<CronfishFire>,
);

pub struct CronfishLoopForever;
impl Predicate<(&CronExpr, &())> for CronfishLoopForever {
    fn eval((_state, _): &(&CronExpr, &())) -> bool {
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
    type State = CronExpr;
    type Seed = CronExpr;
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
    let seed: CronExpr = std::env::args()
        .nth(1)
        .expect("usage: cronfish \"<cron expression>\"");

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
