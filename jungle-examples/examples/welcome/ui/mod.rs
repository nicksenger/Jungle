use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use crate::UiClient;
use async_trait::async_trait;
use futures::StreamExt;
use iced::widget::{column, container, stack, text, Row};
use iced::{Color, Element, Font, Length, Subscription, Task};
use jungle_sdk::client::JourneyUpdateSubscription;
use jungle_sdk::{ExecutorError, JungleClient, RunnerOut, SupportedAnimal, Work};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

const DEFERRED_STREAM_LOG_INTERVAL: usize = 512;
const DEFERRED_STREAM_SLOW_WAIT_WARN_MS: u64 = 400;
const DEFERRED_STREAM_LAG_WARN_MS: u64 = 150;
const DEFERRED_STREAM_SOURCE_EVENT_AGE_WARN_MS: i64 = 2_000;
const DEFERRED_STREAM_SLOW_DECISION_WARN_US: u128 = 500;

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
    shutdown: ShutdownFlag,
) -> iced::Result {
    let title = "Welcome to the Jungle";
    iced::application(
        move || WelcomeUi::new(client.clone(), journeys, shutdown.clone()),
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

#[derive(Debug, Clone)]
enum Message {
    Panel(Panel, jungle_vision::EjectedViewerMessage),
    AvOverlay(iced_av1::widget::Message),
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
    av_overlay: Option<iced_av1::widget::State>,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: DeferredJungleClient<UiClient>,
        journeys: JourneyIds,
        shutdown: ShutdownFlag,
    ) -> (Self, Task<Message>) {
        let lead_vocalist = journeys.lead_vocalist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Vocalist")
                .eject_live_animal::<LeadVocalist, _>(client.clone(), journey)
        });
        let lead_guitarist = journeys.lead_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Guitarist")
                .eject_live_animal::<LeadGuitarist, _>(client.clone(), journey)
        });
        let rhythm_guitarist = journeys.rhythm_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Rhythm Guitarist")
                .eject_live_animal::<RhythmGuitarist, _>(client.clone(), journey)
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
        let av_overlay = match iced_av1::widget::State::new() {
            Ok(state) => Some(state),
            Err(error) => {
                warn!(
                    error = %error,
                    "failed to initialize AV overlay state; continuing without video overlay"
                );
                None
            }
        };

        (
            Self {
                lead_vocalist,
                lead_guitarist,
                rhythm_guitarist,
                bass,
                drums,
                av_overlay,
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
                Task::none()
            }
            Message::AvOverlay(event) => {
                if let Some(av_overlay) = self.av_overlay.as_mut() {
                    av_overlay.update(event);
                }
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
        if let Some(av_overlay) = self.av_overlay.as_ref() {
            subscriptions.push(av_overlay.subscription(Message::AvOverlay));
        }
        subscriptions
            .push(iced::time::every(std::time::Duration::from_millis(200)).map(|_| Message::Tick));
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let mut panels = Row::new()
            .spacing(12)
            .height(Length::Fill)
            .width(Length::Fill);
        let mut panel_count = 0usize;

        if let Some(viewer) = self.bass.as_ref() {
            panels = panels.push(panel("Bass", viewer.view(), Panel::Bass));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            panels = panels.push(panel("Lead Guitarist", viewer.view(), Panel::LeadGuitarist));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            panels = panels.push(panel("Lead Vocalist", viewer.view(), Panel::LeadVocalist));
            panel_count += 1;
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            panels = panels.push(panel(
                "Rhythm Guitarist",
                viewer.view(),
                Panel::RhythmGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.drums.as_ref() {
            panels = panels.push(panel("Drums", viewer.view(), Panel::Drums));
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

        if let Some(av_overlay) = self.av_overlay.as_ref() {
            if let Some(overlay) = av_overlay.overlay_view(Message::AvOverlay) {
                return stack([app, overlay])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }
        }

        app
    }
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
