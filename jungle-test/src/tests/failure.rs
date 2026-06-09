use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::{Animals, FusedClient, JourneyStatus, JungleClient, RunnerOut};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureState {
    steps_completed: u8,
    attempt_failures: u8,
    attempt_successes: u8,
    choose_left: bool,
    loop_remaining: u8,
}

pub struct PassStep;
#[jungle::action]
impl Action for PassStep {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &FailureState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("pass step should complete"))?;
        state.steps_completed = state.steps_completed.saturating_add(1);
        Ok(())
    }
}

pub struct FailStep;
pub struct FailStepEffect;
#[jungle::effect(id = 520)]
impl<J> Effect<J> for FailStepEffect {
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        _jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(()))
    }
}

#[jungle::action]
impl Action for FailStep {
    type Effect = FailStepEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &FailureState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("intentional absorb failure"))
    }
}

pub struct ExpectAttemptErrStep;
pub struct AttemptOutcomeEffect;
#[jungle::effect(id = 521)]
impl<J> Effect<J> for AttemptOutcomeEffect {
    type In = Result<(), Failure>;
    type Out = Result<(), Failure>;
    type Err = String;

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

#[jungle::action]
impl Action for ExpectAttemptErrStep {
    type Effect = AttemptOutcomeEffect;
    type Input = Result<(), Failure>;
    type Output = ();

    fn emit(_state: &FailureState, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let attempt_outcome = output?;
        match attempt_outcome {
            Ok(()) => Err(Failure::from("expected attempt to return Err")),
            Err(_failure) => {
                state.attempt_failures = state.attempt_failures.saturating_add(1);
                Ok(())
            }
        }
    }
}

pub struct ExpectAttemptOkStep;
#[jungle::action]
impl Action for ExpectAttemptOkStep {
    type Effect = AttemptOutcomeEffect;
    type Input = Result<(), Failure>;
    type Output = ();

    fn emit(_state: &FailureState, input: Self::Input) -> <Self::Effect as EffectSchema>::In {
        input
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let attempt_outcome = output?;
        match attempt_outcome {
            Ok(()) => {
                state.attempt_successes = state.attempt_successes.saturating_add(1);
                Ok(())
            }
            Err(_failure) => Err(Failure::from("expected attempt to return Ok")),
        }
    }
}

#[derive(Flow)]
pub struct FailureJourney(Step<PassStep>, Step<FailStep>);

pub struct FailureAnimal;

#[jungle::animal(id = 97, generation = 0)]
impl Animal for FailureAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = FailureJourney;
}

#[derive(Flow)]
pub struct AttemptFailureJourney(
    Step<PassStep>,
    Attempt<Step<FailStep>>,
    Step<ExpectAttemptErrStep>,
    Step<PassStep>,
);

pub struct AttemptFailureAnimal;

#[jungle::animal(id = 98, generation = 0)]
impl Animal for AttemptFailureAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = AttemptFailureJourney;
}

#[derive(Flow)]
pub struct AttemptSuccessJourney(
    Step<PassStep>,
    Attempt<Step<PassStep>>,
    Step<ExpectAttemptOkStep>,
    Step<PassStep>,
);

pub struct AttemptSuccessAnimal;

#[jungle::animal(id = 99, generation = 0)]
impl Animal for AttemptSuccessAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = AttemptSuccessJourney;
}

#[derive(Animals)]
pub struct FailureAnimals(FailureAnimal, AttemptFailureAnimal, AttemptSuccessAnimal);

pub struct FailureZoo;
impl Ecosystem for FailureZoo {
    const NAME: &'static str = "failure-zoo";
    type Animals = FailureAnimals;
}

