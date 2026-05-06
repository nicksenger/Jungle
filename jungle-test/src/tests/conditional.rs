use jungle_sdk::{Flow, Jungle};
use jungle_types::{
    ActionCompletion, ActionStep, AspectStep, Condition, Conditional, Either, Identity, Running,
    Waiting,
};
use typosaurus::num::consts::{U0, U1};

action!(
    LeftAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| {
        std::future::ready(Ok(input + 1))
    }
);

action!(
    RightAction,
    U1,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| {
        std::future::ready(Ok(input + 2))
    }
);

animal!(ConditionalCreature, U0, state = i32, instinct = ConditionalInstinct);

struct LeftMapper;
impl AspectStep<ConditionalCreature, LeftAction> for LeftMapper {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state + input
    }

    fn apply(state: &mut i32, output: ActionCompletion<LeftAction>) -> Self::Out {
        let value = output.expect("left action should succeed");
        *state = value;
        value
    }
}

struct RightMapper;
impl AspectStep<ConditionalCreature, RightAction> for RightMapper {
    type Aspect = Identity;
    type In = i32;
    type Out = bool;

    fn prepare(state: &i32, input: Self::In) -> i32 {
        *state - input
    }

    fn apply(state: &mut i32, output: ActionCompletion<RightAction>) -> Self::Out {
        let value = output.expect("right action should succeed");
        *state = value;
        value % 2 == 0
    }
}

type LeftFlow = ActionStep<ConditionalCreature, LeftAction, LeftMapper>;
type RightFlow = ActionStep<ConditionalCreature, RightAction, RightMapper>;

struct PreferLeftWhenStateIsNonNegative;
impl Condition<(i32, i32)> for PreferLeftWhenStateIsNonNegative {
    fn choose((state, _): &(i32, i32)) -> bool {
        *state >= 0
    }
}

type ConditionalFlow = Conditional<PreferLeftWhenStateIsNonNegative, LeftFlow, RightFlow>;

#[derive(Jungle, Flow)]
struct ConditionalInstinct(ConditionalFlow);

#[test]
fn conditional_run_selects_branch_from_predicate() {
    let left = <ConditionalFlow as Running>::run((5, 3));
    match left {
        Either::Left((_state, request)) => assert_eq!(request.into_input(), 8),
        Either::Right(_) => panic!("expected left branch"),
    }

    let right = <ConditionalFlow as Running>::run((-2, 3));
    match right {
        Either::Left(_) => panic!("expected right branch"),
        Either::Right((_state, request)) => assert_eq!(request.into_input(), -5),
    }
}

#[test]
fn conditional_waiting_accept_returns_either_branch_output() {
    let left = <ConditionalFlow as Waiting>::accept(Either::Left((1, Ok(9))));
    match left {
        Either::Left((state, emitted)) => {
            assert_eq!(state, 9);
            assert_eq!(emitted, 9);
        }
        Either::Right(_) => panic!("expected left output"),
    }

    let right = <ConditionalFlow as Waiting>::accept(Either::Right((1, Ok(6))));
    match right {
        Either::Left(_) => panic!("expected right output"),
        Either::Right((state, emitted)) => {
            assert_eq!(state, 6);
            assert!(emitted);
        }
    }
}
