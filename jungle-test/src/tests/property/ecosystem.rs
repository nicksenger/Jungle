use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ReplayRainforest(Arc<Mutex<ReplayInner>>);

struct ReplayInner {
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: Arc<Mutex<UnboundedReceiver<bool>>>,
}

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest";
    type Animals = ReplayRainforestAnimals;
}

impl ReplayRainforest {
    async fn next(&self) -> bool {
        let recv = {
            let mut inner = self.0.lock().await;
            match inner.query.pop() {
                Some(value) => return value,
                None => {
                    let _ = inner.end.unbounded_send(());
                    Arc::clone(&inner.recv)
                }
            }
        };

        let mut recv = recv.lock().await;
        match recv.next().await {
            Some(value) => value,
            None => std::future::pending().await,
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Left;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Right;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayFrame<L, R> {
    pub color: bool,
    pub history: String,
    pub left: L,
    pub right: R,
}

pub type ReplayState = ReplayFrame<Left, Right>;
pub type Depth1State = ReplayState;
pub type Depth2LeftState = ReplayFrame<Left, ()>;
pub type Depth2RightState = ReplayFrame<(), Right>;
pub type Depth2State = ReplayFrame<Depth2LeftState, Depth2RightState>;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Depth3LeftLeft;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Depth3LeftRight;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Depth3RightLeft;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Depth3RightRight;

pub type Depth3LeftLeftState = ReplayFrame<Depth3LeftLeft, ()>;
pub type Depth3LeftRightState = ReplayFrame<(), Depth3LeftRight>;
pub type Depth3LeftState = ReplayFrame<Depth3LeftLeftState, Depth3LeftRightState>;
pub type Depth3RightLeftState = ReplayFrame<Depth3RightLeft, ()>;
pub type Depth3RightRightState = ReplayFrame<(), Depth3RightRight>;
pub type Depth3RightState = ReplayFrame<Depth3RightLeftState, Depth3RightRightState>;
pub type Depth3State = ReplayFrame<Depth3LeftState, Depth3RightState>;

impl ViewProject<Depth2LeftState> for Depth2State {
    fn project_view(state: &mut Self) -> &mut Depth2LeftState {
        &mut state.left
    }
}

impl ViewProject<Depth2RightState> for Depth2State {
    fn project_view(state: &mut Self) -> &mut Depth2RightState {
        &mut state.right
    }
}

impl ViewProject<Depth3LeftState> for Depth3State {
    fn project_view(state: &mut Self) -> &mut Depth3LeftState {
        &mut state.left
    }
}

impl ViewProject<Depth3RightState> for Depth3State {
    fn project_view(state: &mut Self) -> &mut Depth3RightState {
        &mut state.right
    }
}

impl ViewProject<Depth3LeftLeftState> for Depth3LeftState {
    fn project_view(state: &mut Self) -> &mut Depth3LeftLeftState {
        &mut state.left
    }
}

impl ViewProject<Depth3LeftRightState> for Depth3LeftState {
    fn project_view(state: &mut Self) -> &mut Depth3LeftRightState {
        &mut state.right
    }
}

impl ViewProject<Depth3RightLeftState> for Depth3RightState {
    fn project_view(state: &mut Self) -> &mut Depth3RightLeftState {
        &mut state.left
    }
}

impl ViewProject<Depth3RightRightState> for Depth3RightState {
    fn project_view(state: &mut Self) -> &mut Depth3RightRightState {
        &mut state.right
    }
}

impl<L, R> From<ReplayFrame<L, R>> for () {
    fn from(_value: ReplayFrame<L, R>) -> Self {}
}

pub trait ReplayNodeState {
    fn replay_color(&self) -> bool;
    fn replay_color_mut(&mut self) -> &mut bool;
    fn replay_history(&self) -> &str;
    fn replay_history_mut(&mut self) -> &mut String;
}

trait ReplayAppearance {
    fn replay_appearance(&self) -> String;
}

impl ReplayAppearance for () {
    fn replay_appearance(&self) -> String {
        String::new()
    }
}

macro_rules! impl_empty_replay_appearance {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl ReplayAppearance for $ty {
                fn replay_appearance(&self) -> String {
                    String::new()
                }
            }
        )+
    };
}

impl_empty_replay_appearance!(
    Left,
    Right,
    Depth3LeftLeft,
    Depth3LeftRight,
    Depth3RightLeft,
    Depth3RightRight,
);

impl<L, R> ReplayAppearance for ReplayFrame<L, R>
where
    L: ReplayAppearance,
    R: ReplayAppearance,
{
    fn replay_appearance(&self) -> String {
        let mut appearance = self.history.clone();
        let left = self.left.replay_appearance();
        let right = self.right.replay_appearance();
        if !left.is_empty() && right.is_empty() {
            appearance.push('(');
            appearance.push_str(&left);
        } else if left.is_empty() && !right.is_empty() {
            appearance.push('(');
            appearance.push('|');
            appearance.push_str(&right);
        } else if !left.is_empty() || !right.is_empty() {
            appearance.push('(');
            appearance.push_str(&left);
            appearance.push('|');
            appearance.push_str(&right);
        }
        appearance
    }
}

impl<L, R> ReplayNodeState for ReplayFrame<L, R> {
    fn replay_color(&self) -> bool {
        self.color
    }

