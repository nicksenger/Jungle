use async_trait::async_trait;
use jungle_sdk::core::JungleWorker;
use jungle_sdk::prelude::*;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::{Animals, JungleClient, RunnerOut};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, Semaphore};

const PRE_STEPS: usize = 2;
const POST_STEPS: usize = 2;
const TEST_OWNER_LEASE_TTL_MS: i64 = 1_500;

#[derive(Clone)]
struct DropHistoryEventsClient {
    inner: jungle_sdk::Client,
    remaining_effect_inputs_to_drop: Arc<AtomicUsize>,
    remaining_effect_success_outputs_to_drop: Arc<AtomicUsize>,
    remaining_effect_failure_outputs_to_drop: Arc<AtomicUsize>,
}

impl DropHistoryEventsClient {
    fn new(
        inner: jungle_sdk::Client,
        effect_inputs_to_drop: usize,
        effect_success_outputs_to_drop: usize,
        effect_failure_outputs_to_drop: usize,
    ) -> Self {
        Self {
            inner,
            remaining_effect_inputs_to_drop: Arc::new(AtomicUsize::new(effect_inputs_to_drop)),
            remaining_effect_success_outputs_to_drop: Arc::new(AtomicUsize::new(
                effect_success_outputs_to_drop,
            )),
            remaining_effect_failure_outputs_to_drop: Arc::new(AtomicUsize::new(
                effect_failure_outputs_to_drop,
            )),
        }
    }

