use jungle_sdk::prelude::*;
use jungle_sdk::effect;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::{JungleClient, LocalClient};
use std::str::FromStr;
use std::time::Duration;
use chrono::Utc;

pub struct ApplyCronfishSeed;
#[jungle::action(carry = String)]
impl Action for ApplyCronfishSeed {
    type Effect = Noop;
    type Input = String;
    type Output = ();

    fn emit(_state: &String, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, String) {
        ((), input)
    }

    fn absorb(
        state: &mut String,
        _output: EffectCompletion<Self::Effect>,
        seed: String,
    ) -> Self::Output {
        *state = seed;
    }
}

pub struct CronfishUntilNextFireEffect;
#[effect(id = 0)]
impl<J> Effect<J> for CronfishUntilNextFireEffect {
    type In = String;
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

    fn emit(state: &String, _input: Self::Input) -> String {
        state.clone()
    }

    fn absorb(_state: &mut String, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cron duration step should complete")
    }
}

pub struct CronfishSleep;
#[jungle::action]
impl Action for CronfishSleep {
    type Effect = Sleep;
    type Input = Duration;
    type Output = ();

    fn emit(_state: &String, input: Self::Input) -> Duration {
        input
    }

    fn absorb(_state: &mut String, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cron sleep step should complete");
    }
}

pub struct CronfishFire;
#[jungle::action]
impl Action for CronfishFire {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &String, _input: Self::Input) -> () {}

    fn absorb(_state: &mut String, output: EffectCompletion<Self::Effect>) -> Self::Output {
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
impl Predicate<(&String, &())> for CronfishLoopForever {
    fn eval((_state, _): &(&String, &())) -> bool {
        true
    }
}

#[derive(Flow)]
pub struct CronfishJourney(
    Step<ApplyCronfishSeed>,
    While<CronfishLoopForever, CronfishLoopBody>,
);

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = String;
    type Seed = String;
    type Journey = CronfishJourney;
}

#[derive(Animals)]
pub struct CronfishAnimals(Cronfish);

pub struct CronfishEcosystem;
impl Ecosystem for CronfishEcosystem {
    const NAME: &'static str = "cronfish-ecosystem";
    type Animals = CronfishAnimals;
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

#[tokio::main]
async fn main() {
    let seed = std::env::args()
        .nth(1)
        .expect("usage: cronfish \"<cron expression>\"");

    let client = LocalClient::builder()
        .build()
        .await
        .expect("cronfish local client should build");
    let worker = JungleWorker::new(CronfishEcosystem, client.clone());
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