    fn replay_color_mut(&mut self) -> &mut bool {
        &mut self.color
    }

    fn replay_history(&self) -> &str {
        &self.history
    }

    fn replay_history_mut(&mut self) -> &mut String {
        &mut self.history
    }
}

pub trait ReplayBranchHostState: ReplayNodeState {
    type LeftBranch: ReplayNodeState;
    type RightBranch: ReplayNodeState;

    fn replay_left(&self) -> &Self::LeftBranch;
    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch;
    fn replay_right(&self) -> &Self::RightBranch;
    fn replay_right_mut(&mut self) -> &mut Self::RightBranch;
}

impl ReplayBranchHostState for Depth2State {
    type LeftBranch = Depth2LeftState;
    type RightBranch = Depth2RightState;

    fn replay_left(&self) -> &Self::LeftBranch {
        &self.left
    }

    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch {
        &mut self.left
    }

    fn replay_right(&self) -> &Self::RightBranch {
        &self.right
    }

    fn replay_right_mut(&mut self) -> &mut Self::RightBranch {
        &mut self.right
    }
}

impl ReplayBranchHostState for Depth3LeftState {
    type LeftBranch = Depth3LeftLeftState;
    type RightBranch = Depth3LeftRightState;

    fn replay_left(&self) -> &Self::LeftBranch {
        &self.left
    }

    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch {
        &mut self.left
    }

    fn replay_right(&self) -> &Self::RightBranch {
        &self.right
    }

    fn replay_right_mut(&mut self) -> &mut Self::RightBranch {
        &mut self.right
    }
}

impl ReplayBranchHostState for Depth3RightState {
    type LeftBranch = Depth3RightLeftState;
    type RightBranch = Depth3RightRightState;

    fn replay_left(&self) -> &Self::LeftBranch {
        &self.left
    }

    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch {
        &mut self.left
    }

    fn replay_right(&self) -> &Self::RightBranch {
        &self.right
    }

    fn replay_right_mut(&mut self) -> &mut Self::RightBranch {
        &mut self.right
    }
}

impl ReplayBranchHostState for Depth3State {
    type LeftBranch = Depth3LeftState;
    type RightBranch = Depth3RightState;

    fn replay_left(&self) -> &Self::LeftBranch {
        &self.left
    }

    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch {
        &mut self.left
    }

    fn replay_right(&self) -> &Self::RightBranch {
        &self.right
    }

    fn replay_right_mut(&mut self) -> &mut Self::RightBranch {
        &mut self.right
    }
}

pub struct ReplayColorIsTrue;

impl<St> Predicate<(&St, &())> for ReplayColorIsTrue
where
    St: ReplayNodeState,
{
    fn eval((state, _): &(&St, &())) -> bool {
        state.replay_color()
    }
}

impl<St> Predicate<(St, ())> for ReplayColorIsTrue
where
    St: ReplayNodeState,
{
    fn eval((state, _): &(St, ())) -> bool {
        state.replay_color()
    }
}

pub struct ReplayAlwaysTrue;

impl<St> Predicate<(&St, &())> for ReplayAlwaysTrue
where
    St: ReplayNodeState,
{
    fn eval((_state, _): &(&St, &())) -> bool {
        true
    }
}

impl<St> Predicate<(St, ())> for ReplayAlwaysTrue
where
    St: ReplayNodeState,
{
    fn eval((_state, _): &(St, ())) -> bool {
        true
    }
}

trait ReplayTockRuntime {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send;
}

impl ReplayTockRuntime for () {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        std::future::ready(false)
    }
}

