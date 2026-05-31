use jungle_sdk::prelude::*;
use jungle_sdk::effect;
use std::str::FromStr;

pub struct CronfishSeedPass;
#[effect(id = 0)]
impl<J> Effect<J> for CronfishSeedPass {
    type In = String;
    type Out = String;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct ApplyCronfishSeed;
#[jungle::action]
impl Action for ApplyCronfishSeed {
    type Effect = CronfishSeedPass;
    type Input = String;
    type Output = ();

    fn emit(_state: &String, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(state: &mut String, output: EffectCompletion<Self::Effect>) -> Self::Output {
        *state = output.expect("cronfish seed step should complete");
    }
}

pub struct CronfishTick;
#[jungle::action]
impl Action for CronfishTick {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &String, _input: Self::Input) -> () {}

    fn absorb(_state: &mut String, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cronfish noop step should complete");
    }
}

#[derive(Flow)]
pub struct CronfishJourney(Step<ApplyCronfishSeed>, Step<CronfishTick>);

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = String;
    type Seed = String;
    type Journey = CronfishJourney;
}

fn main() {
    let _schedule = cron::Schedule::from_str("0 */5 * * * * *")
        .expect("example cronfish schedule should parse");
    let _executor = Executor::<Cronfish>::new(String::new());
    println!("cronfish animal initialized");
}