async fn wait_for_status(
    client: &FusedClient,
    journey_id: uuid::Uuid,
    target: JourneyStatus,
) -> JourneyStatus {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            if status == target {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("journey should reach expected status before timeout")
}

async fn wait_for_terminal(client: &FusedClient, journey_id: uuid::Uuid) -> JourneyStatus {
    let terminal = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed");
            match status {
                JourneyStatus::Completed | JourneyStatus::Dead | JourneyStatus::Stopped => {
                    break status;
                }
                JourneyStatus::Created | JourneyStatus::Alive => {}
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    if let Ok(status) = terminal {
        return status;
    }

    let status = client
        .journey_details(journey_id)
        .await
        .expect("journey_details should succeed after timeout");
    let history = client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed after timeout");
    panic!(
        "journey should reach terminal status before timeout; status={status:?}, history={history:?}"
    );
}

macro_rules! run_case {
    ($zoo:expr, $animal:ty, $namespace:expr) => {{
        let client = FusedClient::builder()
            .namespace($namespace)
            .build()
            .await
            .expect("local client should build");
        let worker = JungleWorker::new($zoo, client.clone());
        let worker_handle = tokio::spawn(async move {
            let _ = worker.spawn().await;
        });

        let seed = ();
        let journey_id = client
            .spawn::<$animal>(&seed)
            .await
            .expect("journey should start")
            .journey_id;
        let status = wait_for_terminal(&client, journey_id).await;
        let history = client
            .journey_history(journey_id)
            .await
            .expect("journey_history should succeed");

        worker_handle.abort();
        let _ = worker_handle.await;

        (status, history)
    }};
}

fn decode_i32_effect_inputs(history: &[RunnerOut]) -> Vec<i32> {
    history
        .iter()
        .filter_map(|entry| match entry {
            RunnerOut::EffectInput { data, .. } => postcard::take_from_bytes::<i32>(data)
                .ok()
                .and_then(|(value, remaining)| remaining.is_empty().then_some(value)),
            _ => None,
        })
        .collect()
}

fn assert_history_effect_counts(history: &[RunnerOut], inputs: usize, successes: usize) {
    let input_count = history
        .iter()
        .filter(|event| matches!(event, RunnerOut::EffectInput { .. }))
        .count();
    let success_count = history
        .iter()
        .filter(|event| matches!(event, RunnerOut::EffectSuccessOutput { .. }))
        .count();
    let failure_count = history
        .iter()
        .filter(|event| matches!(event, RunnerOut::EffectFailureOutput { .. }))
        .count();

    assert_eq!(input_count, inputs);
    assert_eq!(success_count, successes);
    assert_eq!(failure_count, 0, "effect should not fail in these tests");
}

pub struct TaggedEffect;
#[jungle::effect(id = 522)]
impl<J> Effect<J> for TaggedEffect {
    type In = i32;
    type Out = i32;
    type Err = ();

    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        std::future::ready(Ok(input))
    }
}

pub struct TimedTaggedEffect;
#[jungle::effect(id = 523)]
impl<J> Effect<J> for TimedTaggedEffect {
    type In = (u64, i32);
    type Out = i32;
    type Err = ();

    #[allow(clippy::manual_async_fn)]
    fn effect(
        _jungle: &J,
        input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        async move {
            std::thread::sleep(Duration::from_millis(input.0));
            Ok(input.1)
        }
    }
}

pub struct StartTagStep;
#[jungle::action]
impl Action for StartTagStep {
    type Effect = TaggedEffect;
    type Input = ();
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        100
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let value = output.map_err(|_err| Failure::from("start tag should succeed"))?;
        state.choose_left = true;
        state.loop_remaining = 1;
        Ok(value)
    }
}

pub struct PassThroughTagStep;
#[jungle::action]
impl Action for PassThroughTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        input
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let value = output.map_err(|_err| Failure::from("pass-through should succeed"))?;
        state.steps_completed = state.steps_completed.saturating_add(1);
        Ok(value)
    }
}

pub struct FailFromTagStep;
#[jungle::action]
impl Action for FailFromTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        input
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("intentional combinator absorb failure"))
    }
}

pub struct LeftFailTagStep;
#[jungle::action]
impl Action for LeftFailTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        201
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("left branch failed"))
    }
}

pub struct RightPassTagStep;
#[jungle::action]
impl Action for RightPassTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        202
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("right branch should succeed"))
    }
}

pub struct LoopPassTagStep;
#[jungle::action]
impl Action for LoopPassTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        301
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let value = output.map_err(|_err| Failure::from("loop pass should succeed"))?;
        state.loop_remaining = state.loop_remaining.saturating_sub(1);
        Ok(value)
    }
}

