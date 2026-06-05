use crate::effect::{ParseNext, RunBash};
use crate::{CronExpr, CronState};
use jungle_sdk::prelude::*;
use std::marker::PhantomData;
use std::time::Duration;

pub struct SeedState<Seed, State>(PhantomData<Seed>, PhantomData<State>);
#[jungle::action(carry = Seed)]
impl<Seed, State> Action for SeedState<Seed, State>
where
    Seed: Into<State>,
{
    type Effect = NoEffect;
    type Input = Seed;
    type Output = ();

    fn emit(_state: &State, input: Self::Input) -> (<Self::Effect as EffectSchema>::In, Seed) {
        ((), input)
    }

    fn absorb(
        state: &mut State,
        _output: EffectCompletion<Self::Effect>,
        seed: Seed,
    ) -> Result<Self::Output, Failure> {
        *state = seed.into();
        Ok(())
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
        Ok(output.map_err(|_err| Failure::from("cron sleep step should complete"))?)
    }
}

pub struct Jump;
#[jungle::action]
impl Action for Jump {
    type Effect = RunBash;
    type Input = ();
    type Output = ();

    fn emit(state: &CronState, _input: Self::Input) -> String {
        state.cmd.clone()
    }

    fn absorb(
        _state: &mut CronState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output?;
        Ok(())
    }
}
