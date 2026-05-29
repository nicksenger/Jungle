use crate::UiClient;
use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use crate::metronome::Metronome;
use async_trait::async_trait;
use futures::StreamExt;
#[cfg(feature = "video")]
use iced::widget::stack;
use iced::widget::{Row, Space, button, column, container, svg, text};
use iced::{Color, Element, Font, Length, Subscription, Task};
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::{ExecutorError, JungleClient, RunnerOut, SupportedAnimal, Work};
#[cfg(feature = "video")]
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

const DEFERRED_STREAM_LOG_INTERVAL: usize = 512;
const DEFERRED_STREAM_SLOW_WAIT_WARN_MS: u64 = 400;
const DEFERRED_STREAM_LAG_WARN_MS: u64 = 150;
const DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS: i64 = 2_000;
const DEFERRED_STREAM_SLOW_DECISION_WARN_US: u128 = 500;
const UI_TICK_INTERVAL: Duration = Duration::from_millis(500);
const LOCK_ICON_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/></svg>"#;
const UNLOCK_ICON_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="10" rx="2"/><path d="M16 11V7a4 4 0 0 0-7.5-2"/></svg>"#;
#[cfg(feature = "video")]
const AV_OVERLAY_SOURCES: [(&str, &[u8]); 21] = [
    ("baboons.mkv", include_bytes!("../assets/baboons.mkv")),
    ("chimp.mkv", include_bytes!("../assets/chimp.mkv")),
    (
        "chimpattack.mkv",
        include_bytes!("../assets/chimpattack.mkv"),
    ),
    ("croc.mkv", include_bytes!("../assets/croc.mkv")),
    ("crocstrike.mkv", include_bytes!("../assets/crocstrike.mkv")),
    ("elephants.mkv", include_bytes!("../assets/elephants.mkv")),
    ("giraffe.mkv", include_bytes!("../assets/giraffe.mkv")),
    ("hippo.mkv", include_bytes!("../assets/hippo.mkv")),
    ("jackfruit.mkv", include_bytes!("../assets/jackfruit.mkv")),
    ("jaguar.mkv", include_bytes!("../assets/jaguar.mkv")),
    ("jaguar2.mkv", include_bytes!("../assets/jaguar2.mkv")),
    ("jungle.mkv", include_bytes!("../assets/jungle.mkv")),
    ("lions.mkv", include_bytes!("../assets/lions.mkv")),
    ("monkey.mkv", include_bytes!("../assets/monkey.mkv")),
    ("ostrich.mkv", include_bytes!("../assets/ostrich.mkv")),
    ("panic.mkv", include_bytes!("../assets/panic.mkv")),
    ("serpentine.mkv", include_bytes!("../assets/serpentine.mkv")),
    ("shrooms.mkv", include_bytes!("../assets/shrooms.mkv")),
    ("toucan.mkv", include_bytes!("../assets/toucan.mkv")),
    ("toucanfly.mkv", include_bytes!("../assets/toucanfly.mkv")),
    ("zebra.mkv", include_bytes!("../assets/zebra.mkv")),
];
#[cfg(feature = "video")]
const VIDEO_FADE_IN: Duration = Duration::from_millis(180);
#[cfg(feature = "video")]
const VIDEO_FADE_OUT: Duration = Duration::from_millis(220);

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
    RhythmGuitarist,
    LeadGuitarist,
    Bass,
    Drums,
}

impl Panel {
    #[cfg(feature = "video")]
    const ALL: [Self; 5] = [
        Self::LeadVocalist,
        Self::RhythmGuitarist,
        Self::LeadGuitarist,
        Self::Bass,
        Self::Drums,
    ];
}

#[cfg(feature = "video")]
#[derive(Debug, Clone, Copy)]
struct VideoPlaybackRequest {
    offset: Duration,
    duration: Duration,
    opacity: f32,
}

#[cfg(feature = "video")]
impl VideoPlaybackRequest {
    const fn new(offset_ms: u64, duration_ms: u64, opacity: f32) -> Self {
        Self {
            offset: Duration::from_millis(offset_ms),
            duration: Duration::from_millis(duration_ms),
            opacity,
        }
    }
}