pub struct LoopFailTagStep;
#[jungle::action]
impl Action for LoopFailTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        302
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("loop body failed"))
    }
}

pub struct JoinLeftFailTagStep;
#[jungle::action]
impl Action for JoinLeftFailTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        401
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("join left failed"))
    }
}

pub struct JoinRightPassTagStep;
#[jungle::action]
impl Action for JoinRightPassTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        402
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join right should succeed"))
    }
}

pub struct JoinLeftPassTagStep;
#[jungle::action]
impl Action for JoinLeftPassTagStep {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        411
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join left pass should succeed"))
    }
}

pub struct FailFromJoinTupleStep;
#[jungle::action]
impl Action for FailFromJoinTupleStep {
    type Effect = TaggedEffect;
    type Input = (i32, i32);
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        901
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("intentional failure after join"))
    }
}

pub struct JoinRightPassTagStep2;
#[jungle::action]
impl Action for JoinRightPassTagStep2 {
    type Effect = TaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        412
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join right pass should succeed"))
    }
}

pub struct SelectFastFailTagStep;
#[jungle::action]
impl Action for SelectFastFailTagStep {
    type Effect = TimedTaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> (u64, i32) {
        (0, 501)
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("select fast failed"))
    }
}

pub struct SelectSlowPassTagStep;
#[jungle::action]
impl Action for SelectSlowPassTagStep {
    type Effect = TimedTaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> (u64, i32) {
        (25, 502)
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("select slow should succeed"))
    }
}

pub struct SelectFastPassTagStep;
#[jungle::action]
impl Action for SelectFastPassTagStep {
    type Effect = TimedTaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> (u64, i32) {
        (0, 511)
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("select fast should succeed"))
    }
}

pub struct FailFromSelectEitherStep;
#[jungle::action]
impl Action for FailFromSelectEitherStep {
    type Effect = TaggedEffect;
    type Input = Either<i32, i32>;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> i32 {
        902
    }

    fn absorb(
        _state: &mut FailureState,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        Err(Failure::from("intentional failure after select"))
    }
}

pub struct SelectSlowPassTagStep2;
#[jungle::action]
impl Action for SelectSlowPassTagStep2 {
    type Effect = TimedTaggedEffect;
    type Input = i32;
    type Output = i32;

    fn emit(_state: &FailureState, _input: Self::Input) -> (u64, i32) {
        (25, 512)
    }

    fn absorb(
        _state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("select slow pass should succeed"))
    }
}

pub struct AttemptResultAssertFailStep;
#[jungle::action]
impl Action for AttemptResultAssertFailStep {
    type Effect = TaggedEffect;
    type Input = Result<i32, Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 799,
            Err(_) => 700,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker = output.map_err(|_err| Failure::from("attempt fail marker should succeed"))?;
        if marker != 700 {
            return Err(Failure::from("expected Attempt to produce Err"));
        }
        state.attempt_failures = state.attempt_failures.saturating_add(1);
        Ok(marker)
    }
}

pub struct AttemptResultAssertOkStep;
#[jungle::action]
impl Action for AttemptResultAssertOkStep {
    type Effect = TaggedEffect;
    type Input = Result<i32, Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 800,
            Err(_) => 899,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker = output.map_err(|_err| Failure::from("attempt ok marker should succeed"))?;
        if marker != 800 {
            return Err(Failure::from("expected Attempt to produce Ok"));
        }
        state.attempt_successes = state.attempt_successes.saturating_add(1);
        Ok(marker)
    }
}

pub struct AttemptResultAssertFailEitherStep;
#[jungle::action]
impl Action for AttemptResultAssertFailEitherStep {
    type Effect = TaggedEffect;
    type Input = Result<Either<i32, i32>, Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 799,
            Err(_) => 700,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker =
            output.map_err(|_err| Failure::from("attempt fail either marker should succeed"))?;
        if marker != 700 {
            return Err(Failure::from("expected Attempt to produce Err"));
        }
        state.attempt_failures = state.attempt_failures.saturating_add(1);
        Ok(marker)
    }
}

