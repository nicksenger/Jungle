use clap::Parser;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{FusedClient, Server};
use jungle_zoo::action_backoff::{
    BackoffPending, BackoffShouldSleep, CloneBackoffInput, ExponentialBackoffInput,
    ExponentialBackoffPolicy, ExponentialBackoffState, FlattenEither, InitializeBackoff,
    RecordBackoffResult, SkipBackoffSleep, SleepBackoffBranch, TakeBackoffSuccess,
};
use jungle_zoo::subflow_backoff::{
    BackoffFlowPending, BackoffFlowShouldSleep, CloneBackoffFlowInput, ExponentialBackoffFlowState,
    InitializeBackoffFlow, RecordBackoffFlowResult, SkipBackoffFlowSleep, SleepBackoffFlowBranch,
    TakeBackoffFlowSuccess,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[cfg(feature = "viewer")]
mod ui;

const DEFAULT_LOG_FILTER: &str = "warn,backoff=info";
const INNER_ATTEMPT_SLEEP_MS: u64 = 200;
const DELAY_MULTIPLIER: u32 = 2;
const SUBFLOW_INITIAL_DELAY_MS: u64 = 250;
const SUBFLOW_MAX_DELAY_MS: u64 = 4_000;
const ACTION_INITIAL_DELAY_MS: u64 = 600;
const ACTION_MAX_DELAY_MS: u64 = 2_400;

fn subflow_backoff_policy() -> ExponentialBackoffPolicy {
    ExponentialBackoffPolicy {
        initial_delay_ms: SUBFLOW_INITIAL_DELAY_MS,
        multiplier: DELAY_MULTIPLIER,
        max_delay_ms: SUBFLOW_MAX_DELAY_MS,
    }
}

fn action_backoff_policy() -> ExponentialBackoffPolicy {
    ExponentialBackoffPolicy {
        initial_delay_ms: ACTION_INITIAL_DELAY_MS,
        multiplier: DELAY_MULTIPLIER,
        max_delay_ms: ACTION_MAX_DELAY_MS,
    }
}

fn with_backoff_policy(
    input: ExponentialBackoffInput<()>,
    policy: ExponentialBackoffPolicy,
) -> ExponentialBackoffInput<()> {
    ExponentialBackoffInput {
        action_input: input.action_input,
        policy,
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct SubflowBranchMetrics {
    started_attempts: u32,
    failed_attempts: u32,
    last_failure_message: Option<String>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct ActionBranchMetrics {
    started_attempts: u32,
    failed_attempts: u32,
    last_failure_message: Option<String>,
}

#[derive(Optic, Default, Clone, Debug, PartialEq)]
pub struct BackoffJourneyState {
    before_join_steps_completed: u32,
    after_join_steps_completed: u32,
    #[jungle(focus)]
    subflow_backoff: ExponentialBackoffFlowState<SubflowBranchMetrics, (), ()>,
    #[jungle(focus)]
    action_backoff: ExponentialBackoffState<ActionBranchMetrics, AlwaysFailingAction>,
}

impl ViewProject<BackoffJourneyState> for BackoffJourneyState {
    fn project_view(state: &mut Self) -> &mut BackoffJourneyState {
        state
    }
}

pub type SubflowBackoffState = ExponentialBackoffFlowState<SubflowBranchMetrics, (), ()>;
pub type ActionBackoffState = ExponentialBackoffState<ActionBranchMetrics, AlwaysFailingAction>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubflowBackoffAppearance {
    attempts: u32,
    next_delay_ms: u64,
    policy: ExponentialBackoffPolicy,
    last_result: Option<String>,
    metrics: SubflowBranchMetricsAppearance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBackoffAppearance {
    attempts: u32,
    next_delay_ms: u64,
    policy: ExponentialBackoffPolicy,
    last_result: Option<String>,
    metrics: ActionBranchMetricsAppearance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackoffAppearance {
    before_join_steps_completed: u32,
    after_join_steps_completed: u32,
    subflow: SubflowBackoffAppearance,
    action: ActionBackoffAppearance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubflowBranchMetricsAppearance {
    started_attempts: u32,
    failed_attempts: u32,
    last_failure_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBranchMetricsAppearance {
    started_attempts: u32,
    failed_attempts: u32,
    last_failure_message: Option<String>,
}

impl From<&SubflowBackoffState> for SubflowBackoffAppearance {
    fn from(state: &SubflowBackoffState) -> Self {
        Self {
            attempts: state.attempts,
            next_delay_ms: state.current_delay_ms,
            policy: state.policy,
            last_result: state.last_result.as_ref().map(|result| match result {
                Ok(()) => "unexpected success".to_owned(),
                Err(err) => err.to_string(),
            }),
            metrics: SubflowBranchMetricsAppearance {
                started_attempts: state.st.started_attempts,
                failed_attempts: state.st.failed_attempts,
                last_failure_message: state.st.last_failure_message.clone(),
            },
        }
    }
}

impl From<&ActionBackoffState> for ActionBackoffAppearance {
    fn from(state: &ActionBackoffState) -> Self {
        Self {
            attempts: state.attempts,
            next_delay_ms: state.current_delay_ms,
            policy: state.policy,
            last_result: state.last_result.as_ref().map(|result| match result {
                Ok(()) => "unexpected success".to_owned(),
                Err(err) => err.to_string(),
            }),
            metrics: ActionBranchMetricsAppearance {
                started_attempts: state.st.started_attempts,
                failed_attempts: state.st.failed_attempts,
                last_failure_message: state.st.last_failure_message.clone(),
            },
        }
    }
}

impl From<&BackoffJourneyState> for BackoffAppearance {
    fn from(state: &BackoffJourneyState) -> Self {
        Self {
            before_join_steps_completed: state.before_join_steps_completed,
            after_join_steps_completed: state.after_join_steps_completed,
            subflow: SubflowBackoffAppearance::from(&state.subflow_backoff),
            action: ActionBackoffAppearance::from(&state.action_backoff),
        }
    }
}

struct CountBeforeJoin;
#[jungle::action]
impl Action for CountBeforeJoin {
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<()>;
    type Output = ExponentialBackoffInput<()>;
    type Carry = ExponentialBackoffInput<()>;

    fn emit(_state: &BackoffJourneyState, input: Self::Input) -> ((), ExponentialBackoffInput<()>) {
        ((), input)
    }

    fn absorb(
        state: &mut BackoffJourneyState,
        output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<()>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("pre-join stub step should complete"))?;
        state.before_join_steps_completed = state.before_join_steps_completed.saturating_add(1);
        Ok(carry)
    }
}

struct CountAfterJoin;
#[jungle::action]
impl Action for CountAfterJoin {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &BackoffJourneyState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut BackoffJourneyState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("post-join stub step should complete"))?;
        state.after_join_steps_completed = state.after_join_steps_completed.saturating_add(1);
        Ok(())
    }
}

struct MarkSubflowAttemptStarted;
#[jungle::action]
impl Action for MarkSubflowAttemptStarted {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SubflowBranchMetrics, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut SubflowBranchMetrics,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("subflow attempt marker should complete"))?;
        state.started_attempts = state.started_attempts.saturating_add(1);
        state.last_failure_message = None;
        Ok(())
    }
}

struct PauseInsideSubflowAttempt;
#[jungle::action]
impl Action for PauseInsideSubflowAttempt {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &SubflowBranchMetrics, _input: Self::Input) -> Duration {
        Duration::from_millis(INNER_ATTEMPT_SLEEP_MS)
    }

    fn absorb(
        _state: &mut SubflowBranchMetrics,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        Ok(())
    }
}

struct FailSubflowAttempt;
#[jungle::action]
impl Action for FailSubflowAttempt {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &SubflowBranchMetrics, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut SubflowBranchMetrics,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("failing subflow step should complete"))?;
        state.failed_attempts = state.failed_attempts.saturating_add(1);
        let message = format!(
            "subflow attempt {} failed on purpose",
            state.started_attempts
        );
        state.last_failure_message = Some(message.clone());
        Err(Failure::from(message))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlwaysFailingAction;
#[jungle::action]
impl Action for AlwaysFailingAction {
    type Effect = Sleep;
    type Input = ();
    type Output = Result<(), Failure>;

    fn emit(_state: &ActionBranchMetrics, _input: Self::Input) -> Duration {
        Duration::from_millis(INNER_ATTEMPT_SLEEP_MS)
    }

    fn absorb(
        state: &mut ActionBranchMetrics,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        state.started_attempts = state.started_attempts.saturating_add(1);
        state.failed_attempts = state.failed_attempts.saturating_add(1);
        let message = format!(
            "single-action attempt {} failed on purpose",
            state.started_attempts
        );
        state.last_failure_message = Some(message.clone());
        Ok(Err(Failure::from(message)))
    }
}

struct FlattenJoinedUnits;
#[jungle::action]
impl Action for FlattenJoinedUnits {
    type Effect = NoEffect;
    type Input = ((), ());
    type Output = ();
    type Carry = ((), ());

    fn emit(_state: &BackoffJourneyState, input: Self::Input) -> ((), ((), ())) {
        ((), input)
    }

    fn absorb(
        _state: &mut BackoffJourneyState,
        _output: EffectCompletion<Self::Effect>,
        _carry: ((), ()),
    ) -> Result<Self::Output, Failure> {
        Ok(())
    }
}

struct EnterSubflowJoinArm;
#[jungle::action]
impl Action for EnterSubflowJoinArm {
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<()>;
    type Output = ExponentialBackoffInput<()>;
    type Carry = ExponentialBackoffInput<()>;

    fn emit(
        _state: &SubflowBackoffState,
        input: Self::Input,
    ) -> ((), ExponentialBackoffInput<()>) {
        ((), with_backoff_policy(input, subflow_backoff_policy()))
    }

    fn absorb(
        _state: &mut SubflowBackoffState,
        output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<()>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("subflow join arm stub should complete"))?;
        Ok(carry)
    }
}

struct EnterActionJoinArm;
#[jungle::action]
impl Action for EnterActionJoinArm {
    type Effect = NoEffect;
    type Input = ExponentialBackoffInput<()>;
    type Output = ExponentialBackoffInput<()>;
    type Carry = ExponentialBackoffInput<()>;

    fn emit(
        _state: &ActionBackoffState,
        input: Self::Input,
    ) -> ((), ExponentialBackoffInput<()>) {
        ((), with_backoff_policy(input, action_backoff_policy()))
    }

    fn absorb(
        _state: &mut ActionBackoffState,
        output: EffectCompletion<Self::Effect>,
        carry: ExponentialBackoffInput<()>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("action join arm stub should complete"))?;
        Ok(carry)
    }
}

