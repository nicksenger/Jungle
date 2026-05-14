use jungle_sdk::types::{
    ActionCompletion, Conditional, Executor, Identity, Join, JourneyAst, JourneyAstSource,
    ManualExecutor, NodeMetadata, Pulse, Select, Step, Transparent, While,
};
use jungle_sdk::typosaurus::num::consts::{U30, U31};
use std::future::ready;

action!(
    TransparentAction,
    U30,
    in = i32,
    out = i32,
    err = (),
    act = |_dependency, input| ready(Ok(input + 1))
);

animal!(
    TransparentAnimal,
    U31,
    state = i32,
    journey = TransparentJourney
);

struct TransparentStep;
impl Pulse<TransparentAnimal> for TransparentStep {
    type Action = TransparentAction;
    type Aspect = Identity;
    type CarryIn = i32;
    type CarryOut = i32;

    fn emit(state: &i32, input: Self::CarryIn) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: ActionCompletion<TransparentAction>) -> Self::CarryOut {
        let value = output.expect("transparent step action should succeed");
        *state = value;
        value
    }
}

struct FlowSectionMetadata;
impl NodeMetadata for FlowSectionMetadata {
    const METADATA: &'static str = "section:checkout/preflight";
}

type BaseFlow = Step<TransparentAnimal, TransparentStep>;
type TransparentFlow = Transparent<FlowSectionMetadata, BaseFlow>;

#[derive(jungle_sdk::Journey)]
struct TransparentJourney(TransparentFlow);

#[test]
fn transparent_flow_runs_as_passthrough_boundary() {
    let mut manual = ManualExecutor::<TransparentAnimal>::new(2);
    let request: i32 = manual
        .next_request_typed::<_, i32>(3)
        .expect("transparent flow should emit request from wrapped step");
    assert_eq!(request, 5);

    let emitted: i32 = manual
        .complete_typed::<i32, (), i32>(Ok(6))
        .expect("transparent flow should absorb wrapped completion");
    assert_eq!(emitted, 6);
    assert!(manual.is_complete());
    assert_eq!(manual.into_state(), 6);

    let mut typed = Executor::<TransparentAnimal>::new(1);
    let request: i32 = typed.next_request().expect("typed executor request");
    assert_eq!(request, 1);
    let emitted: i32 = typed
        .complete(Ok::<i32, ()>(2))
        .expect("typed executor completion");
    assert_eq!(emitted, 2);
}

#[test]
fn transparent_flow_exposes_custom_metadata() {
    assert_eq!(
        <TransparentFlow as NodeMetadata>::METADATA,
        "section:checkout/preflight"
    );
    assert_eq!(<BaseFlow as NodeMetadata>::METADATA, "");
}

struct AnnotatedNonTransparentStep;

impl NodeMetadata for AnnotatedNonTransparentStep {
    const METADATA: &'static str = "node:custom/non-transparent-step";
}

#[test]
fn non_transparent_node_can_customize_metadata() {
    assert_eq!(
        <AnnotatedNonTransparentStep as NodeMetadata>::METADATA,
        "node:custom/non-transparent-step"
    );
}

struct ControlMetadata;
impl NodeMetadata for ControlMetadata {
    const METADATA: &'static str = "control:branching";
}

type MetadataConditionalFlow =
    Conditional<AnnotatedNonTransparentStep, BaseFlow, BaseFlow, ControlMetadata>;
type MetadataWhileFlow = While<AnnotatedNonTransparentStep, BaseFlow, ControlMetadata>;
type MetadataSelectFlow = Select<BaseFlow, BaseFlow, ControlMetadata>;
type MetadataJoinFlow = Join<BaseFlow, BaseFlow, ControlMetadata>;

#[test]
fn control_flow_nodes_expose_custom_metadata_through_ast() {
    let conditional = <MetadataConditionalFlow as JourneyAstSource>::journey_ast();
    let while_loop = <MetadataWhileFlow as JourneyAstSource>::journey_ast();
    let select = <MetadataSelectFlow as JourneyAstSource>::journey_ast();
    let join = <MetadataJoinFlow as JourneyAstSource>::journey_ast();

    match conditional {
        JourneyAst::Conditional { metadata, .. } => assert_eq!(metadata, "control:branching"),
        other => panic!("expected conditional ast node, got {other:?}"),
    }
    match while_loop {
        JourneyAst::While { metadata, .. } => assert_eq!(metadata, "control:branching"),
        other => panic!("expected while ast node, got {other:?}"),
    }
    match select {
        JourneyAst::Select { metadata, .. } => assert_eq!(metadata, "control:branching"),
        other => panic!("expected select ast node, got {other:?}"),
    }
    match join {
        JourneyAst::Join { metadata, .. } => assert_eq!(metadata, "control:branching"),
        other => panic!("expected join ast node, got {other:?}"),
    }
}
