use jungle_sdk::types::{ActionCompletion, Identity, Pulse, Running, StateLens, Step, Waiting};
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

action!(EchoI32, U72, in = i32, out = i32, err = (), act = |_d, input| std::future::ready(Ok(input + 1)));
action!(SumPair, U73, in = (i32, i32), out = i32, err = (), act = |_d, input| std::future::ready(Ok(input.0 + input.1)));
action!(EchoPair, U74, in = (i32, i32), out = (i32, i32), err = (), act = |_d, input| std::future::ready(Ok(input)));
action!(EchoRootState, U75, in = RootState, out = RootState, err = (), act = |_d, input| std::future::ready(Ok(input)));

struct LensOnBranch;
impl Pulse<OpticAnimal> for LensOnBranch {
    type Action = EchoI32;
    type StateAspect = StateLens<RootState, U0>;
    type Arg = i32;
    type Ret = i32;

    fn emit(view: &Branch, input: Self::Arg) -> i32 {
        view.leaf.value + input
    }

    fn absorb(view: &mut Branch, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let out = output.expect("lens single should succeed");
        view.spare = out;
        out
    }
}

struct LensOnLeafValue;
impl Pulse<OpticAnimal> for LensOnLeafValue {
    type Action = EchoI32;
    type StateAspect = StateLens<RootState, list![U0, U0, U0]>;
    type Arg = i32;
    type Ret = i32;

    fn emit(view: &i32, input: Self::Arg) -> i32 {
        *view + input
    }

    fn absorb(view: &mut i32, output: ActionCompletion<Self::Action>) -> Self::Ret {
        let out = output.expect("lens list should succeed");
        *view = out;
        out
    }
}

struct RootStatePulse;
impl Pulse<OpticAnimal> for RootStatePulse {
    type Action = EchoRootState;
    type StateAspect = Identity;
    type Arg = ();
    type Ret = RootState;

    fn emit(view: &RootState, input: Self::Arg) -> RootState {
        view.clone()
    }

    fn absorb(view: &mut RootState, output: ActionCompletion<Self::Action>) -> Self::Ret {
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
//impl Pulse OpticAnimal for IoLensPulse {
//    type Action = EchoI32;
//    type StateAspect = Identity;
//    type Arg = i32;
//    type Ret = i32;
//
//    fn emit(view: &RootState, input: Self::Arg) -> i32 {
//        input
//    }
//
//    fn absorb(view: &mut RootState, output: ActionCompletion<Self::Action>) -> Self::Ret {
//        let out = output.expect("lens list should succeed");
//        out
//    }
//}
//type IoLensSingle = Step<OpticAnimal, IoLens<IoLensPulse, U1>>;
//type IoLensList = Step<OpticAnimal, IoLens<IoLensPulse, list![U0, U1]>>;
//#[derive(Journey)]
//struct IoLensSingleSequence(
//    Step<IoSingleAnimal, EchoRootState>,
//    Step<IoSingleAnimal, IoLensSingle>,
//);
//#[derive(Journey)]
//struct IoLensListSequence(
//    Step<IoListAnimal, EchoRootState>,
//    Step<IoListAnimal, IoLensList>,
//);
//animal!(IoSingleAnimal, jungle_sdk::typosaurus::num::consts::U10, RootState, IoLensSingleSequence);
//animal!(IoListAnimal, jungle_sdk::typosaurus::num::consts::U11, RootState, IoLensListSequence);
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