#[derive(Flow)]
struct AlwaysFailingSubflow(
    Step<MarkSubflowAttemptStarted>,
    Step<PauseInsideSubflowAttempt>,
    Step<FailSubflowAttempt>,
);

#[derive(Flow)]
#[jungle(focus = SubflowBackoffState)]
struct ScopedSubflowBackoffBody(
    Step<CloneBackoffFlowInput<SubflowBranchMetrics, (), ()>>,
    Attempt<Scoped<SubflowBranchMetrics, AlwaysFailingSubflow>>,
    Step<RecordBackoffFlowResult<SubflowBranchMetrics, (), ()>>,
    Conditional<
        FocusedCondition<BackoffFlowShouldSleep<SubflowBranchMetrics, (), ()>, SubflowBackoffState>,
        SleepBackoffFlowBranch<SubflowBranchMetrics, (), ()>,
        Step<SkipBackoffFlowSleep<SubflowBranchMetrics, (), ()>>,
    >,
    Step<FlattenEither<(), SubflowBackoffState>>,
);

#[derive(Flow)]
#[jungle(focus = SubflowBackoffState)]
struct SubflowBackoffBranch(
    Step<InitializeBackoffFlow<SubflowBranchMetrics, (), ()>>,
    While<
        FocusedLoopCondition<BackoffFlowPending<SubflowBranchMetrics, (), ()>, SubflowBackoffState>,
        ScopedSubflowBackoffBody,
    >,
    Step<TakeBackoffFlowSuccess<SubflowBranchMetrics, (), ()>>,
);

