use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use crate::metronome::Metronome;
use crate::UiClient;
use async_trait::async_trait;
use futures::StreamExt;
use iced::widget::{column, container, stack, text, Row};
use iced::{Color, Element, Font, Length, Subscription, Task};
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::{ExecutorError, JungleClient, RunnerOut, SupportedAnimal, Work};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

const DEFERRED_STREAM_LOG_INTERVAL: usize = 512;
const DEFERRED_STREAM_SLOW_WAIT_WARN_MS: u64 = 400;
const DEFERRED_STREAM_LAG_WARN_MS: u64 = 150;
const DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS: i64 = 2_000;
const DEFERRED_STREAM_SLOW_DECISION_WARN_US: u128 = 500;
const AV_OVERLAY_BYTES: &[u8] = include_bytes!("../assets/jungle.mkv");
const VIDEO_FADE_IN: Duration = Duration::from_millis(180);
const VIDEO_FADE_OUT: Duration = Duration::from_millis(220);
const VIDEO_OVERLAY_TICK_INTERVAL: Duration = Duration::from_millis(50);

static DEFERRED_STREAM_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_WAIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_WAIT_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_LAG_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_DECISION_US: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct JourneyIds {
    pub lead_vocalist: Option<Uuid>,
    pub lead_guitarist: Option<Uuid>,
    pub rhythm_guitarist: Option<Uuid>,
    pub bass: Option<Uuid>,
    pub drums: Option<Uuid>,
}

#[derive(Clone)]
pub struct DeferredJungleClient<C> {
    inner: C,
    playback_delay: Duration,
    event_lead_time: Duration,
}

impl<C> DeferredJungleClient<C> {
    pub fn new(inner: C, playback_delay: Duration, event_lead_time: Duration) -> Self {
        Self {
            inner,
            playback_delay,
            event_lead_time,
        }
    }
}