impl ReplayTockRuntime for ReplayRainforest {
    fn run_tock(&self) -> impl std::future::Future<Output = bool> + Send {
        self.next()
    }
}

pub struct Tock;

#[jungle::effect(id = 1001)]
impl<J> Effect<J> for Tock
where
    J: ReplayTockRuntime + Sync,
{
    type In = ();
    type Out = bool;
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move { Ok(jungle.run_tock().await) }
    }
}

pub struct Tick<St>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St> Action for Tick<St>
where
    St: ReplayNodeState,
{
    type Effect = Tock;
    type Input = ();
    type Output = ();

    fn emit(_state: &St, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let tocked = output.map_err(|_err| Failure::from("tock should succeed"))?;
        if tocked {
            state.replay_history_mut().push('1');
        } else {
            state.replay_history_mut().push('0');
        }
        *state.replay_color_mut() = tocked;
        Ok(())
    }
}

pub struct Label<St, const CH: char>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St, const CH: char> Action for Label<St, CH>
where
    St: ReplayNodeState,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &St, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("label should complete without effect"))?;
        state.replay_history_mut().push(CH);
        Ok(())
    }
}

#[allow(dead_code)]
pub struct MaybeFail<St>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St> Action for MaybeFail<St>
where
    St: ReplayNodeState,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &St, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("maybe fail should complete without effect"))?;
        if !state.replay_color() {
            return Err(Failure::from(
                "maybe fail should fail when replay color is false",
            ));
        }
        Ok(())
    }
}

pub struct FlattenReplayChoice<St>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St> Action for FlattenReplayChoice<St>
where
    St: ReplayNodeState,
{
    type Effect = NoEffect;
    type Input = Either<(), ()>;
    type Output = ();
    type Carry = Either<(), ()>;

    fn emit(_state: &St, input: Self::Input) -> ((), Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut St,
        output: EffectCompletion<Self::Effect>,
        carry: Self::Carry,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("flatten replay choice should succeed"))?;
        match carry {
            Either::Left(()) | Either::Right(()) => (),
        }
        Ok(())
    }
}

#[derive(Flow)]
pub struct Depth1LeftBranch<St: ReplayNodeState>(
    Step<Label<St, 'L'>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
);

#[derive(Flow)]
pub struct Depth1RightBranch<St: ReplayNodeState>(
    Step<Label<St, 'R'>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
);

#[derive(Flow)]
pub struct Depth1InnerBody<St: ReplayNodeState>(
    Step<Label<St, 'I'>>,
    Step<Tick<St>>,
    Step<Tick<St>>,
    Conditional<ReplayColorIsTrue, Depth1LeftBranch<St>, Depth1RightBranch<St>>,
    Step<FlattenReplayChoice<St>>,
);

#[derive(Flow)]
pub struct Depth1TailLeftBranch<St: ReplayNodeState>(Step<Label<St, 'A'>>);

#[derive(Flow)]
pub struct Depth1TailRightBranch<St: ReplayNodeState>(Step<Label<St, 'B'>>);

#[derive(Flow)]
pub struct Depth1OuterBody(
    Step<Label<ReplayState, 'O'>>,
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    While<ReplayColorIsTrue, Depth1InnerBody<ReplayState>>,
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Conditional<
        ReplayColorIsTrue,
        Depth1TailLeftBranch<ReplayState>,
        Depth1TailRightBranch<ReplayState>,
    >,
    Step<FlattenReplayChoice<ReplayState>>,
);

#[derive(Flow)]
// Wrapping `Depth1OuterBody` in `Attempt` currently causes the replay property
// to time out before the first outer-body boundary is reached.
pub struct Depth1Flow(While<ReplayAlwaysTrue, Depth1OuterBody>);

pub struct Depth1;

#[jungle::animal(observe, id = 1002, generation = 0)]
impl Animal for Depth1 {
    type State = Depth1State;
    type Seed = Depth1State;
    type Flow = Depth1Flow;
}

impl Observe for Depth1 {
    type Appearance = String;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.replay_appearance()
    }
}

pub struct SeedReplayBranches<St>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St> Action for SeedReplayBranches<St>
where
    St: ReplayBranchHostState,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &St, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("seed replay branches should succeed"))?;
        let color = state.replay_color();
        {
            let left = state.replay_left_mut();
            *left.replay_color_mut() = color;
            left.replay_history_mut().clear();
        }
        {
            let right = state.replay_right_mut();
            *right.replay_color_mut() = color;
            right.replay_history_mut().clear();
        }
        Ok(())
    }
}

