use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use crate::metronome::Metronome;
use crate::UiClient;
use async_trait::async_trait;
use futures::StreamExt;
use iced::widget::{button, column, container, svg, text, Row, Space};
use iced::{Color, Element, Font, Length, Subscription, Task};
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::{ExecutorError, JungleClient, RunnerOut, SupportedAnimal, Work};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;
#[cfg(feature = "video")]
use welcome_video::{VideoMessage, VideoOverlayController, VideoPlaybackPlan};

const DEFERRED_STREAM_LOG_INTERVAL: usize = 512;
const DEFERRED_STREAM_SLOW_WAIT_WARN_MS: u64 = 400;
const DEFERRED_STREAM_LAG_WARN_MS: u64 = 150;
const DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS: i64 = 2_000;
const DEFERRED_STREAM_SLOW_DECISION_WARN_US: u128 = 500;
const UI_TICK_INTERVAL: Duration = Duration::from_millis(500);
const PANEL_PULSE_DURATION: Duration = Duration::from_millis(100);
const PANEL_PULSE_FRAME_INTERVAL: Duration = Duration::from_millis(16);
const LOCK_ICON_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>"#;
const UNLOCK_ICON_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M16 11V7a4 4 0 0 0-7.5-2"/></svg>"#;

static DEFERRED_STREAM_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_WAIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_WAIT_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_LAG_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_SOURCE_EVENT_AGE_MS: AtomicUsize = AtomicUsize::new(0);
static DEFERRED_STREAM_MAX_DECISION_US: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct JourneyIds {
    pub lead_vocalist: Option<Uuid>,
    pub rhythm_guitarist: Option<Uuid>,
    pub lead_guitarist: Option<Uuid>,
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
                        debug!(
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

    async fn dead_journey(&self, id: Uuid) -> Result<(), ExecutorError> {
        self.inner.dead_journey(id).await
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
    #[cfg(feature = "video")] video_plan: Option<VideoPlaybackPlan>,
) -> iced::Result {
    let title = "Welcome to the Jungle";
    iced::application(
        move || {
            WelcomeUi::new(
                client.clone(),
                journeys,
                metronome.clone(),
                shutdown.clone(),
                #[cfg(feature = "video")]
                video_plan.clone(),
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
    RhythmGuitarist,
    LeadGuitarist,
    Bass,
    Drums,
}

#[cfg(feature = "video")]
const fn to_video_panel(panel: Panel) -> welcome_video::Panel {
    match panel {
        Panel::LeadVocalist => welcome_video::Panel::LeadVocalist,
        Panel::RhythmGuitarist => welcome_video::Panel::RhythmGuitarist,
        Panel::LeadGuitarist => welcome_video::Panel::LeadGuitarist,
        Panel::Bass => welcome_video::Panel::Bass,
        Panel::Drums => welcome_video::Panel::Drums,
    }
}

#[derive(Debug, Clone)]
enum Message {
    Panel(Panel, jungle_vision::EjectedViewerMessage),
    #[cfg(feature = "video")]
    Video(VideoMessage),
    Keyboard(iced::keyboard::Event),
    TogglePanelAutoViewport(Panel),
    Tick,
    PulseFrame,
}

impl Message {
    fn name(&self) -> &'static str {
        match self {
            Message::Tick => "Tick",
            Message::PulseFrame => "PulseFrame",
            Message::Panel(panel, _) => match panel {
                Panel::LeadVocalist => "Panel(LeadVocalist)",
                Panel::RhythmGuitarist => "Panel(RhythmGuitarist)",
                Panel::LeadGuitarist => "Panel(LeadGuitarist)",
                Panel::Bass => "Panel(Bass)",
                Panel::Drums => "Panel(Drums)",
            },
            Message::Keyboard(_) => "Keyboard",
            #[cfg(feature = "video")]
            Message::Video(_) => "Video",
            Message::TogglePanelAutoViewport(panel) => match panel {
                Panel::LeadVocalist => "TogglePanelAutoViewport(LeadVocalist)",
                Panel::RhythmGuitarist => "TogglePanelAutoViewport(RhythmGuitarist)",
                Panel::LeadGuitarist => "TogglePanelAutoViewport(LeadGuitarist)",
                Panel::Bass => "TogglePanelAutoViewport(Bass)",
                Panel::Drums => "TogglePanelAutoViewport(Drums)",
            },
        }
    }
}

struct WelcomeUi {
    lead_vocalist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    rhythm_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    lead_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    bass:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    drums:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    metronome: Metronome,
    #[cfg(feature = "video")]
    video_overlays: VideoOverlayController,
    lead_vocalist_pulses: Vec<Instant>,
    rhythm_guitarist_pulses: Vec<Instant>,
    lead_guitarist_pulses: Vec<Instant>,
    bass_pulses: Vec<Instant>,
    drums_pulses: Vec<Instant>,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: DeferredJungleClient<UiClient>,
        journeys: JourneyIds,
        metronome: Metronome,
        shutdown: ShutdownFlag,
        #[cfg(feature = "video")] video_plan: Option<VideoPlaybackPlan>,
    ) -> (Self, Task<Message>) {
        let lead_vocalist = journeys.lead_vocalist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Vocalist")
                .eject_live_animal::<LeadVocalist, _>(client.clone(), journey)
        });
        let rhythm_guitarist = journeys.rhythm_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Rhythm Guitarist")
                .eject_live_animal::<RhythmGuitarist, _>(client.clone(), journey)
        });
        let lead_guitarist = journeys.lead_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Guitarist")
                .eject_live_animal::<LeadGuitarist, _>(client.clone(), journey)
        });
        let bass = journeys.bass.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Bass")
                .eject_live_animal::<Bass, _>(client.clone(), journey)
        });
        let drums = journeys.drums.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Drums")
                .eject_live_animal::<Drums, _>(client, journey)
        });

        (
            Self {
                lead_vocalist,
                rhythm_guitarist,
                lead_guitarist,
                bass,
                drums,
                metronome,
                #[cfg(feature = "video")]
                video_overlays: VideoOverlayController::new(video_plan),
                lead_vocalist_pulses: Vec::new(),
                rhythm_guitarist_pulses: Vec::new(),
                lead_guitarist_pulses: Vec::new(),
                bass_pulses: Vec::new(),
                drums_pulses: Vec::new(),
                shutdown,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        trace!(message = message.name(), "welcome iced app update");
        match message {
            Message::Tick => {
                if self.shutdown.should_shutdown() {
                    return iced::exit();
                }
                self.prune_panel_pulses(Instant::now());
                #[cfg(feature = "video")]
                self.apply_playback_plan();
                #[cfg(feature = "video")]
                self.update_playback_regions();
                Task::none()
            }
            Message::PulseFrame => {
                self.prune_panel_pulses(Instant::now());
                Task::none()
            }
            Message::Keyboard(iced::keyboard::Event::KeyPressed { key, repeat, .. }) => {
                if !repeat
                    && matches!(
                        key,
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Space)
                    )
                {
                    info!("Tick: {}", self.current_rhythm_tick());
                }
                Task::none()
            }
            Message::Keyboard(_) => Task::none(),
            #[cfg(feature = "video")]
            Message::Video(video_event) => {
                self.video_overlays.update(video_event);
                Task::none()
            }
            Message::TogglePanelAutoViewport(panel) => {
                self.toggle_panel_auto_viewport(panel);
                Task::none()
            }
            Message::Panel(panel, event) => {
                self.trigger_panel_pulse(panel);
                match panel {
                    Panel::LeadVocalist => {
                        self.lead_vocalist.as_mut().map_or_else(Task::none, |v| {
                            v.update(event)
                                .map(move |next| Message::Panel(Panel::LeadVocalist, next))
                        })
                    }
                    Panel::RhythmGuitarist => {
                        self.rhythm_guitarist.as_mut().map_or_else(Task::none, |v| {
                            v.update(event)
                                .map(move |next| Message::Panel(Panel::RhythmGuitarist, next))
                        })
                    }
                    Panel::LeadGuitarist => {
                        self.lead_guitarist.as_mut().map_or_else(Task::none, |v| {
                            v.update(event)
                                .map(move |next| Message::Panel(Panel::LeadGuitarist, next))
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
                }
            }
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
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::RhythmGuitarist, event)),
            );
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::LeadGuitarist, event)),
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
        subscriptions.push(iced::keyboard::listen().map(Message::Keyboard));
        subscriptions.push(iced::time::every(UI_TICK_INTERVAL).map(|_| Message::Tick));
        if self.any_panel_pulse_active() {
            subscriptions
                .push(iced::time::every(PANEL_PULSE_FRAME_INTERVAL).map(|_| Message::PulseFrame));
        }
        #[cfg(feature = "video")]
        {
            subscriptions.extend(self.video_overlays.subscriptions(Message::Video));
        }
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let mut panels = Row::new()
            .spacing(12)
            .height(Length::Fill)
            .width(Length::Fill);
        let mut panel_count = 0usize;

        if let Some(viewer) = self.bass.as_ref() {
            panels = panels.push(self.panel_with_overlay("Foo (Bass)", viewer.view(), Panel::Bass));
            panel_count += 1;
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Bar (Guitar)",
                viewer.view(),
                Panel::RhythmGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Baz (Vocals)",
                viewer.view(),
                Panel::LeadVocalist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            panels = panels.push(self.panel_with_overlay(
                "Qux (Guitar)",
                viewer.view(),
                Panel::LeadGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.drums.as_ref() {
            panels =
                panels.push(self.panel_with_overlay("Corge (Drums)", viewer.view(), Panel::Drums));
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

        #[cfg(feature = "video")]
        {
            return self.video_overlays.stack_app_overlays(app, Message::Video);
        }

        #[cfg(not(feature = "video"))]
        app
    }

    fn panel_with_overlay<'a>(
        &'a self,
        label: &'a str,
        content: Element<'a, jungle_vision::EjectedViewerMessage>,
        panel_kind: Panel,
    ) -> Element<'a, Message> {
        let auto_viewport_enabled = self.panel_auto_viewport_enabled(panel_kind);
        let pulse_intensity = self.panel_pulse_intensity(panel_kind, Instant::now());
        let base = panel(
            label,
            content,
            panel_kind,
            auto_viewport_enabled,
            pulse_intensity,
        );
        #[cfg(feature = "video")]
        {
            return self.video_overlays.stack_panel_overlays(
                base,
                to_video_panel(panel_kind),
                Message::Video,
            );
        }

        #[cfg(not(feature = "video"))]
        base
    }

    fn panel_pulses(&self, panel: Panel) -> &[Instant] {
        match panel {
            Panel::LeadVocalist => &self.lead_vocalist_pulses,
            Panel::RhythmGuitarist => &self.rhythm_guitarist_pulses,
            Panel::LeadGuitarist => &self.lead_guitarist_pulses,
            Panel::Bass => &self.bass_pulses,
            Panel::Drums => &self.drums_pulses,
        }
    }

    fn panel_pulses_mut(&mut self, panel: Panel) -> &mut Vec<Instant> {
        match panel {
            Panel::LeadVocalist => &mut self.lead_vocalist_pulses,
            Panel::RhythmGuitarist => &mut self.rhythm_guitarist_pulses,
            Panel::LeadGuitarist => &mut self.lead_guitarist_pulses,
            Panel::Bass => &mut self.bass_pulses,
            Panel::Drums => &mut self.drums_pulses,
        }
    }

    fn trigger_panel_pulse(&mut self, panel: Panel) {
        let now = Instant::now();
        let pulses = self.panel_pulses_mut(panel);
        pulses
            .retain(|started_at| now.saturating_duration_since(*started_at) < PANEL_PULSE_DURATION);
        pulses.push(now);
        if pulses.len() > 32 {
            let keep_from = pulses.len().saturating_sub(32);
            pulses.drain(0..keep_from);
        }
    }

    fn prune_panel_pulses(&mut self, now: Instant) {
        for panel in [
            Panel::LeadVocalist,
            Panel::RhythmGuitarist,
            Panel::LeadGuitarist,
            Panel::Bass,
            Panel::Drums,
        ] {
            self.panel_pulses_mut(panel).retain(|started_at| {
                now.saturating_duration_since(*started_at) < PANEL_PULSE_DURATION
            });
        }
    }

    fn panel_pulse_intensity(&self, panel: Panel, now: Instant) -> f32 {
        self.panel_pulses(panel)
            .iter()
            .map(|started_at| {
                let elapsed = now.saturating_duration_since(*started_at);
                if elapsed >= PANEL_PULSE_DURATION {
                    return 0.0;
                }
                let progress =
                    (elapsed.as_secs_f32() / PANEL_PULSE_DURATION.as_secs_f32()).clamp(0.0, 1.0);
                (std::f32::consts::PI * progress).sin().max(0.0)
            })
            .sum()
    }

    fn any_panel_pulse_active(&self) -> bool {
        let now = Instant::now();
        [
            Panel::LeadVocalist,
            Panel::RhythmGuitarist,
            Panel::LeadGuitarist,
            Panel::Bass,
            Panel::Drums,
        ]
        .into_iter()
        .any(|panel| self.panel_pulse_intensity(panel, now) > 0.0)
    }

    fn panel_auto_viewport_enabled(&self, panel: Panel) -> bool {
        match panel {
            Panel::LeadVocalist => self
                .lead_vocalist
                .as_ref()
                .map(|viewer| viewer.auto_viewport_enabled())
                .unwrap_or(true),
            Panel::RhythmGuitarist => self
                .rhythm_guitarist
                .as_ref()
                .map(|viewer| viewer.auto_viewport_enabled())
                .unwrap_or(true),
            Panel::LeadGuitarist => self
                .lead_guitarist
                .as_ref()
                .map(|viewer| viewer.auto_viewport_enabled())
                .unwrap_or(true),
            Panel::Bass => self
                .bass
                .as_ref()
                .map(|viewer| viewer.auto_viewport_enabled())
                .unwrap_or(true),
            Panel::Drums => self
                .drums
                .as_ref()
                .map(|viewer| viewer.auto_viewport_enabled())
                .unwrap_or(true),
        }
    }

    fn toggle_panel_auto_viewport(&mut self, panel: Panel) {
        match panel {
            Panel::LeadVocalist => {
                if let Some(viewer) = self.lead_vocalist.as_mut() {
                    viewer.set_auto_viewport_enabled(!viewer.auto_viewport_enabled());
                }
            }
            Panel::RhythmGuitarist => {
                if let Some(viewer) = self.rhythm_guitarist.as_mut() {
                    viewer.set_auto_viewport_enabled(!viewer.auto_viewport_enabled());
                }
            }
            Panel::LeadGuitarist => {
                if let Some(viewer) = self.lead_guitarist.as_mut() {
                    viewer.set_auto_viewport_enabled(!viewer.auto_viewport_enabled());
                }
            }
            Panel::Bass => {
                if let Some(viewer) = self.bass.as_mut() {
                    viewer.set_auto_viewport_enabled(!viewer.auto_viewport_enabled());
                }
            }
            Panel::Drums => {
                if let Some(viewer) = self.drums.as_mut() {
                    viewer.set_auto_viewport_enabled(!viewer.auto_viewport_enabled());
                }
            }
        }
    }

    #[cfg(feature = "video")]
    fn apply_playback_plan(&mut self) {
        self.video_overlays
            .apply_playback_plan(self.current_rhythm_tick());
    }

    #[cfg(feature = "video")]
    fn update_playback_regions(&mut self) {
        self.video_overlays.update_playback_regions();
    }

    fn current_rhythm_tick(&self) -> u64 {
        let tick = self
            .metronome
            .tick_duration(crate::effect::TICKS_PER_BEAT)
            .as_secs_f64();
        if tick <= f64::EPSILON {
            return 0;
        }
        (self.metronome.elapsed().as_secs_f64() / tick).floor() as u64
    }
}

