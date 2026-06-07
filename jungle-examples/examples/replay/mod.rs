use clap::Parser;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::FusedClient;
use serde::{Deserialize, Serialize};
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
    #[arg(long, help = "Bitstring query, for example 01000101")]
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
    recv: UnboundedReceiver<bool>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayState {
    color: bool,
    history: String,
}

impl From<ReplayState> for () {
    fn from(_value: ReplayState) -> Self {}
}

pub(crate) struct ReplayColorIsTrue;

impl Predicate<(&ReplayState, &())> for ReplayColorIsTrue {
    fn eval((state, _): &(&ReplayState, &())) -> bool {
        state.color
    }
}

impl Predicate<(ReplayState, ())> for ReplayColorIsTrue {
    fn eval((state, _): &(ReplayState, ())) -> bool {
        state.color
    }
}

pub(crate) struct ReplayAlwaysTrue;

impl Predicate<(&ReplayState, &())> for ReplayAlwaysTrue {
    fn eval((_state, _): &(&ReplayState, &())) -> bool {
        true
    }
}

impl Ecosystem for ReplayRainforest {
    const NAME: &'static str = "replay-rainforest-example";
    type Animals = ReplayRainforestAnimals;
}

impl ReplayRainforest {
    async fn next(&self) -> bool {
        let mut inner = self.0.lock().await;
        match inner.query.pop() {
            Some(value) => value,
            None => {
                let _ = inner.end.unbounded_send(());
                inner
                    .recv
                    .next()
                    .await
                    .expect("replay receiver should yield a bool after query exhaustion")
            }
        }
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

pub struct Tick;

#[jungle::action]
impl Action for Tick {
    type Effect = Tock;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let tocked = output.map_err(|_err| Failure::from("tock should succeed"))?;
        if tocked {
            state.color = true;
            state.history.push('1');
        } else {
            state.color = false;
            state.history.push('0');
        }
        Ok(())
    }
}

pub struct Label<const CH: char>;

#[jungle::action]
impl<const CH: char> Action for Label<CH> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("label should complete without effect"))?;
        state.history.push(CH);
        Ok(())
    }
}

pub struct FlattenReplayChoice;

#[jungle::action]
impl Action for FlattenReplayChoice {
    type Effect = NoEffect;
    type Input = Either<(), ()>;
    type Output = ();
    type Carry = Either<(), ()>;

    fn emit(_state: &ReplayState, input: Self::Input) -> ((), Self::Carry) {
        ((), input)
    }

    fn absorb(
        _state: &mut ReplayState,
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

pub struct SleepMillis<const T: u64>;
#[jungle::action]
impl<const T: u64> Action for SleepMillis<T> {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayState, input: Self::Input) -> Duration {
        Duration::from_millis(T)
    }
    fn absorb(
        _state: &mut ReplayState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

#[derive(Flow)]
struct Depth1LeftBranch(
    Step<Label<'L'>>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
);

#[derive(Flow)]
struct Depth1RightBranch(
    Step<SleepMillis<100>>,
    Step<Label<'R'>>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
);

#[derive(Flow)]
struct Depth1InnerBody(
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Conditional<ReplayColorIsTrue, Depth1LeftBranch, Depth1RightBranch>,
    Step<FlattenReplayChoice>,
    Step<SleepMillis<100>>,
);

#[derive(Flow)]
struct Depth1OuterBody(
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    While<ReplayColorIsTrue, Depth1InnerBody>,
    Step<SleepMillis<100>>,
    Step<Tick>,
    Step<SleepMillis<100>>,
    Step<Tick>,
);

#[derive(Flow)]
struct Depth1Flow(While<ReplayAlwaysTrue, Depth1OuterBody>);

pub(crate) struct Depth1;

#[jungle::animal(id = 1004, generation = 0)]
impl Animal for Depth1 {
    type State = ReplayState;
    type Seed = ReplayState;
    type Flow = Depth1Flow;
}

#[derive(Animals)]
struct ReplayRainforestAnimals(Depth1);

fn replay_rainforest(
    query: Vec<bool>,
    end: UnboundedSender<()>,
    recv: UnboundedReceiver<bool>,
) -> ReplayRainforest {
    ReplayRainforest(Arc::new(tokio::sync::Mutex::new(ReplayInner {
        query,
        end,
        recv,
    })))
}

fn spawn_depth1_worker(
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
    let worker_one = spawn_depth1_worker(
        client.clone(),
        replay_rainforest(query, end_tx.clone(), worker_one_resume_rx),
    );
    let worker_one_abort = worker_one.abort_handle();

    let journey_id = client
        .spawn::<Depth1>(&ReplayState::default())
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
            let worker_two = spawn_depth1_worker(
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
