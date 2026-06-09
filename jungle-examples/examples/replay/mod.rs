use clap::Parser;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::FusedClient;
use jungle_zoo::time::{Millis, SleepFor};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

mod ui;

const DEFAULT_LOG_FILTER: &str = "warn,replay=info,jungle_vision=info";
const REPLAY_VIEWER_LINGER_AFTER_END: Duration = Duration::from_secs(20);

#[derive(Debug, Parser)]
#[command(name = "replay")]
struct Cli {
    #[arg(
        long,
        help = "Bitstring query, for example 01000101",
        default_value = "001000111100011111111111110000000000000101010101011001110100010010101011010110001011100000110110100101001"
    )]
    query: String,
    #[arg(
        long = "img-dump",
        help = "Capture the replay UI to this PNG path and then exit"
    )]
    img_dump: Option<PathBuf>,
    #[arg(
        long = "img-dump-time-secs",
        requires = "img_dump",
        value_parser = parse_img_dump_time_secs,
        help = "Seconds to wait after the UI starts before capturing --img-dump"
    )]
    img_dump_time_secs: Option<f64>,
}

fn parse_query_bits(input: &str) -> Result<Vec<bool>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if let Some((index, ch)) = input
        .char_indices()
        .find(|(_, ch)| !matches!(ch, '0' | '1'))
    {
        return Err(format!(
            "query must contain only '0' or '1', found {ch:?} at position {index}"
        ));
    }

    Ok(input.chars().rev().map(|ch| ch == '1').collect())
}

fn parse_img_dump_time_secs(value: &str) -> Result<f64, String> {
    let secs = value
        .parse::<f64>()
        .map_err(|err| format!("invalid img dump time `{value}`: {err}"))?;
    if secs.is_sign_negative() {
        return Err("img dump time must be non-negative".to_owned());
    }
    Ok(secs)
}

fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .try_init();
}

struct ReplayRainforest(Arc<tokio::sync::Mutex<ReplayInner>>);

struct ReplayInner {
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: Arc<tokio::sync::Mutex<UnboundedReceiver<bool>>>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowLeftLeft;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowLeftRight;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowRightLeft;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowRightRight;

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayFrame<L, R> {
    color: bool,
    history: String,
    left: L,
    right: R,
}

pub type DoubleFlowLeftInnerLeftState = ReplayFrame<DoubleFlowLeftLeft, ()>;
pub type DoubleFlowLeftInnerRightState = ReplayFrame<(), DoubleFlowLeftRight>;
#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowLeftState {
    color: bool,
    history: String,
    #[jungle(focus)]
    left: DoubleFlowLeftInnerLeftState,
    #[jungle(focus)]
    right: DoubleFlowLeftInnerRightState,
}

pub type DoubleFlowRightInnerLeftState = ReplayFrame<DoubleFlowRightLeft, ()>;
pub type DoubleFlowRightInnerRightState = ReplayFrame<(), DoubleFlowRightRight>;
#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleFlowRightState {
    color: bool,
    history: String,
    #[jungle(focus)]
    left: DoubleFlowRightInnerLeftState,
    #[jungle(focus)]
    right: DoubleFlowRightInnerRightState,
}

#[derive(Optic, Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayState {
    color: bool,
    history: String,
    #[jungle(focus)]
    left: DoubleFlowLeftState,
    #[jungle(focus)]
    right: DoubleFlowRightState,
}

impl<L, R> From<ReplayFrame<L, R>> for () {
    fn from(_value: ReplayFrame<L, R>) -> Self {}
}

impl From<DoubleFlowLeftState> for () {
    fn from(_value: DoubleFlowLeftState) -> Self {}
}

impl From<DoubleFlowRightState> for () {
    fn from(_value: DoubleFlowRightState) -> Self {}
}

impl From<ReplayState> for () {
    fn from(_value: ReplayState) -> Self {}
}

