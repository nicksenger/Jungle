use jungle_sdk::prelude::*;
use jungle_sdk::Optic;
use serde::{Deserialize, Serialize};

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leaf {
    value: i32,
    noise: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    leaf: Leaf,
    spare: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootState {
    branch: Branch,
    top: i32,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewWrapped(#[jungle(focus)] Leaf);

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewRoot {
    #[jungle(focus)]
    wrapped: ViewWrapped,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoArg {
    left: i32,
    right: i32,
}

pub struct EchoI32;

#[jungle::effect(id = 72)]
impl<J> Effect<J> for EchoI32 {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input + 1))
    }
}
#[allow(dead_code)]
pub struct SumPair;

#[jungle::effect(id = 73)]
impl<J> Effect<J> for SumPair {
    type In = (i32, i32);
    type Out = i32;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input.0 + input.1))
    }
}
#[allow(dead_code)]
pub struct EchoPair;

#[jungle::effect(id = 74)]
impl<J> Effect<J> for EchoPair {
    type In = (i32, i32);
    type Out = (i32, i32);
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}
#[allow(dead_code)]
pub struct EchoRootState;

#[jungle::effect(id = 75)]
impl<J> Effect<J> for EchoRootState {
    type In = RootState;
    type Out = RootState;
    type Err = ();

    fn effect(
        _d: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct BranchCarrier;
impl StateCarrier<RootState> for BranchCarrier {
    type Focus = Branch;

    fn focus(state: &mut RootState) -> &mut Self::Focus {
        &mut state.branch
    }
}

pub struct LeafValueCarrier;
impl StateCarrier<RootState> for LeafValueCarrier {
    type Focus = i32;

    fn focus(state: &mut RootState) -> &mut Self::Focus {
        &mut state.branch.leaf.value
    }
}

pub struct LensOnLeafValue;
impl BoundAction<OpticAnimal> for LensOnLeafValue {
    const NAME: &'static str = "LensOnLeafValue";
    type Effect = EchoI32;
    type Aspect = LeafValueCarrier;
    type Input = i32;
    type Output = i32;
    type Carry = ();

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<OpticAnimal>>::Aspect as StateCarrier<
            <OpticAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<OpticAnimal>>::emit(view, input), ())
    }

    fn absorb(
        view: &mut i32,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let out = output.map_err(|_err| Failure::from("lens list should succeed"))?;
        *view = out;
        Ok(out)
    }
}

#[allow(dead_code)]
pub struct RootStatePulse;
impl BoundAction<OpticAnimal> for RootStatePulse {
    const NAME: &'static str = "RootStatePulse";
    type Effect = EchoRootState;
    type Aspect = Identity;
    type Input = ();
    type Output = RootState;
    type Carry = ();

    fn emit(view: &RootState, _input: Self::Input) -> RootState {
        *view
    }

    fn emit_with_carry(
        view: &<<Self as BoundAction<OpticAnimal>>::Aspect as StateCarrier<
            <OpticAnimal as Animal>::State,
        >>::Focus,
        input: Self::Input,
    ) -> (<Self::Effect as EffectSchema>::In, Self::Carry) {
        (<Self as BoundAction<OpticAnimal>>::emit(view, input), ())
    }

    fn absorb(
        _view: &mut RootState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(output.map_err(|_err| Failure::from("echo root should succeed"))?)
    }
}

pub struct OpticAnimal;

pub struct LensOnBranchSpec;
#[jungle::action(aspect = BranchCarrier)]
impl Action for LensOnBranchSpec {
    type Effect = EchoI32;
    type Input = i32;
    type Output = i32;

    fn emit(view: &Branch, input: Self::Input) -> i32 {
        view.leaf.value + input
    }

    fn absorb(
        view: &mut Branch,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            let out = output.map_err(|_err| Failure::from("lens single should succeed"))?;
            view.spare = out;
            out
        };
        Ok(__absorb_out_1)
    }
}

#[derive(Flow)]
pub struct OpticJourneyTemplate(Step<LensOnBranchSpec>);

#[jungle::animal(id = 9, generation = 0)]
impl Animal for OpticAnimal {
    type State = RootState;
    type Seed = RootState;
    type Flow = OpticJourneyTemplate;
}

fn seed_state() -> RootState {
    RootState {
        branch: Branch {
            leaf: Leaf { value: 4, noise: 1 },
            spare: 0,
        },
        top: 99,
    }
}

#[test]
fn state_lens_single_index_short_flow() {
    let (state, request) = <BoundFlowStep<
        OpticAnimal,
        <LensOnBranchSpec as Action>::Bind<OpticAnimal>,
    > as Running>::run((seed_state(), 3));
    assert_eq!(request.0.into_input(), 7);

    let (state, emitted) = <BoundFlowStep<
        OpticAnimal,
        <LensOnBranchSpec as Action>::Bind<OpticAnimal>,
    > as Waiting>::accept((state, Ok(8), ()));
    assert_eq!(emitted, 8);
    assert_eq!(state.branch.spare, 8);
    assert_eq!(state.top, 99);
}

#[test]
fn state_lens_list_multi_index_short_flow() {
    let (state, request) =
        <BoundFlowStep<OpticAnimal, LensOnLeafValue> as Running>::run((seed_state(), 2));
    assert_eq!(request.0.into_input(), 6);

    let (state, emitted) =
        <BoundFlowStep<OpticAnimal, LensOnLeafValue> as Waiting>::accept((state, Ok(7), ()));
    assert_eq!(emitted, 7);
    assert_eq!(state.branch.leaf.value, 7);
    assert_eq!(state.branch.leaf.noise, 1);
}

#[test]
fn optic_view_marker_generates_direct_projection_impls() {
    let mut root = ViewRoot {
        wrapped: ViewWrapped(Leaf { value: 9, noise: 4 }),
    };
    let wrapped = <ViewRoot as ViewProject<ViewWrapped>>::project_view(&mut root);
    wrapped.0.value += 1;

    let leaf = <ViewWrapped as ViewProject<Leaf>>::project_view(wrapped);
    leaf.noise += 3;

    assert_eq!(root.wrapped.0.value, 10);
    assert_eq!(root.wrapped.0.noise, 7);
}