    fn try_drop(counter: &AtomicUsize) -> bool {
        let mut current = counter.load(Ordering::SeqCst);
        while current > 0 {
            match counter.compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
            {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
        false
    }
}

#[async_trait]
impl JungleClient for DropHistoryEventsClient {
    async fn spawn<A>(&self, seed: &A::Seed) -> Result<jungle_sdk::JourneyHandle, ExecutorError>
    where
        Self: Sized,
        A: jungle_sdk::Animal,
        A::Id: jungle_sdk::AnimalIdValue,
        A::Generation: jungle_sdk::typosaurus::num::Unsigned,
        A::Seed: Sync,
    {
        self.inner.spawn::<A>(seed).await
    }

    async fn journey_history(&self, id: uuid::Uuid) -> Result<Vec<RunnerOut>, ExecutorError> {
        self.inner.journey_history(id).await
    }

    async fn journey_replay_page(
        &self,
        journey_id: uuid::Uuid,
        after_sequence_id: Option<u64>,
        snapshot_end_sequence_id: Option<u64>,
        limit: u32,
    ) -> Result<jungle_sdk::JourneyReplayPage, ExecutorError> {
        self.inner
            .journey_replay_page(
                journey_id,
                after_sequence_id,
                snapshot_end_sequence_id,
                limit,
            )
            .await
    }

    async fn list_journeys(
        &self,
        namespace: String,
    ) -> Result<Vec<jungle_sdk::JourneyRecord>, ExecutorError> {
        self.inner.list_journeys(namespace).await
    }

    async fn subscribe_step_updates(
        &self,
        journey_id: uuid::Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<jungle_sdk::client::JourneyUpdateSubscription, ExecutorError> {
        self.inner
            .subscribe_step_updates(journey_id, after_sequence_id)
            .await
    }

    async fn journey_details(
        &self,
        id: uuid::Uuid,
    ) -> Result<jungle_sdk::JourneyStatus, ExecutorError> {
        self.inner.journey_details(id).await
    }

    async fn animal_appearance(&self, id: uuid::Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        self.inner.animal_appearance(id).await
    }

    async fn animal_appearance_update(
        &self,
        id: uuid::Uuid,
        data: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.animal_appearance_update(id, data).await
    }

    async fn perturb_animal(&self, id: uuid::Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        self.inner.perturb_animal(id, payload).await
    }

    async fn claim_animal_perturbation(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<jungle_sdk::ClaimedPerturbable>, ExecutorError> {
        self.inner.claim_animal_perturbation(id).await
    }

    async fn ack_animal_perturbation(
        &self,
        id: uuid::Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .ack_animal_perturbation(id, perturbation_id)
            .await
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: uuid::Uuid,
        owner_id: uuid::Uuid,
        lease_ttl_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .heartbeat_journey_lease(journey_id, owner_id, lease_ttl_ms)
            .await
    }

    async fn poll_owner_wake(
        &self,
        owner_id: uuid::Uuid,
    ) -> Result<Option<jungle_sdk::OwnerWake>, ExecutorError> {
        self.inner.poll_owner_wake(owner_id).await
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: uuid::Uuid,
        timer_id: uuid::Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
            .await
    }

    async fn complete_journey(&self, id: uuid::Uuid) -> Result<(), ExecutorError> {
        self.inner.complete_journey(id).await
    }

    async fn dead_journey(&self, id: uuid::Uuid) -> Result<(), ExecutorError> {
        self.inner.dead_journey(id).await
    }

    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError> {
        self.inner.poll_timers().await
    }

    async fn poll_work(
        &self,
        supported_animals: Vec<jungle_sdk::SupportedAnimal>,
    ) -> Result<Option<jungle_sdk::Work>, ExecutorError> {
        self.inner.poll_work(supported_animals).await
    }

    async fn wait_for_worker_wake(
        &self,
        owner_id: uuid::Uuid,
        supported_animals: Vec<jungle_sdk::SupportedAnimal>,
        timeout: Duration,
    ) -> Result<(), ExecutorError> {
        self.inner
            .wait_for_worker_wake(owner_id, supported_animals, timeout)
            .await
    }

    async fn effect_input(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        input: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_input(id, node_id, input).await
    }

    async fn effect_success_output(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_success_output(id, node_id, output).await
    }

    async fn effect_failure_output(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_failure_output(id, node_id, err).await
    }

    async fn submit_history_event(&self, event: RunnerOut) -> Result<(), ExecutorError> {
        let should_drop = match &event {
            RunnerOut::EffectInput { .. } => Self::try_drop(&self.remaining_effect_inputs_to_drop),
            RunnerOut::EffectSuccessOutput { .. } => {
                Self::try_drop(&self.remaining_effect_success_outputs_to_drop)
            }
            RunnerOut::EffectFailureOutput { .. } => {
                Self::try_drop(&self.remaining_effect_failure_outputs_to_drop)
            }
            _ => false,
        };
        if should_drop {
            return Ok(());
        }
        self.inner.submit_history_event(event).await
    }
}

#[derive(Clone)]
struct SnapshotProbeClient {
    inner: jungle_sdk::Client,
    replay_snapshot_args: Arc<Mutex<Vec<Option<u64>>>>,
    frozen_snapshot_end_sequence_id: Arc<Mutex<Option<u64>>>,
    injected_snapshot_growth: Arc<AtomicBool>,
}

impl SnapshotProbeClient {
    fn new(inner: jungle_sdk::Client) -> Self {
        Self {
            inner,
            replay_snapshot_args: Arc::new(Mutex::new(Vec::new())),
            frozen_snapshot_end_sequence_id: Arc::new(Mutex::new(None)),
            injected_snapshot_growth: Arc::new(AtomicBool::new(false)),
        }
    }

    fn replay_snapshot_args(&self) -> Vec<Option<u64>> {
        self.replay_snapshot_args
            .lock()
            .expect("snapshot args mutex should lock")
            .clone()
    }

    fn frozen_snapshot_end_sequence_id(&self) -> Option<u64> {
        *self
            .frozen_snapshot_end_sequence_id
            .lock()
            .expect("snapshot end mutex should lock")
    }
}

#[async_trait]
impl JungleClient for SnapshotProbeClient {
    async fn spawn<A>(&self, seed: &A::Seed) -> Result<jungle_sdk::JourneyHandle, ExecutorError>
    where
        Self: Sized,
        A: jungle_sdk::Animal,
        A::Id: jungle_sdk::AnimalIdValue,
        A::Generation: jungle_sdk::typosaurus::num::Unsigned,
        A::Seed: Sync,
    {
        self.inner.spawn::<A>(seed).await
    }

    async fn journey_history(&self, id: uuid::Uuid) -> Result<Vec<RunnerOut>, ExecutorError> {
        self.inner.journey_history(id).await
    }

    async fn journey_replay_page(
        &self,
        journey_id: uuid::Uuid,
        after_sequence_id: Option<u64>,
        snapshot_end_sequence_id: Option<u64>,
        limit: u32,
    ) -> Result<jungle_sdk::JourneyReplayPage, ExecutorError> {
        self.replay_snapshot_args
            .lock()
            .expect("snapshot args mutex should lock")
            .push(snapshot_end_sequence_id);
        let page = self
            .inner
            .journey_replay_page(
                journey_id,
                after_sequence_id,
                snapshot_end_sequence_id,
                limit,
            )
            .await?;
        if after_sequence_id.is_none()
            && !page.events.is_empty()
            && !self.injected_snapshot_growth.swap(true, Ordering::SeqCst)
        {
            *self
                .frozen_snapshot_end_sequence_id
                .lock()
                .expect("snapshot end mutex should lock") = page.snapshot_end_sequence_id;
            self.inner
                .submit_history_event(RunnerOut::EffectInput {
                    node_id: u32::MAX,
                    uuid: journey_id,
                    data: vec![0x42],
                })
                .await?;
        }
        Ok(page)
    }

    async fn list_journeys(
        &self,
        namespace: String,
    ) -> Result<Vec<jungle_sdk::JourneyRecord>, ExecutorError> {
        self.inner.list_journeys(namespace).await
    }

    async fn subscribe_step_updates(
        &self,
        journey_id: uuid::Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<jungle_sdk::client::JourneyUpdateSubscription, ExecutorError> {
        self.inner
            .subscribe_step_updates(journey_id, after_sequence_id)
            .await
    }

    async fn journey_details(
        &self,
        id: uuid::Uuid,
    ) -> Result<jungle_sdk::JourneyStatus, ExecutorError> {
        self.inner.journey_details(id).await
    }

    async fn animal_appearance(&self, id: uuid::Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        self.inner.animal_appearance(id).await
    }

    async fn animal_appearance_update(
        &self,
        id: uuid::Uuid,
        data: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.animal_appearance_update(id, data).await
    }

    async fn perturb_animal(&self, id: uuid::Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        self.inner.perturb_animal(id, payload).await
    }

    async fn claim_animal_perturbation(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<jungle_sdk::ClaimedPerturbable>, ExecutorError> {
        self.inner.claim_animal_perturbation(id).await
    }

    async fn ack_animal_perturbation(
        &self,
        id: uuid::Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .ack_animal_perturbation(id, perturbation_id)
            .await
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: uuid::Uuid,
        owner_id: uuid::Uuid,
        lease_ttl_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .heartbeat_journey_lease(journey_id, owner_id, lease_ttl_ms)
            .await
    }

    async fn poll_owner_wake(
        &self,
        owner_id: uuid::Uuid,
    ) -> Result<Option<jungle_sdk::OwnerWake>, ExecutorError> {
        self.inner.poll_owner_wake(owner_id).await
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: uuid::Uuid,
        timer_id: uuid::Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
            .await
    }

    async fn complete_journey(&self, id: uuid::Uuid) -> Result<(), ExecutorError> {
        self.inner.complete_journey(id).await
    }

    async fn dead_journey(&self, id: uuid::Uuid) -> Result<(), ExecutorError> {
        self.inner.dead_journey(id).await
    }

    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError> {
        self.inner.poll_timers().await
    }

    async fn poll_work(
        &self,
        supported_animals: Vec<jungle_sdk::SupportedAnimal>,
    ) -> Result<Option<jungle_sdk::Work>, ExecutorError> {
        self.inner.poll_work(supported_animals).await
    }

    async fn wait_for_worker_wake(
        &self,
        owner_id: uuid::Uuid,
        supported_animals: Vec<jungle_sdk::SupportedAnimal>,
        timeout: Duration,
    ) -> Result<(), ExecutorError> {
        self.inner
            .wait_for_worker_wake(owner_id, supported_animals, timeout)
            .await
    }

    async fn effect_input(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        input: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_input(id, node_id, input).await
    }

    async fn effect_success_output(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_success_output(id, node_id, output).await
    }

    async fn effect_failure_output(
        &self,
        id: uuid::Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_failure_output(id, node_id, err).await
    }

    async fn submit_history_event(&self, event: RunnerOut) -> Result<(), ExecutorError> {
        self.inner.submit_history_event(event).await
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayGateState {
    phase: u8,
}

#[derive(Clone)]
pub struct ReplayGateZoo {
    pre_counter: Arc<AtomicUsize>,
    post_counter: Arc<AtomicUsize>,
    reached_tx: mpsc::UnboundedSender<()>,
    gate: Arc<Semaphore>,
}

trait ReplayPreIncrementRuntime {
    fn run_replay_pre_increment(&self);
}

impl ReplayPreIncrementRuntime for () {
    fn run_replay_pre_increment(&self) {}
}

impl ReplayPreIncrementRuntime for ReplayGateZoo {
    fn run_replay_pre_increment(&self) {
        self.pre_counter.fetch_add(1, Ordering::SeqCst);
    }
}

trait ReplayPostIncrementRuntime {
    fn run_replay_post_increment(&self);
}

impl ReplayPostIncrementRuntime for () {
    fn run_replay_post_increment(&self) {}
}

impl ReplayPostIncrementRuntime for ReplayGateZoo {
    fn run_replay_post_increment(&self) {
        self.post_counter.fetch_add(1, Ordering::SeqCst);
    }
}

trait ReplayGateRuntime {
    fn run_replay_gate(&self) -> impl std::future::Future<Output = Result<(), ()>> + Send;
}

impl ReplayGateRuntime for () {
    fn run_replay_gate(&self) -> impl std::future::Future<Output = Result<(), ()>> + Send {
        std::future::ready(Ok(()))
    }
}

impl ReplayGateRuntime for ReplayGateZoo {
    fn run_replay_gate(&self) -> impl std::future::Future<Output = Result<(), ()>> + Send {
        let reached_tx = self.reached_tx.clone();
        let gate = Arc::clone(&self.gate);
        async move {
            reached_tx.send(()).map_err(|_| ())?;
            let permit = gate.acquire().await.map_err(|_| ())?;
            permit.forget();
            Ok(())
        }
    }
}

pub struct ReplayPreIncrementEffect;
#[jungle::effect(id = 41)]
impl<J> Effect<J> for ReplayPreIncrementEffect
where
    J: ReplayPreIncrementRuntime,
{
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.run_replay_pre_increment();
        std::future::ready(Ok(()))
    }
}

pub struct ReplayPostIncrementEffect;
#[jungle::effect(id = 42)]
impl<J> Effect<J> for ReplayPostIncrementEffect
where
    J: ReplayPostIncrementRuntime,
{
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.run_replay_post_increment();
        std::future::ready(Ok(()))
    }
}

pub struct ReplayGateEffect;
#[jungle::effect(id = 43)]
impl<J> Effect<J> for ReplayGateEffect
where
    J: ReplayGateRuntime,
{
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.run_replay_gate()
    }
}

trait ReplayPhaseState {
    fn phase(&self) -> u8;
}

impl ReplayPhaseState for ReplayGateState {
    fn phase(&self) -> u8 {
        self.phase
    }
}

pub struct ReplayPhaseNotComplete;
impl<S> Predicate<(&S, &())> for ReplayPhaseNotComplete
where
    S: ReplayPhaseState,
{
    fn eval((state, _): &(&S, &())) -> bool {
        state.phase() < 5
    }
}

pub struct ReplayPhaseIs<const N: u8>;
impl<S, Arg, const N: u8> Predicate<(S, Arg)> for ReplayPhaseIs<N>
where
    S: ReplayPhaseState,
{
    fn eval((state, _): &(S, Arg)) -> bool {
        state.phase() == N
    }
}

type ReplayPhaseRouterFlow<Pre, Mid, Post> = While<
    ReplayPhaseNotComplete,
    Conditional<
        ReplayPhaseIs<0>,
        Step<Pre>,
        Conditional<
            ReplayPhaseIs<1>,
            Step<Pre>,
            Conditional<
                ReplayPhaseIs<2>,
                Step<Mid>,
                Conditional<ReplayPhaseIs<3>, Step<Post>, Step<Post>>,
            >,
        >,
    >,
>;

pub struct ReplayPreSpec;
#[jungle::action]
impl Action for ReplayPreSpec {
    type Effect = ReplayPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayGateState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_1 = {
            output.map_err(|_err| Failure::from("pre increment should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_1)
    }
}

pub struct ReplayGateSpec;
#[jungle::action]
impl Action for ReplayGateSpec {
    type Effect = ReplayGateEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayGateState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_2 = {
            output.map_err(|_err| Failure::from("gate effect should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_2)
    }
}

pub struct ReplayPostSpec;
#[jungle::action]
impl Action for ReplayPostSpec {
    type Effect = ReplayPostIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayGateState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayGateState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_3 = {
            output.map_err(|_err| Failure::from("post increment should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_3)
    }
}

#[derive(Flow)]
pub struct ReplayGateTemplate(ReplayPhaseRouterFlow<ReplayPreSpec, ReplayGateSpec, ReplayPostSpec>);

type ReplayGateJourney = ReplayGateTemplate;

pub struct ReplayGateAnimal;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for ReplayGateAnimal {
    type State = ReplayGateState;
    type Seed = ReplayGateState;
    type Flow = ReplayGateJourney;
}

#[derive(Animals)]
pub struct ReplayGateAnimals(ReplayGateAnimal);

impl Ecosystem for ReplayGateZoo {
    const NAME: &'static str = "replay-gate-zoo";
    type Animals = ReplayGateAnimals;
}

impl From<ReplayGateState> for () {
    fn from(_value: ReplayGateState) -> Self {}
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayJoinLeftState {
    ran: bool,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayJoinRightState {
    ran: bool,
}

#[derive(Optic, Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayJoinState {
    #[jungle(focus)]
    left: ReplayJoinLeftState,
    #[jungle(focus)]
    right: ReplayJoinRightState,
    gate_opened: bool,
    post_ran: bool,
}

pub struct ReplayJoinLeftSpec;
#[jungle::action]
impl Action for ReplayJoinLeftSpec {
    type Effect = ReplayPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayJoinLeftState, _input: Self::Input) {}

    fn absorb(
        state: &mut ReplayJoinLeftState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("left join branch should succeed"))?;
        state.ran = true;
        Ok(())
    }
}

pub struct ReplayJoinRightSpec;
#[jungle::action]
impl Action for ReplayJoinRightSpec {
    type Effect = ReplayPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayJoinRightState, _input: Self::Input) {}

    fn absorb(
        state: &mut ReplayJoinRightState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("right join branch should succeed"))?;
        state.ran = true;
        Ok(())
    }
}

pub struct ReplayJoinGateSpec;
#[jungle::action]
impl Action for ReplayJoinGateSpec {
    type Effect = ReplayGateEffect;
    type Input = ((), ());
    type Output = ();

    fn emit(_state: &ReplayJoinState, _input: Self::Input) {}

    fn absorb(
        state: &mut ReplayJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join gate should succeed"))?;
        state.gate_opened = true;
        Ok(())
    }
}

pub struct ReplayJoinPostSpec;
#[jungle::action]
impl Action for ReplayJoinPostSpec {
    type Effect = ReplayPostIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayJoinState, _input: Self::Input) {}

    fn absorb(
        state: &mut ReplayJoinState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("join post step should succeed"))?;
        state.post_ran = true;
        Ok(())
    }
}

#[derive(Flow)]
#[jungle(focus = ReplayJoinLeftState)]
pub struct ReplayJoinLeftFlow(Step<ReplayJoinLeftSpec>);

#[derive(Flow)]
#[jungle(focus = ReplayJoinRightState)]
pub struct ReplayJoinRightFlow(Step<ReplayJoinRightSpec>);

#[derive(Flow)]
pub struct ReplayJoinJourney(
    jungle_zoo::ClonedJoinUnit<ReplayJoinLeftFlow, ReplayJoinRightFlow>,
    Step<ReplayJoinGateSpec>,
    Step<ReplayJoinPostSpec>,
);

pub struct ReplayJoinAnimal;
#[jungle::animal(id = 1, generation = 0)]
impl Animal for ReplayJoinAnimal {
    type State = ReplayJoinState;
    type Seed = ReplayJoinState;
    type Flow = ReplayJoinJourney;
}

#[derive(Animals)]
pub struct ReplayJoinAnimals(ReplayJoinAnimal);

impl Ecosystem for ReplayJoinZoo {
    const NAME: &'static str = "replay-join-zoo";
    type Animals = ReplayJoinAnimals;
}

#[derive(Clone)]
pub struct ReplayJoinZoo {
    pre_counter: Arc<AtomicUsize>,
    post_counter: Arc<AtomicUsize>,
    reached_tx: mpsc::UnboundedSender<()>,
    gate: Arc<Semaphore>,
}

impl ReplayPreIncrementRuntime for ReplayJoinZoo {
    fn run_replay_pre_increment(&self) {
        self.pre_counter.fetch_add(1, Ordering::SeqCst);
    }
}

impl ReplayPostIncrementRuntime for ReplayJoinZoo {
    fn run_replay_post_increment(&self) {
        self.post_counter.fetch_add(1, Ordering::SeqCst);
    }
}

impl ReplayGateRuntime for ReplayJoinZoo {
    fn run_replay_gate(&self) -> impl std::future::Future<Output = Result<(), ()>> + Send {
        let reached_tx = self.reached_tx.clone();
        let gate = Arc::clone(&self.gate);
        async move {
            reached_tx.send(()).map_err(|_| ())?;
            let permit = gate.acquire().await.map_err(|_| ())?;
            permit.forget();
            Ok(())
        }
    }
}

impl From<ReplayJoinState> for () {
    fn from(_value: ReplayJoinState) -> Self {}
}

#[tokio::test]
async fn replay_after_worker_crash_does_not_repeat_pre_gate_side_effects() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = connect_client_with_retry(listen_addr).await;
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx: reached_tx.clone(),
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client)
                .with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS)
                .with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    let seed = ReplayGateState { phase: 0 };
    let journey_id = control_client
        .spawn::<ReplayGateAnimal>(&seed)
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("first gate notification should arrive")
        .expect("first gate notification channel should remain open");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        PRE_STEPS,
        "pre-gate side effects should run exactly once before crash"
    );
    assert_eq!(post_counter.load(Ordering::SeqCst), 0);

    worker_one.abort();
    let _ = worker_one.await;

    let worker_two = tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx,
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if reached_rx.try_recv().is_ok() {
                break;
            }
            let _ = control_client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for replay gate");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("replay gate notification should arrive after reclaim");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        PRE_STEPS,
        "replay should not rerun pre-gate side effects"
    );
    assert_eq!(post_counter.load(Ordering::SeqCst), 0);

    gate.add_permits(1);

    wait_for_completed(listen_addr, journey_id, Duration::from_secs(10)).await;

    assert_eq!(pre_counter.load(Ordering::SeqCst), PRE_STEPS);
    assert_eq!(post_counter.load(Ordering::SeqCst), POST_STEPS);

    worker_two.abort();
    let _ = worker_two.await;
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn replay_after_join_live_history_crash_skips_child_events_and_resumes() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = connect_client_with_retry(listen_addr).await;
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayJoinZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx: reached_tx.clone(),
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client)
                .with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS)
                .with_replay_page_size(1);
            worker
                .spawn()
                .await
                .expect("first replay join worker should keep running");
        }
    });

    let journey_id = control_client
        .spawn::<ReplayJoinAnimal>(&ReplayJoinState::default())
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("first gate notification should arrive")
        .expect("first gate notification channel should remain open");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        2,
        "join child side effects should run exactly once before crash"
    );
    assert_eq!(post_counter.load(Ordering::SeqCst), 0);

    worker_one.abort();
    let _ = worker_one.await;

    let worker_two = tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayJoinZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx,
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            worker
                .spawn()
                .await
                .expect("second replay join worker should keep running");
        }
    });

    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if reached_rx.try_recv().is_ok() {
                break;
            }
            let _ = control_client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for replay gate");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("replay gate notification should arrive after reclaim");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        2,
        "replay should not rerun join child side effects"
    );
    assert_eq!(post_counter.load(Ordering::SeqCst), 0);

    gate.add_permits(1);
    wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

    assert_eq!(pre_counter.load(Ordering::SeqCst), 2);
    assert_eq!(post_counter.load(Ordering::SeqCst), 1);

    let history = control_client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed after replay");
    let effect_inputs = history
        .iter()
        .filter_map(|event| match event {
            RunnerOut::EffectInput { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        effect_inputs.len() >= 4,
        "expected join parent, join children, and gate effect inputs in history: {effect_inputs:?}"
    );

    worker_two.abort();
    let _ = worker_two.await;
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn replay_join_runner_reaches_gate_after_parallel_focused_join() {
    use jungle_sdk::client::{MockClient, RunnerChannelTx};
    use jungle_sdk::core::JungleRunner;

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));
    let runner = JungleRunner::new(ReplayJoinZoo {
        pre_counter: Arc::clone(&pre_counter),
        post_counter: Arc::clone(&post_counter),
        reached_tx,
        gate: Arc::clone(&gate),
    });
    let client = MockClient::builder().build();
    let (tx, rx): (RunnerChannelTx, _) = futures::channel::mpsc::channel(32);
    let resolver = tokio::spawn(async move {
        client.serve_runner_channel(rx).await;
    });
    let run_tx = tx.clone();

    let run = tokio::spawn(async move {
        runner
            .spawn::<ReplayJoinAnimal>(
                ReplayJoinState::default(),
                uuid::Uuid::from_u128(99),
                run_tx,
            )
            .await
    });

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("runner should reach the join gate");
    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        2,
        "both focused join branches should complete before the gate opens"
    );

    gate.add_permits(1);
    let state = run
        .await
        .expect("runner task should finish")
        .expect("runner should complete the replay join animal");
    assert!(state.left.ran);
    assert!(state.right.ran);
    assert!(state.gate_opened);
    assert!(state.post_ran);
    assert_eq!(post_counter.load(Ordering::SeqCst), 1);

    drop(tx);
    resolver
        .await
        .expect("runner transport resolver should complete");
}

