use crate::action::{DetermineNextTick, Jump, SeedState, SleepFor};
use crate::CronState;
use jungle_sdk::prelude::*;

type SeedCronState = SeedState<CronState, CronState>;

#[derive(Flow)]
pub struct CronfishLoopBody(Step<DetermineNextTick>, Step<SleepFor>, Step<Jump>);

pub struct CronfishLoopForever;
impl Predicate<(&CronState, &())> for CronfishLoopForever {
    fn eval((_state, _): &(&CronState, &())) -> bool {
        true
    }
}

#[derive(Flow)]
pub struct CronJob(
    Step<SeedCronState>,
    While<CronfishLoopForever, CronfishLoopBody>,
);