fn panel<'a>(
    label: &'a str,
    content: Element<'a, jungle_vision::EjectedViewerMessage>,
    target: Panel,
    auto_viewport_enabled: bool,
    pulse_strength: f32,
) -> Element<'a, Message> {
    // Keep the base jungle panel color, but make the active pulse read clearly yellow.
    let border_base = Color::from_rgb8(24, 63, 43);
    let border_bright = Color::from_rgb8(154, 140, 48);
    let border_peak = Color::from_rgb8(198, 182, 68);
    let header_base = Color::from_rgb8(112, 171, 104);
    let header_bright = Color::from_rgb8(198, 188, 92);
    let header_peak = Color::from_rgb8(224, 214, 118);
    let scaled_strength = pulse_strength.max(0.0) * 0.72;
    let primary = scaled_strength.clamp(0.0, 1.0);
    let additive = ((scaled_strength - 1.0) * 0.35).clamp(0.0, 1.0);
    let border_primary = primary * 0.65;
    let border_additive = additive * 0.45;
    let border_color = lerp_color(
        lerp_color(border_base, border_bright, border_primary),
        border_peak,
        border_additive,
    );
    let header_color = lerp_color(
        lerp_color(header_base, header_bright, primary),
        header_peak,
        additive,
    );
    let border_width = 1.35 + border_primary * 0.08 + border_additive * 0.08;
    let lock_icon = if auto_viewport_enabled {
        LOCK_ICON_SVG
    } else {
        UNLOCK_ICON_SVG
    };
    let lock_button = button(
        svg(svg::Handle::from_memory(lock_icon))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|_theme, _status| svg::Style {
                color: Some(Color::from_rgb8(223, 245, 230)),
            }),
    )
    .padding([4, 4])
    .style(move |theme, status| panel_lock_button_style(auto_viewport_enabled, theme, status))
    .on_press(Message::TogglePanelAutoViewport(target));

    container(
        column![
            Row::new()
                .push(text(label).size(13).color(header_color))
                .push(Space::new().width(Length::Fill))
                .push(lock_button),
            container(content.map(move |event| Message::Panel(target, event)))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .style(move |_theme| panel_style(border_color, border_width))
    .into()
}

fn app_background(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(8, 19, 13))),
        ..Default::default()
    }
}

fn panel_style(border_color: Color, border_width: f32) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(10, 26, 17))),
        border: iced::border::rounded(8)
            .color(border_color)
            .width(border_width),
        ..Default::default()
    }
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

fn panel_lock_button_style(
    enabled: bool,
    _theme: &iced::Theme,
    status: button::Status,
) -> iced::widget::button::Style {
    let background = if enabled {
        match status {
            button::Status::Hovered => Color::from_rgb8(28, 89, 55),
            _ => Color::from_rgb8(20, 71, 45),
        }
    } else {
        match status {
            button::Status::Hovered => Color::from_rgb8(89, 60, 26),
            _ => Color::from_rgb8(71, 48, 20),
        }
    };

    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: Color::from_rgb8(223, 245, 230),
        border: iced::border::rounded(6)
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
