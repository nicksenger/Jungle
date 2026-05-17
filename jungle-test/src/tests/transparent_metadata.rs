use jungle_sdk::animal;
use jungle_sdk::effect;
use jungle_sdk::types::Animal;
use jungle_sdk::types::Id;
use jungle_sdk::types::{
    BoundAct, BoundStep, Conditional, EffectCompletion, Executor, Identity, Join, JourneyAst,
    JourneyAstSource, ManualExecutor, NodeMetadata, Select, Transparent, While,
};
use jungle_sdk::typosaurus::num::consts::{U0, U30, U31};
use std::future::ready;

struct TransparentEffect;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for TransparentEffect {
    type Id = Id<U30>;
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _dependency: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(input + 1))
    }
}

struct TransparentAnimal;

#[animal]
impl Animal for TransparentAnimal {
    type Id = Id<U31>;
    type Generation = U0;
    type State = i32;
    type Seed = i32;
    type Journey = TransparentJourney;
}

struct TransparentStep;
impl BoundAct<TransparentAnimal> for TransparentStep {
    type Effect = TransparentEffect;
    type Aspect = Identity;
    type Input = i32;
    type Output = i32;

    fn emit(state: &i32, input: Self::Input) -> i32 {
        *state + input
    }

    fn absorb(state: &mut i32, output: EffectCompletion<TransparentEffect>) -> Self::Output {
        let value = output.expect("transparent step effect should succeed");
        *state = value;
        value
    }
}

struct FlowSectionMetadata;
impl NodeMetadata for FlowSectionMetadata {
    const METADATA: &'static str = "section:checkout/preflight";
}

type BaseFlow = BoundStep<TransparentAnimal, TransparentStep>;
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
