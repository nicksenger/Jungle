use jungle_sdk::prelude::*;
use std::str::FromStr;

#[derive(Default)]
pub struct CronfishState(pub Option<cron::Schedule>);

pub struct CronfishTick;
#[jungle::action]
impl Action for CronfishTick {
    type Effect = Noop;
    type Input = ();
    type Output = ();

    fn emit(_state: &CronfishState, _input: Self::Input) -> () {}

    fn absorb(_state: &mut CronfishState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        output.expect("cronfish noop step should complete");
    }
}

#[derive(Flow)]
pub struct CronfishJourney(Step<CronfishTick>);

pub struct Cronfish;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for Cronfish {
    type State = CronfishState;
    type Seed = ();
    type Journey = CronfishJourney;
}

fn main() {
    let mut state = CronfishState::default();
    state.0 = Some(
        cron::Schedule::from_str("0 */5 * * * * *")
            .expect("example cronfish schedule should parse"),
    );
    let _executor = Executor::<Cronfish>::new(state);
    println!("cronfish animal initialized");
}
