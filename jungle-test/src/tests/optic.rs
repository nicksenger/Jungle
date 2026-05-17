use jungle_sdk::types::Id;
use jungle_sdk::types::Animal;
use serde::{Deserialize, Serialize};
use jungle_sdk::effect;
use jungle_sdk::animal;
use jungle_sdk::types::{
    Act, EffectCompletion, Identity, Running, StateLens, Step, ViewProject, Waiting,
};
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U9, U72, U73, U74, U75};
use jungle_sdk::Optic;

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
struct Leaf {
    value: i32,
    noise: i32,
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
struct Branch {
    leaf: Leaf,
    spare: i32,
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
struct RootState {
    branch: Branch,
    top: i32,
}

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
struct ViewWrapped(#[view] Leaf);

#[derive(
    Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize,
)]
struct ViewRoot {
    #[view]
    wrapped: ViewWrapped,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct IoArg {
    left: i32,
    right: i32,
}

struct EchoI32;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for EchoI32 {
    type Id = Id<U72>;
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
struct SumPair;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for SumPair {
    type Id = Id<U73>;
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
struct EchoPair;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for EchoPair {
    type Id = Id<U74>;
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
struct EchoRootState;

#[effect]
impl<J> jungle_sdk::types::Effect<J> for EchoRootState {
    type Id = Id<U75>;
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

struct LensOnBranch;
impl Act<OpticAnimal> for LensOnBranch {
    type Effect = EchoI32;
    type StateAspect = StateLens<RootState, U0>;
    type Input = i32;
    type Output = i32;

    fn emit(view: &Branch, input: Self::Input) -> i32 {
        view.leaf.value + input
    }

    fn absorb(view: &mut Branch, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("lens single should succeed");
        view.spare = out;
        out
    }
}

struct LensOnLeafValue;
impl Act<OpticAnimal> for LensOnLeafValue {
    type Effect = EchoI32;
    type StateAspect = StateLens<RootState, list![U0, U0, U0]>;
    type Input = i32;
    type Output = i32;

    fn emit(view: &i32, input: Self::Input) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("lens list should succeed");
        *view = out;
        out
    }
}

struct RootStatePulse;
impl Act<OpticAnimal> for RootStatePulse {
    type Effect = EchoRootState;
    type StateAspect = Identity;
    type Input = ();
    type Output = RootState;

    fn emit(view: &RootState, input: Self::Input) -> RootState {
        view.clone()
    }

    fn absorb(view: &mut RootState, output: EffectCompletion<Self::Effect>) -> Self::Output {
        let out = output.expect("echo root should succeed");
        out
    }
}

struct OpticAnimal;

#[animal]
impl Animal for OpticAnimal {
    type Id = Id<U9>;
    type Generation = U0;
    type State = RootState;
    type Seed = RootState;
    type Journey = Step<OpticAnimal, LensOnBranch>;
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
    let (state, request) = <Step<OpticAnimal, LensOnBranch> as Running>::run((seed_state(), 3));
    assert_eq!(request.into_input(), 7);

    let (state, emitted) = <Step<OpticAnimal, LensOnBranch> as Waiting>::accept((state, Ok(8)));
    assert_eq!(emitted, 8);
    assert_eq!(state.branch.spare, 8);
    assert_eq!(state.top, 99);
}

#[test]
fn state_lens_list_multi_index_short_flow() {
    let (state, request) = <Step<OpticAnimal, LensOnLeafValue> as Running>::run((seed_state(), 2));
    assert_eq!(request.into_input(), 6);

    let (state, emitted) = <Step<OpticAnimal, LensOnLeafValue> as Waiting>::accept((state, Ok(7)));
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

//
//struct IoLensPulse;
//impl Act OpticAnimal for IoLensPulse {
//    type Effect = EchoI32;
//    type StateAspect = Identity;
//    type Arg = i32;
//    type Ret = i32;
//
//    fn emit(view: &RootState, input: Self::Arg) -> i32 {
//        input
//    }
//
//    fn absorb(view: &mut RootState, output: EffectCompletion<Self::Effect>) -> Self::Ret {
//        let out = output.expect("lens list should succeed");
//        out
//    }
//}
//type _LensSingle = Step<OpticAnimal, _Lens<IoLensPulse, _View<i32, U1>>>;
//type _LensList = Step<OpticAnimal, _Lens<IoLensPulse, _View<i32, list![U0, U1]>>>;
//#[derive(Journey)]
//struct _LensSingleSequence(
//    Step<IoSingleAnimal, EchoRootState>,
//    Step<IoSingleAnimal, _LensSingle>,
//);
//#[derive(Journey)]
//struct _LensListSequence(
//    Step<IoListAnimal, EchoRootState>,
//    Step<IoListAnimal, _LensList>,
//);
// TODO: add attributed `IoSingleAnimal` once lens io sequence tests are implemented.
// TODO: add attributed `IoListAnimal` once lens io list tests are implemented.
//
//
//#[test]
//fn io_single_lens_flow() {
//    // Assert that list lens returns RootState.top
//}
//
//#[test]
//fn io_list_lens_flow() {
//    // Assert that list lens returns RootState.branch.spare
//}