fn current_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[async_trait]
impl<C> JungleClient for DeferredJungleClient<C>
where
    C: JungleClient + Clone + 'static,
{
    async fn start_journey<A>(&self, seed: Vec<u8>) -> Result<Uuid, ExecutorError>
    where
        Self: Sized,
        A: jungle_sdk::Animal,
        A::Id: jungle_sdk::AnimalIdValue,
        A::Generation: jungle_sdk::typosaurus::num::Unsigned,
    {
        self.inner.start_journey::<A>(seed).await
    }

    async fn journey_history(&self, id: Uuid) -> Result<Vec<RunnerOut>, ExecutorError> {
        self.inner.journey_history(id).await
    }

    async fn subscribe_step_updates(
        &self,
        journey_id: Uuid,
        after_sequence_id: Option<u64>,
    ) -> Result<JourneyUpdateSubscription, ExecutorError> {
        let subscription = self
            .inner
            .subscribe_step_updates(journey_id, after_sequence_id)
            .await?;
        let playback_delay = self.playback_delay;
        let event_lead_time = self.event_lead_time;
        let stream = futures::stream::unfold(subscription, move |mut subscription| async move {
            let next = subscription.next().await?;
            if let Ok(update) = &next {
                let decision_started_at = Instant::now();
                let event_count = DEFERRED_STREAM_EVENT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                let target_unix_ms = update
                    .event_unix_ms
                    .saturating_add(i64::try_from(playback_delay.as_millis()).unwrap_or(i64::MAX))
                    .saturating_sub(i64::try_from(event_lead_time.as_millis()).unwrap_or(i64::MAX));
                let now_unix_ms = current_unix_ms();
                let source_event_age_ms = now_unix_ms.saturating_sub(update.event_unix_ms);
                update_max_usize(
                    &DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS,
                    usize::try_from(source_event_age_ms.max(0)).unwrap_or(usize::MAX),
                );
                let decision_elapsed_us = decision_started_at.elapsed().as_micros();
                update_max_usize(
                    &DEFERRED_STREAM_MAX_DECISION_US,
                    usize::try_from(decision_elapsed_us).unwrap_or(usize::MAX),
                );
                if target_unix_ms > now_unix_ms {
                    let wait_ms = u64::try_from(target_unix_ms - now_unix_ms).unwrap_or(u64::MAX);
                    DEFERRED_STREAM_WAIT_COUNT.fetch_add(1, Ordering::Relaxed);
                    update_max_usize(
                        &DEFERRED_STREAM_MAX_WAIT_MS,
                        usize::try_from(wait_ms).unwrap_or(usize::MAX),
                    );
                    if wait_ms >= DEFERRED_STREAM_SLOW_WAIT_WARN_MS {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            wait_ms,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_wait_ms = DEFERRED_STREAM_MAX_WAIT_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream waiting a long time for playback alignment"
                        );
                    } else if source_event_age_ms >= DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            wait_ms,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_wait_ms = DEFERRED_STREAM_MAX_WAIT_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream source event age is high before waiting"
                        );
                    } else if decision_elapsed_us >= DEFERRED_STREAM_SLOW_DECISION_WARN_US {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            wait_ms,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_wait_ms = DEFERRED_STREAM_MAX_WAIT_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream timing decision path was unexpectedly slow"
                        );
                    } else if event_count % DEFERRED_STREAM_LOG_INTERVAL == 0 {
                        debug!(
                            journey_id = %journey_id,
                            event_count,
                            sequence_id = update.sequence_id,
                            wait_ms,
                            source_event_age_ms,
                            decision_elapsed_us,
                            wait_count = DEFERRED_STREAM_WAIT_COUNT.load(Ordering::Relaxed),
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_wait_ms = DEFERRED_STREAM_MAX_WAIT_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream heartbeat"
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                } else {
                    let lag_ms = u64::try_from(now_unix_ms.saturating_sub(target_unix_ms))
                        .unwrap_or(u64::MAX);
                    update_max_usize(
                        &DEFERRED_STREAM_MAX_LAG_MS,
                        usize::try_from(lag_ms).unwrap_or(usize::MAX),
                    );
                    if lag_ms >= DEFERRED_STREAM_LAG_WARN_MS {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            lag_ms,
                            event_count,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_lag_ms = DEFERRED_STREAM_MAX_LAG_MS.load(Ordering::Relaxed),
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream is behind target playback timestamp"
                        );
                    } else if source_event_age_ms >= DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            lag_ms,
                            event_count,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_lag_ms = DEFERRED_STREAM_MAX_LAG_MS.load(Ordering::Relaxed),
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream source event age is high while behind target"
                        );
                    } else if decision_elapsed_us >= DEFERRED_STREAM_SLOW_DECISION_WARN_US {
                        warn!(
                            journey_id = %journey_id,
                            sequence_id = update.sequence_id,
                            lag_ms,
                            event_count,
                            source_event_age_ms,
                            decision_elapsed_us,
                            max_lag_ms = DEFERRED_STREAM_MAX_LAG_MS.load(Ordering::Relaxed),
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream timing decision path was unexpectedly slow"
                        );
                    } else if event_count % DEFERRED_STREAM_LOG_INTERVAL == 0 {
                        debug!(
                            journey_id = %journey_id,
                            event_count,
                            sequence_id = update.sequence_id,
                            lag_ms,
                            source_event_age_ms,
                            decision_elapsed_us,
                            wait_count = DEFERRED_STREAM_WAIT_COUNT.load(Ordering::Relaxed),
                            max_lag_ms = DEFERRED_STREAM_MAX_LAG_MS.load(Ordering::Relaxed),
                            max_source_event_age_ms = DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS.load(Ordering::Relaxed),
                            max_wait_ms = DEFERRED_STREAM_MAX_WAIT_MS.load(Ordering::Relaxed),
                            max_decision_us = DEFERRED_STREAM_MAX_DECISION_US.load(Ordering::Relaxed),
                            "deferred welcome stream heartbeat (no wait)"
                        );
                    }
                }
            }
            Some((next, subscription))
        });
        Ok(JourneyUpdateSubscription::from_stream(stream))
    }

    async fn journey_details(&self, id: Uuid) -> Result<jungle_sdk::JourneyStatus, ExecutorError> {
        self.inner.journey_details(id).await
    }

    async fn animal_appearance(&self, id: Uuid) -> Result<Option<Vec<u8>>, ExecutorError> {
        self.inner.animal_appearance(id).await
    }

    async fn animal_appearance_update(&self, id: Uuid, data: Vec<u8>) -> Result<(), ExecutorError> {
        self.inner.animal_appearance_update(id, data).await
    }

    async fn perturb_animal(&self, id: Uuid, payload: Vec<u8>) -> Result<(), ExecutorError> {
        self.inner.perturb_animal(id, payload).await
    }

    async fn claim_animal_perturbation(
        &self,
        id: Uuid,
    ) -> Result<Option<jungle_sdk::ClaimedPerturbable>, ExecutorError> {
        self.inner.claim_animal_perturbation(id).await
    }

    async fn ack_animal_perturbation(
        &self,
        id: Uuid,
        perturbation_id: u64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .ack_animal_perturbation(id, perturbation_id)
            .await
    }

    async fn heartbeat_journey_lease(
        &self,
        journey_id: Uuid,
        owner_id: Uuid,
        lease_ttl_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .heartbeat_journey_lease(journey_id, owner_id, lease_ttl_ms)
            .await
    }

    async fn poll_owner_wake(
        &self,
        owner_id: Uuid,
    ) -> Result<Option<jungle_sdk::OwnerWake>, ExecutorError> {
        self.inner.poll_owner_wake(owner_id).await
    }

    async fn schedule_sleep_timer(
        &self,
        journey_id: Uuid,
        timer_id: Uuid,
        wake_at_unix_ms: i64,
    ) -> Result<(), ExecutorError> {
        self.inner
            .schedule_sleep_timer(journey_id, timer_id, wake_at_unix_ms)
            .await
    }

    async fn complete_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        self.inner.complete_journey(id).await
    }

    async fn poll_timers(&self) -> Result<Option<()>, ExecutorError> {
        self.inner.poll_timers().await
    }

    async fn poll_work(
        &self,
        supported_animals: Vec<SupportedAnimal>,
    ) -> Result<Option<Work>, ExecutorError> {
        self.inner.poll_work(supported_animals).await
    }

    async fn wait_for_worker_wake(
        &self,
        owner_id: Uuid,
        supported_animals: Vec<SupportedAnimal>,
        timeout: Duration,
    ) -> Result<(), ExecutorError> {
        self.inner
            .wait_for_worker_wake(owner_id, supported_animals, timeout)
            .await
    }

    async fn effect_input(
        &self,
        id: Uuid,
        node_id: u32,
        input: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_input(id, node_id, input).await
    }

    async fn effect_success_output(
        &self,
        id: Uuid,
        node_id: u32,
        output: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_success_output(id, node_id, output).await
    }

    async fn effect_failure_output(
        &self,
        id: Uuid,
        node_id: u32,
        err: Vec<u8>,
    ) -> Result<(), ExecutorError> {
        self.inner.effect_failure_output(id, node_id, err).await
    }
}