pub(crate) struct ReplayColorIsTrue;

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
    DoubleFlowLeftLeft,
    DoubleFlowLeftRight,
    DoubleFlowRightLeft,
    DoubleFlowRightRight,
);

fn compose_replay_appearance<L, R>(history: &str, left: &L, right: &R) -> String
where
    L: ReplayAppearance,
    R: ReplayAppearance,
{
    let mut appearance = history.to_owned();
    let left = left.replay_appearance();
    let right = right.replay_appearance();
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

impl<L, R> ReplayAppearance for ReplayFrame<L, R>
where
    L: ReplayAppearance,
    R: ReplayAppearance,
{
    fn replay_appearance(&self) -> String {
        compose_replay_appearance(&self.history, &self.left, &self.right)
    }
}

impl ReplayAppearance for DoubleFlowLeftState {
    fn replay_appearance(&self) -> String {
        compose_replay_appearance(&self.history, &self.left, &self.right)
    }
}

impl ReplayAppearance for DoubleFlowRightState {
    fn replay_appearance(&self) -> String {
        compose_replay_appearance(&self.history, &self.left, &self.right)
    }
}

impl ReplayAppearance for ReplayState {
    fn replay_appearance(&self) -> String {
        compose_replay_appearance(&self.history, &self.left, &self.right)
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

macro_rules! impl_replay_node_state_for_struct {
    ($ty:ty) => {
        impl ReplayNodeState for $ty {
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
    };
}

impl_replay_node_state_for_struct!(DoubleFlowLeftState);
impl_replay_node_state_for_struct!(DoubleFlowRightState);
impl_replay_node_state_for_struct!(ReplayState);

pub trait ReplayBranchHostState: ReplayNodeState {
    type LeftBranch: ReplayNodeState;
    type RightBranch: ReplayNodeState;

    fn replay_left(&self) -> &Self::LeftBranch;
    fn replay_left_mut(&mut self) -> &mut Self::LeftBranch;
    fn replay_right(&self) -> &Self::RightBranch;
    fn replay_right_mut(&mut self) -> &mut Self::RightBranch;
}

impl ReplayBranchHostState for ReplayState {
    type LeftBranch = DoubleFlowLeftState;
    type RightBranch = DoubleFlowRightState;

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

impl ReplayBranchHostState for DoubleFlowLeftState {
    type LeftBranch = DoubleFlowLeftInnerLeftState;
    type RightBranch = DoubleFlowLeftInnerRightState;

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

impl ReplayBranchHostState for DoubleFlowRightState {
    type LeftBranch = DoubleFlowRightInnerLeftState;
    type RightBranch = DoubleFlowRightInnerRightState;

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

pub(crate) struct ReplayAlwaysTrue;

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

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest-example";
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
        recv.next().await.expect("All done.")
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

#[jungle::effect(id = 1003)]
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
        state
            .replay_history_mut()
            .push(if tocked { '1' } else { '0' });
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
struct ReplayHeadLeftBranch<St: ReplayNodeState>(
    Step<Label<St, 'O'>>,
    Step<SleepFor<St, Millis<100>>>,
    Step<Tick<St>>,
    Step<SleepFor<St, Millis<100>>>,
);

#[derive(Flow)]
struct ReplayHeadRightBranch<St: ReplayNodeState>(
    Step<Label<St, 'Q'>>,
    Step<SleepFor<St, Millis<100>>>,
    Step<Tick<St>>,
    Step<SleepFor<St, Millis<100>>>,
);

#[derive(Flow)]
struct ReplayTailLeftBranch<St: ReplayNodeState>(
    Step<Label<St, 'A'>>,
    Step<SleepFor<St, Millis<100>>>,
    Step<Tick<St>>,
);

#[derive(Flow)]
struct ReplayTailRightBranch<St: ReplayNodeState>(
    Step<Label<St, 'B'>>,
    Step<SleepFor<St, Millis<100>>>,
    Step<Tick<St>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowLeftInnerLeftState)]
struct DoubleFlowLeftInnerLeftFlow(
    Step<Label<DoubleFlowLeftInnerLeftState, '2'>>,
    Step<SleepFor<DoubleFlowLeftInnerLeftState, Millis<100>>>,
    Step<Tick<DoubleFlowLeftInnerLeftState>>,
    Step<SleepFor<DoubleFlowLeftInnerLeftState, Millis<100>>>,
    Step<Tick<DoubleFlowLeftInnerLeftState>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowLeftInnerRightState)]
struct DoubleFlowLeftInnerRightFlow(
    Step<Label<DoubleFlowLeftInnerRightState, 'R'>>,
    Step<SleepFor<DoubleFlowLeftInnerRightState, Millis<100>>>,
    Step<Tick<DoubleFlowLeftInnerRightState>>,
    Step<SleepFor<DoubleFlowLeftInnerRightState, Millis<100>>>,
    Step<Tick<DoubleFlowLeftInnerRightState>>,
);

#[derive(Flow)]
struct DoubleFlowLeftJoinedInnerBody(
    jungle_zoo::ClonedJoinUnit<DoubleFlowLeftInnerLeftFlow, DoubleFlowLeftInnerRightFlow>,
    Step<MergeReplayJoin<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowLeftOuterBody(
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    Conditional<
        ReplayColorIsTrue,
        ReplayHeadLeftBranch<DoubleFlowLeftState>,
        ReplayHeadRightBranch<DoubleFlowLeftState>,
    >,
    Step<FlattenReplayChoice<DoubleFlowLeftState>>,
    Step<SeedReplayBranches<DoubleFlowLeftState>>,
    While<ReplayColorIsTrue, DoubleFlowLeftJoinedInnerBody>,
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    Conditional<
        ReplayColorIsTrue,
        ReplayTailLeftBranch<DoubleFlowLeftState>,
        ReplayTailRightBranch<DoubleFlowLeftState>,
    >,
    Step<FlattenReplayChoice<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowLeftOuterLeft(
    Step<Label<DoubleFlowLeftState, 'T'>>,
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    Step<Label<DoubleFlowLeftState, 'L'>>,
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    DoubleFlowLeftOuterBody,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowLeftOuterRight(
    Step<Label<DoubleFlowLeftState, 'T'>>,
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    Step<Label<DoubleFlowLeftState, 'R'>>,
    Step<Tick<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    DoubleFlowLeftOuterBody,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowLeftState)]
struct DoubleFlowLeft(
    Step<Label<DoubleFlowLeftState, 'D'>>,
    Step<Label<DoubleFlowLeftState, 'L'>>,
    Conditional<ReplayColorIsTrue, DoubleFlowLeftOuterLeft, DoubleFlowLeftOuterRight>,
    Step<FlattenReplayChoice<DoubleFlowLeftState>>,
    Step<SleepFor<DoubleFlowLeftState, Millis<100>>>,
    Step<Tick<DoubleFlowLeftState>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowRightInnerLeftState)]
struct DoubleFlowRightInnerLeftFlow(
    Step<Label<DoubleFlowRightInnerLeftState, '2'>>,
    Step<SleepFor<DoubleFlowRightInnerLeftState, Millis<100>>>,
    Step<Tick<DoubleFlowRightInnerLeftState>>,
    Step<SleepFor<DoubleFlowRightInnerLeftState, Millis<100>>>,
    Step<Tick<DoubleFlowRightInnerLeftState>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowRightInnerRightState)]
struct DoubleFlowRightInnerRightFlow(
    Step<Label<DoubleFlowRightInnerRightState, 'R'>>,
    Step<SleepFor<DoubleFlowRightInnerRightState, Millis<100>>>,
    Step<Tick<DoubleFlowRightInnerRightState>>,
    Step<SleepFor<DoubleFlowRightInnerRightState, Millis<100>>>,
    Step<Tick<DoubleFlowRightInnerRightState>>,
);

#[derive(Flow)]
struct DoubleFlowRightJoinedInnerBody(
    jungle_zoo::ClonedJoinUnit<DoubleFlowRightInnerLeftFlow, DoubleFlowRightInnerRightFlow>,
    Step<MergeReplayJoin<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowRightOuterBody(
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    Conditional<
        ReplayColorIsTrue,
        ReplayHeadLeftBranch<DoubleFlowRightState>,
        ReplayHeadRightBranch<DoubleFlowRightState>,
    >,
    Step<FlattenReplayChoice<DoubleFlowRightState>>,
    Step<SeedReplayBranches<DoubleFlowRightState>>,
    While<ReplayColorIsTrue, DoubleFlowRightJoinedInnerBody>,
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    Conditional<
        ReplayColorIsTrue,
        ReplayTailLeftBranch<DoubleFlowRightState>,
        ReplayTailRightBranch<DoubleFlowRightState>,
    >,
    Step<FlattenReplayChoice<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowRightOuterLeft(
    Step<Label<DoubleFlowRightState, 'T'>>,
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    Step<Label<DoubleFlowRightState, 'L'>>,
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    DoubleFlowRightOuterBody,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
);

#[derive(Flow)]
struct DoubleFlowRightOuterRight(
    Step<Label<DoubleFlowRightState, 'T'>>,
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    Step<Label<DoubleFlowRightState, 'R'>>,
    Step<Tick<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    DoubleFlowRightOuterBody,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
);

#[derive(Flow)]
#[jungle(focus = DoubleFlowRightState)]
struct DoubleFlowRight(
    Step<Label<DoubleFlowRightState, 'D'>>,
    Step<Label<DoubleFlowRightState, 'R'>>,
    Conditional<ReplayColorIsTrue, DoubleFlowRightOuterLeft, DoubleFlowRightOuterRight>,
    Step<FlattenReplayChoice<DoubleFlowRightState>>,
    Step<SleepFor<DoubleFlowRightState, Millis<100>>>,
    Step<Tick<DoubleFlowRightState>>,
);

#[derive(Flow)]
struct QuadFlow(
    jungle_zoo::ClonedJoinUnit<DoubleFlowLeft, DoubleFlowRight>,
    Step<MergeReplayJoin<ReplayState>>,
    Step<SleepFor<ReplayState, Millis<100>>>,
    Step<Tick<ReplayState>>,
);

#[derive(Flow)]
struct ReplayFlow(While<ReplayAlwaysTrue, QuadFlow>);

pub(crate) struct Depth2;

#[jungle::animal(observe, id = 1004, generation = 0)]
impl Animal for Depth2 {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = ReplayFlow;
}

#[derive(Animals)]
struct ReplayRainforestAnimals(Depth2);

impl Observe for Depth2 {
    type Appearance = String;

    fn observe(state: &Self::State) -> Self::Appearance {
        state.replay_appearance()
    }
}

fn replay_rainforest(
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
) -> ReplayRainforest {
    ReplayRainforest(Arc::new(tokio::sync::Mutex::new(ReplayInner {
        query,
        end,
        recv: Arc::new(tokio::sync::Mutex::new(recv)),
    })))
}

fn spawn_replay_worker(
    client: FusedClient,
    jungle: ReplayRainforest,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let worker = JungleWorker::new(jungle, client);
        if let Err(err) = worker.spawn().await {
            warn!(error = %err, "replay worker exited");
        }
    })
}

#[derive(Clone)]
struct ReplayLifecycle(Arc<AtomicU8>);

impl ReplayLifecycle {
    const INITIAL: u8 = 0;
    const REPLAY_READY: u8 = 1;

    fn new() -> Self {
        Self(Arc::new(AtomicU8::new(Self::INITIAL)))
    }

    fn request_replay_viewer(&self) {
        self.0.store(Self::REPLAY_READY, Ordering::Relaxed);
    }

    fn take_replay_viewer_request(&self) -> bool {
        self.0
            .compare_exchange(
                Self::REPLAY_READY,
                Self::INITIAL,
                Ordering::Relaxed,
                Ordering::Relaxed,
            )
            .is_ok()
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    let query = parse_query_bits(&cli.query)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    let image_dump = cli.img_dump.map(|output_path| {
        ui::ImageDumpConfig::new(
            output_path,
            Duration::from_secs_f64(cli.img_dump_time_secs.unwrap_or(0.0)),
        )
    });

    let client = FusedClient::builder()
        .namespace(format!("{}-{}", ReplayRainforest::NAME, Uuid::new_v4()))
        .build()
        .await?;

    let (end_tx, mut end_rx) = futures::channel::mpsc::unbounded::<()>();
    let (worker_one_resume_tx, worker_one_resume_rx) = futures::channel::mpsc::unbounded::<bool>();
    let worker_one = spawn_replay_worker(
        client.clone(),
        replay_rainforest(query, end_tx.clone(), worker_one_resume_rx),
    );
    let worker_one_abort = worker_one.abort_handle();

    let journey_id = client
        .spawn::<Depth2>(&ReplayState::default())
        .await?
        .journey_id;
    info!(%journey_id, "started replay example journey");

    let lifecycle = ReplayLifecycle::new();
    let lifecycle_on_boundary = lifecycle.clone();
    let worker_two_slot = Arc::new(tokio::sync::Mutex::new(None));
    let worker_two_slot_on_boundary = worker_two_slot.clone();
    let replay_client = client.clone();
    let boundary_task = tokio::spawn(async move {
        if end_rx.next().await.is_some() {
            info!("initial execution hit replay boundary; restarting worker and viewer");
            worker_one_abort.abort();
            tokio::time::sleep(REPLAY_VIEWER_LINGER_AFTER_END).await;

            let (_worker_two_resume_tx, worker_two_resume_rx) =
                futures::channel::mpsc::unbounded::<bool>();
            let worker_two = spawn_replay_worker(
                replay_client.clone(),
                replay_rainforest(Vec::new(), end_tx, worker_two_resume_rx),
            );
            *worker_two_slot_on_boundary.lock().await = Some(worker_two);
            lifecycle_on_boundary.request_replay_viewer();
        }
    });

    let ui_result = tokio::task::block_in_place(|| {
        ui::run_ui(client.clone(), journey_id, lifecycle, image_dump)
    });

    boundary_task.abort();
    let _ = boundary_task.await;

    worker_one.abort();
    let _ = worker_one.await;
    drop(worker_one_resume_tx);

    if let Some(worker_two) = worker_two_slot.lock().await.take() {
        worker_two.abort();
        let _ = worker_two.await;
    }

    ui_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_query_bits() {
        assert_eq!(parse_query_bits("").unwrap(), Vec::<bool>::new());
        assert_eq!(
            parse_query_bits("0101").unwrap(),
            vec![true, false, true, false]
        );
    }

    #[test]
    fn rejects_non_binary_query_bits() {
        assert!(parse_query_bits("012").is_err());
    }

    #[test]
    fn clap_accepts_query_as_string() {
        let cli = Cli::try_parse_from(["replay", "--query", "0101"]).unwrap();
        assert_eq!(cli.query, "0101");
    }

    #[test]
    fn parses_non_negative_img_dump_time_secs() {
        assert_eq!(parse_img_dump_time_secs("0").unwrap(), 0.0);
        assert_eq!(parse_img_dump_time_secs("30.5").unwrap(), 30.5);
    }

    #[test]
    fn rejects_negative_img_dump_time_secs() {
        assert!(parse_img_dump_time_secs("-1").is_err());
    }
}