pub struct MergeReplayJoin<St>(PhantomData<fn() -> St>);

#[jungle::action]
impl<St> Action for MergeReplayJoin<St>
where
    St: ReplayBranchHostState,
{
    type Effect = NoEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &St, _input: Self::Input) {}

    fn absorb(
        state: &mut St,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("merge replay join should succeed"))?;
        let left_history = state.replay_left().replay_history().to_string();
        let right_history = state.replay_right().replay_history().to_string();
        let next_color = state.replay_left().replay_color() || state.replay_right().replay_color();
        state.replay_history_mut().push('(');
        state.replay_history_mut().push_str(&left_history);
        state.replay_history_mut().push('|');
        state.replay_history_mut().push_str(&right_history);
        state.replay_history_mut().push(')');
        *state.replay_color_mut() = next_color;
        state.replay_left_mut().replay_history_mut().clear();
        state.replay_right_mut().replay_history_mut().clear();
        Ok(())
    }
}

#[derive(Flow)]
#[jungle(focus = Depth2LeftState)]
pub struct Depth2LeftInnerFlow(
    Step<Label<Depth2LeftState, '2'>>,
    Step<Label<Depth2LeftState, 'L'>>,
    Depth1InnerBody<Depth2LeftState>,
);

#[derive(Flow)]
#[jungle(focus = Depth2RightState)]
pub struct Depth2RightInnerFlow(
    Step<Label<Depth2RightState, '2'>>,
    Step<Label<Depth2RightState, 'R'>>,
    Depth1InnerBody<Depth2RightState>,
);

#[derive(Flow)]
pub struct Depth2JoinedInnerBody(
    Join<Depth2LeftInnerFlow, Depth2RightInnerFlow>,
    Step<MergeReplayJoin<Depth2State>>,
);

#[derive(Flow)]
pub struct Depth2OuterBody(
    Step<Label<Depth2State, 'O'>>,
    Step<Tick<Depth2State>>,
    Step<Tick<Depth2State>>,
    Step<Tick<Depth2State>>,
    Step<SeedReplayBranches<Depth2State>>,
    While<ReplayColorIsTrue, Depth2JoinedInnerBody>,
    Step<Tick<Depth2State>>,
    Step<Tick<Depth2State>>,
    Conditional<
        ReplayColorIsTrue,
        Depth1TailLeftBranch<Depth2State>,
        Depth1TailRightBranch<Depth2State>,
    >,
    Step<FlattenReplayChoice<Depth2State>>,
);

#[derive(Flow)]
pub struct Depth2Flow(While<ReplayAlwaysTrue, Depth2OuterBody>);

pub struct Depth2;

#[jungle::animal(observe, id = 1005, generation = 0)]
impl Animal for Depth2 {
    type State = Depth2State;
    type Seed = Depth2State;
    type Flow = Depth2Flow;
}

impl Observe for Depth2 {
    type Appearance = String;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.replay_appearance()
    }
}

#[derive(Flow)]
#[jungle(focus = Depth3LeftLeftState)]
pub struct Depth3LeftNestedLeftFlow(
    Step<Label<Depth3LeftLeftState, '3'>>,
    Step<Label<Depth3LeftLeftState, 'L'>>,
    Depth1InnerBody<Depth3LeftLeftState>,
);

#[derive(Flow)]
#[jungle(focus = Depth3LeftRightState)]
pub struct Depth3LeftNestedRightFlow(
    Step<Label<Depth3LeftRightState, '3'>>,
    Step<Label<Depth3LeftRightState, 'R'>>,
    Depth1InnerBody<Depth3LeftRightState>,
);

#[derive(Flow)]
pub struct Depth3LeftNestedJoin(
    Join<Depth3LeftNestedLeftFlow, Depth3LeftNestedRightFlow>,
    Step<MergeReplayJoin<Depth3LeftState>>,
);

#[derive(Flow)]
#[jungle(focus = Depth3LeftState)]
pub struct Depth3OuterLeftFlow(
    Step<Label<Depth3LeftState, '2'>>,
    Step<Label<Depth3LeftState, 'L'>>,
    Step<Tick<Depth3LeftState>>,
    Step<SeedReplayBranches<Depth3LeftState>>,
    Depth3LeftNestedJoin,
    Step<Tick<Depth3LeftState>>,
);