#[derive(Clone)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn request_shutdown(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn should_shutdown(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub fn run_ui(
    client: DeferredJungleClient<UiClient>,
    journeys: JourneyIds,
    metronome: Metronome,
    shutdown: ShutdownFlag,
) -> iced::Result {
    let title = "Welcome to the Jungle";
    iced::application(
        move || {
            WelcomeUi::new(
                client.clone(),
                journeys,
                metronome.clone(),
                shutdown.clone(),
            )
        },
        WelcomeUi::update,
        WelcomeUi::view,
    )
    .title(move |_app: &WelcomeUi| title.to_string())
    .subscription(WelcomeUi::subscription)
    .window_size((1800.0, 700.0))
    .antialiasing(true)
    .default_font(Font::with_name("Iosevka"))
    .run()
}

#[derive(Debug, Clone, Copy)]
enum Panel {
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    Bass,
    Drums,
}

impl Panel {
    const ALL: [Self; 5] = [
        Self::LeadVocalist,
        Self::LeadGuitarist,
        Self::RhythmGuitarist,
        Self::Bass,
        Self::Drums,
    ];
}

#[derive(Debug, Clone, Copy)]
struct VideoPlaybackRequest {
    offset: Duration,
    duration: Duration,
    opacity: f32,
}

impl VideoPlaybackRequest {
    const fn new(offset_ms: u64, duration_ms: u64, opacity: f32) -> Self {
        Self {
            offset: Duration::from_millis(offset_ms),
            duration: Duration::from_millis(duration_ms),
            opacity,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TickPlaybackPlan {
    tick: u32,
    app_overlay: Option<VideoPlaybackRequest>,
    lead_vocalist_panel: Option<VideoPlaybackRequest>,
    lead_guitarist_panel: Option<VideoPlaybackRequest>,
    rhythm_guitarist_panel: Option<VideoPlaybackRequest>,
    bass_panel: Option<VideoPlaybackRequest>,
    drums_panel: Option<VideoPlaybackRequest>,
    lead_vocalist_cluster: Option<VideoPlaybackRequest>,
    lead_guitarist_cluster: Option<VideoPlaybackRequest>,
    rhythm_guitarist_cluster: Option<VideoPlaybackRequest>,
    bass_cluster: Option<VideoPlaybackRequest>,
    drums_cluster: Option<VideoPlaybackRequest>,
}

impl TickPlaybackPlan {
    fn panel_request(self, panel: Panel) -> Option<VideoPlaybackRequest> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel,
            Panel::LeadGuitarist => self.lead_guitarist_panel,
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel,
            Panel::Bass => self.bass_panel,
            Panel::Drums => self.drums_panel,
        }
    }

    fn cluster_request(self, panel: Panel) -> Option<VideoPlaybackRequest> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_cluster,
            Panel::LeadGuitarist => self.lead_guitarist_cluster,
            Panel::RhythmGuitarist => self.rhythm_guitarist_cluster,
            Panel::Bass => self.bass_cluster,
            Panel::Drums => self.drums_cluster,
        }
    }
}

const VIDEO_PLAYBACK_PLAN: [TickPlaybackPlan; 3] = [
    TickPlaybackPlan {
        tick: 0,
        app_overlay: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        lead_vocalist_panel: None,
        lead_guitarist_panel: None,
        rhythm_guitarist_panel: None,
        bass_panel: None,
        drums_panel: None,
        lead_vocalist_cluster: None,
        lead_guitarist_cluster: None,
        rhythm_guitarist_cluster: None,
        bass_cluster: None,
        drums_cluster: None,
    },
    TickPlaybackPlan {
        tick: 4,
        app_overlay: None,
        lead_vocalist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        lead_guitarist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        rhythm_guitarist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        bass_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        drums_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        lead_vocalist_cluster: None,
        lead_guitarist_cluster: None,
        rhythm_guitarist_cluster: None,
        bass_cluster: None,
        drums_cluster: None,
    },
    TickPlaybackPlan {
        tick: 40,
        app_overlay: None,
        lead_vocalist_panel: None,
        lead_guitarist_panel: None,
        rhythm_guitarist_panel: None,
        bass_panel: None,
        drums_panel: None,
        lead_vocalist_cluster: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        lead_guitarist_cluster: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        rhythm_guitarist_cluster: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        bass_cluster: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
        drums_cluster: Some(VideoPlaybackRequest::new(0, 2_000, 0.5)),
    },
];

#[derive(Debug, Clone)]
struct RegionPlayback {
    enabled: bool,
    visible_until: Option<Instant>,
    fade_out_at: Option<Instant>,
    fade_out_started: bool,
}

