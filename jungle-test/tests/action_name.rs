use jungle_sdk::prelude::*;
use std::future::ready;

pub struct NamedEffect;

#[jungle::effect(id = 250)]
impl<J> Effect<J> for NamedEffect {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        ready(Ok(()))
    }
}

pub struct NamedSpec;

#[jungle::action(name = "Render Name")]
impl Action for NamedSpec {
    type Effect = NamedEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("named action effect should succeed"))?;
        Ok(())
    }
}

pub struct DefaultNamedSpec;

#[jungle::action]
impl Action for DefaultNamedSpec {
    type Effect = NamedEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &(), _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut (),
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("default named action effect should succeed"))?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct NamedFlow(Step<NamedSpec>);

pub struct NamedAnimal;

#[jungle::animal(id = 250, generation = 0)]
impl Animal for NamedAnimal {
    type State = ();
    type Seed = ();
    type Flow = NamedFlow;
}

#[test]
fn action_name_flows_into_step_and_bound_step_ast_labels() {
    assert_eq!(<NamedSpec as Action>::NAME, "Render Name");

    let flow_ast = <NamedFlow as JourneyAstSource>::journey_ast();
    assert_eq!(
        flow_ast,
        JourneyAst::Step {
            label: "Render Name"
        }
    );

    let bound_nodes =
        <BoundFlowStep<NamedAnimal, <NamedSpec as Action>::Bind<NamedAnimal>> as BuildJourneyAst<
            Vec<JourneyAst>,
        >>::push_ast(Vec::new());
    assert_eq!(
        bound_nodes,
        vec![JourneyAst::Step {
            label: "Render Name"
        }]
    );
}

#[test]
fn action_name_defaults_to_the_action_ident() {
    assert_eq!(<DefaultNamedSpec as Action>::NAME, "DefaultNamedSpec");
}