#[cfg(feature = "video")]
#[derive(Debug, Clone, Copy)]
struct TickPlaybackPlan {
    tick: u32,
    app_overlay: Option<VideoPlaybackRequest>,
    lead_vocalist_panel: Option<VideoPlaybackRequest>,
    rhythm_guitarist_panel: Option<VideoPlaybackRequest>,
    lead_guitarist_panel: Option<VideoPlaybackRequest>,
    bass_panel: Option<VideoPlaybackRequest>,
    drums_panel: Option<VideoPlaybackRequest>,
}

#[cfg(feature = "video")]
impl TickPlaybackPlan {
    fn panel_request(self, panel: Panel) -> Option<VideoPlaybackRequest> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel,
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel,
            Panel::LeadGuitarist => self.lead_guitarist_panel,
            Panel::Bass => self.bass_panel,
            Panel::Drums => self.drums_panel,
        }
    }
}

#[cfg(feature = "video")]
const VIDEO_PLAYBACK_PLAN: [TickPlaybackPlan; 3] = [
    TickPlaybackPlan {
        tick: 0,
        app_overlay: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
        lead_vocalist_panel: None,
        rhythm_guitarist_panel: None,
        lead_guitarist_panel: None,
        bass_panel: None,
        drums_panel: None,
    },
    TickPlaybackPlan {
        tick: 4,
        app_overlay: None,
        lead_vocalist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
        rhythm_guitarist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
        lead_guitarist_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
        bass_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
        drums_panel: Some(VideoPlaybackRequest::new(0, 2_000, 0.3)),
    },
    TickPlaybackPlan {
        tick: 40,
        app_overlay: None,
        lead_vocalist_panel: None,
        rhythm_guitarist_panel: None,
        lead_guitarist_panel: None,
        bass_panel: None,
        drums_panel: None,
    },
];

#[cfg(feature = "video")]
#[derive(Debug, Clone)]
struct RegionPlayback {
    enabled: bool,
    visible_until: Option<Instant>,
    fade_out_at: Option<Instant>,
    fade_out_started: bool,
}

#[cfg(feature = "video")]
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
    #[cfg(feature = "video")]
    AppVideo(iced_av1::widget::Message),
    #[cfg(feature = "video")]
    PanelVideo(Panel, iced_av1::widget::Message),
    Keyboard(iced::keyboard::Event),
    TogglePanelAutoViewport(Panel),
    Tick,
}

