use crate::action::{DetermineNextTick, Fire, SeedState, SleepFor};
use crate::CronState;
use jungle_sdk::prelude::*;

#[derive(Flow)]
pub struct CronfishLoopBody(
    Step<DetermineNextTick>,
    Step<SleepFor>,
    Step<Fire>,
);

pub struct CronfishLoopForever;
impl Predicate<(&CronState, &())> for CronfishLoopForever {
    fn eval((_state, _): &(&CronState, &())) -> bool {
        true
    }
}

#[derive(Flow)]
pub struct CronJob(
    Step<SeedState>,
    While<CronfishLoopForever, CronfishLoopBody>,
);
