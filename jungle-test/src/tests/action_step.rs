use jungle_sdk::types::{ActionCompletion, ActionStep, AspectStep, Identity, Running, Waiting};
use jungle_sdk::typosaurus::num::consts::U0;
use jungle_sdk::Flow;
use std::future::ready;

action!(
    GatherAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

animal!(GatherCreature, U0, instinct = GatherInstinct);

struct Gather;
impl AspectStep<GatherCreature, GatherAction> for Gather {
    type Aspect = Identity;
    type In = i32;
    type Out = i32;

    fn prepare(_state: &(), input: Self::In) -> i32 {
        input + 4
    }

    fn apply(_state: &mut (), output: ActionCompletion<GatherAction>) -> Self::Out {
        output.expect("gather action should succeed")
    }
}

#[derive(Flow)]
struct GatherInstinct(ActionStep<GatherCreature, GatherAction, Gather>);

#[test]
fn action_step_adapts_action() {
    let (dependency, request) =
        <ActionStep<GatherCreature, GatherAction, Gather> as Running>::run(((), 3));
    assert_eq!(request.into_input(), 7);

    let (next_dependency, emitted) =
        <ActionStep<GatherCreature, GatherAction, Gather> as Waiting>::accept((dependency, Ok(9)));
    assert_eq!(emitted, 9);
    assert_eq!(next_dependency, ());
}
