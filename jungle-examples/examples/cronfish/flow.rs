use crate::action::{ApplyCronfishSeed, CronfishFire, CronfishSleep, CronfishUntilNextFire};
use crate::CronState;
use jungle_sdk::prelude::*;

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