impl Message {
    fn name(&self) -> &'static str {
        match self {
            Message::Tick => "Tick",
            Message::Panel(panel, _) => match panel {
                Panel::LeadVocalist => "Panel(LeadVocalist)",
                Panel::RhythmGuitarist => "Panel(RhythmGuitarist)",
                Panel::LeadGuitarist => "Panel(LeadGuitarist)",
                Panel::Bass => "Panel(Bass)",
                Panel::Drums => "Panel(Drums)",
            },
            Message::Keyboard(_) => "Keyboard",
            #[cfg(feature = "video")]
            Message::AppVideo(_) => "AppVideo",
            #[cfg(feature = "video")]
            Message::PanelVideo(panel, _) => match panel {
                Panel::LeadVocalist => "PanelVideo(LeadVocalist)",
                Panel::RhythmGuitarist => "PanelVideo(RhythmGuitarist)",
                Panel::LeadGuitarist => "PanelVideo(LeadGuitarist)",
                Panel::Bass => "PanelVideo(Bass)",
                Panel::Drums => "PanelVideo(Drums)",
            },
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
    applied_ticks: HashSet<u32>,
    #[cfg(feature = "video")]
    app_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    app_overlay_playback: RegionPlayback,
    #[cfg(feature = "video")]
    lead_vocalist_panel_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    rhythm_guitarist_panel_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    lead_guitarist_panel_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    bass_panel_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    drums_panel_overlay: Option<iced_av1::widget::State>,
    #[cfg(feature = "video")]
    lead_vocalist_panel_playback: RegionPlayback,
    #[cfg(feature = "video")]
    rhythm_guitarist_panel_playback: RegionPlayback,
    #[cfg(feature = "video")]
    lead_guitarist_panel_playback: RegionPlayback,
    #[cfg(feature = "video")]
    bass_panel_playback: RegionPlayback,
    #[cfg(feature = "video")]
    drums_panel_playback: RegionPlayback,
    #[cfg(feature = "video")]
    video_source_cursor: usize,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: DeferredJungleClient<UiClient>,
        journeys: JourneyIds,
        metronome: Metronome,
        shutdown: ShutdownFlag,
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

        #[cfg(feature = "video")]
        let (_, initial_video_bytes) = video_source_at_index(0);
        #[cfg(feature = "video")]
        let app_overlay = init_video_state(
            "app overlay",
            iced_av1::ScaleMode::Stretch,
            initial_video_bytes,
        );
        #[cfg(feature = "video")]
        let lead_vocalist_panel_overlay = init_video_state(
            "lead vocalist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            initial_video_bytes,
        );
        #[cfg(feature = "video")]
        let rhythm_guitarist_panel_overlay = init_video_state(
            "rhythm guitarist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            initial_video_bytes,
        );
        #[cfg(feature = "video")]
        let lead_guitarist_panel_overlay = init_video_state(
            "lead guitarist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            initial_video_bytes,
        );
        #[cfg(feature = "video")]
        let bass_panel_overlay = init_video_state(
            "bass panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            initial_video_bytes,
        );
        #[cfg(feature = "video")]
        let drums_panel_overlay = init_video_state(
            "drums panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            initial_video_bytes,
        );
        (
            Self {
                lead_vocalist,
                rhythm_guitarist,
                lead_guitarist,
                bass,
                drums,
                metronome,
                #[cfg(feature = "video")]
                applied_ticks: HashSet::new(),
                #[cfg(feature = "video")]
                app_overlay,
                #[cfg(feature = "video")]
                app_overlay_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                lead_vocalist_panel_overlay,
                #[cfg(feature = "video")]
                rhythm_guitarist_panel_overlay,
                #[cfg(feature = "video")]
                lead_guitarist_panel_overlay,
                #[cfg(feature = "video")]
                bass_panel_overlay,
                #[cfg(feature = "video")]
                drums_panel_overlay,
                #[cfg(feature = "video")]
                lead_vocalist_panel_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                rhythm_guitarist_panel_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                lead_guitarist_panel_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                bass_panel_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                drums_panel_playback: RegionPlayback::hidden(),
                #[cfg(feature = "video")]
                video_source_cursor: 1,
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
                #[cfg(feature = "video")]
                self.apply_playback_plan();
                #[cfg(feature = "video")]
                self.update_playback_regions();
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
            Message::AppVideo(event) => {
                self.app_overlay.as_mut().map_or_else(Task::none, |video| {
                    video.update(event);
                    Task::none()
                })
            }
            #[cfg(feature = "video")]
            Message::PanelVideo(panel, event) => {
                if let Some(video) = self.panel_overlay_mut(panel) {
                    video.update(event);
                }
                Task::none()
            }
            Message::TogglePanelAutoViewport(panel) => {
                self.toggle_panel_auto_viewport(panel);
                Task::none()
            }
            Message::Panel(panel, event) => match panel {
                Panel::LeadVocalist => self.lead_vocalist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadVocalist, next))
                }),
                Panel::RhythmGuitarist => {
                    self.rhythm_guitarist.as_mut().map_or_else(Task::none, |v| {
                        v.update(event)
                            .map(move |next| Message::Panel(Panel::RhythmGuitarist, next))
                    })
                }
                Panel::LeadGuitarist => self.lead_guitarist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadGuitarist, next))
                }),
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
        #[cfg(feature = "video")]
        {
            if let Some(video) = self.app_overlay.as_ref() {
                subscriptions.push(video.subscription(Message::AppVideo));
            }
            if let Some(video) = self.lead_vocalist_panel_overlay.as_ref() {
                subscriptions.push(
                    video.subscription(|event| Message::PanelVideo(Panel::LeadVocalist, event)),
                );
            }
            if let Some(video) = self.rhythm_guitarist_panel_overlay.as_ref() {
                subscriptions.push(
                    video.subscription(|event| Message::PanelVideo(Panel::RhythmGuitarist, event)),
                );
            }
            if let Some(video) = self.lead_guitarist_panel_overlay.as_ref() {
                subscriptions.push(
                    video.subscription(|event| Message::PanelVideo(Panel::LeadGuitarist, event)),
                );
            }
            if let Some(video) = self.bass_panel_overlay.as_ref() {
                subscriptions
                    .push(video.subscription(|event| Message::PanelVideo(Panel::Bass, event)));
            }
            if let Some(video) = self.drums_panel_overlay.as_ref() {
                subscriptions
                    .push(video.subscription(|event| Message::PanelVideo(Panel::Drums, event)));
            }
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
                "Bar (Rhythm Guitar)",
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
                "Qux (Lead Guitar)",
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
        let auto_viewport_enabled = self.panel_auto_viewport_enabled(panel_kind);
        let base = panel(label, content, panel_kind, auto_viewport_enabled);
        #[cfg(feature = "video")]
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
        let beat = self.current_beat_tick();
        for plan in VIDEO_PLAYBACK_PLAN {
            if beat < u64::from(plan.tick) {
                continue;
            }
            if !self.applied_ticks.insert(plan.tick) {
                continue;
            }

            if plan.app_overlay.is_some()
                || Panel::ALL
                    .into_iter()
                    .any(|panel| plan.panel_request(panel).is_some())
            {
                let (video_name, video_bytes) = self.next_video_source();
                info!(
                    tick = plan.tick,
                    video = video_name,
                    "selected welcome video for playback plan tick"
                );
                self.reinitialize_video_overlays(video_bytes);
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
        }
    }

    #[cfg(feature = "video")]
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
        }
    }

    #[cfg(feature = "video")]
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
                    if let Err(error) = overlay.pause() {
                        warn!(error = %error, "failed to pause AV overlay after visibility ended");
                    }
                }
                *playback = RegionPlayback::hidden();
            }
        }
    }

    #[cfg(feature = "video")]
    fn start_region_playback(
        overlay: Option<&iced_av1::widget::State>,
        playback: &mut RegionPlayback,
        request: VideoPlaybackRequest,
        now: Instant,
    ) {
        if let Some(overlay) = overlay {
            if let Err(error) = overlay.resume() {
                warn!(error = %error, "failed to resume AV overlay before playback");
                return;
            }
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

    #[cfg(feature = "video")]
    fn stop_region_playback(
        overlay: Option<&iced_av1::widget::State>,
        playback: &mut RegionPlayback,
    ) {
        if let Some(overlay) = overlay {
            overlay.set_opacity(0.0);
            if let Err(error) = overlay.pause() {
                warn!(error = %error, "failed to pause AV overlay for hidden region");
            }
        }
        *playback = RegionPlayback::hidden();
    }

    #[cfg(feature = "video")]
    fn current_beat_tick(&self) -> u64 {
        let beat = self.metronome.beat_duration().as_secs_f64();
        if beat <= f64::EPSILON {
            return 0;
        }
        (self.metronome.elapsed().as_secs_f64() / beat).floor() as u64
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

    #[cfg(feature = "video")]
    fn panel_overlay(&self, panel: Panel) -> Option<&iced_av1::widget::State> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel_overlay.as_ref(),
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel_overlay.as_ref(),
            Panel::LeadGuitarist => self.lead_guitarist_panel_overlay.as_ref(),
            Panel::Bass => self.bass_panel_overlay.as_ref(),
            Panel::Drums => self.drums_panel_overlay.as_ref(),
        }
    }

    #[cfg(feature = "video")]
    fn panel_overlay_mut(&mut self, panel: Panel) -> Option<&mut iced_av1::widget::State> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel_overlay.as_mut(),
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel_overlay.as_mut(),
            Panel::LeadGuitarist => self.lead_guitarist_panel_overlay.as_mut(),
            Panel::Bass => self.bass_panel_overlay.as_mut(),
            Panel::Drums => self.drums_panel_overlay.as_mut(),
        }
    }

    #[cfg(feature = "video")]
    fn panel_playback(&self, panel: Panel) -> &RegionPlayback {
        match panel {
            Panel::LeadVocalist => &self.lead_vocalist_panel_playback,
            Panel::RhythmGuitarist => &self.rhythm_guitarist_panel_playback,
            Panel::LeadGuitarist => &self.lead_guitarist_panel_playback,
            Panel::Bass => &self.bass_panel_playback,
            Panel::Drums => &self.drums_panel_playback,
        }
    }

    #[cfg(feature = "video")]
    fn panel_slot_mut(
        &mut self,
        panel: Panel,
    ) -> (&mut Option<iced_av1::widget::State>, &mut RegionPlayback) {
        match panel {
            Panel::LeadVocalist => (
                &mut self.lead_vocalist_panel_overlay,
                &mut self.lead_vocalist_panel_playback,
            ),
            Panel::RhythmGuitarist => (
                &mut self.rhythm_guitarist_panel_overlay,
                &mut self.rhythm_guitarist_panel_playback,
            ),
            Panel::LeadGuitarist => (
                &mut self.lead_guitarist_panel_overlay,
                &mut self.lead_guitarist_panel_playback,
            ),
            Panel::Bass => (&mut self.bass_panel_overlay, &mut self.bass_panel_playback),
            Panel::Drums => (
                &mut self.drums_panel_overlay,
                &mut self.drums_panel_playback,
            ),
        }
    }

    #[cfg(feature = "video")]
    fn next_video_source(&mut self) -> (&'static str, &'static [u8]) {
        let source = video_source_at_index(self.video_source_cursor);
        self.video_source_cursor = self.video_source_cursor.wrapping_add(1);
        source
    }

    #[cfg(feature = "video")]
    fn reinitialize_video_overlays(&mut self, video_bytes: &'static [u8]) {
        self.app_overlay =
            init_video_state("app overlay", iced_av1::ScaleMode::Stretch, video_bytes);
        self.lead_vocalist_panel_overlay = init_video_state(
            "lead vocalist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            video_bytes,
        );
        self.rhythm_guitarist_panel_overlay = init_video_state(
            "rhythm guitarist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            video_bytes,
        );
        self.lead_guitarist_panel_overlay = init_video_state(
            "lead guitarist panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            video_bytes,
        );
        self.bass_panel_overlay = init_video_state(
            "bass panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            video_bytes,
        );
        self.drums_panel_overlay = init_video_state(
            "drums panel overlay",
            iced_av1::ScaleMode::Cover { offset: 0.5 },
            video_bytes,
        );
    }
}

