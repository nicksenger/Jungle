use jungle_types::{
    Action, ActionCompletion, ActionInputMapper, ActionOutputMapper, ActionStep, Animal, Awaiting,
    Id, Yielding,
};
use typosaurus::num::consts::U0;

use super::ApeInstinct;

struct GatherAction;
impl Action for GatherAction {
    type Id = Id<U0>;
    type Dependency = ();
    type In = i32;
    type Out = i32;
    type Err = ();

    fn act(
        _dependency: &Self::Dependency,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}

struct GatherAnimal;
impl Animal for GatherAnimal {
    type Id = Id<U0>;
    type State = ();
    type Instinct = ApeInstinct;
}

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