#[derive(Flow)]
#[jungle(focus = ActionBackoffState)]
struct ScopedActionBackoffBody(
    Step<CloneBackoffInput<ActionBranchMetrics, AlwaysFailingAction>>,
    Scoped<ActionBranchMetrics, Step<AlwaysFailingAction>>,
    Step<RecordBackoffResult<ActionBranchMetrics, AlwaysFailingAction>>,
    Conditional<
        FocusedCondition<
            BackoffShouldSleep<ActionBranchMetrics, AlwaysFailingAction>,
            ActionBackoffState,
        >,
        SleepBackoffBranch<ActionBranchMetrics, AlwaysFailingAction>,
        Step<SkipBackoffSleep<ActionBranchMetrics, AlwaysFailingAction>>,
    >,
    Step<FlattenEither<(), ActionBackoffState>>,
);

#[derive(Flow)]
#[jungle(focus = ActionBackoffState)]
struct ActionBackoffBranch(
    Step<InitializeBackoff<ActionBranchMetrics, AlwaysFailingAction>>,
    While<
        FocusedLoopCondition<
            BackoffPending<ActionBranchMetrics, AlwaysFailingAction>,
            ActionBackoffState,
        >,
        ScopedActionBackoffBody,
    >,
    Step<TakeBackoffSuccess<ActionBranchMetrics, AlwaysFailingAction>>,
);