#[cfg(feature = "video")]
fn init_video_state(
    region: &str,
    scale_mode: iced_av1::ScaleMode,
    video_bytes: &[u8],
) -> Option<iced_av1::widget::State> {
    let playback_options = iced_av1::PlaybackOptions::default();
    let opacity_options = iced_av1::OpacityOptions {
        opacity: 0.0,
        tween: iced_av1::OpacityTween {
            duration: VIDEO_FADE_IN,
        },
    };
    let source = iced_av1::MediaSource::from_bytes(video_bytes.to_vec());
    match iced_av1::widget::State::new_with_media_source_and_opacity_options(
        source,
        playback_options,
        opacity_options,
    ) {
        Ok(mut state) => {
            state.set_scale_mode(scale_mode);
            if let Err(error) = state.pause() {
                warn!(
                    error = %error,
                    region,
                    "failed to pause AV overlay state at initialization"
                );
            }
            Some(state)
        }
        Err(error) => {
            warn!(error = %error, region, "failed to initialize AV overlay state");
            None
        }
    }
}

#[cfg(feature = "video")]
fn map_panel_video_message(message: iced_av1::widget::Message) -> iced_av1::widget::Message {
    message
}

#[cfg(feature = "video")]
fn video_source_at_index(index: usize) -> (&'static str, &'static [u8]) {
    AV_OVERLAY_SOURCES[index % AV_OVERLAY_SOURCES.len()]
}

#[cfg(feature = "video")]
fn duration_to_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn panel<'a>(
    label: &'a str,
    content: Element<'a, jungle_vision::EjectedViewerMessage>,
    target: Panel,
    auto_viewport_enabled: bool,
) -> Element<'a, Message> {
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
                .push(text(label).size(13).color(Color::from_rgb8(198, 229, 211)))
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
