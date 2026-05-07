use jungle_sdk::types as jungle_types;
use jungle_sdk::types::{
    Action, ActionCompletion, ActionTask, Aspect, Condition, Conditional, Either, Executor,
    Identity, Lens, LoopCondition, Running, Task, Waiting, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U1, U2, U3};
use jungle_sdk::{Instinct, Optic};
use std::future::ready;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

action!(Sleep, U0, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
action!(Eat, U1, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input + 1)));
action!(Forage, U2, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input - 1)));
action!(Hunt, U3, in = i32, out = i32, err = (), act = |_d, input| ready(Ok(input - 1)));

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
struct CoreState {
    energy: i32,
    age: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
struct GorillaState {
    core: CoreState,
    bananas: i32,
}

#[derive(Optic, Clone, Debug, PartialEq, Eq)]
struct TigerState {
    stripes: u8,
    core: CoreState,
}

struct CoreEnergyStep<Focus>(PhantomData<fn() -> Focus>);

impl<T, Focus> Task<T, Sleep> for CoreEnergyStep<Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn process(core: &mut CoreState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        core.energy = value;
        value
    }
}

impl<T, Focus> Task<T, Eat> for CoreEnergyStep<Focus>
where
    T: jungle_types::Creature,
    Focus: Aspect<T::State, View = CoreState>,
{
    type Aspect = Focus;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn process(core: &mut CoreState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

struct GorillaEat;
impl Task<Gorilla, Eat> for GorillaEat {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn process(state: &mut GorillaState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        state.core.energy = value;
        state.bananas -= 1;
        value
    }
}

struct GorillaSleep;
impl Task<Gorilla, Sleep> for GorillaSleep {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn process(state: &mut GorillaState, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        state.core.energy = value;
        state.core.age += 1;
        value
    }
}

struct GorillaForage;
impl Task<Gorilla, Forage> for GorillaForage {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &GorillaState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn process(state: &mut GorillaState, output: ActionCompletion<Forage>) -> Self::Out {
        let value = output.expect("forage should succeed");
        state.core.energy = value;
        state.bananas += 1;
        value
    }
}

struct TigerEat;
impl Task<Tiger, Eat> for TigerEat {
    type Aspect = Lens<TigerState, U1>;
    type In = i32;
    type Out = i32;

    fn prepare(core: &CoreState, input: Self::In) -> i32 {
        core.energy + input
    }

    fn process(core: &mut CoreState, output: ActionCompletion<Eat>) -> Self::Out {
        let value = output.expect("eat should succeed");
        core.energy = value;
        value
    }
}

struct TigerSleep;
impl Task<Tiger, Sleep> for TigerSleep {
    type Aspect = Lens<TigerState, jungle_sdk::typosaurus::list![U1, U0]>;
    type In = i32;
    type Out = i32;

    fn prepare(energy: &i32, input: Self::In) -> i32 {
        *energy + input
    }

    fn process(energy: &mut i32, output: ActionCompletion<Sleep>) -> Self::Out {
        let value = output.expect("sleep should succeed");
        *energy = value;
        value
    }
}

struct TigerHunt;
impl Task<Tiger, Hunt> for TigerHunt {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &TigerState, input: Self::In) -> i32 {
        state.core.energy + input
    }

    fn process(state: &mut TigerState, output: ActionCompletion<Hunt>) -> Self::Out {
        let value = output.expect("hunt should succeed");
        state.core.energy = value;
        state.stripes += 1;
        value
    }
}

#[derive(Instinct)]
struct GorillaLoopSequence(
    ActionTask<Gorilla, Eat, GorillaEat>,
    ActionTask<Gorilla, Sleep, GorillaSleep>,
    ActionTask<Gorilla, Forage, GorillaForage>,
);

struct GorillaUnderAgeHundred;
impl LoopCondition<GorillaState> for GorillaUnderAgeHundred {
    fn should_continue(state: &GorillaState) -> bool {
        state.core.age < 100
    }
}

#[derive(Instinct)]
struct GorillaInstinct(While<GorillaUnderAgeHundred, GorillaLoopSequence>);

struct TigerStripesAreEven;
impl Condition<(TigerState, i32)> for TigerStripesAreEven {
    fn choose((state, _): &(TigerState, i32)) -> bool {
        state.stripes % 2 == 0
    }
}

#[derive(Instinct)]
struct TigerLoopSequence(
    Conditional<
        TigerStripesAreEven,
        ActionTask<Tiger, Eat, TigerEat>,
        ActionTask<Tiger, Sleep, TigerSleep>,
    >,
    ActionTask<Tiger, Sleep, TigerSleep>,
    ActionTask<Tiger, Hunt, TigerHunt>,
);

struct TigerUnderHundredStripes;
impl LoopCondition<TigerState> for TigerUnderHundredStripes {
    fn should_continue(state: &TigerState) -> bool {
        state.stripes < 100
    }
}

#[derive(Instinct)]
struct TigerInstinct(While<TigerUnderHundredStripes, TigerLoopSequence>);

animal!(
    Gorilla,
    U1,
    state = GorillaState,
    instinct = GorillaInstinct
);
animal!(Tiger, U2, state = TigerState, instinct = TigerInstinct);

fn run_now<F: Future>(future: F) -> F::Output {
    fn raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            raw_waker()
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}
        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
        )
    }

    // These test actions resolve immediately (`ready(...)`), so a single poll is enough.
    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    match Future::poll(future.as_mut(), &mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test action future must resolve immediately"),
    }
}