#[derive(Flow)]
#[jungle(focus = SubflowBackoffState)]
struct SubflowJoinArm(Step<EnterSubflowJoinArm>, SubflowBackoffBranch);

#[derive(Flow)]
#[jungle(focus = ActionBackoffState)]
struct ActionJoinArm(Step<EnterActionJoinArm>, ActionBackoffBranch);

#[derive(Flow)]
struct BackoffJourney(
    Step<CountBeforeJoin>,
    Step<CountBeforeJoin>,
    Join<SubflowJoinArm, ActionJoinArm>,
    Step<FlattenJoinedUnits>,
    Step<CountAfterJoin>,
    Step<CountAfterJoin>,
);

struct BackoffAnimal;

#[jungle::animal(id = 211, generation = 0)]
impl Animal for BackoffAnimal {
    type State = BackoffJourneyState;
    type Seed = ExponentialBackoffInput<()>;
    type Flow = BackoffJourney;
}

impl Observe for BackoffAnimal {
    type Appearance = BackoffAppearance;

    fn observe(state: &Self::State) -> Self::Appearance {
        BackoffAppearance::from(state)
    }
}

#[derive(Animals)]
struct BackoffAnimals(BackoffAnimal);

struct BackoffZoo;
impl Ecosystem for BackoffZoo {
    const NAME: &'static str = "backoff-zoo";
    type Animals = BackoffAnimals;
}

#[derive(Debug, Parser)]
#[command(name = "backoff")]
struct Cli {
    #[arg(
        long = "img-dump",
        help = "Capture the backoff UI to this PNG path and then exit"
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

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    let cli = Cli::parse();
    let db_path = std::env::temp_dir().join(format!("jungle-backoff-{}.redb", Uuid::new_v4()));

    info!(
        db_path = %db_path.display(),
        multiplier = DELAY_MULTIPLIER,
        subflow_initial_delay_ms = SUBFLOW_INITIAL_DELAY_MS,
        subflow_max_delay_ms = SUBFLOW_MAX_DELAY_MS,
        action_initial_delay_ms = ACTION_INITIAL_DELAY_MS,
        action_max_delay_ms = ACTION_MAX_DELAY_MS,
        inner_attempt_sleep_ms = INNER_ATTEMPT_SLEEP_MS,
        "starting backoff runtime"
    );

    let backend = Server::builder().redb_path(&db_path).build().await?;
    let client = FusedClient::builder()
        .namespace(BackoffZoo::NAME)
        .backend(backend)
        .build()
        .await?;