#[tokio::test]
async fn replay_recovery_synthesizes_missing_effect_inputs_without_reading_its_own_writes() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = DropHistoryEventsClient::new(
        connect_client_with_retry(listen_addr).await,
        PRE_STEPS,
        0,
        0,
    );
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx: reached_tx.clone(),
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    let journey_id = control_client
        .spawn::<ReplayGateAnimal>(&ReplayGateState { phase: 0 })
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("first gate notification should arrive")
        .expect("first gate notification channel should remain open");

    worker_one.abort();
    let _ = worker_one.await;

    let worker_two = tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx,
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if reached_rx.try_recv().is_ok() {
                break;
            }
            let _ = control_client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for replay gate");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("replay gate notification should arrive after reclaim");

    gate.add_permits(1);
    wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        PRE_STEPS,
        "recovery replay should not rerun pre-gate side effects"
    );
    assert_eq!(
        post_counter.load(Ordering::SeqCst),
        POST_STEPS,
        "post-gate side effects should still run exactly once"
    );

    let history = control_client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed after recovery replay");
    let effect_input_count = history
        .iter()
        .filter(|event| matches!(event, RunnerOut::EffectInput { .. }))
        .count();
    assert_eq!(
        effect_input_count,
        PRE_STEPS + POST_STEPS + 1,
        "replay should synthesize the dropped pre-gate effect inputs exactly once"
    );

    worker_two.abort();
    let _ = worker_two.await;
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn replay_recovery_synthesizes_missing_effect_success_outputs_once() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = DropHistoryEventsClient::new(
        connect_client_with_retry(listen_addr).await,
        0,
        PRE_STEPS,
        0,
    );
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx: reached_tx.clone(),
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    let journey_id = control_client
        .spawn::<ReplayGateAnimal>(&ReplayGateState { phase: 0 })
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("first gate notification should arrive")
        .expect("first gate notification channel should remain open");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        PRE_STEPS,
        "original pre-gate side effects should run before reclaim"
    );

    worker_one.abort();
    let _ = worker_one.await;

    let worker_two = tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx,
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if reached_rx.try_recv().is_ok() {
                break;
            }
            let _ = control_client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for replay gate");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("replay gate notification should arrive after reclaim");

    assert_eq!(
        pre_counter.load(Ordering::SeqCst),
        PRE_STEPS * 2,
        "missing pre-gate success outputs should rerun exactly once during recovery"
    );
    assert_eq!(post_counter.load(Ordering::SeqCst), 0);

    gate.add_permits(1);
    wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

    assert_eq!(post_counter.load(Ordering::SeqCst), POST_STEPS);

    let history = control_client
        .journey_history(journey_id)
        .await
        .expect("journey_history should succeed after completion");
    let effect_success_output_count = history
        .iter()
        .filter(|event| matches!(event, RunnerOut::EffectSuccessOutput { .. }))
        .count();
    assert_eq!(
        effect_success_output_count,
        PRE_STEPS + POST_STEPS + 1,
        "history should contain one success output per completed effect after recovery"
    );

    worker_two.abort();
    let _ = worker_two.await;
    server_task.abort();
    let _ = server_task.await;
}