#[test]
fn aspect_step_reuses_focused_mapper_across_animals() {
    let gorilla_state = GorillaState {
        core: CoreState {
            energy: 10,
            age: 25,
        },
        bananas: 3,
    };
    let (gorilla_state, gorilla_request) = <ActionTask<
        Gorilla,
        Sleep,
        CoreEnergyStep<Lens<GorillaState, U0>>,
    > as Running>::run((gorilla_state, 2));
    assert_eq!(gorilla_request.into_input(), 12);
    let (gorilla_state, gorilla_emitted) = <ActionTask<
        Gorilla,
        Sleep,
        CoreEnergyStep<Lens<GorillaState, U0>>,
    > as Waiting>::accept((gorilla_state, Ok(20)));
    assert_eq!(gorilla_emitted, 20);
    assert_eq!(gorilla_state.core.energy, 20);
    assert_eq!(gorilla_state.core.age, 25);
    assert_eq!(gorilla_state.bananas, 3);

    let tiger_state = TigerState {
        stripes: 9,
        core: CoreState { energy: 6, age: 12 },
    };
    let (tiger_state, tiger_request) = <ActionTask<
        Tiger,
        Sleep,
        CoreEnergyStep<Lens<TigerState, U1>>,
    > as Running>::run((tiger_state, 4));
    assert_eq!(tiger_request.into_input(), 10);
    let (tiger_state, tiger_emitted) = <ActionTask<
        Tiger,
        Sleep,
        CoreEnergyStep<Lens<TigerState, U1>>,
    > as Waiting>::accept((tiger_state, Ok(15)));
    assert_eq!(tiger_emitted, 15);
    assert_eq!(tiger_state.core.energy, 15);
    assert_eq!(tiger_state.core.age, 12);
    assert_eq!(tiger_state.stripes, 9);
}

#[test]
fn executor_runs_aspected_steps() {
    let mut gorilla = Executor::<Gorilla>::new(GorillaState {
        core: CoreState { energy: 5, age: 97 },
        bananas: 2,
    });
    assert!(!gorilla.is_complete());

    let mut gorilla_emitted: Vec<i32> = Vec::new();
    for step in 0..8 {
        let request: i32 = gorilla
            .next_request()
            .expect("gorilla request should advance");
        let completion: i32 = match step % 3 {
            0 => run_now(Eat::act(&(), request)).expect("eat should succeed"),
            1 => run_now(Sleep::act(&(), request)).expect("sleep should succeed"),
            2 => run_now(Forage::act(&(), request)).expect("forage should succeed"),
            _ => unreachable!(),
        };
        let emitted: i32 = gorilla
            .complete(Ok::<i32, ()>(completion))
            .expect("gorilla completion should advance");
        gorilla_emitted.push(emitted);
    }
    assert_eq!(gorilla_emitted, vec![6, 13, 25, 51, 103, 205, 411, 823]);
    assert!(gorilla.is_complete());
    let gorilla_state = gorilla.into_state();
    assert_eq!(gorilla_state.core.energy, 823);
    assert_eq!(gorilla_state.core.age, 100);
    assert_eq!(gorilla_state.bananas, 1);

    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });
    assert!(!tiger.is_complete());

    let mut tiger_emitted: Vec<i32> = Vec::new();
    let mut tiger_stripes: u8 = 98;
    for step in 0..6 {
        let request: i32 = tiger.next_request().expect("tiger request should advance");
        let completion: i32 = match step % 3 {
            0 => {
                if tiger_stripes % 2 == 0 {
                    run_now(Eat::act(&(), request)).expect("eat should succeed")
                } else {
                    run_now(Sleep::act(&(), request)).expect("sleep should succeed")
                }
            }
            1 => run_now(Sleep::act(&(), request)).expect("sleep should succeed"),
            2 => run_now(Hunt::act(&(), request)).expect("hunt should succeed"),
            _ => unreachable!(),
        };
        let emitted: i32 = tiger
            .complete(Ok::<i32, ()>(completion))
            .expect("tiger completion should advance");
        tiger_emitted.push(emitted);
        if step % 3 == 2 {
            tiger_stripes += 1;
        }
    }
    assert_eq!(tiger_emitted, vec![9, 19, 37, 75, 151, 301]);
    assert!(tiger.is_complete());
    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 301);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 100);
}

#[test]
fn tiger_first_step_conditional_selects_branch_from_stripe_parity() {
    let even = <Conditional<
        TigerStripesAreEven,
        ActionTask<Tiger, Eat, TigerEat>,
        ActionTask<Tiger, Sleep, TigerSleep>,
    > as Running>::run((
        TigerState {
            stripes: 8,
            core: CoreState { energy: 5, age: 1 },
        },
        0,
    ));
    match even {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 5),
        Either::Right(_) => panic!("expected eat branch"),
    }

    let odd = <Conditional<
        TigerStripesAreEven,
        ActionTask<Tiger, Eat, TigerEat>,
        ActionTask<Tiger, Sleep, TigerSleep>,
    > as Running>::run((
        TigerState {
            stripes: 9,
            core: CoreState { energy: 5, age: 1 },
        },
        0,
    ));
    match odd {
        Either::Left(_) => panic!("expected sleep branch"),
        Either::Right((_state, request)) => assert_eq!(request.into_input(), 5),
    }
}

#[test]
fn executor_advances_with_executable_requests_and_dynamic_action_order() {
    let mut tiger = Executor::<Tiger>::new(TigerState {
        stripes: 98,
        core: CoreState { energy: 8, age: 4 },
    });

    let emitted = run_now(tiger.advance_to_end_with(0i32))
        .expect("tiger flow should execute through dynamic requests");
    assert_eq!(emitted.len(), 6);
    assert!(tiger.is_complete());

    let tiger_state = tiger.into_state();
    assert_eq!(tiger_state.core.energy, 301);
    assert_eq!(tiger_state.core.age, 4);
    assert_eq!(tiger_state.stripes, 100);
}
