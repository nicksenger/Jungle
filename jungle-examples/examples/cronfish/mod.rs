use jungle_sdk::prelude::*;
use std::str::FromStr;

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
pub struct CronfishJourney(Step<CronfishTick>);

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