pub struct AttemptResultAssertOkEitherStep;
#[jungle::action]
impl Action for AttemptResultAssertOkEitherStep {
    type Effect = TaggedEffect;
    type Input = Result<Either<i32, i32>, Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 800,
            Err(_) => 899,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker =
            output.map_err(|_err| Failure::from("attempt ok either marker should succeed"))?;
        if marker != 800 {
            return Err(Failure::from("expected Attempt to produce Ok"));
        }
        state.attempt_successes = state.attempt_successes.saturating_add(1);
        Ok(marker)
    }
}

pub struct AttemptResultAssertFailTupleStep;
#[jungle::action]
impl Action for AttemptResultAssertFailTupleStep {
    type Effect = TaggedEffect;
    type Input = Result<(i32, i32), Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 799,
            Err(_) => 700,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker =
            output.map_err(|_err| Failure::from("attempt fail tuple marker should succeed"))?;
        if marker != 700 {
            return Err(Failure::from("expected Attempt to produce Err"));
        }
        state.attempt_failures = state.attempt_failures.saturating_add(1);
        Ok(marker)
    }
}

pub struct AttemptResultAssertOkTupleStep;
#[jungle::action]
impl Action for AttemptResultAssertOkTupleStep {
    type Effect = TaggedEffect;
    type Input = Result<(i32, i32), Failure>;
    type Output = i32;

    fn emit(_state: &FailureState, input: Self::Input) -> i32 {
        match input {
            Ok(_) => 800,
            Err(_) => 899,
        }
    }

    fn absorb(
        state: &mut FailureState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let marker =
            output.map_err(|_err| Failure::from("attempt ok tuple marker should succeed"))?;
        if marker != 800 {
            return Err(Failure::from("expected Attempt to produce Ok"));
        }
        state.attempt_successes = state.attempt_successes.saturating_add(1);
        Ok(marker)
    }
}

pub struct AlwaysChooseLeft;
impl Predicate<(FailureState, i32)> for AlwaysChooseLeft {
    fn eval((state, _): &(FailureState, i32)) -> bool {
        state.choose_left
    }
}

pub struct KeepLooping;
impl Predicate<(&FailureState, &i32)> for KeepLooping {
    fn eval((state, _): &(&FailureState, &i32)) -> bool {
        state.loop_remaining > 0
    }
}

#[derive(Flow)]
pub struct ConditionalOutsideAttemptFailJourney(
    Step<StartTagStep>,
    Conditional<AlwaysChooseLeft, Step<LeftFailTagStep>, Step<RightPassTagStep>>,
);