    let worker_client = client.clone();
    let worker_handle = tokio::spawn(async move {
        let worker = JungleWorker::new(BackoffZoo, worker_client);
        if let Err(err) = worker.spawn().await {
            warn!(error = %err, "backoff worker exited");
        }
    });

    let seed = ExponentialBackoffInput {
        action_input: (),
        policy: subflow_backoff_policy(),
    };
    let journey_id = client.spawn::<BackoffAnimal>(&seed).await?.journey_id;

    info!(
        %journey_id,
        db_path = %db_path.display(),
        "backoff demo active"
    );

    #[cfg(feature = "viewer")]
    {
        let image_dump = cli.img_dump.map(|output_path| {
            ui::ImageDumpConfig::new(
                output_path,
                Duration::from_secs_f64(cli.img_dump_time_secs.unwrap_or(0.0)),
            )
        });
        tokio::task::block_in_place(|| ui::run_ui(client.clone(), journey_id, image_dump))?;
    }

    #[cfg(not(feature = "viewer"))]
    if cli.img_dump.is_some() {
        warn!("--img-dump was ignored because backoff was built without the `viewer` feature");
    }
    #[cfg(not(feature = "viewer"))]
    tokio::signal::ctrl_c().await?;
    #[cfg(not(feature = "viewer"))]
    info!("received ctrl-c; shutting down backoff worker");
    #[cfg(feature = "viewer")]
    info!("backoff viewer closed; shutting down worker");

    worker_handle.abort();
    let _ = worker_handle.await;

    Ok(())
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
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .compact()
        .try_init();
    debug!("backoff tracing initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use jungle_zoo::action_backoff::{BackoffLogKind, BackoffSleepLog};
    use tokio::time::{Duration, Instant};

    #[test]
    fn parses_non_negative_img_dump_time_secs() {
        assert_eq!(parse_img_dump_time_secs("0").unwrap(), 0.0);
        assert_eq!(parse_img_dump_time_secs("30.5").unwrap(), 30.5);
    }

    #[test]
    fn rejects_negative_img_dump_time_secs() {
        assert!(parse_img_dump_time_secs("-1").is_err());
    }

    #[test]
    fn join_arms_use_distinct_backoff_policies() {
        assert_ne!(subflow_backoff_policy(), action_backoff_policy());
        assert_ne!(
            subflow_backoff_policy().initial_delay_ms,
            action_backoff_policy().initial_delay_ms
        );
        assert_ne!(
            subflow_backoff_policy().max_delay_ms,
            action_backoff_policy().max_delay_ms
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn join_runs_both_backoff_arms() {
        let namespace = format!("{}-{}", BackoffZoo::NAME, Uuid::new_v4());
        let client = FusedClient::builder()
            .namespace(namespace)
            .build()
            .await
            .expect("local client should build");
        let worker = JungleWorker::new(BackoffZoo, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = ExponentialBackoffInput {
            action_input: (),
            policy: subflow_backoff_policy(),
        };

        let journey_id = client
            .spawn::<BackoffAnimal>(&seed)
            .await
            .expect("backoff journey should start")
            .journey_id;
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut saw_subflow_sleep = false;
        let mut saw_action_sleep = false;
        while Instant::now() < deadline {
            for event in client
                .journey_history(journey_id)
                .await
                .expect("journey_history should succeed while the demo is running")
            {
                let RunnerOut::EffectInput { data, .. } = event else {
                    continue;
                };
                let Ok(log) = postcard::from_bytes::<BackoffSleepLog>(&data) else {
                    continue;
                };
                match log.kind {
                    BackoffLogKind::Subflow => saw_subflow_sleep = true,
                    BackoffLogKind::Action => saw_action_sleep = true,
                }
            }
            if saw_subflow_sleep && saw_action_sleep {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        worker_handle.abort();

        assert!(
            saw_subflow_sleep,
            "expected subflow backoff arm activity"
        );
        assert!(
            saw_action_sleep,
            "expected single-action backoff arm activity"
        );
    }
}
