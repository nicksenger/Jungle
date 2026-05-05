use inception::Inception;
use jungle_types::{
    ActionCompletion, ActionStep, AspectStep, Waiting, JungleFlow, Identity, Running,
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

animal!(GatherCreature, U0, instinct = GatherInstinct);

struct GatherMapper;
impl AspectStep<GatherCreature, GatherAction> for GatherMapper {
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

#[derive(Inception)]
#[inception(properties = [JungleFlow])]
struct GatherInstinct(ActionStep<GatherCreature, GatherAction, GatherMapper>);

#[test]
fn action_step_adapts_action() {
    let (dependency, request) =
        <ActionStep<GatherCreature, GatherAction, GatherMapper> as Running>::run(((), 3));
    assert_eq!(request.into_input(), 7);

    let (next_dependency, emitted) =
        <ActionStep<GatherCreature, GatherAction, GatherMapper> as Waiting>::accept((
            dependency,
            Ok(9),
        ));
    assert_eq!(emitted, 9);
    assert_eq!(next_dependency, ());
}