#[derive(Flow)]
pub struct ConditionalInsideAttemptFailJourney(
    Step<StartTagStep>,
    Attempt<Conditional<AlwaysChooseLeft, Step<LeftFailTagStep>, Step<RightPassTagStep>>>,
    Step<AttemptResultAssertFailEitherStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct ConditionalInsideAttemptOkJourney(
    Step<StartTagStep>,
    Attempt<Conditional<AlwaysChooseLeft, Step<RightPassTagStep>, Step<RightPassTagStep>>>,
    Step<AttemptResultAssertOkEitherStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct WhileOutsideAttemptFailJourney(
    Step<StartTagStep>,
    While<KeepLooping, Step<LoopFailTagStep>>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct WhileInsideAttemptFailJourney(
    Step<StartTagStep>,
    Attempt<While<KeepLooping, Step<LoopFailTagStep>>>,
    Step<AttemptResultAssertFailStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct WhileInsideAttemptOkJourney(
    Step<StartTagStep>,
    Attempt<While<KeepLooping, Step<LoopPassTagStep>>>,
    Step<AttemptResultAssertOkStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct TransparentOutsideAttemptFailJourney(
    Step<StartTagStep>,
    Transparent<NoMetadata, Step<FailFromTagStep>>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct TransparentInsideAttemptFailJourney(
    Step<StartTagStep>,
    Attempt<Transparent<NoMetadata, Step<FailFromTagStep>>>,
    Step<AttemptResultAssertFailStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct TransparentInsideAttemptOkJourney(
    Step<StartTagStep>,
    Attempt<Transparent<NoMetadata, Step<PassThroughTagStep>>>,
    Step<AttemptResultAssertOkStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct JoinOutsideAttemptFailJourney(
    Step<StartTagStep>,
    jungle_zoo::ClonedJoin<i32, Step<JoinLeftPassTagStep>, Step<JoinRightPassTagStep2>>,
    Step<FailFromJoinTupleStep>,
);

#[derive(Flow)]
pub struct JoinInsideAttemptFailJourney(
    Step<StartTagStep>,
    jungle_zoo::ClonedJoin<i32, Step<JoinLeftPassTagStep>, Step<JoinRightPassTagStep2>>,
    Attempt<Step<FailFromJoinTupleStep>>,
    Step<AttemptResultAssertFailStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct JoinInsideAttemptOkJourney(
    Step<StartTagStep>,
    Attempt<jungle_zoo::ClonedJoin<i32, Step<JoinLeftPassTagStep>, Step<JoinRightPassTagStep2>>>,
    Step<AttemptResultAssertOkTupleStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct SelectOutsideAttemptFailJourney(
    Step<StartTagStep>,
    jungle_zoo::ClonedSelect<i32, Step<SelectFastPassTagStep>, Step<SelectSlowPassTagStep2>>,
    Step<FailFromSelectEitherStep>,
);

#[derive(Flow)]
pub struct SelectInsideAttemptFailJourney(
    Step<StartTagStep>,
    jungle_zoo::ClonedSelect<i32, Step<SelectFastPassTagStep>, Step<SelectSlowPassTagStep2>>,
    Attempt<Step<FailFromSelectEitherStep>>,
    Step<AttemptResultAssertFailStep>,
    Step<PassThroughTagStep>,
);

#[derive(Flow)]
pub struct SelectInsideAttemptOkJourney(
    Step<StartTagStep>,
    Attempt<
        jungle_zoo::ClonedSelect<i32, Step<SelectFastPassTagStep>, Step<SelectSlowPassTagStep2>>,
    >,
    Step<AttemptResultAssertOkEitherStep>,
    Step<PassThroughTagStep>,
);

pub struct ConditionalOutsideAttemptFailAnimal;
#[jungle::animal(id = 100, generation = 0)]
impl Animal for ConditionalOutsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = ConditionalOutsideAttemptFailJourney;
}

pub struct ConditionalInsideAttemptFailAnimal;
#[jungle::animal(id = 101, generation = 0)]
impl Animal for ConditionalInsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = ConditionalInsideAttemptFailJourney;
}

pub struct ConditionalInsideAttemptOkAnimal;
#[jungle::animal(id = 102, generation = 0)]
impl Animal for ConditionalInsideAttemptOkAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = ConditionalInsideAttemptOkJourney;
}

pub struct WhileOutsideAttemptFailAnimal;
#[jungle::animal(id = 103, generation = 0)]
impl Animal for WhileOutsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = WhileOutsideAttemptFailJourney;
}

pub struct WhileInsideAttemptFailAnimal;
#[jungle::animal(id = 104, generation = 0)]
impl Animal for WhileInsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = WhileInsideAttemptFailJourney;
}

pub struct WhileInsideAttemptOkAnimal;
#[jungle::animal(id = 105, generation = 0)]
impl Animal for WhileInsideAttemptOkAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = WhileInsideAttemptOkJourney;
}

pub struct TransparentOutsideAttemptFailAnimal;
#[jungle::animal(id = 106, generation = 0)]
impl Animal for TransparentOutsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = TransparentOutsideAttemptFailJourney;
}

pub struct TransparentInsideAttemptFailAnimal;
#[jungle::animal(id = 107, generation = 0)]
impl Animal for TransparentInsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = TransparentInsideAttemptFailJourney;
}

pub struct TransparentInsideAttemptOkAnimal;
#[jungle::animal(id = 108, generation = 0)]
impl Animal for TransparentInsideAttemptOkAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = TransparentInsideAttemptOkJourney;
}

pub struct JoinOutsideAttemptFailAnimal;
#[jungle::animal(id = 109, generation = 0)]
impl Animal for JoinOutsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = JoinOutsideAttemptFailJourney;
}

pub struct JoinInsideAttemptFailAnimal;
#[jungle::animal(id = 110, generation = 0)]
impl Animal for JoinInsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = JoinInsideAttemptFailJourney;
}