impl RegionPlayback {
    fn hidden() -> Self {
        Self {
            enabled: false,
            visible_until: None,
            fade_out_at: None,
            fade_out_started: false,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Panel(Panel, jungle_vision::EjectedViewerMessage),
    AppVideo(iced_av1::widget::Message),
    PanelVideo(Panel, iced_av1::widget::Message),
    ClusterVideo(Panel, iced_av1::widget::Message),
    Tick,
}

struct WelcomeUi {
    lead_vocalist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    lead_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    rhythm_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    bass:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    drums:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    metronome: Metronome,
    applied_ticks: HashSet<u32>,
    app_overlay: Option<iced_av1::widget::State>,
    app_overlay_playback: RegionPlayback,
    lead_vocalist_panel_overlay: Option<iced_av1::widget::State>,
    lead_guitarist_panel_overlay: Option<iced_av1::widget::State>,
    rhythm_guitarist_panel_overlay: Option<iced_av1::widget::State>,
    bass_panel_overlay: Option<iced_av1::widget::State>,
    drums_panel_overlay: Option<iced_av1::widget::State>,
    lead_vocalist_panel_playback: RegionPlayback,
    lead_guitarist_panel_playback: RegionPlayback,
    rhythm_guitarist_panel_playback: RegionPlayback,
    bass_panel_playback: RegionPlayback,
    drums_panel_playback: RegionPlayback,
    lead_vocalist_cluster_overlay: Arc<Mutex<Option<iced_av1::widget::State>>>,
    lead_guitarist_cluster_overlay: Arc<Mutex<Option<iced_av1::widget::State>>>,
    rhythm_guitarist_cluster_overlay: Arc<Mutex<Option<iced_av1::widget::State>>>,
    bass_cluster_overlay: Arc<Mutex<Option<iced_av1::widget::State>>>,
    drums_cluster_overlay: Arc<Mutex<Option<iced_av1::widget::State>>>,
    lead_vocalist_cluster_playback: RegionPlayback,
    lead_guitarist_cluster_playback: RegionPlayback,
    rhythm_guitarist_cluster_playback: RegionPlayback,
    bass_cluster_playback: RegionPlayback,
    drums_cluster_playback: RegionPlayback,
    lead_vocalist_target_cluster_id: Arc<Mutex<Option<u32>>>,
    lead_guitarist_target_cluster_id: Arc<Mutex<Option<u32>>>,
    rhythm_guitarist_target_cluster_id: Arc<Mutex<Option<u32>>>,
    bass_target_cluster_id: Arc<Mutex<Option<u32>>>,
    drums_target_cluster_id: Arc<Mutex<Option<u32>>>,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: DeferredJungleClient<UiClient>,
        journeys: JourneyIds,
        metronome: Metronome,
        shutdown: ShutdownFlag,
    ) -> (Self, Task<Message>) {
        let mut lead_vocalist = journeys.lead_vocalist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Vocalist")
                .eject_live_animal::<LeadVocalist, _>(client.clone(), journey)
        });
        let mut lead_guitarist = journeys.lead_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Guitarist")
                .eject_live_animal::<LeadGuitarist, _>(client.clone(), journey)
        });
        let mut rhythm_guitarist = journeys.rhythm_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Rhythm Guitarist")
                .eject_live_animal::<RhythmGuitarist, _>(client.clone(), journey)
        });
        let mut bass = journeys.bass.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Bass")
                .eject_live_animal::<Bass, _>(client.clone(), journey)
        });
        let mut drums = journeys.drums.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Drums")
                .eject_live_animal::<Drums, _>(client, journey)
        });

        let app_overlay = init_video_state("app overlay");
        let lead_vocalist_panel_overlay = init_video_state("lead vocalist panel overlay");
        let lead_guitarist_panel_overlay = init_video_state("lead guitarist panel overlay");
        let rhythm_guitarist_panel_overlay = init_video_state("rhythm guitarist panel overlay");
        let bass_panel_overlay = init_video_state("bass panel overlay");
        let drums_panel_overlay = init_video_state("drums panel overlay");
        let lead_vocalist_cluster_overlay = Arc::new(Mutex::new(init_video_state(
            "lead vocalist cluster overlay",
        )));
        let lead_guitarist_cluster_overlay = Arc::new(Mutex::new(init_video_state(
            "lead guitarist cluster overlay",
        )));
        let rhythm_guitarist_cluster_overlay = Arc::new(Mutex::new(init_video_state(
            "rhythm guitarist cluster overlay",
        )));
        let bass_cluster_overlay = Arc::new(Mutex::new(init_video_state("bass cluster overlay")));
        let drums_cluster_overlay = Arc::new(Mutex::new(init_video_state("drums cluster overlay")));

        let lead_vocalist_target_cluster_id = Arc::new(Mutex::new(None));
        let lead_guitarist_target_cluster_id = Arc::new(Mutex::new(None));
        let rhythm_guitarist_target_cluster_id = Arc::new(Mutex::new(None));
        let bass_target_cluster_id = Arc::new(Mutex::new(None));
        let drums_target_cluster_id = Arc::new(Mutex::new(None));

        if let Some(viewer) = lead_vocalist.as_mut() {
            let target_cluster = lead_vocalist_target_cluster_id.clone();
            let overlay = lead_vocalist_cluster_overlay.clone();
            viewer.set_cluster_overlay_provider(move |cx| {
                build_cluster_overlay_element(cx, &target_cluster, &overlay)
            });
        }
        if let Some(viewer) = lead_guitarist.as_mut() {
            let target_cluster = lead_guitarist_target_cluster_id.clone();
            let overlay = lead_guitarist_cluster_overlay.clone();
            viewer.set_cluster_overlay_provider(move |cx| {
                build_cluster_overlay_element(cx, &target_cluster, &overlay)
            });
        }
        if let Some(viewer) = rhythm_guitarist.as_mut() {
            let target_cluster = rhythm_guitarist_target_cluster_id.clone();
            let overlay = rhythm_guitarist_cluster_overlay.clone();
            viewer.set_cluster_overlay_provider(move |cx| {
                build_cluster_overlay_element(cx, &target_cluster, &overlay)
            });
        }
        if let Some(viewer) = bass.as_mut() {
            let target_cluster = bass_target_cluster_id.clone();
            let overlay = bass_cluster_overlay.clone();
            viewer.set_cluster_overlay_provider(move |cx| {
                build_cluster_overlay_element(cx, &target_cluster, &overlay)
            });
        }
        if let Some(viewer) = drums.as_mut() {
            let target_cluster = drums_target_cluster_id.clone();
            let overlay = drums_cluster_overlay.clone();
            viewer.set_cluster_overlay_provider(move |cx| {
                build_cluster_overlay_element(cx, &target_cluster, &overlay)
            });
        }

        (
            Self {
                lead_vocalist,
                lead_guitarist,
                rhythm_guitarist,
                bass,
                drums,
                metronome,
                applied_ticks: HashSet::new(),
                app_overlay,
                app_overlay_playback: RegionPlayback::hidden(),
                lead_vocalist_panel_overlay,
                lead_guitarist_panel_overlay,
                rhythm_guitarist_panel_overlay,
                bass_panel_overlay,
                drums_panel_overlay,
                lead_vocalist_panel_playback: RegionPlayback::hidden(),
                lead_guitarist_panel_playback: RegionPlayback::hidden(),
                rhythm_guitarist_panel_playback: RegionPlayback::hidden(),
                bass_panel_playback: RegionPlayback::hidden(),
                drums_panel_playback: RegionPlayback::hidden(),
                lead_vocalist_cluster_overlay,
                lead_guitarist_cluster_overlay,
                rhythm_guitarist_cluster_overlay,
                bass_cluster_overlay,
                drums_cluster_overlay,
                lead_vocalist_cluster_playback: RegionPlayback::hidden(),
                lead_guitarist_cluster_playback: RegionPlayback::hidden(),
                rhythm_guitarist_cluster_playback: RegionPlayback::hidden(),
                bass_cluster_playback: RegionPlayback::hidden(),
                drums_cluster_playback: RegionPlayback::hidden(),
                lead_vocalist_target_cluster_id,
                lead_guitarist_target_cluster_id,
                rhythm_guitarist_target_cluster_id,
                bass_target_cluster_id,
                drums_target_cluster_id,
                shutdown,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.shutdown.should_shutdown() {
                    return iced::exit();
                }
                self.apply_playback_plan();
                self.update_playback_regions();
                Task::none()
            }
            Message::AppVideo(event) => {
                self.app_overlay.as_mut().map_or_else(Task::none, |video| {
                    video.update(event);
                    Task::none()
                })
            }
            Message::PanelVideo(panel, event) => {
                if let Some(video) = self.panel_overlay_mut(panel) {
                    video.update(event);
                }
                Task::none()
            }
            Message::ClusterVideo(panel, event) => {
                let _ = self.with_cluster_overlay_mut(panel, |video| video.update(event));
                Task::none()
            }
            Message::Panel(panel, event) => match panel {
                Panel::LeadVocalist => self.lead_vocalist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadVocalist, next))
                }),
                Panel::LeadGuitarist => self.lead_guitarist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadGuitarist, next))
                }),
                Panel::RhythmGuitarist => {
                    self.rhythm_guitarist.as_mut().map_or_else(Task::none, |v| {
                        v.update(event)
                            .map(move |next| Message::Panel(Panel::RhythmGuitarist, next))
                    })
                }
                Panel::Bass => self.bass.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::Bass, next))
                }),
                Panel::Drums => self.drums.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::Drums, next))
                }),
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::LeadVocalist, event)),
            );
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::LeadGuitarist, event)),
            );
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::RhythmGuitarist, event)),
            );
        }
        if let Some(viewer) = self.bass.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::Bass, event)),
            );
        }
        if let Some(viewer) = self.drums.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::Drums, event)),
            );
        }
        if let Some(video) = self.app_overlay.as_ref() {
            subscriptions.push(video.subscription(Message::AppVideo));
        }
        if let Some(video) = self.lead_vocalist_panel_overlay.as_ref() {
            subscriptions
                .push(video.subscription(|event| Message::PanelVideo(Panel::LeadVocalist, event)));
        }
        if let Some(video) = self.lead_guitarist_panel_overlay.as_ref() {
            subscriptions
                .push(video.subscription(|event| Message::PanelVideo(Panel::LeadGuitarist, event)));
        }
        if let Some(video) = self.rhythm_guitarist_panel_overlay.as_ref() {
            subscriptions.push(
                video.subscription(|event| Message::PanelVideo(Panel::RhythmGuitarist, event)),
            );
        }
        if let Some(video) = self.bass_panel_overlay.as_ref() {
            subscriptions.push(video.subscription(|event| Message::PanelVideo(Panel::Bass, event)));
        }
        if let Some(video) = self.drums_panel_overlay.as_ref() {
            subscriptions
                .push(video.subscription(|event| Message::PanelVideo(Panel::Drums, event)));
        }
        for panel in Panel::ALL {
            if let Some(subscription) = self.with_cluster_overlay(panel, |video| {
                video.subscription(move |event| Message::ClusterVideo(panel, event))
            }) {
                subscriptions.push(subscription);
            }
        }
        subscriptions.push(iced::time::every(VIDEO_OVERLAY_TICK_INTERVAL).map(|_| Message::Tick));
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let mut panels = Row::new()
            .spacing(12)
            .height(Length::Fill)
            .width(Length::Fill);
        let mut panel_count = 0usize;

        if let Some(viewer) = self.bass.as_ref() {
            panels = panels.push(self.panel_with_overlay("Bass", viewer.view(), Panel::Bass));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Lead Guitarist",
                viewer.view(),
                Panel::LeadGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Lead Vocalist",
                viewer.view(),
                Panel::LeadVocalist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Rhythm Guitarist",
                viewer.view(),
                Panel::RhythmGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.drums.as_ref() {
            panels = panels.push(self.panel_with_overlay("Drums", viewer.view(), Panel::Drums));
            panel_count += 1;
        }

        let content: Element<'_, Message> = if panel_count == 0 {
            text("No animals selected. Pass --animals with one or more names to show panels.")
                .size(16)
                .color(Color::from_rgb8(198, 229, 211))
                .into()
        } else {
            panels.into()
        };

        let app: Element<'_, Message> = container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background)
            .into();

        if self.app_overlay_playback.enabled {
            if let Some(overlay) = self
                .app_overlay
                .as_ref()
                .and_then(|video| video.overlay_view(Message::AppVideo))
            {
                return stack([app, overlay])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }
        }

        app
    }

    fn panel_with_overlay<'a>(
        &'a self,
        label: &'a str,
        content: Element<'a, jungle_vision::EjectedViewerMessage>,
        panel_kind: Panel,
    ) -> Element<'a, Message> {
        let base = panel(label, content, panel_kind);
        if self.panel_playback(panel_kind).enabled {
            if let Some(overlay) = self
                .panel_overlay(panel_kind)
                .and_then(|video| video.overlay_view(map_panel_video_message))
            {
                return stack([
                    base,
                    overlay.map(move |event| Message::PanelVideo(panel_kind, event)),
                ])
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .into();
            }
        }

        base
    }

    fn apply_playback_plan(&mut self) {
        let beat = self.current_beat_tick();
        for plan in VIDEO_PLAYBACK_PLAN {
            if beat < u64::from(plan.tick) {
                continue;
            }
            if !self.applied_ticks.insert(plan.tick) {
                continue;
            }

            let now = Instant::now();
            if let Some(request) = plan.app_overlay {
                Self::start_region_playback(
                    self.app_overlay.as_ref(),
                    &mut self.app_overlay_playback,
                    request,
                    now,
                );
            } else {
                Self::stop_region_playback(
                    self.app_overlay.as_ref(),
                    &mut self.app_overlay_playback,
                );
            }

            for panel in Panel::ALL {
                if let Some(request) = plan.panel_request(panel) {
                    let (overlay, playback) = self.panel_slot_mut(panel);
                    Self::start_region_playback(overlay.as_ref(), playback, request, now);
                } else {
                    let (overlay, playback) = self.panel_slot_mut(panel);
                    Self::stop_region_playback(overlay.as_ref(), playback);
                }
            }

            for panel in Panel::ALL {
                if let Some(request) = plan.cluster_request(panel) {
                    self.set_target_cluster_id(panel, self.latest_cluster_id(panel));
                    self.start_cluster_region_playback(panel, request, now);
                } else {
                    self.set_target_cluster_id(panel, None);
                    self.stop_cluster_region_playback(panel);
                }
            }
        }
    }

    fn update_playback_regions(&mut self) {
        let now = Instant::now();
        Self::tick_region(
            self.app_overlay.as_ref(),
            &mut self.app_overlay_playback,
            now,
        );
        for panel in Panel::ALL {
            let (overlay, playback) = self.panel_slot_mut(panel);
            Self::tick_region(overlay.as_ref(), playback, now);
            self.tick_cluster_region(panel, now);
        }
    }

    fn tick_region(
        overlay: Option<&iced_av1::widget::State>,
        playback: &mut RegionPlayback,
        now: Instant,
    ) {
        if !playback.enabled {
            return;
        }

        if !playback.fade_out_started {
            if let Some(fade_out_at) = playback.fade_out_at {
                if now >= fade_out_at {
                    if let Some(overlay) = overlay {
                        overlay.tween_to_opacity_with(
                            0.0,
                            iced_av1::OpacityTween {
                                duration: VIDEO_FADE_OUT,
                            },
                        );
                    }
                    playback.fade_out_started = true;
                }
            }
        }

        if let Some(visible_until) = playback.visible_until {
            if now >= visible_until {
                if let Some(overlay) = overlay {
                    overlay.set_opacity(0.0);
                }
                *playback = RegionPlayback::hidden();
            }
        }
    }

    fn start_region_playback(
        overlay: Option<&iced_av1::widget::State>,
        playback: &mut RegionPlayback,
        request: VideoPlaybackRequest,
        now: Instant,
    ) {
        if let Some(overlay) = overlay {
            if let Err(error) = overlay.seek(duration_to_ns(request.offset)) {
                warn!(error = %error, "failed to seek AV overlay to requested offset");
                return;
            }
            overlay.set_opacity(0.0);
            overlay.tween_to_opacity_with(
                request.opacity,
                iced_av1::OpacityTween {
                    duration: VIDEO_FADE_IN.min(request.duration),
                },
            );
        }

        let visible_until = now + request.duration;
        playback.enabled = true;
        playback.visible_until = Some(visible_until);
        playback.fade_out_at = Some(
            visible_until
                .checked_sub(VIDEO_FADE_OUT.min(request.duration))
                .unwrap_or(now),
        );
        playback.fade_out_started = false;
    }

    fn stop_region_playback(
        overlay: Option<&iced_av1::widget::State>,
        playback: &mut RegionPlayback,
    ) {
        if let Some(overlay) = overlay {
            overlay.set_opacity(0.0);
        }
        *playback = RegionPlayback::hidden();
    }

    fn current_beat_tick(&self) -> u64 {
        let beat = self.metronome.beat_duration().as_secs_f64();
        if beat <= f64::EPSILON {
            return 0;
        }
        (self.metronome.elapsed().as_secs_f64() / beat).floor() as u64
    }

    fn panel_overlay(&self, panel: Panel) -> Option<&iced_av1::widget::State> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel_overlay.as_ref(),
            Panel::LeadGuitarist => self.lead_guitarist_panel_overlay.as_ref(),
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel_overlay.as_ref(),
            Panel::Bass => self.bass_panel_overlay.as_ref(),
            Panel::Drums => self.drums_panel_overlay.as_ref(),
        }
    }

    fn panel_overlay_mut(&mut self, panel: Panel) -> Option<&mut iced_av1::widget::State> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel_overlay.as_mut(),
            Panel::LeadGuitarist => self.lead_guitarist_panel_overlay.as_mut(),
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel_overlay.as_mut(),
            Panel::Bass => self.bass_panel_overlay.as_mut(),
            Panel::Drums => self.drums_panel_overlay.as_mut(),
        }
    }

    fn panel_playback(&self, panel: Panel) -> &RegionPlayback {
        match panel {
            Panel::LeadVocalist => &self.lead_vocalist_panel_playback,
            Panel::LeadGuitarist => &self.lead_guitarist_panel_playback,
            Panel::RhythmGuitarist => &self.rhythm_guitarist_panel_playback,
            Panel::Bass => &self.bass_panel_playback,
            Panel::Drums => &self.drums_panel_playback,
        }
    }

    fn latest_cluster_id(&self, panel: Panel) -> Option<u32> {
        match panel {
            Panel::LeadVocalist => self
                .lead_vocalist
                .as_ref()
                .and_then(jungle_vision::EjectedViewer::latest_visible_cluster_id),
            Panel::LeadGuitarist => self
                .lead_guitarist
                .as_ref()
                .and_then(jungle_vision::EjectedViewer::latest_visible_cluster_id),
            Panel::RhythmGuitarist => self
                .rhythm_guitarist
                .as_ref()
                .and_then(jungle_vision::EjectedViewer::latest_visible_cluster_id),
            Panel::Bass => self
                .bass
                .as_ref()
                .and_then(jungle_vision::EjectedViewer::latest_visible_cluster_id),
            Panel::Drums => self
                .drums
                .as_ref()
                .and_then(jungle_vision::EjectedViewer::latest_visible_cluster_id),
        }
    }

    fn set_target_cluster_id(&mut self, panel: Panel, id: Option<u32>) {
        match panel {
            Panel::LeadVocalist => {
                if let Ok(mut target) = self.lead_vocalist_target_cluster_id.lock() {
                    *target = id;
                }
            }
            Panel::LeadGuitarist => {
                if let Ok(mut target) = self.lead_guitarist_target_cluster_id.lock() {
                    *target = id;
                }
            }
            Panel::RhythmGuitarist => {
                if let Ok(mut target) = self.rhythm_guitarist_target_cluster_id.lock() {
                    *target = id;
                }
            }
            Panel::Bass => {
                if let Ok(mut target) = self.bass_target_cluster_id.lock() {
                    *target = id;
                }
            }
            Panel::Drums => {
                if let Ok(mut target) = self.drums_target_cluster_id.lock() {
                    *target = id;
                }
            }
        }
    }

    fn cluster_overlay_cell(&self, panel: Panel) -> &Arc<Mutex<Option<iced_av1::widget::State>>> {
        match panel {
            Panel::LeadVocalist => &self.lead_vocalist_cluster_overlay,
            Panel::LeadGuitarist => &self.lead_guitarist_cluster_overlay,
            Panel::RhythmGuitarist => &self.rhythm_guitarist_cluster_overlay,
            Panel::Bass => &self.bass_cluster_overlay,
            Panel::Drums => &self.drums_cluster_overlay,
        }
    }

    fn with_cluster_overlay<R>(
        &self,
        panel: Panel,
        f: impl FnOnce(&iced_av1::widget::State) -> R,
    ) -> Option<R> {
        let Ok(overlay) = self.cluster_overlay_cell(panel).lock() else {
            return None;
        };
        overlay.as_ref().map(f)
    }

    fn with_cluster_overlay_mut<R>(
        &self,
        panel: Panel,
        f: impl FnOnce(&mut iced_av1::widget::State) -> R,
    ) -> Option<R> {
        let Ok(mut overlay) = self.cluster_overlay_cell(panel).lock() else {
            return None;
        };
        overlay.as_mut().map(f)
    }

    fn panel_slot_mut(
        &mut self,
        panel: Panel,
    ) -> (&mut Option<iced_av1::widget::State>, &mut RegionPlayback) {
        match panel {
            Panel::LeadVocalist => (
                &mut self.lead_vocalist_panel_overlay,
                &mut self.lead_vocalist_panel_playback,
            ),
            Panel::LeadGuitarist => (
                &mut self.lead_guitarist_panel_overlay,
                &mut self.lead_guitarist_panel_playback,
            ),
            Panel::RhythmGuitarist => (
                &mut self.rhythm_guitarist_panel_overlay,
                &mut self.rhythm_guitarist_panel_playback,
            ),
            Panel::Bass => (&mut self.bass_panel_overlay, &mut self.bass_panel_playback),
            Panel::Drums => (
                &mut self.drums_panel_overlay,
                &mut self.drums_panel_playback,
            ),
        }
    }

    fn start_cluster_region_playback(
        &mut self,
        panel: Panel,
        request: VideoPlaybackRequest,
        now: Instant,
    ) {
        match panel {
            Panel::LeadVocalist => start_cluster_region_with_lock(
                &self.lead_vocalist_cluster_overlay,
                &mut self.lead_vocalist_cluster_playback,
                request,
                now,
            ),
            Panel::LeadGuitarist => start_cluster_region_with_lock(
                &self.lead_guitarist_cluster_overlay,
                &mut self.lead_guitarist_cluster_playback,
                request,
                now,
            ),
            Panel::RhythmGuitarist => start_cluster_region_with_lock(
                &self.rhythm_guitarist_cluster_overlay,
                &mut self.rhythm_guitarist_cluster_playback,
                request,
                now,
            ),
            Panel::Bass => start_cluster_region_with_lock(
                &self.bass_cluster_overlay,
                &mut self.bass_cluster_playback,
                request,
                now,
            ),
            Panel::Drums => start_cluster_region_with_lock(
                &self.drums_cluster_overlay,
                &mut self.drums_cluster_playback,
                request,
                now,
            ),
        }
    }

    fn stop_cluster_region_playback(&mut self, panel: Panel) {
        match panel {
            Panel::LeadVocalist => stop_cluster_region_with_lock(
                &self.lead_vocalist_cluster_overlay,
                &mut self.lead_vocalist_cluster_playback,
            ),
            Panel::LeadGuitarist => stop_cluster_region_with_lock(
                &self.lead_guitarist_cluster_overlay,
                &mut self.lead_guitarist_cluster_playback,
            ),
            Panel::RhythmGuitarist => stop_cluster_region_with_lock(
                &self.rhythm_guitarist_cluster_overlay,
                &mut self.rhythm_guitarist_cluster_playback,
            ),
            Panel::Bass => stop_cluster_region_with_lock(
                &self.bass_cluster_overlay,
                &mut self.bass_cluster_playback,
            ),
            Panel::Drums => stop_cluster_region_with_lock(
                &self.drums_cluster_overlay,
                &mut self.drums_cluster_playback,
            ),
        }
    }

    fn tick_cluster_region(&mut self, panel: Panel, now: Instant) {
        match panel {
            Panel::LeadVocalist => tick_cluster_region_with_lock(
                &self.lead_vocalist_cluster_overlay,
                &mut self.lead_vocalist_cluster_playback,
                now,
            ),
            Panel::LeadGuitarist => tick_cluster_region_with_lock(
                &self.lead_guitarist_cluster_overlay,
                &mut self.lead_guitarist_cluster_playback,
                now,
            ),
            Panel::RhythmGuitarist => tick_cluster_region_with_lock(
                &self.rhythm_guitarist_cluster_overlay,
                &mut self.rhythm_guitarist_cluster_playback,
                now,
            ),
            Panel::Bass => tick_cluster_region_with_lock(
                &self.bass_cluster_overlay,
                &mut self.bass_cluster_playback,
                now,
            ),
            Panel::Drums => tick_cluster_region_with_lock(
                &self.drums_cluster_overlay,
                &mut self.drums_cluster_playback,
                now,
            ),
        }
    }
}