#[tokio::test]
async fn replay_cursor_freezes_snapshot_end_sequence_id_while_history_grows() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = DropHistoryEventsClient::new(
        connect_client_with_retry(listen_addr).await,
        PRE_STEPS,
        0,
        0,
    );
    let worker_two_client = SnapshotProbeClient::new(connect_client_with_retry(listen_addr).await);

    let pre_counter = Arc::new(AtomicUsize::new(0));
    let post_counter = Arc::new(AtomicUsize::new(0));
    let (reached_tx, mut reached_rx) = mpsc::unbounded_channel::<()>();
    let gate = Arc::new(Semaphore::new(0));

    let worker_one = tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx: reached_tx.clone(),
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    let journey_id = control_client
        .spawn::<ReplayGateAnimal>(&ReplayGateState { phase: 0 })
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), reached_rx.recv())
        .await
        .expect("first gate notification should arrive")
        .expect("first gate notification channel should remain open");

    worker_one.abort();
    let _ = worker_one.await;

    let worker_two = tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayGateZoo {
            pre_counter: Arc::clone(&pre_counter),
            post_counter: Arc::clone(&post_counter),
            reached_tx,
            gate: Arc::clone(&gate),
        };
        async move {
            let worker = JungleWorker::new(zoo, client).with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    });

    tokio::time::timeout(Duration::from_secs(45), async {
        loop {
            if reached_rx.try_recv().is_ok() {
                break;
            }
            let _ = control_client
                .journey_details(journey_id)
                .await
                .expect("journey_details should succeed while waiting for replay gate");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("replay gate notification should arrive after reclaim");

    let frozen_snapshot_end = worker_two_client
        .frozen_snapshot_end_sequence_id()
        .expect("replay should freeze a snapshot end sequence id");
    let snapshot_args = worker_two_client.replay_snapshot_args();
    assert!(
        snapshot_args.len() > 1,
        "replay should fetch multiple pages with page size 1"
    );
    assert_eq!(snapshot_args[0], None);
    assert!(
        snapshot_args
            .iter()
            .skip(1)
            .all(|arg| *arg == Some(frozen_snapshot_end)),
        "replay should keep sending the initial snapshot end on later page fetches"
    );

    gate.add_permits(1);
    wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

    let tail = control_client
        .journey_replay_page(journey_id, Some(frozen_snapshot_end), None, 100)
        .await
        .expect("tail replay page should succeed after completion");
    assert!(
        !tail.events.is_empty(),
        "history should grow beyond the frozen replay snapshot"
    );
    assert!(
        tail.events.iter().any(|event| matches!(
            &event.event,
            RunnerOut::EffectInput { node_id, data, .. }
                if *node_id == u32::MAX && data.as_slice() == [0x42]
        )),
        "tail history should include the injected post-snapshot effect input"
    );

    worker_two.abort();
    let _ = worker_two.await;
    server_task.abort();
    let _ = server_task.await;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayTimeoutState {
    phase: u8,
    sleep_for_ms: u64,
}

impl Default for ReplayTimeoutState {
    fn default() -> Self {
        Self {
            phase: 0,
            sleep_for_ms: 4_000,
        }
    }
}

impl ReplayPhaseState for ReplayTimeoutState {
    fn phase(&self) -> u8 {
        self.phase
    }
}

#[derive(Clone)]
pub struct ReplayTimeoutZoo {
    global_pre_counter: Arc<AtomicUsize>,
    global_post_counter: Arc<AtomicUsize>,
    worker_pre_counter: Arc<AtomicUsize>,
}

trait ReplayTimeoutPreIncrementRuntime {
    fn run_replay_timeout_pre_increment(&self);
}

impl ReplayTimeoutPreIncrementRuntime for () {
    fn run_replay_timeout_pre_increment(&self) {}
}

impl ReplayTimeoutPreIncrementRuntime for ReplayTimeoutZoo {
    fn run_replay_timeout_pre_increment(&self) {
        self.global_pre_counter.fetch_add(1, Ordering::SeqCst);
        self.worker_pre_counter.fetch_add(1, Ordering::SeqCst);
    }
}

trait ReplayTimeoutPostIncrementRuntime {
    fn run_replay_timeout_post_increment(&self);
}

impl ReplayTimeoutPostIncrementRuntime for () {
    fn run_replay_timeout_post_increment(&self) {}
}

impl ReplayTimeoutPostIncrementRuntime for ReplayTimeoutZoo {
    fn run_replay_timeout_post_increment(&self) {
        self.global_post_counter.fetch_add(1, Ordering::SeqCst);
    }
}

pub struct ReplayTimeoutPreIncrementEffect;
#[jungle::effect(id = 44)]
impl<J> Effect<J> for ReplayTimeoutPreIncrementEffect
where
    J: ReplayTimeoutPreIncrementRuntime,
{
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.run_replay_timeout_pre_increment();
        std::future::ready(Ok(()))
    }
}

pub struct ReplayTimeoutPostIncrementEffect;
#[jungle::effect(id = 45)]
impl<J> Effect<J> for ReplayTimeoutPostIncrementEffect
where
    J: ReplayTimeoutPostIncrementRuntime,
{
    type In = ();
    type Out = ();
    type Err = ();

    fn effect(
        jungle: &J,
        _input: Self::In,
    ) -> impl std::future::Future<Output = Result<Self::Out, Self::Err>> {
        jungle.run_replay_timeout_post_increment();
        std::future::ready(Ok(()))
    }
}

pub struct ReplayTimeoutPreSpec;
#[jungle::action]
impl Action for ReplayTimeoutPreSpec {
    type Effect = ReplayTimeoutPreIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_4 = {
            output.map_err(|_err| Failure::from("pre-timeout increment should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_4)
    }
}

pub struct ReplayTimeoutSleepSpec;
#[jungle::action]
impl Action for ReplayTimeoutSleepSpec {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &ReplayTimeoutState, _input: Self::Input) -> Duration {
        Duration::from_millis(state.sleep_for_ms)
    }

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_5 = {
            output.map_err(|_err| Failure::from("timeout sleep should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_5)
    }
}

pub struct ReplayTimeoutPostSpec;
#[jungle::action]
impl Action for ReplayTimeoutPostSpec {
    type Effect = ReplayTimeoutPostIncrementEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &ReplayTimeoutState, _input: Self::Input) -> Self::Input {}

    fn absorb(
        state: &mut ReplayTimeoutState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let __absorb_out_6 = {
            output.map_err(|_err| Failure::from("post-timeout increment should succeed"))?;
            state.phase += 1;
        };
        Ok(__absorb_out_6)
    }
}

#[derive(Flow)]
pub struct ReplayTimeoutTemplate(
    ReplayPhaseRouterFlow<ReplayTimeoutPreSpec, ReplayTimeoutSleepSpec, ReplayTimeoutPostSpec>,
);

type ReplayTimeoutJourney = ReplayTimeoutTemplate;

pub struct ReplayTimeoutAnimal;

#[jungle::animal(id = 0, generation = 0)]
impl Animal for ReplayTimeoutAnimal {
    type State = ReplayTimeoutState;
    type Seed = ReplayTimeoutState;
    type Flow = ReplayTimeoutJourney;
}

#[derive(Animals)]
pub struct ReplayTimeoutAnimals(ReplayTimeoutAnimal);

impl Ecosystem for ReplayTimeoutZoo {
    const NAME: &'static str = "replay-timeout-zoo";
    type Animals = ReplayTimeoutAnimals;
}

impl From<ReplayTimeoutState> for () {
    fn from(_value: ReplayTimeoutState) -> Self {}
}

#[tokio::test]
async fn replay_after_owner_dies_during_timeout_uses_other_worker_without_repeating_pre_steps() {
    let tempdir = tempfile::tempdir().expect("temp dir should be created");
    let db_path = tempdir.path().join("jungle.redb");
    let listen_addr = super::reserve_local_addr();

    let server_task = tokio::spawn({
        let db_path = db_path.clone();
        async move {
            ServerBuilder::new()
                .listen(listen_addr)
                .redb_path(db_path)
                .run()
                .await
        }
    });

    let control_client = connect_client_with_retry(listen_addr).await;
    let worker_one_client = connect_client_with_retry(listen_addr).await;
    let worker_two_client = connect_client_with_retry(listen_addr).await;

    let global_pre_counter = Arc::new(AtomicUsize::new(0));
    let global_post_counter = Arc::new(AtomicUsize::new(0));
    let worker_one_pre_counter = Arc::new(AtomicUsize::new(0));
    let worker_two_pre_counter = Arc::new(AtomicUsize::new(0));

    let mut worker_one = Some(tokio::spawn({
        let client = worker_one_client.clone();
        let zoo = ReplayTimeoutZoo {
            global_pre_counter: Arc::clone(&global_pre_counter),
            global_post_counter: Arc::clone(&global_post_counter),
            worker_pre_counter: Arc::clone(&worker_one_pre_counter),
        };
        async move {
            let worker = JungleWorker::new(zoo, client)
                .with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS)
                .with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    }));
    let mut worker_two = Some(tokio::spawn({
        let client = worker_two_client.clone();
        let zoo = ReplayTimeoutZoo {
            global_pre_counter: Arc::clone(&global_pre_counter),
            global_post_counter: Arc::clone(&global_post_counter),
            worker_pre_counter: Arc::clone(&worker_two_pre_counter),
        };
        async move {
            let worker = JungleWorker::new(zoo, client)
                .with_owner_lease_ttl_ms(TEST_OWNER_LEASE_TTL_MS)
                .with_replay_page_size(1);
            let _ = worker.spawn().await;
        }
    }));

    let seed = ReplayTimeoutState {
        phase: 0,
        sleep_for_ms: 4_000,
    };
    let journey_id = control_client
        .spawn::<ReplayTimeoutAnimal>(&seed)
        .await
        .expect("spawn should succeed")
        .journey_id;

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if global_pre_counter.load(Ordering::SeqCst) == PRE_STEPS {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("pre-timeout increments should finish");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let history = control_client
                .journey_history(journey_id)
                .await
                .expect("journey_history should succeed");
            if history
                .iter()
                .any(|event| matches!(event, RunnerOut::SleepScheduled { .. }))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("sleep should be scheduled before owner kill");

    let owner_is_worker_one = worker_one_pre_counter.load(Ordering::SeqCst) > 0;
    if owner_is_worker_one {
        if let Some(handle) = worker_one.take() {
            handle.abort();
            let _ = handle.await;
        }
    } else if let Some(handle) = worker_two.take() {
        handle.abort();
        let _ = handle.await;
    }

    wait_for_completed(listen_addr, journey_id, Duration::from_secs(20)).await;

    assert_eq!(
        global_pre_counter.load(Ordering::SeqCst),
        PRE_STEPS,
        "replay after timeout failover should not rerun pre-timeout side effects"
    );
    assert_eq!(
        global_post_counter.load(Ordering::SeqCst),
        POST_STEPS,
        "post-timeout side effects should run once after resume"
    );

    let worker_one_pre = worker_one_pre_counter.load(Ordering::SeqCst);
    let worker_two_pre = worker_two_pre_counter.load(Ordering::SeqCst);
    assert_eq!(
        worker_one_pre + worker_two_pre,
        PRE_STEPS,
        "only one worker should have executed original pre-timeout steps"
    );

    if let Some(handle) = worker_one {
        handle.abort();
        let _ = handle.await;
    }
    if let Some(handle) = worker_two {
        handle.abort();
        let _ = handle.await;
    }
    server_task.abort();
    let _ = server_task.await;
}

async fn wait_for_completed(remote: SocketAddr, journey_id: uuid::Uuid, timeout: Duration) {
    let deadline = std::time::Instant::now() + timeout;
    let mut client = connect_client_with_retry(remote).await;
    loop {
        match client.journey_details(journey_id).await {
            Ok(status) => {
                if status == JourneyStatus::Completed {
                    return;
                }
            }
            Err(_) => {
                client = connect_client_with_retry(remote).await;
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "journey should complete before timeout"
        );
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

async fn connect_client_with_retry(remote: SocketAddr) -> jungle_sdk::Client {
    for attempt in 0..40 {
        match jungle_sdk::client::Client::builder()
            .remote(remote)
            .server_name("localhost")
            .build()
            .await
        {
            Ok(client) => return client,
            Err(err) if attempt < 39 => {
                std::thread::sleep(Duration::from_millis(25));
                let _ = err;
            }
            Err(err) => panic!("failed to connect to test server: {err}"),
        }
    }
    unreachable!("retry loop always returns or panics")
}
