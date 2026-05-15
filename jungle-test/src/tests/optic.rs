use jungle_sdk::types::{EffectCompletion, Identity, Act, Running, StateLens, Step, Waiting};
use jungle_sdk::typosaurus::list;
use jungle_sdk::typosaurus::num::consts::{U0, U1, U72, U73, U74, U75};
use jungle_sdk::Optic;

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Leaf {
    value: i32,
    noise: i32,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Branch {
    leaf: Leaf,
    spare: i32,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct RootState {
    branch: Branch,
    top: i32,
}

#[derive(Optic, Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct IoArg {
    left: i32,
    right: i32,
}

effect!(EchoI32, U72, in = i32, out = i32, err = (), act = |_d, input| std::future::ready(Ok(input + 1)));
effect!(SumPair, U73, in = (i32, i32), out = i32, err = (), act = |_d, input| std::future::ready(Ok(input.0 + input.1)));
effect!(EchoPair, U74, in = (i32, i32), out = (i32, i32), err = (), act = |_d, input| std::future::ready(Ok(input)));
effect!(EchoRootState, U75, in = RootState, out = RootState, err = (), act = |_d, input| std::future::ready(Ok(input)));

struct LensOnBranch;
impl Act<OpticAnimal> for LensOnBranch {
    type Effect = EchoI32;
    type StateAspect = StateLens<RootState, U0>;
    type Arg = i32;
    type Ret = i32;

    fn emit(view: &Branch, input: Self::Arg) -> i32 {
        view.leaf.value + input
    }

    fn absorb(view: &mut Branch, output: EffectCompletion<Self::Effect>) -> Self::Ret {
        let out = output.expect("lens single should succeed");
        view.spare = out;
        out
    }
}

struct LensOnLeafValue;
impl Act<OpticAnimal> for LensOnLeafValue {
    type Effect = EchoI32;
    type StateAspect = StateLens<RootState, list![U0, U0, U0]>;
    type Arg = i32;
    type Ret = i32;

    fn emit(view: &i32, input: Self::Arg) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: EffectCompletion<Self::Effect>) -> Self::Ret {
        let out = output.expect("lens list should succeed");
        *view = out;
        out
    }
}

struct RootStatePulse;
impl Act<OpticAnimal> for RootStatePulse {
    type Effect = EchoRootState;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = RootState;

    fn emit(view: &RootState, input: Self::Arg) -> RootState {
        view.clone()
    }

    fn absorb(view: &mut RootState, output: EffectCompletion<Self::Effect>) -> Self::Ret {
        let out = output.expect("echo root should succeed");
        out
    }
}

animal!(OpticAnimal, jungle_sdk::typosaurus::num::consts::U9, RootState, Step<OpticAnimal, LensOnBranch>);

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
//animal!(IoSingleAnimal, jungle_sdk::typosaurus::num::consts::U10, RootState, _LensSingleSequence);
//animal!(IoListAnimal, jungle_sdk::typosaurus::num::consts::U11, RootState, _LensListSequence);
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