fn start_cluster_region_with_lock(
    overlay_cell: &Arc<Mutex<Option<iced_av1::widget::State>>>,
    playback: &mut RegionPlayback,
    request: VideoPlaybackRequest,
    now: Instant,
) {
    let overlay_guard = overlay_cell.lock().ok();
    let overlay = overlay_guard.as_ref().and_then(|state| state.as_ref());
    WelcomeUi::start_region_playback(overlay, playback, request, now);
}

fn stop_cluster_region_with_lock(
    overlay_cell: &Arc<Mutex<Option<iced_av1::widget::State>>>,
    playback: &mut RegionPlayback,
) {
    let overlay_guard = overlay_cell.lock().ok();
    let overlay = overlay_guard.as_ref().and_then(|state| state.as_ref());
    WelcomeUi::stop_region_playback(overlay, playback);
}

fn tick_cluster_region_with_lock(
    overlay_cell: &Arc<Mutex<Option<iced_av1::widget::State>>>,
    playback: &mut RegionPlayback,
    now: Instant,
) {
    let overlay_guard = overlay_cell.lock().ok();
    let overlay = overlay_guard.as_ref().and_then(|state| state.as_ref());
    WelcomeUi::tick_region(overlay, playback, now);
}

fn init_video_state(region: &str) -> Option<iced_av1::widget::State> {
    let playback_options = iced_av1::PlaybackOptions::default();
    let opacity_options = iced_av1::OpacityOptions {
        opacity: 0.0,
        tween: iced_av1::OpacityTween {
            duration: VIDEO_FADE_IN,
        },
    };
    let source = iced_av1::MediaSource::from_bytes(AV_OVERLAY_BYTES.to_vec());
    match iced_av1::widget::State::new_with_media_source_and_opacity_options(
        source,
        playback_options,
        opacity_options,
    ) {
        Ok(state) => Some(state),
        Err(error) => {
            warn!(error = %error, region, "failed to initialize AV overlay state");
            None
        }
    }
}

