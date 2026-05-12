//! Reusable flow sections built from testing fixtures.

use crate::testing::adapt::{
    CaptureJoinSum, CaptureSelectWinner, CounterAddOne, CounterAddTwo, CounterStateIsEven,
    RaceFastBranch, RaceSlowBranch, SleepAddAfter, SleepAddBefore, SleepCycleNotComplete,
    SleepCyclePhaseOne, SleepCyclePhaseZero, SleepForStateWake,
};
use jungle_types::{Conditional, Join, Select, Step, While};

pub type CounterBranch<T> =
    Conditional<CounterStateIsEven, Step<T, CounterAddOne>, Step<T, CounterAddTwo>>;

pub type RaceSelect<T> = Select<Step<T, RaceFastBranch>, Step<T, RaceSlowBranch>>;

pub type RaceJoin<T> = Join<Step<T, RaceFastBranch>, Step<T, RaceSlowBranch>>;

pub type RaceSelectThenCaptureWinner<T> = (RaceSelect<T>, Step<T, CaptureSelectWinner>);

pub type RaceJoinThenCaptureSum<T> = (RaceJoin<T>, Step<T, CaptureJoinSum>);

pub type SleepCycleBranch<T> = Conditional<
    SleepCyclePhaseZero,
    Step<T, SleepAddBefore>,
    Conditional<SleepCyclePhaseOne, Step<T, SleepForStateWake>, Step<T, SleepAddAfter>>,
>;

pub type SleepCycleFlow<T> = While<SleepCycleNotComplete, SleepCycleBranch<T>>;
