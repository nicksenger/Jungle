use jungle_types::{
    Action, ActionCompletion, ActionInputMapper, ActionOutputMapper, ActionStep, Animal, Awaiting,
    Id, Yielding,
};
use typosaurus::num::consts::U0;

use super::ApeInstinct;

action!(
    GatherAction,
    U0,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| {
        std::future::ready(Ok(input + 1))
    }
);

animal!(GatherAnimal, U0, instinct = ApeInstinct);

struct PrepareGather;
impl ActionInputMapper<GatherAnimal, GatherAction> for PrepareGather {
    type In = i32;

    fn map_input(&self, _state: &(), input: Self::In) -> i32 {
        input + 4
    }
}

struct ApplyGather;
impl ActionOutputMapper<GatherAnimal, GatherAction> for ApplyGather {
    type Out = i32;

    fn map_output(&self, _state: &mut (), output: ActionCompletion<GatherAction>) -> Self::Out {
        output.expect("gather action should succeed")
    }
}

#[test]
fn action_step_adapts_action_to_temporal_protocol() {
    let step = ActionStep::<GatherAnimal, GatherAction, PrepareGather, ApplyGather>::new(
        PrepareGather,
        ApplyGather,
    );
    let (dependency, request) = step.run(((), 3));
    assert_eq!(request.into_input(), 7);

    let apply_step = ActionStep::<GatherAnimal, GatherAction, PrepareGather, ApplyGather>::new(
        PrepareGather,
        ApplyGather,
    );
    let (next_dependency, emitted) = apply_step.accept((dependency, Ok(9)));
    assert_eq!(emitted, 9);
    assert_eq!(next_dependency, ());
}