fn build_cluster_overlay_element(
    ctx: &jungle_vision::ClusterViewCtx<'_>,
    target_cluster_id: &Arc<Mutex<Option<u32>>>,
    overlay_state: &Arc<Mutex<Option<iced_av1::widget::State>>>,
) -> Option<Element<'static, jungle_vision::ViewerEvent<()>>> {
    let active_cluster = target_cluster_id.lock().ok().and_then(|value| *value);
    if active_cluster != Some(ctx.cluster_id) {
        return None;
    }

    let Ok(overlay) = overlay_state.lock() else {
        return None;
    };
    let video = overlay
        .as_ref()?
        .overlay_view_owned(map_cluster_video_message)?;
    Some(
        container(video)
            .clip(true)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    )
}

fn map_panel_video_message(message: iced_av1::widget::Message) -> iced_av1::widget::Message {
    message
}

fn map_cluster_video_message(
    _message: iced_av1::widget::Message,
) -> jungle_vision::ViewerEvent<()> {
    jungle_vision::ViewerEvent::Message(())
}

fn duration_to_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn panel<'a>(
    label: &'a str,
    content: Element<'a, jungle_vision::EjectedViewerMessage>,
    target: Panel,
) -> Element<'a, Message> {
    container(
        column![
            text(label).size(13).color(Color::from_rgb8(198, 229, 211)),
            container(content.map(move |event| Message::Panel(target, event)))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .style(panel_style)
    .into()
}

fn app_background(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(8, 19, 13))),
        ..Default::default()
    }
}

fn panel_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(10, 26, 17))),
        border: iced::border::rounded(8)
            .color(Color::from_rgb8(24, 63, 43))
            .width(1.0),
        ..Default::default()
    }
}

fn update_max_usize(max_value: &AtomicUsize, candidate: usize) {
    let mut current = max_value.load(Ordering::Relaxed);
    while candidate > current {
        match max_value.compare_exchange_weak(
            current,
            candidate,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(updated) => current = updated,
        }
    }
}