pub struct JoinInsideAttemptOkAnimal;
#[jungle::animal(id = 111, generation = 0)]
impl Animal for JoinInsideAttemptOkAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = JoinInsideAttemptOkJourney;
}

pub struct SelectOutsideAttemptFailAnimal;
#[jungle::animal(id = 112, generation = 0)]
impl Animal for SelectOutsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = SelectOutsideAttemptFailJourney;
}

pub struct SelectInsideAttemptFailAnimal;
#[jungle::animal(id = 113, generation = 0)]
impl Animal for SelectInsideAttemptFailAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = SelectInsideAttemptFailJourney;
}

pub struct SelectInsideAttemptOkAnimal;
#[jungle::animal(id = 114, generation = 0)]
impl Animal for SelectInsideAttemptOkAnimal {
    type State = FailureState;
    type Seed = ();
    type Flow = SelectInsideAttemptOkJourney;
}

#[derive(Animals)]
pub struct ConditionalFailureAnimals(
    ConditionalOutsideAttemptFailAnimal,
    ConditionalInsideAttemptFailAnimal,
    ConditionalInsideAttemptOkAnimal,
);

pub struct ConditionalFailureZoo;
impl Ecosystem for ConditionalFailureZoo {
    const NAME: &'static str = "conditional-failure-zoo";
    type Animals = ConditionalFailureAnimals;
}

#[derive(Animals)]
pub struct WhileFailureAnimals(
    WhileOutsideAttemptFailAnimal,
    WhileInsideAttemptFailAnimal,
    WhileInsideAttemptOkAnimal,
);

pub struct WhileFailureZoo;
impl Ecosystem for WhileFailureZoo {
    const NAME: &'static str = "while-failure-zoo";
    type Animals = WhileFailureAnimals;
}

#[derive(Animals)]
pub struct TransparentFailureAnimals(
    TransparentOutsideAttemptFailAnimal,
    TransparentInsideAttemptFailAnimal,
    TransparentInsideAttemptOkAnimal,
);

pub struct TransparentFailureZoo;
impl Ecosystem for TransparentFailureZoo {
    const NAME: &'static str = "transparent-failure-zoo";
    type Animals = TransparentFailureAnimals;
}

#[derive(Animals)]
pub struct JoinFailureAnimals(
    JoinOutsideAttemptFailAnimal,
    JoinInsideAttemptFailAnimal,
    JoinInsideAttemptOkAnimal,
);

pub struct JoinFailureZoo;
impl Ecosystem for JoinFailureZoo {
    const NAME: &'static str = "join-failure-zoo";
    type Animals = JoinFailureAnimals;
}

#[derive(Animals)]
pub struct SelectFailureAnimals(
    SelectOutsideAttemptFailAnimal,
    SelectInsideAttemptFailAnimal,
    SelectInsideAttemptOkAnimal,
);

pub struct SelectFailureZoo;
impl Ecosystem for SelectFailureZoo {
    const NAME: &'static str = "select-failure-zoo";
    type Animals = SelectFailureAnimals;
}

#[tokio::test]
async fn local_client_marks_journey_dead_when_absorb_returns_failure() {
    let client = FusedClient::builder()
        .namespace("absorb-failure-dead")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = ();
    let journey_id = client
        .spawn::<FailureAnimal>(&seed)
        .await
        .expect("journey should start")
        .journey_id;

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Dead).await;

    assert_eq!(final_status, JourneyStatus::Dead);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn attempt_catches_failure_and_journey_completes() {
    let client = FusedClient::builder()
        .namespace("attempt-catches-failure")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = ();
    let journey_id = client
        .spawn::<AttemptFailureAnimal>(&seed)
        .await
        .expect("journey should start")
        .journey_id;

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Completed).await;
    assert_eq!(final_status, JourneyStatus::Completed);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn attempt_wraps_success_and_journey_completes() {
    let client = FusedClient::builder()
        .namespace("attempt-wraps-success")
        .build()
        .await
        .expect("local client should build");

    let worker = JungleWorker::new(FailureZoo, client.clone());
    let worker_handle = tokio::spawn(async move {
        let _ = worker.spawn().await;
    });

    let seed = ();
    let journey_id = client
        .spawn::<AttemptSuccessAnimal>(&seed)
        .await
        .expect("journey should start")
        .journey_id;

    let final_status = wait_for_status(&client, journey_id, JourneyStatus::Completed).await;
    assert_eq!(final_status, JourneyStatus::Completed);

    worker_handle.abort();
    let _ = worker_handle.await;
}