#[derive(Flow)]
#[jungle(focus = Depth3RightLeftState)]
pub struct Depth3RightNestedLeftFlow(
    Step<Label<Depth3RightLeftState, '3'>>,
    Step<Label<Depth3RightLeftState, 'L'>>,
    Depth1InnerBody<Depth3RightLeftState>,
);

#[derive(Flow)]
#[jungle(focus = Depth3RightRightState)]
pub struct Depth3RightNestedRightFlow(
    Step<Label<Depth3RightRightState, '3'>>,
    Step<Label<Depth3RightRightState, 'R'>>,
    Depth1InnerBody<Depth3RightRightState>,
);

#[derive(Flow)]
pub struct Depth3RightNestedJoin(
    Join<Depth3RightNestedLeftFlow, Depth3RightNestedRightFlow>,
    Step<MergeReplayJoin<Depth3RightState>>,
);

#[derive(Flow)]
#[jungle(focus = Depth3RightState)]
pub struct Depth3OuterRightFlow(
    Step<Label<Depth3RightState, '2'>>,
    Step<Label<Depth3RightState, 'R'>>,
    Step<Tick<Depth3RightState>>,
    Step<SeedReplayBranches<Depth3RightState>>,
    Depth3RightNestedJoin,
    Step<Tick<Depth3RightState>>,
);

#[derive(Flow)]
pub struct Depth3JoinedInnerBody(
    Join<Depth3OuterLeftFlow, Depth3OuterRightFlow>,
    Step<MergeReplayJoin<Depth3State>>,
);

#[derive(Flow)]
pub struct Depth3OuterBody(
    Step<Label<Depth3State, 'O'>>,
    Step<Tick<Depth3State>>,
    Step<Tick<Depth3State>>,
    Step<Tick<Depth3State>>,
    Step<SeedReplayBranches<Depth3State>>,
    While<ReplayColorIsTrue, Depth3JoinedInnerBody>,
    Step<Tick<Depth3State>>,
    Step<Tick<Depth3State>>,
    Conditional<
        ReplayColorIsTrue,
        Depth1TailLeftBranch<Depth3State>,
        Depth1TailRightBranch<Depth3State>,
    >,
    Step<FlattenReplayChoice<Depth3State>>,
);

#[derive(Flow)]
pub struct Depth3Flow(While<ReplayAlwaysTrue, Depth3OuterBody>);

pub struct Depth3;

#[jungle::animal(observe, id = 1006, generation = 0)]
impl Animal for Depth3 {
    type State = Depth3State;
    type Seed = Depth3State;
    type Flow = Depth3Flow;
}

impl Observe for Depth3 {
    type Appearance = String;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.replay_appearance()
    }
}

#[derive(Flow)]
pub struct ConditionalProbeLeft(Step<Label<ReplayState, 'L'>>, Step<Tick<ReplayState>>);

#[derive(Flow)]
pub struct ConditionalProbeRight(Step<Label<ReplayState, 'R'>>, Step<Tick<ReplayState>>);

#[derive(Flow)]
pub struct ConditionalProbeFlow(
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Conditional<ReplayColorIsTrue, ConditionalProbeLeft, ConditionalProbeRight>,
);

pub struct ConditionalProbe;

#[jungle::animal(id = 1003, generation = 0)]
impl Animal for ConditionalProbe {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = ConditionalProbeFlow;
}

#[derive(Flow)]
pub struct ConditionalCompleteProbeFlow(
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Step<Tick<ReplayState>>,
    Conditional<ReplayColorIsTrue, Step<Label<ReplayState, 'L'>>, Step<Label<ReplayState, 'R'>>>,
    Step<FlattenReplayChoice<ReplayState>>,
    Step<Tick<ReplayState>>,
);

pub struct ConditionalCompleteProbe;

#[jungle::animal(id = 1004, generation = 0)]
impl Animal for ConditionalCompleteProbe {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = ConditionalCompleteProbeFlow;
}

#[derive(Animals)]
pub struct ReplayRainforestAnimals(Depth1, Depth2, Depth3);

pub fn replay_rainforest(
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
) -> ReplayRainforest {
    ReplayRainforest(Arc::new(Mutex::new(ReplayInner {
        query,
        end,
        recv: Arc::new(Mutex::new(recv)),
    })))
}
