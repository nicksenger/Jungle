use crate::effect::{ParseNext, RunBash};
use crate::{CronExpr, CronState};
use jungle_sdk::prelude::*;
use std::time::Duration;

pub struct SeedState;
#[jungle::action(carry = CronState)]
impl Action for SeedState {
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
        let __absorb_out_1 = {
            *state = seed;
        };
        Ok(__absorb_out_1)
    }
}

pub struct DetermineNextTick;
#[jungle::action]
impl Action for DetermineNextTick {
    type Effect = ParseNext;
    type Input = ();
    type Output = Duration;

    fn emit(state: &CronState, _input: Self::Input) -> CronExpr {
        state.expr.clone()
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(output?)
    }
}

pub struct SleepFor;
#[jungle::action]
impl Action for SleepFor {
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
        let __absorb_out_3 = {
            output.map_err(|_err| Failure::from("cron sleep step should complete"))?;
        };
        Ok(__absorb_out_3)
    }
}

pub struct Fire;
#[jungle::action]
impl Action for Fire {
    type Effect = RunBash;
    type Input = ();
    type Output = ();

    fn emit(state: &CronState, _input: Self::Input) -> String {
        state.script.clone()
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        Ok(())
    }
}