#[tokio::test]
async fn combinator_failures_outside_attempt_mark_journey_dead_and_emit_expected_events() {
    let (status, history) = run_case!(
        ConditionalFailureZoo,
        ConditionalOutsideAttemptFailAnimal,
        "cond-outside-fail"
    );
    assert_eq!(status, JourneyStatus::Dead);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 201]);
    assert_history_effect_counts(&history, 2, 2);

    let (status, history) = run_case!(
        WhileFailureZoo,
        WhileOutsideAttemptFailAnimal,
        "while-outside-fail"
    );
    assert_eq!(status, JourneyStatus::Dead);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 302]);
    assert_history_effect_counts(&history, 2, 2);

    let (status, history) = run_case!(
        TransparentFailureZoo,
        TransparentOutsideAttemptFailAnimal,
        "transparent-outside-fail"
    );
    assert_eq!(status, JourneyStatus::Dead);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 100]);
    assert_history_effect_counts(&history, 2, 2);

    let (status, history) = run_case!(
        JoinFailureZoo,
        JoinOutsideAttemptFailAnimal,
        "join-outside-fail"
    );
    assert_eq!(status, JourneyStatus::Dead);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 411, 412, 901]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        SelectFailureZoo,
        SelectOutsideAttemptFailAnimal,
        "select-outside-fail"
    );
    assert_eq!(status, JourneyStatus::Dead);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 902]);
    assert_history_effect_counts(&history, 5, 5);
}

#[tokio::test]
async fn combinator_failures_inside_attempt_complete_and_emit_expected_events() {
    let (status, history) = run_case!(
        ConditionalFailureZoo,
        ConditionalInsideAttemptFailAnimal,
        "cond-inside-fail"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 201, 700, 700]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        WhileFailureZoo,
        WhileInsideAttemptFailAnimal,
        "while-inside-fail"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 302, 700, 700]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        TransparentFailureZoo,
        TransparentInsideAttemptFailAnimal,
        "transparent-inside-fail"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 100, 700, 700]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        JoinFailureZoo,
        JoinInsideAttemptFailAnimal,
        "join-inside-fail"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(
        decode_i32_effect_inputs(&history),
        vec![100, 411, 412, 901, 700, 700]
    );
    assert_history_effect_counts(&history, 6, 6);

    let (status, history) = run_case!(
        SelectFailureZoo,
        SelectInsideAttemptFailAnimal,
        "select-inside-fail"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 902, 700, 700]);
    assert_history_effect_counts(&history, 7, 7);
}

#[tokio::test]
async fn combinator_attempt_successes_complete_and_emit_expected_events() {
    let (status, history) = run_case!(
        ConditionalFailureZoo,
        ConditionalInsideAttemptOkAnimal,
        "cond-inside-ok"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 202, 800, 800]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        WhileFailureZoo,
        WhileInsideAttemptOkAnimal,
        "while-inside-ok"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 301, 800, 800]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(
        TransparentFailureZoo,
        TransparentInsideAttemptOkAnimal,
        "transparent-inside-ok"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 100, 800, 800]);
    assert_history_effect_counts(&history, 4, 4);

    let (status, history) = run_case!(JoinFailureZoo, JoinInsideAttemptOkAnimal, "join-inside-ok");
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(
        decode_i32_effect_inputs(&history),
        vec![100, 411, 412, 800, 800]
    );
    assert_history_effect_counts(&history, 5, 5);

    let (status, history) = run_case!(
        SelectFailureZoo,
        SelectInsideAttemptOkAnimal,
        "select-inside-ok"
    );
    assert_eq!(status, JourneyStatus::Completed);
    assert_eq!(decode_i32_effect_inputs(&history), vec![100, 800, 800]);
    assert_history_effect_counts(&history, 6, 6);
}
