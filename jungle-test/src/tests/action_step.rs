use inception::Inception;
use jungle_types::{
    ActionCompletion, ActionMapper, ActionMapperStep, Awaiting,
    JungleFlowActions, Yielding,
};
use typosaurus::num::consts::U0;

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

animal!(GatherAnimal, U0, instinct = GatherInstinct);

struct GatherMapper;
impl ActionMapper<GatherAnimal, GatherAction> for GatherMapper {
    type In = i32;
    type Out = i32;

    fn map_input(_state: &(), input: Self::In) -> i32 {
        input + 4
    }

    fn map_output(_state: &mut (), output: ActionCompletion<GatherAction>) -> Self::Out {
        output.expect("gather action should succeed")
    }
}

#[derive(Inception)]
#[inception(properties = [JungleFlowActions])]
struct GatherInstinct(ActionMapperStep<GatherAnimal, GatherAction, GatherMapper>);

#[test]
fn action_step_adapts_action_to_temporal_protocol() {
    let (dependency, request) =
        <ActionMapperStep<GatherAnimal, GatherAction, GatherMapper> as Yielding>::run(
            ((), 3),
        );
    assert_eq!(request.into_input(), 7);

    let (next_dependency, emitted) =
        <ActionMapperStep<GatherAnimal, GatherAction, GatherMapper> as Awaiting>::accept(
            (dependency, Ok(9)),
        );
    assert_eq!(emitted, 9);
    assert_eq!(next_dependency, ());
}
