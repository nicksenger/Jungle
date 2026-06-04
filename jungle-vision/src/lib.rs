mod cluster_panel;
mod widgets;

use iced::futures::{self, Stream, StreamExt};
use iced::widget::{button, column, container, row, text, Space};
use iced::window;
use iced::window::Screenshot;
use iced::{Color, Element, Font, Length, Subscription, Task};
use iced_sugiyama::{AutoFit, Cluster, Graph, OutgoingEdgeStyle, Sugiyama, ViewportInteraction};
use jungle_client::client::JourneyUpdateSubscription;
use jungle_client::JungleClient;
use jungle_types::{Animal, JourneyAst, JourneyAstSource, JourneyUpdateEvent, RunnerUpdateOut};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, warn};
use uuid::Uuid;
use widgets::animated_cluster::AnimatedClusterView;
use widgets::animated_step::AnimatedStepNode;

const WINDOW_WIDTH: f32 = 1360.0;
const WINDOW_HEIGHT: f32 = 900.0;
const NODE_WIDTH: f64 = 240.0;
const NODE_HEIGHT: f64 = 80.0;
const GRAPH_WIDGET_ID: &str = "jungle-vision";
const DEFAULT_CLUSTER_FILL: Color = Color::from_rgba8(20, 46, 30, 0.10);
const NODE_ANIMATION_DURATION: Duration = Duration::from_millis(320);
const CLUSTER_BORDER_ANIMATION_DURATION: Duration = Duration::from_millis(320);
const CLUSTER_RECOLLAPSE_DELAY: Duration = Duration::from_secs(2);
const VISION_LIVE_EVENT_LOG_INTERVAL: usize = 512;
const VISION_STALE_EVENT_WARN_MS: i64 = 1_000;
const VISION_APPLY_QUEUE_DELAY_WARN_MS: i64 = 100;
const VISION_END_TO_END_AGE_WARN_MS: i64 = 2_000;
const VISION_SLOW_APPLY_WARN_MS: u128 = 20;
const VISION_SLOW_THEME_UPDATE_WARN_MS: u128 = 20;
const RUNNING_REPAIR_PROMOTION_SEQUENCE_GAP: usize = 2;
const INITIAL_LIVE_BATCH_WINDOW: Duration = Duration::from_millis(20);

static CLUSTER_FILL_COLORS: OnceLock<RwLock<Vec<Color>>> = OnceLock::new();
static VISION_LIVE_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static VISION_APPLY_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_APPLY_QUEUE_DELAY_MS: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_END_TO_END_EVENT_AGE_MS: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_APPLY_ELAPSED_MS: AtomicUsize = AtomicUsize::new(0);

pub struct AnyAnimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase<T> {
    Static,
    Live(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Step,
    Conditional,
    Select,
    Join,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterKind {
    While,
    Transparent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterExpansionMode {
    Automatic,
    AlwaysExpanded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterExpansionConfig {
    pub while_clusters: ClusterExpansionMode,
    pub transparent_clusters: ClusterExpansionMode,
}

impl ClusterExpansionConfig {
    fn mode_for(self, kind: ClusterKind) -> ClusterExpansionMode {
        match kind {
            ClusterKind::While => self.while_clusters,
            ClusterKind::Transparent => self.transparent_clusters,
        }
    }
}

impl Default for ClusterExpansionConfig {
    fn default() -> Self {
        Self {
            while_clusters: ClusterExpansionMode::Automatic,
            transparent_clusters: ClusterExpansionMode::Automatic,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepViewCtx<'a> {
    pub display_id: u32,
    pub runtime_id: Option<u32>,
    pub proxy_runtime_ids: &'a [u32],
    pub successor_runtime_ids: &'a [u32],
    pub kind: StepKind,
    pub label: &'a str,
    pub metadata: Option<&'a str>,
    pub phase: Phase<RuntimeState>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClusterLive {
    pub has_running: bool,
    pub has_failed: bool,
    pub has_completed: bool,
}

#[derive(Debug, Clone)]
pub struct ClusterViewCtx<'a> {
    pub cluster_id: u32,
    pub cluster_index: usize,
    pub kind: ClusterKind,
    pub label: &'a str,
    pub metadata: Option<&'a str>,
    pub parent_cluster_id: Option<u32>,
    pub depth: usize,
    pub member_display_ids: &'a [u32],
    pub entry_runtime_ids: &'a [u32],
    pub member_runtime_ids: &'a [u32],
    pub successor_runtime_ids: &'a [u32],
    pub phase: Phase<ClusterLive>,
}

pub enum ClusterView<Message: Clone + 'static> {
    Expanded {
        overlay: Option<Element<'static, ViewerEvent<Message>>>,
        fill: Color,
    },
    Collapsed {
        element: Element<'static, ViewerEvent<Message>>,
        size: (f64, f64),
    },
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeStyleCtx {
    pub edge_index: usize,
    pub source_display_id: u32,
    pub target_display_id: u32,
    pub source_runtime_id: Option<u32>,
    pub target_runtime_id: Option<u32>,
    pub source_has_proxy_runtime: bool,
    pub target_has_proxy_runtime: bool,
    pub source_phase: Phase<RuntimeState>,
    pub target_phase: Phase<RuntimeState>,
    pub extent: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct EdgeStyle {
    pub width: f32,
    pub start: Color,
    pub end: Color,
}

#[derive(Debug, Clone)]
pub enum ViewerEvent<Message: Clone + 'static> {
    JourneyUpdate(JourneyUpdateEvent),
    Message(Message),
}

pub trait JunglePanelTheme<A = AnyAnimal>: Send + Sync + 'static {
    type State: 'static;
    type Message: Clone + 'static;

    fn init(&self) -> Self::State;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: ViewerEvent<Self::Message>,
    ) -> Task<ViewerEvent<Self::Message>> {
        Task::none()
    }

    fn view_step(
        &self,
        state: &Self::State,
        cx: &StepViewCtx<'_>,
    ) -> (Element<'static, ViewerEvent<Self::Message>>, (f64, f64));

    fn view_cluster(
        &self,
        state: &Self::State,
        cx: &ClusterViewCtx<'_>,
    ) -> ClusterView<Self::Message>;

    fn edge_style(&self, _state: &Self::State, _ctx: EdgeStyleCtx) -> Option<EdgeStyle> {
        None
    }
}

#[derive(Clone)]
pub struct JungleViewerBuilder {
    title: String,
    width: f32,
    height: f32,
    screenshot_path: Option<PathBuf>,
    headless: bool,
    animation_duration: Option<Duration>,
    animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
}

impl Default for JungleViewerBuilder {
    fn default() -> Self {
        Self {
            title: "Jungle Viewer".to_string(),
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            screenshot_path: None,
            headless: false,
            animation_duration: None,
            animation_easing: None,
        }
    }
}

impl JungleViewerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = value.into();
        self
    }

    pub fn window_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn screenshot_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.screenshot_path = Some(path.into());
        self
    }

    pub fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn animation_duration(mut self, duration: Duration) -> Self {
        self.animation_duration = Some(duration);
        self
    }

    pub fn animation_easing(
        mut self,
        easing: &'static iced_sugiyama::motion::easing::Easing,
    ) -> Self {
        self.animation_easing = Some(easing);
        self
    }

    pub fn view_animal<A>(self) -> iced::Result
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
    {
        self.view_animal_with_theme::<A, _, AnyAnimal>(DefaultTheme::default())
    }

    pub fn view_animal_with_theme<A, T, Scope>(self, theme: T) -> iced::Result
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Flow as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Flow>();
        let model = GraphModel::from_ast(ast);

        self.run(
            ViewMode::Static {
                journey_name,
                model,
            },
            theme,
        )
    }

    pub fn eject_animal_with_theme<A, T, Scope>(self, theme: T) -> EjectedViewer<T, Scope>
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Flow as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Flow>();
        let model = GraphModel::from_ast(ast);
        EjectedViewer::new(
            ViewMode::Static {
                journey_name,
                model,
            },
            theme,
            self.animation_duration,
            self.animation_easing,
            iced_sugiyama::Id::new(format!("{GRAPH_WIDGET_ID}-{}", Uuid::new_v4())),
        )
    }

    pub fn eject_animal<A>(self) -> EjectedViewer<DefaultTheme, AnyAnimal>
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
    {
        self.eject_animal_with_theme::<A, _, AnyAnimal>(DefaultTheme::default())
    }

    pub fn view_live_animal<A, C>(self, client: C, journey_id: Uuid) -> iced::Result
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        C: JungleClient + 'static,
    {
        self.view_live_animal_with_theme::<A, C, _, AnyAnimal>(
            client,
            journey_id,
            DefaultTheme::default(),
        )
    }

    pub fn view_live_animal_with_theme<A, C, T, Scope>(
        self,
        client: C,
        journey_id: Uuid,
        theme: T,
    ) -> iced::Result
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        C: JungleClient + 'static,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Flow as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Flow>();
        let model = GraphModel::from_ast(ast);
        let client: Arc<dyn JungleClient> = Arc::new(client);

        self.run(
            ViewMode::Live {
                journey_name,
                model,
                client,
                journey_id,
            },
            theme,
        )
    }

    pub fn eject_live_animal_with_theme<A, C, T, Scope>(
        self,
        client: C,
        journey_id: Uuid,
        theme: T,
    ) -> EjectedViewer<T, Scope>
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        C: JungleClient + 'static,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Flow as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Flow>();
        let model = GraphModel::from_ast(ast);
        let client: Arc<dyn JungleClient> = Arc::new(client);

        EjectedViewer::new(
            ViewMode::Live {
                journey_name,
                model,
                client,
                journey_id,
            },
            theme,
            self.animation_duration,
            self.animation_easing,
            iced_sugiyama::Id::new(format!("{GRAPH_WIDGET_ID}-{}", Uuid::new_v4())),
        )
    }

    pub fn eject_live_animal<A, C>(
        self,
        client: C,
        journey_id: Uuid,
    ) -> EjectedViewer<DefaultTheme, AnyAnimal>
    where
        A: Animal + 'static,
        A::Flow: JourneyAstSource,
        C: JungleClient + 'static,
    {
        self.eject_live_animal_with_theme::<A, C, _, AnyAnimal>(
            client,
            journey_id,
            DefaultTheme::default(),
        )
    }

    fn run<T, Scope>(self, mode: ViewMode, theme: T) -> iced::Result
    where
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let title = self.title.clone();
        let width = self.width;
        let height = self.height;
        let capture = self.screenshot_path.clone().map(|path| CaptureConfig {
            output_path: path,
            close_after_capture: self.headless,
        });
        let animation_duration = self.animation_duration;
        let animation_easing = self.animation_easing;
        let theme = Arc::new(theme);
        iced::application(
            move || {
                ViewerApp::new(
                    mode.clone(),
                    capture.clone(),
                    theme.clone(),
                    animation_duration,
                    animation_easing,
                )
            },
            ViewerApp::<T, Scope>::update,
            ViewerApp::<T, Scope>::view,
        )
        .title(move |_app: &ViewerApp<T, Scope>| title.clone())
        .subscription(ViewerApp::<T, Scope>::subscription)
        .window_size((width, height))
        .antialiasing(true)
        .default_font(Font::with_name("Iosevka"))
        .run()
    }
}

pub fn view_animal<A>() -> iced::Result
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
{
    JungleViewerBuilder::new().view_animal::<A>()
}

pub fn view_live_animal<A, C>(client: C, journey_id: Uuid) -> iced::Result
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
    C: JungleClient + 'static,
{
    JungleViewerBuilder::new().view_live_animal::<A, C>(client, journey_id)
}

#[derive(Debug, Clone)]
pub enum EjectedViewerMessage {
    LiveEvent(Result<JourneyUpdateEvent, String>),
    HydrateLiveEvents(Vec<JourneyUpdateEvent>),
    ApplyLiveEvent {
        update: JourneyUpdateEvent,
        received_unix_ms: i64,
    },
    Theme(ViewerEvent<()>),
    ViewportInteraction(ViewportInteraction),
    Retry,
}

pub struct EjectedViewer<T, Scope>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    mode: ViewMode,
    state: LiveState,
    live_generation: u64,
    theme: Arc<T>,
    theme_state: T::State,
    animation_duration: Option<Duration>,
    animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
    graph_widget_id: iced_sugiyama::Id,
    auto_pan_enabled: bool,
    auto_zoom_enabled: bool,
    _scope: std::marker::PhantomData<Scope>,
}

impl<T, Scope> EjectedViewer<T, Scope>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    fn new(
        mode: ViewMode,
        theme: T,
        animation_duration: Option<Duration>,
        animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
        graph_widget_id: iced_sugiyama::Id,
    ) -> Self {
        let state = match &mode {
            ViewMode::Live { .. } => LiveState::Loading,
            ViewMode::Static { .. } => LiveState::Idle,
        };
        let theme = Arc::new(theme);
        let theme_state = theme.init();

        Self {
            mode,
            state,
            live_generation: 0,
            theme,
            theme_state,
            animation_duration,
            animation_easing,
            graph_widget_id,
            auto_pan_enabled: true,
            auto_zoom_enabled: true,
            _scope: std::marker::PhantomData,
        }
    }

    pub fn journey_name(&self) -> &str {
        match &self.mode {
            ViewMode::Static { journey_name, .. } | ViewMode::Live { journey_name, .. } => {
                journey_name
            }
        }
    }

    pub fn retry(&mut self) {
        if matches!(self.mode, ViewMode::Live { .. }) {
            self.live_generation = self.live_generation.saturating_add(1);
            self.state = LiveState::Loading;
        }
    }

    pub fn auto_viewport_enabled(&self) -> bool {
        self.auto_pan_enabled && self.auto_zoom_enabled
    }

    pub fn set_auto_viewport_enabled(&mut self, enabled: bool) {
        self.auto_pan_enabled = enabled;
        self.auto_zoom_enabled = enabled;
    }

    pub fn update(&mut self, message: EjectedViewerMessage) -> Task<EjectedViewerMessage> {
        match message {
            EjectedViewerMessage::LiveEvent(result) => {
                match result {
                    Ok(update) => {
                        let event_count =
                            VISION_LIVE_EVENT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                        let event_age_ms = current_unix_ms().saturating_sub(update.event_unix_ms);
                        if event_age_ms > VISION_STALE_EVENT_WARN_MS {
                            warn!(
                                journey = self.journey_name(),
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                "jungle-vision received stale live update"
                            );
                        } else if event_count % VISION_LIVE_EVENT_LOG_INTERVAL == 0 {
                            debug!(
                                journey = self.journey_name(),
                                event_count,
                                sequence_id = update.sequence_id,
                                event_age_ms,
                                "jungle-vision live event heartbeat"
                            );
                        }
                        return Task::done(EjectedViewerMessage::ApplyLiveEvent {
                            update,
                            received_unix_ms: current_unix_ms(),
                        });
                    }
                    Err(error) => {
                        self.state = LiveState::Error(error);
                    }
                }
                Task::none()
            }
            EjectedViewerMessage::HydrateLiveEvents(updates) => {
                let mut theme_tasks = Vec::with_capacity(updates.len());
                let data = match &mut self.state {
                    LiveState::Loaded(data) => data,
                    _ => {
                        self.state = LiveState::Loaded(LiveData::default());
                        match &mut self.state {
                            LiveState::Loaded(data) => data,
                            _ => unreachable!("state was set to loaded"),
                        }
                    }
                };
                let mut highlight_changed = false;
                for update in updates {
                    theme_tasks.push(
                        self.theme
                            .update(
                                &mut self.theme_state,
                                ViewerEvent::JourneyUpdate(update.clone()),
                            )
                            .map(EjectedViewerMessage::Theme),
                    );
                    highlight_changed |= data.apply_update(update);
                }
                let theme_task = Task::batch(theme_tasks);
                if highlight_changed {
                    iced_sugiyama::invalidate::<EjectedViewerMessage>(self.graph_widget_id.clone())
                        .chain(theme_task)
                } else {
                    theme_task
                }
            }
            EjectedViewerMessage::ApplyLiveEvent {
                update,
                received_unix_ms,
            } => {
                let apply_started_at = Instant::now();
                let now_unix_ms = current_unix_ms();
                let apply_queue_delay_ms = now_unix_ms.saturating_sub(received_unix_ms);
                let end_to_end_age_ms = now_unix_ms.saturating_sub(update.event_unix_ms);
                update_max_usize(
                    &VISION_MAX_APPLY_QUEUE_DELAY_MS,
                    usize::try_from(apply_queue_delay_ms.max(0)).unwrap_or(usize::MAX),
                );
                update_max_usize(
                    &VISION_MAX_END_TO_END_EVENT_AGE_MS,
                    usize::try_from(end_to_end_age_ms.max(0)).unwrap_or(usize::MAX),
                );
                let theme_task = self
                    .theme
                    .update(
                        &mut self.theme_state,
                        ViewerEvent::JourneyUpdate(update.clone()),
                    )
                    .map(EjectedViewerMessage::Theme);
                let data = match &mut self.state {
                    LiveState::Loaded(data) => data,
                    _ => {
                        self.state = LiveState::Loaded(LiveData::default());
                        match &mut self.state {
                            LiveState::Loaded(data) => data,
                            _ => unreachable!("state was set to loaded"),
                        }
                    }
                };
                let highlight_changed = data.apply_update(update);
                let apply_elapsed_ms = apply_started_at.elapsed().as_millis();
                let apply_count = VISION_APPLY_EVENT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                update_max_usize(
                    &VISION_MAX_APPLY_ELAPSED_MS,
                    usize::try_from(apply_elapsed_ms).unwrap_or(usize::MAX),
                );
                if apply_elapsed_ms > VISION_SLOW_APPLY_WARN_MS {
                    warn!(
                        journey = self.journey_name(),
                        apply_count,
                        apply_elapsed_ms,
                        apply_queue_delay_ms,
                        end_to_end_age_ms,
                        max_apply_queue_delay_ms =
                            VISION_MAX_APPLY_QUEUE_DELAY_MS.load(Ordering::Relaxed),
                        max_end_to_end_event_age_ms =
                            VISION_MAX_END_TO_END_EVENT_AGE_MS.load(Ordering::Relaxed),
                        max_apply_elapsed_ms = VISION_MAX_APPLY_ELAPSED_MS.load(Ordering::Relaxed),
                        "slow jungle-vision ApplyLiveEvent handling"
                    );
                } else if apply_queue_delay_ms > VISION_APPLY_QUEUE_DELAY_WARN_MS {
                    warn!(
                        journey = self.journey_name(),
                        apply_count,
                        apply_elapsed_ms,
                        apply_queue_delay_ms,
                        end_to_end_age_ms,
                        max_apply_queue_delay_ms =
                            VISION_MAX_APPLY_QUEUE_DELAY_MS.load(Ordering::Relaxed),
                        max_end_to_end_event_age_ms =
                            VISION_MAX_END_TO_END_EVENT_AGE_MS.load(Ordering::Relaxed),
                        max_apply_elapsed_ms = VISION_MAX_APPLY_ELAPSED_MS.load(Ordering::Relaxed),
                        "jungle-vision ApplyLiveEvent queueing delay is growing"
                    );
                } else if end_to_end_age_ms > VISION_END_TO_END_AGE_WARN_MS {
                    warn!(
                        journey = self.journey_name(),
                        apply_count,
                        apply_elapsed_ms,
                        apply_queue_delay_ms,
                        end_to_end_age_ms,
                        max_apply_queue_delay_ms =
                            VISION_MAX_APPLY_QUEUE_DELAY_MS.load(Ordering::Relaxed),
                        max_end_to_end_event_age_ms =
                            VISION_MAX_END_TO_END_EVENT_AGE_MS.load(Ordering::Relaxed),
                        max_apply_elapsed_ms = VISION_MAX_APPLY_ELAPSED_MS.load(Ordering::Relaxed),
                        "jungle-vision end-to-end event age is high at apply"
                    );
                } else if apply_count % VISION_LIVE_EVENT_LOG_INTERVAL == 0 {
                    debug!(
                        journey = self.journey_name(),
                        apply_count,
                        apply_elapsed_ms,
                        apply_queue_delay_ms,
                        end_to_end_age_ms,
                        max_apply_queue_delay_ms =
                            VISION_MAX_APPLY_QUEUE_DELAY_MS.load(Ordering::Relaxed),
                        max_end_to_end_event_age_ms =
                            VISION_MAX_END_TO_END_EVENT_AGE_MS.load(Ordering::Relaxed),
                        max_apply_elapsed_ms = VISION_MAX_APPLY_ELAPSED_MS.load(Ordering::Relaxed),
                        "jungle-vision apply heartbeat"
                    );
                }
                if highlight_changed {
                    iced_sugiyama::invalidate::<EjectedViewerMessage>(self.graph_widget_id.clone())
                        .chain(theme_task)
                } else {
                    theme_task
                }
            }
            EjectedViewerMessage::Theme(event) => {
                let theme_started_at = Instant::now();
                let theme_task = self
                    .theme
                    .update(&mut self.theme_state, event)
                    .map(EjectedViewerMessage::Theme);
                let theme_elapsed_ms = theme_started_at.elapsed().as_millis();
                if theme_elapsed_ms > VISION_SLOW_THEME_UPDATE_WARN_MS {
                    warn!(
                        journey = self.journey_name(),
                        theme_elapsed_ms, "slow jungle-vision theme update"
                    );
                }
                Task::batch(vec![
                    theme_task,
                    iced_sugiyama::force_review::<EjectedViewerMessage>(
                        self.graph_widget_id.clone(),
                    ),
                ])
            }
            EjectedViewerMessage::ViewportInteraction(interaction) => {
                match interaction {
                    ViewportInteraction::UserPanned => {
                        self.auto_pan_enabled = false;
                    }
                    ViewportInteraction::UserZoomed => {
                        self.auto_zoom_enabled = false;
                    }
                }
                Task::none()
            }
            EjectedViewerMessage::Retry => {
                self.retry();
                Task::none()
            }
        }
    }

    pub fn subscription(&self) -> Subscription<EjectedViewerMessage> {
        match &self.mode {
            ViewMode::Live {
                client, journey_id, ..
            } => Subscription::run_with(
                LiveSubscription {
                    client: client.clone(),
                    journey_id: *journey_id,
                    generation: self.live_generation,
                },
                live_updates_stream_for_panel,
            ),
            ViewMode::Static { .. } => Subscription::none(),
        }
    }

    pub fn view(&self) -> Element<'_, EjectedViewerMessage> {
        let model = match &self.mode {
            ViewMode::Static { model, .. } | ViewMode::Live { model, .. } => model,
        };
        let live_data = match (&self.mode, &self.state) {
            (ViewMode::Live { .. }, LiveState::Loaded(data)) => Some(data),
            _ => None,
        };

        graph_panel(
            model,
            live_data,
            self.theme.as_ref(),
            &self.theme_state,
            self.animation_duration,
            self.animation_easing,
            self.graph_widget_id.clone(),
            self.auto_pan_enabled,
            self.auto_zoom_enabled,
        )
        .map(|message| match message {
            Message::Theme(event) => EjectedViewerMessage::Theme(event),
            Message::LiveEvent(result) => EjectedViewerMessage::LiveEvent(result),
            Message::HydrateLiveEvents(updates) => EjectedViewerMessage::HydrateLiveEvents(updates),
            Message::ApplyLiveEvent(update) => EjectedViewerMessage::ApplyLiveEvent {
                update,
                received_unix_ms: current_unix_ms(),
            },
            Message::ViewportInteraction(interaction) => {
                EjectedViewerMessage::ViewportInteraction(interaction)
            }
            Message::Retry => EjectedViewerMessage::Retry,
            Message::AppStarted
            | Message::CaptureView
            | Message::ViewCaptured(_)
            | Message::ViewSaved(_) => EjectedViewerMessage::Retry,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DebugGraphNode {
    pub id: u32,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct DebugGraph {
    pub nodes: Vec<DebugGraphNode>,
    pub edges: Vec<(u32, u32)>,
    pub while_clusters: Vec<Vec<u32>>,
}

pub fn debug_graph_for_animal<A>() -> DebugGraph
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
{
    let ast = <A::Flow as JourneyAstSource>::journey_ast();
    let model = GraphModel::from_ast(ast);
    DebugGraph {
        nodes: model
            .nodes
            .iter()
            .map(|node| DebugGraphNode {
                id: node.id,
                label: node.label.clone(),
            })
            .collect(),
        edges: model.edges.clone(),
        while_clusters: model
            .clusters
            .iter()
            .map(|cluster| cluster.nodes.clone())
            .collect(),
    }
}

#[derive(Clone)]
enum ViewMode {
    Static {
        journey_name: String,
        model: GraphModel,
    },
    Live {
        journey_name: String,
        model: GraphModel,
        client: Arc<dyn JungleClient>,
        journey_id: Uuid,
    },
}

struct ViewerApp<T, Scope>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    mode: ViewMode,
    state: LiveState,
    live_generation: u64,
    capture: Option<CaptureConfig>,
    theme: Arc<T>,
    theme_state: T::State,
    animation_duration: Option<Duration>,
    animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
    graph_widget_id: iced_sugiyama::Id,
    auto_pan_enabled: bool,
    auto_zoom_enabled: bool,
    _scope: std::marker::PhantomData<Scope>,
}

#[derive(Clone)]
struct CaptureConfig {
    output_path: PathBuf,
    close_after_capture: bool,
}

#[derive(Clone)]
enum LiveState {
    Idle,
    Loading,
    Error(String),
    Loaded(LiveData),
}

#[derive(Debug, Clone, Default)]
struct LiveData {
    active_runtime_ids: BTreeSet<u32>,
    finished_runtime_ids: BTreeSet<u32>,
    failed_runtime_ids: BTreeSet<u32>,
    runtime_update_sequence: HashMap<u32, usize>,
    latest_event_count: usize,
}

#[derive(Debug, Clone)]
enum Message {
    AppStarted,
    LiveEvent(Result<JourneyUpdateEvent, String>),
    HydrateLiveEvents(Vec<JourneyUpdateEvent>),
    ApplyLiveEvent(JourneyUpdateEvent),
    Theme(ViewerEvent<()>),
    ViewportInteraction(ViewportInteraction),
    Retry,
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

impl<T, Scope> ViewerApp<T, Scope>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    fn new(
        mode: ViewMode,
        capture: Option<CaptureConfig>,
        theme: Arc<T>,
        animation_duration: Option<Duration>,
        animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
    ) -> (Self, Task<Message>) {
        let state = match &mode {
            ViewMode::Live { .. } => LiveState::Loading,
            ViewMode::Static { .. } => LiveState::Idle,
        };
        let theme_state = theme.init();

        (
            Self {
                mode,
                state,
                live_generation: 0,
                capture,
                theme,
                theme_state,
                animation_duration,
                animation_easing,
                graph_widget_id: iced_sugiyama::Id::new(GRAPH_WIDGET_ID),
                auto_pan_enabled: true,
                auto_zoom_enabled: true,
                _scope: std::marker::PhantomData,
            },
            Task::done(Message::AppStarted),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppStarted => {
                if self.capture.is_some() {
                    Task::done(Message::CaptureView)
                } else {
                    Task::none()
                }
            }
            Message::LiveEvent(result) => {
                match result {
                    Ok(update) => {
                        return Task::done(Message::ApplyLiveEvent(update));
                    }
                    Err(error) => {
                        self.state = LiveState::Error(error);
                    }
                }
                Task::none()
            }
            Message::HydrateLiveEvents(updates) => {
                let mut theme_tasks = Vec::with_capacity(updates.len());
                let data = match &mut self.state {
                    LiveState::Loaded(data) => data,
                    _ => {
                        self.state = LiveState::Loaded(LiveData::default());
                        match &mut self.state {
                            LiveState::Loaded(data) => data,
                            _ => unreachable!("state was set to loaded"),
                        }
                    }
                };
                let mut highlight_changed = false;
                for update in updates {
                    theme_tasks.push(
                        self.theme
                            .update(
                                &mut self.theme_state,
                                ViewerEvent::JourneyUpdate(update.clone()),
                            )
                            .map(Message::Theme),
                    );
                    highlight_changed |= data.apply_update(update);
                }
                let theme_task = Task::batch(theme_tasks);
                if highlight_changed {
                    iced_sugiyama::invalidate::<Message>(self.graph_widget_id.clone())
                        .chain(theme_task)
                } else {
                    theme_task
                }
            }
            Message::ApplyLiveEvent(update) => {
                let theme_task = self
                    .theme
                    .update(
                        &mut self.theme_state,
                        ViewerEvent::JourneyUpdate(update.clone()),
                    )
                    .map(Message::Theme);
                let data = match &mut self.state {
                    LiveState::Loaded(data) => data,
                    _ => {
                        self.state = LiveState::Loaded(LiveData::default());
                        match &mut self.state {
                            LiveState::Loaded(data) => data,
                            _ => unreachable!("state was set to loaded"),
                        }
                    }
                };
                let highlight_changed = data.apply_update(update);
                if highlight_changed {
                    iced_sugiyama::invalidate::<Message>(self.graph_widget_id.clone())
                        .chain(theme_task)
                } else {
                    theme_task
                }
            }
            Message::Theme(event) => {
                let theme_task = self
                    .theme
                    .update(&mut self.theme_state, event)
                    .map(Message::Theme);
                Task::batch(vec![
                    theme_task,
                    iced_sugiyama::force_review::<Message>(self.graph_widget_id.clone()),
                ])
            }
            Message::ViewportInteraction(interaction) => {
                match interaction {
                    ViewportInteraction::UserPanned => {
                        self.auto_pan_enabled = false;
                    }
                    ViewportInteraction::UserZoomed => {
                        self.auto_zoom_enabled = false;
                    }
                }
                Task::none()
            }
            Message::Retry => match &self.mode {
                ViewMode::Live { .. } => {
                    self.live_generation = self.live_generation.saturating_add(1);
                    self.state = LiveState::Loading;
                    Task::none()
                }
                ViewMode::Static { .. } => Task::none(),
            },
            Message::CaptureView => window::latest().then(|id| match id {
                Some(id) => window::screenshot(id).map(Message::ViewCaptured),
                None => Task::none(),
            }),
            Message::ViewCaptured(screenshot) => {
                let Some(capture) = self.capture.clone() else {
                    return Task::none();
                };
                Task::perform(
                    save_screenshot_png(capture.output_path, screenshot),
                    Message::ViewSaved,
                )
            }
            Message::ViewSaved(result) => {
                let close_after_capture = self
                    .capture
                    .as_ref()
                    .map(|capture| capture.close_after_capture)
                    .unwrap_or(false);
                match result {
                    Ok(path) => println!("Wrote {}", path.display()),
                    Err(error) => eprintln!("Failed to save screenshot: {error}"),
                }
                if close_after_capture {
                    close_latest_window()
                } else {
                    Task::none()
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        match &self.mode {
            ViewMode::Live {
                client, journey_id, ..
            } => Subscription::run_with(
                LiveSubscription {
                    client: client.clone(),
                    journey_id: *journey_id,
                    generation: self.live_generation,
                },
                live_updates_stream,
            ),
            ViewMode::Static { .. } => Subscription::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let (journey_name, model) = match &self.mode {
            ViewMode::Static {
                journey_name,
                model,
            }
            | ViewMode::Live {
                journey_name,
                model,
                ..
            } => (journey_name, model),
        };

        let live_data = match (&self.mode, &self.state) {
            (ViewMode::Live { .. }, LiveState::Loaded(data)) => Some(data),
            _ => None,
        };
        let body = row![
            sidebar(journey_name, model, &self.state),
            graph_panel(
                model,
                live_data,
                self.theme.as_ref(),
                &self.theme_state,
                self.animation_duration,
                self.animation_easing,
                self.graph_widget_id.clone(),
                self.auto_pan_enabled,
                self.auto_zoom_enabled,
            )
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .spacing(0);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background)
            .into()
    }
}

#[derive(Clone)]
struct LiveSubscription {
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
    generation: u64,
}

struct PreparedInitialLiveUpdates {
    initial_updates: Vec<JourneyUpdateEvent>,
    hydrate_initial: bool,
    subscription: Option<JourneyUpdateSubscription>,
    terminal_error: Option<String>,
}

impl Hash for LiveSubscription {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.journey_id.hash(state);
        self.generation.hash(state);
    }
}

fn live_updates_stream(config: &LiveSubscription) -> impl Stream<Item = Message> {
    let client = config.client.clone();
    let journey_id = config.journey_id;
    futures::stream::once(async move {
        let prepared = prepare_initial_live_updates(client, journey_id).await;
        let mut stream: Pin<Box<dyn Stream<Item = Message> + Send>> =
            Box::pin(futures::stream::empty());
        if !prepared.initial_updates.is_empty() {
            if prepared.hydrate_initial {
                let updates = prepared.initial_updates;
                stream = Box::pin(stream.chain(futures::stream::once(async move {
                    Message::HydrateLiveEvents(updates)
                })));
            } else {
                let update = prepared
                    .initial_updates
                    .into_iter()
                    .next()
                    .expect("non-empty initial updates should contain a first update");
                stream = Box::pin(stream.chain(futures::stream::once(async move {
                    Message::LiveEvent(Ok(update))
                })));
            }
        }
        if let Some(error) = prepared.terminal_error {
            stream = Box::pin(stream.chain(futures::stream::once(async move {
                Message::LiveEvent(Err(error))
            })));
        } else if let Some(subscription) = prepared.subscription {
            stream = Box::pin(stream.chain(
                subscription.map(|event| Message::LiveEvent(event.map_err(|err| err.to_string()))),
            ));
        }
        stream
    })
    .flatten()
}

fn live_updates_stream_for_panel(
    config: &LiveSubscription,
) -> impl Stream<Item = EjectedViewerMessage> {
    let client = config.client.clone();
    let journey_id = config.journey_id;
    futures::stream::once(async move {
        let prepared = prepare_initial_live_updates(client, journey_id).await;
        let mut stream: Pin<Box<dyn Stream<Item = EjectedViewerMessage> + Send>> =
            Box::pin(futures::stream::empty());
        if !prepared.initial_updates.is_empty() {
            if prepared.hydrate_initial {
                let updates = prepared.initial_updates;
                stream = Box::pin(stream.chain(futures::stream::once(async move {
                    EjectedViewerMessage::HydrateLiveEvents(updates)
                })));
            } else {
                let update = prepared
                    .initial_updates
                    .into_iter()
                    .next()
                    .expect("non-empty initial updates should contain a first update");
                stream = Box::pin(stream.chain(futures::stream::once(async move {
                    EjectedViewerMessage::LiveEvent(Ok(update))
                })));
            }
        }
        if let Some(error) = prepared.terminal_error {
            stream = Box::pin(stream.chain(futures::stream::once(async move {
                EjectedViewerMessage::LiveEvent(Err(error))
            })));
        } else if let Some(subscription) = prepared.subscription {
            stream = Box::pin(stream.chain(subscription.map(|event| {
                EjectedViewerMessage::LiveEvent(event.map_err(|err| err.to_string()))
            })));
        }
        stream
    })
    .flatten()
}

async fn prepare_initial_live_updates(
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
) -> PreparedInitialLiveUpdates {
    let mut subscription = match client.subscribe_step_updates(journey_id, None).await {
        Ok(subscription) => subscription,
        Err(err) => {
            return PreparedInitialLiveUpdates {
                initial_updates: Vec::new(),
                hydrate_initial: false,
                subscription: None,
                terminal_error: Some(format!("live update stream setup failed: {err}")),
            };
        }
    };

    let Some(first_result) = subscription.next().await else {
        return PreparedInitialLiveUpdates {
            initial_updates: Vec::new(),
            hydrate_initial: false,
            subscription: None,
            terminal_error: None,
        };
    };
    let first_update = match first_result {
        Ok(update) => update,
        Err(err) => {
            return PreparedInitialLiveUpdates {
                initial_updates: Vec::new(),
                hydrate_initial: false,
                subscription: None,
                terminal_error: Some(err.to_string()),
            };
        }
    };

    let first_event_age_ms = current_unix_ms().saturating_sub(first_update.event_unix_ms);
    if first_event_age_ms <= VISION_STALE_EVENT_WARN_MS {
        return PreparedInitialLiveUpdates {
            initial_updates: vec![first_update],
            hydrate_initial: false,
            subscription: Some(subscription),
            terminal_error: None,
        };
    }

    let mut initial_updates = vec![first_update];
    loop {
        match timeout(INITIAL_LIVE_BATCH_WINDOW, subscription.next()).await {
            Ok(Some(Ok(update))) => initial_updates.push(update),
            Ok(Some(Err(err))) => {
                return PreparedInitialLiveUpdates {
                    initial_updates,
                    hydrate_initial: true,
                    subscription: None,
                    terminal_error: Some(err.to_string()),
                };
            }
            Ok(None) | Err(_) => {
                return PreparedInitialLiveUpdates {
                    initial_updates,
                    hydrate_initial: true,
                    subscription: Some(subscription),
                    terminal_error: None,
                };
            }
        }
    }
}

fn close_latest_window() -> Task<Message> {
    window::latest().then(|id| match id {
        Some(id) => window::close(id),
        None => Task::none(),
    })
}

async fn save_screenshot_png(path: PathBuf, screenshot: Screenshot) -> Result<PathBuf, String> {
    let image = image::RgbaImage::from_raw(
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
    .ok_or_else(|| "failed to build image buffer from screenshot".to_string())?;

    image
        .save(&path)
        .map_err(|error| format!("failed to save screenshot to {}: {error}", path.display()))?;
    Ok(path)
}

impl LiveData {
    fn apply_update(&mut self, update: JourneyUpdateEvent) -> bool {
        let mut highlight_changed = false;
        let sequence = update.sequence_id as usize;
        self.latest_event_count = sequence;
        match update.event {
            RunnerUpdateOut::EffectInput { node_id, .. } => {
                highlight_changed |= self.finished_runtime_ids.remove(&node_id);
                highlight_changed |= self.failed_runtime_ids.remove(&node_id);
                highlight_changed |= self.active_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => {
                highlight_changed |= self.active_runtime_ids.remove(&node_id);
                highlight_changed |= self.finished_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::EffectFailureOutput { node_id, .. } => {
                highlight_changed |= self.active_runtime_ids.remove(&node_id);
                highlight_changed |= self.failed_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
        highlight_changed
    }
}

fn runtime_state_for_live_data(
    live: &LiveData,
    runtime_id: u32,
    runtime_sequence_floors: &HashMap<u32, usize>,
) -> RuntimeState {
    let state = if live.failed_runtime_ids.contains(&runtime_id) {
        RuntimeState::Failed
    } else if live.active_runtime_ids.contains(&runtime_id) {
        RuntimeState::Running
    } else if live.finished_runtime_ids.contains(&runtime_id) {
        RuntimeState::Completed
    } else {
        RuntimeState::Pending
    };

    let Some(sequence) = live.runtime_update_sequence.get(&runtime_id).copied() else {
        return state;
    };
    let Some(floor) = runtime_sequence_floors.get(&runtime_id).copied() else {
        return state;
    };
    if sequence < floor {
        RuntimeState::Pending
    } else {
        state
    }
}

#[cfg(test)]
fn infer_condition_runtime_state(live: &LiveData, successor_runtime_ids: &[u32]) -> RuntimeState {
    infer_condition_runtime_state_with_runtime_floors(live, successor_runtime_ids, &HashMap::new())
}

fn infer_condition_runtime_state_with_runtime_floors(
    live: &LiveData,
    successor_runtime_ids: &[u32],
    runtime_sequence_floors: &HashMap<u32, usize>,
) -> RuntimeState {
    let mut newest: Option<(usize, RuntimeState)> = None;

    for runtime_id in successor_runtime_ids {
        let Some(sequence) = live.runtime_update_sequence.get(runtime_id).copied() else {
            continue;
        };
        if newest
            .map(|(best_sequence, _)| sequence > best_sequence)
            .unwrap_or(true)
        {
            newest = Some((
                sequence,
                runtime_state_for_live_data(live, *runtime_id, runtime_sequence_floors),
            ));
        }
    }

    newest
        .map(|(_, state)| state)
        .unwrap_or(RuntimeState::Pending)
}

fn current_unix_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_millis()).unwrap_or(i64::MAX),
        Err(_) => 0,
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

fn node_phase_for_display(
    live_data: Option<&LiveData>,
    display_id: u32,
    runtime_id: Option<u32>,
    proxy_runtime_ids: &[u32],
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> Phase<RuntimeState> {
    node_phase_for_display_with_runtime_floors(
        live_data,
        display_id,
        runtime_id,
        proxy_runtime_ids,
        condition_successor_runtime_ids,
        &HashMap::new(),
    )
}

fn node_phase_for_display_with_runtime_floors(
    live_data: Option<&LiveData>,
    display_id: u32,
    runtime_id: Option<u32>,
    proxy_runtime_ids: &[u32],
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
    runtime_sequence_floors: &HashMap<u32, usize>,
) -> Phase<RuntimeState> {
    let Some(live) = live_data else {
        return Phase::Static;
    };

    let state = match runtime_id {
        Some(id) => runtime_state_for_live_data(live, id, runtime_sequence_floors),
        None => condition_successor_runtime_ids
            .get(&display_id)
            .map(|successors| {
                infer_condition_runtime_state_with_runtime_floors(
                    live,
                    successors,
                    runtime_sequence_floors,
                )
            })
            .unwrap_or(RuntimeState::Pending),
    };

    let mut newest = runtime_id.and_then(|id| {
        live.runtime_update_sequence
            .get(&id)
            .copied()
            .map(|sequence| (sequence, state))
    });
    for proxy_runtime_id in proxy_runtime_ids {
        let Some(sequence) = live.runtime_update_sequence.get(proxy_runtime_id).copied() else {
            continue;
        };
        let proxy_state =
            runtime_state_for_live_data(live, *proxy_runtime_id, runtime_sequence_floors);
        if !matches!(proxy_state, RuntimeState::Running | RuntimeState::Failed) {
            continue;
        }
        if newest
            .map(|(best_sequence, _)| sequence > best_sequence)
            .unwrap_or(true)
        {
            newest = Some((sequence, proxy_state));
        }
    }

    Phase::Live(newest.map(|(_, inferred)| inferred).unwrap_or(state))
}

fn repaired_live_states_for_display(
    model: &GraphModel,
    live_data: Option<&LiveData>,
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> HashMap<u32, RuntimeState> {
    let Some(live) = live_data else {
        return HashMap::new();
    };
    let runtime_sequence_floors = runtime_sequence_floors_for_display(model, live);

    let mut states = HashMap::<u32, RuntimeState>::new();
    let mut latest_sequence_by_display_id = HashMap::<u32, usize>::new();
    for node in &model.nodes {
        let mut latest_sequence = node
            .runtime_node_id
            .and_then(|runtime_id| live.runtime_update_sequence.get(&runtime_id).copied());
        for proxy_runtime_id in &node.proxy_runtime_ids {
            let Some(sequence) = live.runtime_update_sequence.get(proxy_runtime_id).copied() else {
                continue;
            };
            let proxy_state =
                runtime_state_for_live_data(live, *proxy_runtime_id, &runtime_sequence_floors);
            if !matches!(proxy_state, RuntimeState::Running | RuntimeState::Failed) {
                continue;
            }
            if latest_sequence
                .map(|current| sequence > current)
                .unwrap_or(true)
            {
                latest_sequence = Some(sequence);
            }
        }
        if let Some(sequence) = latest_sequence {
            latest_sequence_by_display_id.insert(node.id, sequence);
        }

        let phase = node_phase_for_display_with_runtime_floors(
            Some(live),
            node.id,
            node.runtime_node_id,
            &node.proxy_runtime_ids,
            condition_successor_runtime_ids,
            &runtime_sequence_floors,
        );
        let state = match phase {
            Phase::Live(state) => state,
            Phase::Static => RuntimeState::Pending,
        };
        states.insert(node.id, state);
    }

    let mut loop_back_edges = HashSet::<(u32, u32)>::new();
    for cluster in &model.cluster_info {
        if !matches!(cluster.kind, ClusterKind::While) {
            continue;
        }
        let root_nodes = cluster.root_nodes.iter().copied().collect::<HashSet<_>>();
        let member_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        for (from, to) in &model.edges {
            if member_nodes.contains(from) && root_nodes.contains(to) {
                loop_back_edges.insert((*from, *to));
            }
        }
    }

    let mut incoming = HashMap::<u32, Vec<u32>>::new();
    for (from, to) in &model.edges {
        if loop_back_edges.contains(&(*from, *to)) {
            continue;
        }
        incoming.entry(*to).or_default().push(*from);
    }

    let mut queue = std::collections::VecDeque::<u32>::new();
    let mut queued = HashSet::<u32>::new();
    for (node_id, state) in &states {
        if matches!(state, RuntimeState::Running | RuntimeState::Completed) {
            queue.push_back(*node_id);
            queued.insert(*node_id);
        }
    }

    while let Some(node_id) = queue.pop_front() {
        queued.remove(&node_id);
        let Some(predecessors) = incoming.get(&node_id) else {
            continue;
        };
        for predecessor in predecessors {
            let predecessor_state = states
                .get(predecessor)
                .copied()
                .unwrap_or(RuntimeState::Pending);
            if !matches!(
                predecessor_state,
                RuntimeState::Pending | RuntimeState::Running
            ) {
                continue;
            }
            if matches!(predecessor_state, RuntimeState::Running) {
                let can_promote_running = match (
                    latest_sequence_by_display_id.get(&node_id).copied(),
                    latest_sequence_by_display_id.get(predecessor).copied(),
                ) {
                    (Some(trigger_sequence), Some(predecessor_sequence)) => {
                        trigger_sequence.saturating_sub(predecessor_sequence)
                            >= RUNNING_REPAIR_PROMOTION_SEQUENCE_GAP
                    }
                    _ => true,
                };
                if !can_promote_running {
                    continue;
                }
            }
            states.insert(*predecessor, RuntimeState::Completed);
            if queued.insert(*predecessor) {
                queue.push_back(*predecessor);
            }
        }
    }

    states
}

fn runtime_sequence_floors_for_display(model: &GraphModel, live: &LiveData) -> HashMap<u32, usize> {
    let mut floors = HashMap::<u32, usize>::new();
    for (index, cluster) in model.cluster_info.iter().enumerate() {
        if !matches!(cluster.kind, ClusterKind::While) {
            continue;
        }

        let iteration_start_sequence = model.derived.cluster_entry_runtime_ids[index]
            .iter()
            .filter_map(|runtime_id| live.runtime_update_sequence.get(runtime_id).copied())
            .max();
        let Some(iteration_start_sequence) = iteration_start_sequence else {
            continue;
        };

        for runtime_id in &model.derived.cluster_member_runtime_ids[index] {
            floors
                .entry(*runtime_id)
                .and_modify(|current| *current = (*current).max(iteration_start_sequence))
                .or_insert(iteration_start_sequence);
        }
    }

    floors
}

fn live_states_for_display(
    model: &GraphModel,
    live_data: Option<&LiveData>,
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> HashMap<u32, RuntimeState> {
    repaired_live_states_for_display(model, live_data, condition_successor_runtime_ids)
}

fn node_phase_for_display_with_repairs(
    live_data: Option<&LiveData>,
    display_id: u32,
    runtime_id: Option<u32>,
    proxy_runtime_ids: &[u32],
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
    repaired_live_states: &HashMap<u32, RuntimeState>,
) -> Phase<RuntimeState> {
    if let Some(repaired) = repaired_live_states.get(&display_id).copied() {
        return Phase::Live(repaired);
    }
    node_phase_for_display(
        live_data,
        display_id,
        runtime_id,
        proxy_runtime_ids,
        condition_successor_runtime_ids,
    )
}

fn cluster_live_from_repaired_states(
    cluster: &ClusterInfo,
    repaired_live_states: &HashMap<u32, RuntimeState>,
) -> ClusterLive {
    let mut has_running = false;
    let mut has_failed = false;
    let mut has_completed = false;

    for node_id in &cluster.nodes {
        let state = repaired_live_states
            .get(node_id)
            .copied()
            .unwrap_or(RuntimeState::Pending);
        match state {
            RuntimeState::Pending => {}
            RuntimeState::Running => {
                has_running = true;
            }
            RuntimeState::Completed => {
                has_completed = true;
            }
            RuntimeState::Failed => {
                has_failed = true;
            }
        }
    }

    ClusterLive {
        has_running,
        has_failed,
        has_completed,
    }
}

fn cluster_phase_for_display(
    live_data: Option<&LiveData>,
    cluster: &ClusterInfo,
    repaired_live_states: &HashMap<u32, RuntimeState>,
) -> Phase<ClusterLive> {
    if live_data.is_some() {
        Phase::Live(cluster_live_from_repaired_states(
            cluster,
            repaired_live_states,
        ))
    } else {
        Phase::Static
    }
}

fn sidebar<'a>(
    journey_name: &'a str,
    model: &'a GraphModel,
    state: &'a LiveState,
) -> Element<'a, Message> {
    let status_text = match state {
        LiveState::Idle => "idle".to_string(),
        LiveState::Loading => "loading live history...".to_string(),
        LiveState::Error(error) => format!("live update failed: {error}"),
        LiveState::Loaded(data) => format!(
            "events: {}  active: {}  done: {}  failed: {}",
            data.latest_event_count,
            data.active_runtime_ids.len(),
            data.finished_runtime_ids.len(),
            data.failed_runtime_ids.len()
        ),
    };

    let mut info = column![
        text("Jungle Viewer")
            .size(26)
            .color(jungle_accent_bright())
            .font(Font::with_name("Iosevka")),
        text(journey_name)
            .size(14)
            .color(jungle_text_muted())
            .font(Font::with_name("Iosevka")),
        Space::new().height(10),
        text(format!("nodes: {}", model.nodes.len()))
            .size(13)
            .color(jungle_text_base()),
        text(format!("edges: {}", model.edges.len()))
            .size(13)
            .color(jungle_text_base()),
        text(format!("clusters: {}", model.clusters.len()))
            .size(13)
            .color(jungle_text_base()),
        Space::new().height(10),
        text(status_text).size(12).color(jungle_text_muted()),
    ]
    .spacing(2);

    if matches!(state, LiveState::Error(_)) {
        info = info.push(
            button(text("retry").size(12).color(jungle_text_base()))
                .style(sidebar_button)
                .on_press(Message::Retry),
        );
    }

    let legend = column![
        text("Legend").size(14).color(jungle_text_base()),
        text("Step: effect request node")
            .size(12)
            .color(jungle_text_muted()),
        text("Conditional: branch fanout")
            .size(12)
            .color(jungle_text_muted()),
        text("While: clustered body + condition label")
            .size(12)
            .color(jungle_text_muted()),
        text("Transparent: clustered boundary label")
            .size(12)
            .color(jungle_text_muted()),
        text("Green glow: completed in live journey")
            .size(12)
            .color(jungle_text_muted()),
        text("Yellow glow: active in live journey")
            .size(12)
            .color(jungle_text_muted()),
        text("Red glow: failed in live journey")
            .size(12)
            .color(jungle_text_muted()),
    ]
    .spacing(2);

    container(column![info, Space::new().height(16), legend].spacing(0))
        .width(320)
        .height(Length::Fill)
        .padding(16)
        .style(sidebar_style)
        .into()
}

fn graph_panel<'a, T, Scope>(
    model: &'a GraphModel,
    live_data: Option<&'a LiveData>,
    theme: &'a T,
    theme_state: &'a T::State,
    animation_duration: Option<Duration>,
    animation_easing: Option<&'static iced_sugiyama::motion::easing::Easing>,
    graph_widget_id: iced_sugiyama::Id,
    auto_pan_enabled: bool,
    auto_zoom_enabled: bool,
) -> Element<'a, Message>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisibleOwner {
        Node(u32),
        Cluster(usize),
    }
    let condition_successor_runtime_ids = &model.derived.condition_successor_runtime_ids;
    let cluster_member_runtime_ids = &model.derived.cluster_member_runtime_ids;
    let cluster_successor_runtime_ids = &model.derived.cluster_successor_runtime_ids;
    let cluster_entry_runtime_ids = &model.derived.cluster_entry_runtime_ids;
    let memberships = &model.derived.memberships;
    let runtime_by_display_id = &model.derived.runtime_by_display_id;
    let proxy_runtime_ids_by_display_id = &model.derived.proxy_runtime_ids_by_display_id;
    let display_live_states = Arc::new(live_states_for_display(
        model,
        live_data,
        condition_successor_runtime_ids,
    ));

    let mut collapsed_clusters = HashSet::<usize>::new();
    for (index, cluster) in model.cluster_info.iter().enumerate() {
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: index,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: cluster
                .parent
                .and_then(|parent| model.cluster_info.get(parent).map(|info| info.id)),
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &cluster_entry_runtime_ids[index],
            member_runtime_ids: &cluster_member_runtime_ids[index],
            successor_runtime_ids: &cluster_successor_runtime_ids[index],
            phase: cluster_phase_for_display(live_data, cluster, display_live_states.as_ref()),
        };
        if matches!(
            theme.view_cluster(theme_state, &cx),
            ClusterView::Collapsed { .. }
        ) {
            collapsed_clusters.insert(index);
        }
    }

    let cluster_hidden_by_collapsed_ancestor = |cluster_index: usize| -> bool {
        let mut parent = model.cluster_info[cluster_index].parent;
        while let Some(parent_index) = parent {
            if collapsed_clusters.contains(&parent_index) {
                return true;
            }
            parent = model.cluster_info[parent_index].parent;
        }
        false
    };

    let owner_for_node = |node_id: u32| -> VisibleOwner {
        if let Some(candidates) = memberships.get(&node_id) {
            for (_, index) in candidates {
                if collapsed_clusters.contains(index) {
                    return VisibleOwner::Cluster(*index);
                }
            }
        }
        VisibleOwner::Node(node_id)
    };

    let owner_to_display = |owner: VisibleOwner| -> Option<u32> {
        match owner {
            VisibleOwner::Node(node_id) => Some(node_id),
            VisibleOwner::Cluster(index) => model.cluster_node_id(index),
        }
    };

    let mut visible_ids = BTreeSet::new();
    let mut visible_real_nodes = HashSet::<u32>::new();
    let mut collapsed_cluster_by_display = HashMap::<u32, usize>::new();
    let mut node_sizes = HashMap::<u32, (f64, f64)>::new();

    for node in &model.nodes {
        let owner = owner_for_node(node.id);
        if owner != VisibleOwner::Node(node.id) {
            continue;
        }
        let phase = node_phase_for_display_with_repairs(
            live_data,
            node.id,
            node.runtime_node_id,
            &node.proxy_runtime_ids,
            &condition_successor_runtime_ids,
            display_live_states.as_ref(),
        );
        let step_ctx = StepViewCtx {
            display_id: node.id,
            runtime_id: node.runtime_node_id,
            proxy_runtime_ids: &node.proxy_runtime_ids,
            successor_runtime_ids: condition_successor_runtime_ids
                .get(&node.id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            kind: node.kind(),
            label: &node.label,
            metadata: node.metadata.as_deref(),
            phase,
        };
        let (element, size) = theme.view_step(theme_state, &step_ctx);
        let _ = element;
        visible_ids.insert(node.id);
        visible_real_nodes.insert(node.id);
        node_sizes.insert(node.id, size);
    }

    for (index, cluster) in model.cluster_info.iter().enumerate() {
        if !collapsed_clusters.contains(&index) {
            continue;
        }
        if cluster_hidden_by_collapsed_ancestor(index) {
            continue;
        }
        let Some(display_id) = model.cluster_node_id(index) else {
            continue;
        };
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: index,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: cluster
                .parent
                .and_then(|parent| model.cluster_info.get(parent).map(|info| info.id)),
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &cluster_entry_runtime_ids[index],
            member_runtime_ids: &cluster_member_runtime_ids[index],
            successor_runtime_ids: &cluster_successor_runtime_ids[index],
            phase: cluster_phase_for_display(live_data, cluster, display_live_states.as_ref()),
        };
        if let ClusterView::Collapsed { element, size } = theme.view_cluster(theme_state, &cx) {
            let _ = element;
            visible_ids.insert(display_id);
            collapsed_cluster_by_display.insert(display_id, index);
            node_sizes.insert(display_id, size);
        }
    }

    let mut edges = Vec::<(u32, u32)>::new();
    let mut edge_set = HashSet::<(u32, u32)>::new();
    for (from, to) in &model.edges {
        let from_display = owner_to_display(owner_for_node(*from));
        let to_display = owner_to_display(owner_for_node(*to));
        let (Some(from_display), Some(to_display)) = (from_display, to_display) else {
            continue;
        };
        if from_display == to_display {
            continue;
        }
        if edge_set.insert((from_display, to_display)) {
            edges.push((from_display, to_display));
        }
    }

    let nodes = visible_ids.into_iter().collect::<Vec<_>>();
    let graph = Graph::new(nodes.clone(), edges.clone());

    let mut visible_clusters = Vec::<Cluster>::new();
    let mut visible_cluster_source_indices = Vec::<usize>::new();
    let mut visible_cluster_fills = Vec::<Color>::new();
    let mut visible_cluster_index_by_source = HashMap::<usize, usize>::new();
    for (source_index, cluster) in model.cluster_info.iter().enumerate() {
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: source_index,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: cluster
                .parent
                .and_then(|parent| model.cluster_info.get(parent).map(|info| info.id)),
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &cluster_entry_runtime_ids[source_index],
            member_runtime_ids: &cluster_member_runtime_ids[source_index],
            successor_runtime_ids: &cluster_successor_runtime_ids[source_index],
            phase: cluster_phase_for_display(live_data, cluster, display_live_states.as_ref()),
        };
        let ClusterView::Expanded { overlay, fill } = theme.view_cluster(theme_state, &cx) else {
            continue;
        };
        let member_nodes = cluster
            .nodes
            .iter()
            .copied()
            .filter(|node_id| matches!(owner_for_node(*node_id), VisibleOwner::Node(id) if id == *node_id))
            .collect::<Vec<_>>();
        if member_nodes.is_empty() {
            continue;
        }
        let mut spec = Cluster::new(member_nodes).padding(24.0);
        if let Some(parent_source) = cluster.parent {
            if let Some(parent_visible) =
                visible_cluster_index_by_source.get(&parent_source).copied()
            {
                spec = spec.parent(parent_visible);
            }
        }
        let visible_index = visible_clusters.len();
        visible_clusters.push(spec);
        visible_cluster_fills.push(fill);
        let _ = overlay;
        visible_cluster_source_indices.push(source_index);
        visible_cluster_index_by_source.insert(source_index, visible_index);
    }

    set_cluster_fill_colors(visible_cluster_fills);

    let default_edge_style = EdgeStyle {
        width: 1.6,
        start: Color::from_rgb8(64, 169, 104),
        end: Color::from_rgb8(40, 104, 67),
    };

    let graph_widget = {
        let node_map = &model.node_map;
        let cluster_info_for_nodes = &model.cluster_info;
        let cluster_info_for_clusters = &model.cluster_info;
        let collapsed_display_map = collapsed_cluster_by_display.clone();
        let visible_nodes = visible_real_nodes.clone();
        let sizes_for_view = node_sizes.clone();
        let visible_cluster_sources = visible_cluster_source_indices.clone();
        let cluster_member_runtime_ids_for_nodes = cluster_member_runtime_ids;
        let cluster_successor_runtime_ids_for_nodes = cluster_successor_runtime_ids;
        let cluster_entry_runtime_ids_for_nodes = cluster_entry_runtime_ids;
        let runtime_ids_for_edge_colors = runtime_by_display_id;
        let runtime_ids_for_edge_strokes = runtime_by_display_id;
        let proxy_runtime_ids_for_edge_colors = proxy_runtime_ids_by_display_id;
        let proxy_runtime_ids_for_edge_strokes = proxy_runtime_ids_by_display_id;
        let condition_successors_for_nodes = condition_successor_runtime_ids;
        let condition_successors_for_edge_colors = condition_successor_runtime_ids;
        let condition_successors_for_edge_strokes = condition_successor_runtime_ids;
        let display_live_states_for_nodes = display_live_states.clone();
        let display_live_states_for_edge_colors = display_live_states.clone();
        let display_live_states_for_edge_strokes = display_live_states.clone();
        let display_live_states_for_cluster_chips = display_live_states.clone();
        let display_live_states_for_cluster_overlays = display_live_states.clone();
        let mut widget = Sugiyama::<Message, iced::Theme, iced::Renderer>::new(
            std::borrow::Cow::Owned(graph.clone()),
            move |node_id| {
                if visible_nodes.contains(&node_id) {
                    if let Some(node) = node_map.get(&node_id) {
                        let phase = node_phase_for_display_with_repairs(
                            live_data,
                            node.id,
                            node.runtime_node_id,
                            &node.proxy_runtime_ids,
                            &condition_successors_for_nodes,
                            display_live_states_for_nodes.as_ref(),
                        );
                        let step_ctx = StepViewCtx {
                            display_id: node.id,
                            runtime_id: node.runtime_node_id,
                            proxy_runtime_ids: &node.proxy_runtime_ids,
                            successor_runtime_ids: condition_successors_for_nodes
                                .get(&node.id)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            kind: node.kind(),
                            label: &node.label,
                            metadata: node.metadata.as_deref(),
                            phase,
                        };
                        let (element, _size) = theme.view_step(theme_state, &step_ctx);
                        return element.map(|_event| Message::Theme(ViewerEvent::Message(())));
                    }
                }
                if let Some(cluster_index) = collapsed_display_map.get(&node_id).copied() {
                    if let Some(cluster) = cluster_info_for_nodes.get(cluster_index) {
                        let cx = ClusterViewCtx {
                            cluster_id: cluster.id,
                            cluster_index,
                            kind: cluster.kind,
                            label: &cluster.label,
                            metadata: cluster.metadata.as_deref(),
                            parent_cluster_id: cluster.parent.and_then(|parent| {
                                cluster_info_for_nodes.get(parent).map(|info| info.id)
                            }),
                            depth: cluster.depth,
                            member_display_ids: &cluster.nodes,
                            entry_runtime_ids: &cluster_entry_runtime_ids_for_nodes[cluster_index],
                            member_runtime_ids: &cluster_member_runtime_ids_for_nodes
                                [cluster_index],
                            successor_runtime_ids: &cluster_successor_runtime_ids_for_nodes
                                [cluster_index],
                            phase: cluster_phase_for_display(
                                live_data,
                                cluster,
                                display_live_states_for_cluster_chips.as_ref(),
                            ),
                        };
                        if let ClusterView::Collapsed { element, .. } =
                            theme.view_cluster(theme_state, &cx)
                        {
                            return element.map(|_event| Message::Theme(ViewerEvent::Message(())));
                        }
                    }
                }
                text(format!("node {node_id}")).into()
            },
        )
        .id(graph_widget_id)
        .edge_color(move |ctx| {
            let source_runtime_id = runtime_ids_for_edge_colors
                .get(&ctx.edge.0)
                .copied()
                .flatten();
            let target_runtime_id = runtime_ids_for_edge_colors
                .get(&ctx.edge.1)
                .copied()
                .flatten();
            let source_has_proxy_runtime = proxy_runtime_ids_for_edge_colors
                .get(&ctx.edge.0)
                .map(|ids| !ids.is_empty())
                .unwrap_or(false);
            let target_has_proxy_runtime = proxy_runtime_ids_for_edge_colors
                .get(&ctx.edge.1)
                .map(|ids| !ids.is_empty())
                .unwrap_or(false);
            let style = theme
                .edge_style(
                    theme_state,
                    EdgeStyleCtx {
                        edge_index: ctx.edge_index,
                        source_display_id: ctx.edge.0,
                        target_display_id: ctx.edge.1,
                        source_runtime_id,
                        target_runtime_id,
                        source_has_proxy_runtime,
                        target_has_proxy_runtime,
                        source_phase: node_phase_for_display_with_repairs(
                            live_data,
                            ctx.edge.0,
                            source_runtime_id,
                            proxy_runtime_ids_for_edge_colors
                                .get(&ctx.edge.0)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            &condition_successors_for_edge_colors,
                            display_live_states_for_edge_colors.as_ref(),
                        ),
                        target_phase: node_phase_for_display_with_repairs(
                            live_data,
                            ctx.edge.1,
                            target_runtime_id,
                            proxy_runtime_ids_for_edge_colors
                                .get(&ctx.edge.1)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            &condition_successors_for_edge_colors,
                            display_live_states_for_edge_colors.as_ref(),
                        ),
                        extent: ctx.transition_progress,
                    },
                )
                .unwrap_or(default_edge_style);
            (style.start, style.end)
        })
        .outgoing_edge_style(move |ctx| {
            let source_runtime_id = runtime_ids_for_edge_strokes
                .get(&ctx.edge.0)
                .copied()
                .flatten();
            let target_runtime_id = runtime_ids_for_edge_strokes
                .get(&ctx.edge.1)
                .copied()
                .flatten();
            let source_has_proxy_runtime = proxy_runtime_ids_for_edge_strokes
                .get(&ctx.edge.0)
                .map(|ids| !ids.is_empty())
                .unwrap_or(false);
            let target_has_proxy_runtime = proxy_runtime_ids_for_edge_strokes
                .get(&ctx.edge.1)
                .map(|ids| !ids.is_empty())
                .unwrap_or(false);
            let style = theme
                .edge_style(
                    theme_state,
                    EdgeStyleCtx {
                        edge_index: ctx.edge_index,
                        source_display_id: ctx.edge.0,
                        target_display_id: ctx.edge.1,
                        source_runtime_id,
                        target_runtime_id,
                        source_has_proxy_runtime,
                        target_has_proxy_runtime,
                        source_phase: node_phase_for_display_with_repairs(
                            live_data,
                            ctx.edge.0,
                            source_runtime_id,
                            proxy_runtime_ids_for_edge_strokes
                                .get(&ctx.edge.0)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            &condition_successors_for_edge_strokes,
                            display_live_states_for_edge_strokes.as_ref(),
                        ),
                        target_phase: node_phase_for_display_with_repairs(
                            live_data,
                            ctx.edge.1,
                            target_runtime_id,
                            proxy_runtime_ids_for_edge_strokes
                                .get(&ctx.edge.1)
                                .map(Vec::as_slice)
                                .unwrap_or(&[]),
                            &condition_successors_for_edge_strokes,
                            display_live_states_for_edge_strokes.as_ref(),
                        ),
                        extent: ctx.transition_progress,
                    },
                )
                .unwrap_or(default_edge_style);
            OutgoingEdgeStyle {
                visible: true,
                width_scale: style.width.max(0.0),
                alpha: 1.0,
                color_override: None,
            }
        })
        .stroke_width(1.0)
        .edge_corner_radius(18.0)
        .node_size(move |node_id| {
            sizes_for_view
                .get(&node_id)
                .copied()
                .unwrap_or((NODE_WIDTH, NODE_HEIGHT))
        })
        .clusters(visible_clusters)
        .cluster_container(move |index, _| {
            let source_index = visible_cluster_sources.get(index).copied()?;
            let cluster = cluster_info_for_clusters.get(source_index)?;
            let cx = ClusterViewCtx {
                cluster_id: cluster.id,
                cluster_index: source_index,
                kind: cluster.kind,
                label: &cluster.label,
                metadata: cluster.metadata.as_deref(),
                parent_cluster_id: cluster
                    .parent
                    .and_then(|parent| cluster_info_for_clusters.get(parent).map(|info| info.id)),
                depth: cluster.depth,
                member_display_ids: &cluster.nodes,
                entry_runtime_ids: &cluster_entry_runtime_ids[source_index],
                member_runtime_ids: &cluster_member_runtime_ids[source_index],
                successor_runtime_ids: &cluster_successor_runtime_ids[source_index],
                phase: cluster_phase_for_display(
                    live_data,
                    cluster,
                    display_live_states_for_cluster_overlays.as_ref(),
                ),
            };
            let base_overlay = match theme.view_cluster(theme_state, &cx) {
                ClusterView::Expanded { overlay, .. } => overlay,
                ClusterView::Collapsed { .. } => None,
            };
            base_overlay
                .map(|overlay| overlay.map(|_event| Message::Theme(ViewerEvent::Message(()))))
        })
        .cluster_color(cluster_fill_color)
        .padding(24)
        .auto_fit(if auto_zoom_enabled {
            AutoFit::Ongoing
        } else {
            AutoFit::Off
        })
        .keep_centered(auto_pan_enabled)
        .on_viewport_interaction(Message::ViewportInteraction);
        if let Some(duration) = animation_duration {
            widget = widget.animation_duration(duration);
        }
        if let Some(easing) = animation_easing {
            widget = widget.animation_easing(easing);
        }
        widget
    };

    container(
        container(graph_widget)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true)
    .style(graph_panel_style)
    .into()
}

#[derive(Clone)]
struct GraphModel {
    nodes: Vec<NodeDisplay>,
    node_map: HashMap<u32, NodeDisplay>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<Cluster>,
    derived: GraphDerived,
    #[cfg(test)]
    while_clusters: Vec<Cluster>,
    #[cfg(test)]
    while_cluster_labels: Vec<String>,
    cluster_info: Vec<ClusterInfo>,
}

impl GraphModel {
    fn from_ast(ast: JourneyAst) -> Self {
        let mut builder = GraphBuilder::default();
        builder.flatten(&ast);

        let nodes = builder.nodes;
        let edges = builder.edges;
        let cluster_info = builder.cluster_info;
        let node_map = nodes
            .iter()
            .map(|node| (node.id, node.clone()))
            .collect::<HashMap<_, _>>();
        let derived = GraphDerived::build(&nodes, &node_map, &edges, &cluster_info);

        Self {
            nodes,
            node_map,
            edges,
            clusters: builder.clusters.clone(),
            derived,
            #[cfg(test)]
            while_clusters: builder.clusters,
            #[cfg(test)]
            while_cluster_labels: builder.cluster_labels,
            cluster_info,
        }
    }

    fn cluster_node_id(&self, index: usize) -> Option<u32> {
        let offset = u32::try_from(index).ok()?;
        Some(
            self.derived
                .max_node_id
                .saturating_add(1)
                .saturating_add(offset),
        )
    }
}

#[derive(Clone)]
struct GraphDerived {
    condition_successor_runtime_ids: HashMap<u32, Vec<u32>>,
    cluster_member_runtime_ids: Vec<Vec<u32>>,
    cluster_successor_runtime_ids: Vec<Vec<u32>>,
    cluster_entry_runtime_ids: Vec<Vec<u32>>,
    memberships: HashMap<u32, Vec<(usize, usize)>>,
    max_node_id: u32,
    runtime_by_display_id: HashMap<u32, Option<u32>>,
    proxy_runtime_ids_by_display_id: HashMap<u32, Vec<u32>>,
}

impl GraphDerived {
    fn build(
        nodes: &[NodeDisplay],
        node_map: &HashMap<u32, NodeDisplay>,
        edges: &[(u32, u32)],
        cluster_info: &[ClusterInfo],
    ) -> Self {
        let mut condition_successor_runtime_ids = HashMap::<u32, Vec<u32>>::new();
        let mut condition_successor_seen = HashMap::<u32, BTreeSet<u32>>::new();
        for (from, to) in edges {
            let Some(source) = node_map.get(from) else {
                continue;
            };
            if !source.is_conditional_branch {
                continue;
            }
            let Some(target) = node_map.get(to) else {
                continue;
            };
            let Some(runtime_id) = target.runtime_node_id else {
                continue;
            };
            let seen = condition_successor_seen.entry(*from).or_default();
            if seen.insert(runtime_id) {
                condition_successor_runtime_ids
                    .entry(*from)
                    .or_default()
                    .push(runtime_id);
            }
        }

        let mut cluster_member_runtime_ids = vec![Vec::<u32>::new(); cluster_info.len()];
        for (index, cluster) in cluster_info.iter().enumerate() {
            let mut seen = BTreeSet::new();
            for node_id in &cluster.nodes {
                let Some(node) = node_map.get(node_id) else {
                    continue;
                };
                if let Some(runtime_id) = node.runtime_node_id {
                    if seen.insert(runtime_id) {
                        cluster_member_runtime_ids[index].push(runtime_id);
                    }
                }
                for proxy_runtime_id in &node.proxy_runtime_ids {
                    if seen.insert(*proxy_runtime_id) {
                        cluster_member_runtime_ids[index].push(*proxy_runtime_id);
                    }
                }
            }
        }

        let mut cluster_entry_runtime_ids = vec![Vec::<u32>::new(); cluster_info.len()];
        for (index, cluster) in cluster_info.iter().enumerate() {
            let mut seen = BTreeSet::new();
            for node_id in &cluster.root_nodes {
                let Some(node) = node_map.get(node_id) else {
                    continue;
                };
                let Some(runtime_id) = node.runtime_node_id else {
                    continue;
                };
                if seen.insert(runtime_id) {
                    cluster_entry_runtime_ids[index].push(runtime_id);
                }
            }
        }

        let mut memberships = HashMap::<u32, Vec<(usize, usize)>>::new();
        for (index, cluster) in cluster_info.iter().enumerate() {
            for node_id in &cluster.nodes {
                memberships
                    .entry(*node_id)
                    .or_default()
                    .push((cluster.depth, index));
            }
        }
        for entry in memberships.values_mut() {
            entry.sort_by_key(|(depth, _)| *depth);
        }

        let runtime_by_display_id = node_map
            .iter()
            .map(|(display_id, node)| (*display_id, node.runtime_node_id))
            .collect::<HashMap<_, _>>();
        let proxy_runtime_ids_by_display_id = node_map
            .iter()
            .map(|(display_id, node)| (*display_id, node.proxy_runtime_ids.clone()))
            .collect::<HashMap<_, _>>();

        Self {
            condition_successor_runtime_ids,
            cluster_member_runtime_ids,
            cluster_successor_runtime_ids: compute_cluster_successor_runtime_ids(
                edges,
                node_map,
                cluster_info,
            ),
            cluster_entry_runtime_ids,
            memberships,
            max_node_id: nodes.iter().map(|node| node.id).max().unwrap_or(0),
            runtime_by_display_id,
            proxy_runtime_ids_by_display_id,
        }
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<NodeDisplay>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<Cluster>,
    cluster_labels: Vec<String>,
    cluster_info: Vec<ClusterInfo>,
    cluster_stack: Vec<usize>,
    cluster_next_id: u32,
    runtime_next_id: u32,
    display_next_id: u32,
    label_occurrences: HashMap<String, u32>,
}

#[derive(Clone)]
struct NodeDisplay {
    id: u32,
    label: String,
    metadata: Option<String>,
    runtime_node_id: Option<u32>,
    proxy_runtime_ids: Vec<u32>,
    is_conditional_branch: bool,
    is_select: bool,
    is_join: bool,
}

#[derive(Clone)]
struct ClusterInfo {
    id: u32,
    kind: ClusterKind,
    label: String,
    metadata: Option<String>,
    parent: Option<usize>,
    nodes: Vec<u32>,
    root_nodes: Vec<u32>,
    depth: usize,
}

impl NodeDisplay {
    fn kind(&self) -> StepKind {
        if self.is_conditional_branch {
            StepKind::Conditional
        } else if self.is_select {
            StepKind::Select
        } else if self.is_join {
            StepKind::Join
        } else {
            StepKind::Step
        }
    }
}

#[derive(Default)]
struct Flattened {
    roots: Vec<u32>,
    exits: Vec<u32>,
    members: Vec<u32>,
}

impl GraphBuilder {
    fn flatten(&mut self, ast: &JourneyAst) -> Flattened {
        match ast {
            JourneyAst::Empty => Flattened::default(),
            JourneyAst::Sequence(items) => {
                let mut acc = Flattened::default();
                let mut previous_exits = Vec::<u32>::new();
                for item in items {
                    let current = self.flatten(item);
                    if current.roots.is_empty() {
                        continue;
                    }

                    if acc.roots.is_empty() {
                        acc.roots = current.roots.clone();
                    }

                    for from in &previous_exits {
                        for to in &current.roots {
                            self.edges.push((*from, *to));
                        }
                    }

                    previous_exits = current.exits.clone();
                    acc.exits = current.exits.clone();
                    acc.members.extend(current.members);
                }
                acc
            }
            JourneyAst::Step { label } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let label = self.unique_label(*label);
                let node = self.push_runtime_node(label, runtime_id);
                Flattened {
                    roots: vec![node],
                    exits: vec![node],
                    members: vec![node],
                }
            }
            JourneyAst::Conditional {
                label,
                metadata,
                left,
                right,
            } => {
                let branch_label = if metadata.trim().is_empty() {
                    short_type_name_str(label).to_string()
                } else {
                    format!("{} :: {}", short_type_name_str(label), metadata)
                };
                let branch = self.push_layout_node(branch_label, |node| {
                    node.is_conditional_branch = true;
                });
                if !metadata.trim().is_empty() {
                    self.mark(branch, |node| node.metadata = Some((*metadata).to_string()));
                }
                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);

                for target in &left_flow.roots {
                    self.edges.push((branch, *target));
                }
                for target in &right_flow.roots {
                    self.edges.push((branch, *target));
                }

                let mut members = vec![branch];
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());

                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits);
                exits = dedup(exits);

                Flattened {
                    roots: vec![branch],
                    exits,
                    members,
                }
            }
            JourneyAst::While {
                label,
                metadata,
                body,
            } => {
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = Cluster::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);
                let cluster_label = if metadata.trim().is_empty() {
                    format!("while: {}", short_type_name_str(label))
                } else {
                    format!("while: {} :: {}", short_type_name_str(label), metadata)
                };
                self.cluster_labels.push(cluster_label.clone());
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::While,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    depth,
                });
                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                for exit in &body_flow.exits {
                    for root in &body_flow.roots {
                        self.edges.push((*exit, *root));
                    }
                }

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                }
            }
            JourneyAst::Transparent {
                label,
                metadata,
                body,
            } => {
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = Cluster::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);

                let cluster_label = if metadata.trim().is_empty() {
                    format!("transparent: {}", short_type_name_str(label))
                } else {
                    format!(
                        "transparent: {} :: {}",
                        short_type_name_str(label),
                        metadata
                    )
                };
                self.cluster_labels.push(cluster_label.clone());
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::Transparent,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    depth,
                });

                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                }
            }
            JourneyAst::Attempt {
                label,
                metadata,
                body,
            } => {
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = Cluster::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);

                let cluster_label = if metadata.trim().is_empty() {
                    format!("attempt: {}", short_type_name_str(label))
                } else {
                    format!("attempt: {} :: {}", short_type_name_str(label), metadata)
                };
                self.cluster_labels.push(cluster_label.clone());
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::Transparent,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    depth,
                });

                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                }
            }
            JourneyAst::Select {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let _ = (label, metadata);

                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                let mut roots = left_flow.roots;
                roots.extend(right_flow.roots.iter().copied());
                roots = dedup(roots);
                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits.iter().copied());
                exits = dedup(exits);
                let mut members = Vec::new();
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());
                members = dedup(members);
                for member_id in &exits {
                    self.mark(*member_id, |node| {
                        if !node.proxy_runtime_ids.contains(&runtime_id) {
                            node.proxy_runtime_ids.push(runtime_id);
                        }
                    });
                }

                Flattened {
                    roots,
                    exits,
                    members,
                }
            }
            JourneyAst::Join {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let _ = (label, metadata);

                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                let mut roots = left_flow.roots;
                roots.extend(right_flow.roots.iter().copied());
                roots = dedup(roots);
                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits.iter().copied());
                exits = dedup(exits);
                let mut members = Vec::new();
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());
                members = dedup(members);
                for member_id in &exits {
                    self.mark(*member_id, |node| {
                        if !node.proxy_runtime_ids.contains(&runtime_id) {
                            node.proxy_runtime_ids.push(runtime_id);
                        }
                    });
                }

                Flattened {
                    roots,
                    exits,
                    members,
                }
            }
        }
    }

    fn push_runtime_node(&mut self, label: impl Into<String>, runtime_id: u32) -> u32 {
        let node_id = self.next_display_id();
        let display = NodeDisplay {
            id: node_id,
            label: label.into(),
            metadata: None,
            runtime_node_id: Some(runtime_id),
            proxy_runtime_ids: Vec::new(),
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
        };
        self.nodes.push(display);
        node_id
    }

    fn push_layout_node(
        &mut self,
        label: impl Into<String>,
        apply: impl FnOnce(&mut NodeDisplay),
    ) -> u32 {
        let node_id = self.next_display_id();
        let mut display = NodeDisplay {
            id: node_id,
            label: label.into(),
            metadata: None,
            runtime_node_id: None,
            proxy_runtime_ids: Vec::new(),
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
        };
        apply(&mut display);
        self.nodes.push(display);
        node_id
    }

    fn mark(&mut self, node_id: u32, apply: impl FnOnce(&mut NodeDisplay)) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|candidate| candidate.id == node_id)
        {
            apply(node);
        }
    }

    fn next_display_id(&mut self) -> u32 {
        let id = self.display_next_id;
        self.display_next_id = self.display_next_id.saturating_add(1);
        id
    }

    fn unique_label(&mut self, raw: impl Into<String>) -> String {
        let full = raw.into();
        let short = short_type_name_str(&full);
        let entry = self.label_occurrences.entry(short.clone()).or_insert(0);
        let label = if *entry == 0 {
            short
        } else {
            format!("{short} #{}", *entry + 1)
        };
        *entry = entry.saturating_add(1);
        label
    }
}

fn short_type_name<T>() -> String {
    short_type_name_str(core::any::type_name::<T>())
}

fn short_type_name_str(value: &str) -> String {
    value
        .split("::")
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(value)
        .to_string()
}

fn dedup(values: Vec<u32>) -> Vec<u32> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for value in values {
        if seen.insert(value) {
            output.push(value);
        }
    }
    output
}

#[cfg(test)]
fn cluster_successor_runtime_ids(model: &GraphModel) -> Vec<Vec<u32>> {
    model.derived.cluster_successor_runtime_ids.clone()
}

fn compute_cluster_successor_runtime_ids(
    edges: &[(u32, u32)],
    node_map: &HashMap<u32, NodeDisplay>,
    cluster_info: &[ClusterInfo],
) -> Vec<Vec<u32>> {
    let mut outgoing_by_node = HashMap::<u32, Vec<u32>>::new();
    for (from, to) in edges {
        outgoing_by_node.entry(*from).or_default().push(*to);
    }

    let mut cluster_successors = vec![Vec::<u32>::new(); cluster_info.len()];
    for (index, cluster) in cluster_info.iter().enumerate() {
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        let mut queue = std::collections::VecDeque::<u32>::new();
        let mut visited = HashSet::<u32>::new();

        for (from, to) in edges {
            if !cluster_nodes.contains(from) || cluster_nodes.contains(to) {
                continue;
            }
            if visited.insert(*to) {
                queue.push_back(*to);
            }
        }

        let mut seen_runtime_ids = BTreeSet::new();
        while let Some(node_id) = queue.pop_front() {
            if cluster_nodes.contains(&node_id) {
                continue;
            }
            if let Some(node) = node_map.get(&node_id) {
                if let Some(runtime_id) = node.runtime_node_id {
                    if seen_runtime_ids.insert(runtime_id) {
                        cluster_successors[index].push(runtime_id);
                    }
                }
            }
            if let Some(neighbors) = outgoing_by_node.get(&node_id) {
                for neighbor in neighbors {
                    if visited.insert(*neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }
    cluster_successors
}

#[derive(Clone, Copy, Debug)]
pub struct DefaultTheme {
    cluster_expansion: ClusterExpansionConfig,
}

impl DefaultTheme {
    pub fn with_cluster_expansion_config(
        mut self,
        cluster_expansion: ClusterExpansionConfig,
    ) -> Self {
        self.cluster_expansion = cluster_expansion;
        self
    }
}

impl Default for DefaultTheme {
    fn default() -> Self {
        Self {
            cluster_expansion: ClusterExpansionConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NodeVisual {
    state: RuntimeState,
}

#[derive(Debug, Clone)]
struct ClusterRuntimeIndex {
    kind: ClusterKind,
    entry_runtime_ids: HashSet<u32>,
    member_runtime_ids: HashSet<u32>,
    successor_runtime_ids: HashSet<u32>,
}

#[derive(Debug, Clone, Copy)]
struct ClusterVisual {
    expanded: bool,
    border_state: RuntimeState,
    completed_at: Option<Instant>,
}

#[derive(Debug)]
pub struct DefaultThemeState {
    node_visuals: HashMap<u32, NodeVisual>,
    cluster_index: HashMap<u32, ClusterRuntimeIndex>,
    cluster_visuals: HashMap<u32, ClusterVisual>,
    force_pending_runtime_ids: HashSet<u32>,
    cluster_expansion: ClusterExpansionConfig,
}

impl DefaultThemeState {
    fn new(cluster_expansion: ClusterExpansionConfig) -> Self {
        Self {
            node_visuals: HashMap::new(),
            cluster_index: HashMap::new(),
            cluster_visuals: HashMap::new(),
            force_pending_runtime_ids: HashSet::new(),
            cluster_expansion,
        }
    }

    fn register_cluster(&mut self, cx: &ClusterViewCtx<'_>) {
        let expansion_mode = self.cluster_expansion.mode_for(cx.kind);
        let expanded = matches!(expansion_mode, ClusterExpansionMode::AlwaysExpanded)
            || matches!(cx.phase, Phase::Live(live) if live.has_running);
        let border_state = match cx.phase {
            Phase::Live(live) if live.has_running => RuntimeState::Running,
            _ => RuntimeState::Pending,
        };
        self.cluster_index
            .entry(cx.cluster_id)
            .or_insert_with(|| ClusterRuntimeIndex {
                kind: cx.kind,
                entry_runtime_ids: cx.entry_runtime_ids.iter().copied().collect(),
                member_runtime_ids: cx.member_runtime_ids.iter().copied().collect(),
                successor_runtime_ids: cx.successor_runtime_ids.iter().copied().collect(),
            });
        self.cluster_visuals
            .entry(cx.cluster_id)
            .or_insert(ClusterVisual {
                expanded,
                border_state,
                completed_at: None,
            });
    }

    fn cluster_is_expanded(&self, cluster_id: u32) -> bool {
        self.cluster_visuals
            .get(&cluster_id)
            .map(|visual| visual.expanded)
            .unwrap_or(false)
    }

    fn update_node_state(&mut self, runtime_id: u32, to: RuntimeState) -> bool {
        if !matches!(to, RuntimeState::Pending) {
            self.force_pending_runtime_ids.remove(&runtime_id);
        }
        let entry = self.node_visuals.entry(runtime_id).or_insert(NodeVisual {
            state: RuntimeState::Pending,
        });

        if entry.state == to {
            return false;
        }

        entry.state = to;
        true
    }

    fn reset_cluster_members_to_pending(
        &mut self,
        cluster_id: u32,
        except_runtime_id: u32,
    ) -> bool {
        let Some(index) = self.cluster_index.get(&cluster_id) else {
            return false;
        };
        let members = index.member_runtime_ids.iter().copied().collect::<Vec<_>>();
        let mut changed = false;
        for member_id in members {
            if member_id == except_runtime_id {
                continue;
            }
            self.force_pending_runtime_ids.insert(member_id);
            changed |= self.update_node_state(member_id, RuntimeState::Pending);
        }
        changed
    }

    fn update_clusters_for_effect_input(&mut self, runtime_id: u32, now: Instant) -> bool {
        let mut changed = false;
        let cluster_ids = self.cluster_index.keys().copied().collect::<Vec<_>>();
        for cluster_id in cluster_ids {
            let Some(index) = self.cluster_index.get(&cluster_id) else {
                continue;
            };
            let contains_member = index.member_runtime_ids.contains(&runtime_id);
            let contains_entry = index.entry_runtime_ids.contains(&runtime_id);
            let contains_successor = index.successor_runtime_ids.contains(&runtime_id);
            let is_while_cluster = matches!(index.kind, ClusterKind::While);
            let expansion_mode = self.cluster_expansion.mode_for(index.kind);

            let mut activated_iteration = false;
            if let Some(visual) = self.cluster_visuals.get_mut(&cluster_id) {
                if matches!(expansion_mode, ClusterExpansionMode::AlwaysExpanded)
                    && !visual.expanded
                {
                    visual.expanded = true;
                    changed = true;
                }
                let while_reentered_via_non_entry_member = is_while_cluster
                    && contains_member
                    && !contains_entry
                    && !contains_successor
                    && !matches!(visual.border_state, RuntimeState::Running);
                let member_activation = contains_member
                    && match expansion_mode {
                        ClusterExpansionMode::Automatic => !visual.expanded,
                        ClusterExpansionMode::AlwaysExpanded => {
                            !matches!(visual.border_state, RuntimeState::Running)
                        }
                    };
                if (is_while_cluster && contains_entry)
                    || member_activation
                    || while_reentered_via_non_entry_member
                {
                    let expansion_changed = !visual.expanded;
                    visual.expanded = true;
                    visual.completed_at = None;
                    let border_changed = visual.border_state != RuntimeState::Running;
                    visual.border_state = RuntimeState::Running;
                    changed |= border_changed || expansion_changed;
                    activated_iteration = true;
                } else if visual.expanded && contains_successor {
                    let border_changed = visual.border_state != RuntimeState::Completed;
                    visual.border_state = RuntimeState::Completed;
                    changed |= border_changed;
                    visual.completed_at.get_or_insert(now);
                }
            }

            if activated_iteration || (is_while_cluster && contains_entry) {
                changed |= self.reset_cluster_members_to_pending(cluster_id, runtime_id);
            }
        }
        changed
    }

    fn maybe_collapse_completed_cluster_for_pending_successor(
        &mut self,
        cx: &ClusterViewCtx<'_>,
        now: Instant,
    ) -> bool {
        if !matches!(cx.kind, ClusterKind::While | ClusterKind::Transparent) {
            return false;
        }
        if matches!(
            self.cluster_expansion.mode_for(cx.kind),
            ClusterExpansionMode::AlwaysExpanded
        ) {
            return false;
        }

        let Phase::Live(live) = cx.phase else {
            return false;
        };
        if live.has_running {
            return false;
        }

        let should_collapse = self
            .cluster_visuals
            .get(&cx.cluster_id)
            .map(|visual| {
                visual.expanded
                    && matches!(visual.border_state, RuntimeState::Completed)
                    && visual
                        .completed_at
                        .map(|completed_at| {
                            now.saturating_duration_since(completed_at) >= CLUSTER_RECOLLAPSE_DELAY
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if !should_collapse {
            return false;
        }

        let Some(visual) = self.cluster_visuals.get_mut(&cx.cluster_id) else {
            return false;
        };
        visual.expanded = false;
        visual.completed_at = None;
        let border_changed = visual.border_state != RuntimeState::Pending;
        visual.border_state = RuntimeState::Pending;
        border_changed
    }

    fn cluster_border_color(&self, cluster_id: u32) -> Color {
        self.cluster_visuals
            .get(&cluster_id)
            .map(|visual| runtime_color(visual.border_state))
            .unwrap_or_else(cluster_border_color_gray)
    }
}

impl JunglePanelTheme<AnyAnimal> for DefaultTheme {
    type State = Mutex<DefaultThemeState>;
    type Message = ();

    fn init(&self) -> Self::State {
        Mutex::new(DefaultThemeState::new(self.cluster_expansion))
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: ViewerEvent<Self::Message>,
    ) -> Task<ViewerEvent<Self::Message>> {
        let now = Instant::now();
        let guard = state.get_mut();

        match event {
            ViewerEvent::JourneyUpdate(update) => match update.event {
                RunnerUpdateOut::EffectInput { node_id, .. } => {
                    let _ = guard.update_node_state(node_id, RuntimeState::Running);
                    let _ = guard.update_clusters_for_effect_input(node_id, now);
                }
                RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => {
                    let _ = guard.update_node_state(node_id, RuntimeState::Completed);
                }
                RunnerUpdateOut::EffectFailureOutput { node_id, .. } => {
                    let _ = guard.update_node_state(node_id, RuntimeState::Failed);
                }
                RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
            },
            ViewerEvent::Message(()) => {}
        }

        Task::none()
    }

    fn view_step(
        &self,
        state: &Self::State,
        cx: &StepViewCtx<'_>,
    ) -> (Element<'static, ViewerEvent<Self::Message>>, (f64, f64)) {
        let role = match cx.kind {
            StepKind::Conditional => "condition",
            StepKind::Select => "select",
            StepKind::Join => "join",
            StepKind::Step => "step",
        };

        let fill = if let Some(runtime_id) = cx.runtime_id {
            let mut phase_target = match cx.phase {
                Phase::Live(target) => target,
                Phase::Static => RuntimeState::Pending,
            };
            if let Ok(guard) = state.try_lock() {
                let forced_pending = guard.force_pending_runtime_ids.contains(&runtime_id);
                if forced_pending && !matches!(phase_target, RuntimeState::Running) {
                    phase_target = RuntimeState::Pending;
                }
            }
            runtime_color(phase_target)
        } else {
            let phase_target = match cx.phase {
                Phase::Live(target) => target,
                Phase::Static => RuntimeState::Pending,
            };
            runtime_color(phase_target)
        };
        (
            AnimatedStepNode::<ViewerEvent<Self::Message>>::new(
                state as *const Self::State as usize as u64,
                cx.display_id,
                cx.runtime_id,
                role,
                cx.label.to_string(),
                cx.metadata.map(str::to_string),
                fill,
                NODE_ANIMATION_DURATION,
            )
            .into(),
            (240.0, 80.0),
        )
    }

    fn view_cluster(
        &self,
        state: &Self::State,
        cx: &ClusterViewCtx<'_>,
    ) -> ClusterView<Self::Message> {
        let now = Instant::now();
        let (expanded, border_color) = if let Ok(mut guard) = state.try_lock() {
            guard.register_cluster(cx);
            guard.maybe_collapse_completed_cluster_for_pending_successor(cx, now);
            (
                guard.cluster_is_expanded(cx.cluster_id),
                guard.cluster_border_color(cx.cluster_id),
            )
        } else {
            (false, cluster_border_color_gray())
        };
        let fill = cluster_panel::target_color(cx.kind, cx.phase);
        let overlay = AnimatedClusterView::<ViewerEvent<Self::Message>>::overlay(
            cx.cluster_id,
            cx.label.to_string(),
            border_color,
            fill,
            CLUSTER_BORDER_ANIMATION_DURATION,
        )
        .into();

        if expanded {
            ClusterView::Expanded {
                overlay: Some(overlay),
                fill,
            }
        } else {
            ClusterView::Collapsed {
                element: AnimatedClusterView::<ViewerEvent<Self::Message>>::chip(
                    cx.cluster_id,
                    cx.label.to_string(),
                    border_color,
                    CLUSTER_BORDER_ANIMATION_DURATION,
                )
                .into(),
                size: (240.0, 46.0),
            }
        }
    }

    fn edge_style(&self, state: &Self::State, cx: EdgeStyleCtx) -> Option<EdgeStyle> {
        let mut phase_target = match cx.source_phase {
            Phase::Live(target) => target,
            Phase::Static => RuntimeState::Pending,
        };
        if let Some(runtime_id) = cx.source_runtime_id {
            if let Ok(guard) = state.try_lock() {
                let forced_pending = guard.force_pending_runtime_ids.contains(&runtime_id);
                if forced_pending && !matches!(phase_target, RuntimeState::Running) {
                    phase_target = RuntimeState::Pending;
                }
            }
        }
        let (from_color, to_color) = {
            let color = runtime_color(phase_target);
            (color, color)
        };

        let progress = cx.extent.clamp(0.0, 1.0);
        let source_t = ease_out_cubic((progress / 0.55).clamp(0.0, 1.0));
        let target_t = ease_out_cubic(((progress - 0.25) / 0.75).clamp(0.0, 1.0));
        let start = lerp_color(from_color, to_color, source_t);
        let end = lerp_color(from_color, to_color, target_t);

        Some(EdgeStyle {
            width: 1.6,
            start,
            end,
        })
    }
}

fn runtime_color(state: RuntimeState) -> Color {
    match state {
        RuntimeState::Pending => Color::from_rgb8(120, 120, 120),
        RuntimeState::Running => Color::from_rgb8(212, 190, 68),
        RuntimeState::Completed => Color::from_rgb8(55, 144, 81),
        RuntimeState::Failed => Color::from_rgb8(165, 61, 61),
    }
}

fn cluster_border_color_gray() -> Color {
    runtime_color(RuntimeState::Pending)
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn app_background(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(8, 19, 13))),
        text_color: Some(jungle_text_base()),
        ..Default::default()
    }
}

fn sidebar_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(10, 26, 17))),
        border: iced::border::rounded(0)
            .color(Color::from_rgb8(24, 63, 43))
            .width(1.0),
        text_color: Some(jungle_text_base()),
        ..Default::default()
    }
}

fn graph_panel_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(7, 17, 11))),
        ..Default::default()
    }
}

fn sidebar_button(_theme: &iced::Theme, status: button::Status) -> iced::widget::button::Style {
    let background = match status {
        button::Status::Hovered => Color::from_rgb8(28, 89, 55),
        _ => Color::from_rgb8(20, 71, 45),
    };
    iced::widget::button::Style {
        background: Some(iced::Background::Color(background)),
        text_color: jungle_text_base(),
        border: iced::border::rounded(8)
            .color(jungle_accent_dark())
            .width(1.0),
        shadow: iced::Shadow::default(),
        ..Default::default()
    }
}

fn set_cluster_fill_colors(colors: Vec<Color>) {
    let store = CLUSTER_FILL_COLORS.get_or_init(|| RwLock::new(Vec::new()));
    if let Ok(mut guard) = store.write() {
        *guard = colors;
    }
}

fn cluster_fill_color(index: usize) -> Color {
    let store = CLUSTER_FILL_COLORS.get_or_init(|| RwLock::new(Vec::new()));
    store
        .read()
        .ok()
        .and_then(|colors| colors.get(index).copied())
        .unwrap_or(DEFAULT_CLUSTER_FILL)
}

fn jungle_text_base() -> Color {
    Color::from_rgb8(223, 245, 230)
}

fn jungle_text_muted() -> Color {
    Color::from_rgb8(145, 183, 157)
}

fn jungle_accent_bright() -> Color {
    Color::from_rgb8(103, 215, 139)
}

fn jungle_accent_dark() -> Color {
    Color::from_rgb8(46, 115, 73)
}

impl fmt::Debug for ViewMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewMode::Static { journey_name, .. } => f
                .debug_struct("Static")
                .field("journey_name", journey_name)
                .finish(),
            ViewMode::Live {
                journey_name,
                journey_id,
                ..
            } => f
                .debug_struct("Live")
                .field("journey_name", journey_name)
                .field("journey_id", journey_id)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    #[test]
    fn live_data_apply_update_reports_runtime_highlight_changes() {
        let mut live = LiveData::default();

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.active_runtime_ids.contains(&9));

        assert!(!live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(!live.active_runtime_ids.contains(&9));
        assert!(live.finished_runtime_ids.contains(&9));

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 4,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.active_runtime_ids.contains(&9));
        assert!(!live.finished_runtime_ids.contains(&9));

        assert!(!live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
            event_unix_ms: 0,
            event: RunnerUpdateOut::SleepScheduled {
                uuid: Uuid::nil(),
                timer_id: Uuid::nil(),
                wake_at_unix_ms: 1,
            },
        }));
        assert_eq!(live.latest_event_count, 5);
    }

    #[test]
    fn condition_runtime_state_tracks_latest_branch_event() {
        let mut live = LiveData::default();

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 11,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 11,
                uuid: Uuid::nil(),
            },
        }));
        assert_eq!(
            infer_condition_runtime_state(&live, &[11, 12]),
            RuntimeState::Completed
        );

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 12,
                uuid: Uuid::nil(),
            },
        }));
        assert_eq!(
            infer_condition_runtime_state(&live, &[11, 12]),
            RuntimeState::Running
        );
    }

    #[test]
    fn join_proxy_runtime_colors_child_nodes_only_while_running() {
        let ast = JourneyAst::Join {
            label: "Join",
            metadata: "",
            left: Box::new(JourneyAst::Step { label: "JoinL" }),
            right: Box::new(JourneyAst::Step { label: "JoinR" }),
        };
        let model = GraphModel::from_ast(ast);
        let node_for = |label: &str| -> &NodeDisplay {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let join_l = node_for("JoinL");
        let join_r = node_for("JoinR");
        let mut live = LiveData::default();

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                join_l.id,
                join_l.runtime_node_id,
                &join_l.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Running)
        );
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                join_r.id,
                join_r.runtime_node_id,
                &join_r.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Running)
        );

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                join_l.id,
                join_l.runtime_node_id,
                &join_l.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Pending)
        );
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                join_r.id,
                join_r.runtime_node_id,
                &join_r.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Pending)
        );
    }

    #[test]
    fn join_proxy_runtime_colors_branch_exit_nodes_while_running() {
        let ast = JourneyAst::Join {
            label: "Join",
            metadata: "",
            left: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "LeftA" },
                JourneyAst::Step { label: "LeftB" },
            ])),
            right: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "RightA" },
                JourneyAst::Step { label: "RightB" },
            ])),
        };
        let model = GraphModel::from_ast(ast);
        let node_for = |label: &str| -> &NodeDisplay {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let left_a = node_for("LeftA");
        let left_b = node_for("LeftB");
        let right_a = node_for("RightA");
        let right_b = node_for("RightB");
        let mut live = LiveData::default();

        for (sequence_id, runtime_id) in [(1, 1), (2, 2), (3, 3), (4, 4)] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id,
                    uuid: Uuid::nil(),
                },
            }));
        }
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));

        assert_eq!(
            node_phase_for_display(
                Some(&live),
                left_a.id,
                left_a.runtime_node_id,
                &left_a.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Completed)
        );
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                left_b.id,
                left_b.runtime_node_id,
                &left_b.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Running)
        );
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                right_a.id,
                right_a.runtime_node_id,
                &right_a.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Completed)
        );
        assert_eq!(
            node_phase_for_display(
                Some(&live),
                right_b.id,
                right_b.runtime_node_id,
                &right_b.proxy_runtime_ids,
                &HashMap::new(),
            ),
            Phase::Live(RuntimeState::Running)
        );
    }

    #[test]
    fn repaired_live_states_delay_running_predecessor_promotion() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
        ]);
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");
        let b_id = id_for("B");

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(repaired.get(&a_id).copied(), Some(RuntimeState::Running));
        assert_eq!(repaired.get(&b_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_eventually_promote_running_predecessor() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
        ]);
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(repaired.get(&a_id).copied(), Some(RuntimeState::Completed));
    }

    #[test]
    fn repaired_live_states_backfill_pending_chain_when_downstream_runs() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
            JourneyAst::Step { label: "C" },
        ]);
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");
        let b_id = id_for("B");
        let c_id = id_for("C");

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 2,
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(repaired.get(&a_id).copied(), Some(RuntimeState::Completed));
        assert_eq!(repaired.get(&b_id).copied(), Some(RuntimeState::Completed));
        assert_eq!(repaired.get(&c_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn while_loop_new_iteration_forces_stale_completed_members_pending() {
        let ast = JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
                JourneyAst::Step { label: "C" },
            ])),
        };
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");
        let b_id = id_for("B");
        let c_id = id_for("C");

        let mut live = LiveData::default();
        for (sequence_id, node_id) in [(1, 0), (2, 1), (3, 2)] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::EffectSuccessOutput {
                    node_id,
                    uuid: Uuid::nil(),
                },
            }));
        }

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 4,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        let restarted_states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            restarted_states.get(&a_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(
            restarted_states.get(&b_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            restarted_states.get(&c_id).copied(),
            Some(RuntimeState::Pending)
        );

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        let between_steps_states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            between_steps_states.get(&a_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            between_steps_states.get(&b_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            between_steps_states.get(&c_id).copied(),
            Some(RuntimeState::Pending)
        );

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 6,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));
        let advanced_states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            advanced_states.get(&a_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            advanced_states.get(&b_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(
            advanced_states.get(&c_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn while_loop_new_iteration_forces_stale_running_members_pending() {
        let ast = JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
                JourneyAst::Step { label: "C" },
            ])),
        };
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");
        let b_id = id_for("B");
        let c_id = id_for("C");

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 2,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));

        let restarted_states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            restarted_states.get(&a_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(
            restarted_states.get(&b_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            restarted_states.get(&c_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn while_loop_new_iteration_forces_stale_join_proxy_running_pending() {
        let ast = JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Begin" },
                JourneyAst::Join {
                    label: "Join",
                    metadata: "",
                    left: Box::new(JourneyAst::Step { label: "Left" }),
                    right: Box::new(JourneyAst::Step { label: "Right" }),
                },
            ])),
        };
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let begin = model
            .nodes
            .iter()
            .find(|node| node.label == "Begin")
            .unwrap_or_else(|| panic!("missing node with label Begin"));
        let left = model
            .nodes
            .iter()
            .find(|node| node.label == "Left")
            .unwrap_or_else(|| panic!("missing node with label Left"));
        let right = model
            .nodes
            .iter()
            .find(|node| node.label == "Right")
            .unwrap_or_else(|| panic!("missing node with label Right"));
        let begin_id = id_for("Begin");
        let left_id = id_for("Left");
        let right_id = id_for("Right");
        let join_runtime_id = left
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("left branch exit should carry hidden join runtime"));
        assert_eq!(right.proxy_runtime_ids, vec![join_runtime_id]);

        let begin_runtime_id = begin
            .runtime_node_id
            .unwrap_or_else(|| panic!("Begin should have a runtime node id"));
        let left_runtime_id = left
            .runtime_node_id
            .unwrap_or_else(|| panic!("Left should have a runtime node id"));
        let right_runtime_id = right
            .runtime_node_id
            .unwrap_or_else(|| panic!("Right should have a runtime node id"));

        let mut live = LiveData::default();
        for (sequence_id, node_id) in [
            (1, begin_runtime_id),
            (2, left_runtime_id),
            (3, right_runtime_id),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::EffectSuccessOutput {
                    node_id,
                    uuid: Uuid::nil(),
                },
            }));
        }
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 4,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: join_runtime_id,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: begin_runtime_id,
                uuid: Uuid::nil(),
            },
        }));

        let restarted_states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            restarted_states.get(&begin_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(
            restarted_states.get(&left_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            restarted_states.get(&right_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn while_loop_prompt_branch_promotes_current_iteration_ancestors() {
        let ast = JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Begin" },
                JourneyAst::Conditional {
                    label: "Branch",
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step { label: "Select" },
                        JourneyAst::Step { label: "Optimize" },
                    ])),
                    right: Box::new(JourneyAst::Step { label: "Skip" }),
                },
                JourneyAst::Step { label: "Flatten" },
            ])),
        };
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let begin_id = id_for("Begin");
        let select_id = id_for("Select");
        let optimize_id = id_for("Optimize");
        let skip_id = id_for("Skip");
        let flatten_id = id_for("Flatten");

        let mut live = LiveData::default();
        for (sequence_id, node_id) in [(1, 0), (2, 1), (3, 2), (4, 4)] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::EffectSuccessOutput {
                    node_id,
                    uuid: Uuid::nil(),
                },
            }));
        }

        for (sequence_id, event) in [
            (
                5,
                RunnerUpdateOut::EffectInput {
                    node_id: 0,
                    uuid: Uuid::nil(),
                },
            ),
            (
                6,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: 0,
                    uuid: Uuid::nil(),
                },
            ),
            (
                7,
                RunnerUpdateOut::EffectInput {
                    node_id: 1,
                    uuid: Uuid::nil(),
                },
            ),
            (
                8,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: 1,
                    uuid: Uuid::nil(),
                },
            ),
            (
                9,
                RunnerUpdateOut::EffectInput {
                    node_id: 2,
                    uuid: Uuid::nil(),
                },
            ),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event,
            }));
        }

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            states.get(&begin_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            states.get(&select_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            states.get(&optimize_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(states.get(&skip_id).copied(), Some(RuntimeState::Pending));
        assert_eq!(
            states.get(&flatten_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn while_loop_join_proxy_running_colors_exit_and_keeps_ancestors_completed() {
        let prompt_branch = || {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: "Branch",
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step { label: "Select" },
                        JourneyAst::Step { label: "Optimize" },
                    ])),
                    right: Box::new(JourneyAst::Step { label: "Skip" }),
                },
                JourneyAst::Step { label: "Flatten" },
            ])
        };
        let ast = JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(prompt_branch()),
                right: Box::new(prompt_branch()),
            }),
        };
        let model = GraphModel::from_ast(ast);
        let all_ids_for = |label: &str| -> Vec<u32> {
            model
                .nodes
                .iter()
                .filter(|node| node.label == label || node.label.starts_with(&format!("{label} #")))
                .map(|node| node.id)
                .collect()
        };
        let select_ids = all_ids_for("Select");
        let optimize_ids = all_ids_for("Optimize");
        let flatten_ids = all_ids_for("Flatten");
        let skip_ids = all_ids_for("Skip");
        assert_eq!(select_ids.len(), 2);
        assert_eq!(optimize_ids.len(), 2);
        assert_eq!(flatten_ids.len(), 2);
        assert_eq!(skip_ids.len(), 2);

        let mut live = LiveData::default();
        for (sequence_id, node_id) in [(1, 1_u32), (2, 2), (3, 4), (4, 5), (5, 6), (6, 8), (7, 0)] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::EffectSuccessOutput {
                    node_id,
                    uuid: Uuid::nil(),
                },
            }));
        }

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 8,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );

        for select_id in select_ids {
            assert_eq!(
                states.get(&select_id).copied(),
                Some(RuntimeState::Completed)
            );
        }
        for optimize_id in optimize_ids {
            assert_eq!(
                states.get(&optimize_id).copied(),
                Some(RuntimeState::Completed)
            );
        }
        for flatten_id in flatten_ids {
            assert_eq!(
                states.get(&flatten_id).copied(),
                Some(RuntimeState::Running)
            );
        }
        for skip_id in skip_ids {
            assert!(
                !matches!(states.get(&skip_id).copied(), Some(RuntimeState::Running)),
                "inactive sibling branch should not appear running"
            );
        }
    }

    #[test]
    fn cluster_live_from_repairs_clears_stale_running_flag_after_downstream_progress() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
        ]);
        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };
        let a_id = id_for("A");
        let b_id = id_for("B");

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 0,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 1,
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let cluster = ClusterInfo {
            id: 1,
            kind: ClusterKind::Transparent,
            label: "cluster".to_string(),
            metadata: None,
            parent: None,
            nodes: vec![a_id, b_id],
            root_nodes: vec![a_id],
            depth: 0,
        };
        let cluster_live = cluster_live_from_repaired_states(&cluster, &repaired);
        assert!(!cluster_live.has_running);
        assert!(!cluster_live.has_failed);
        assert!(cluster_live.has_completed);
    }

    #[test]
    fn view_step_does_not_mutate_theme_state() {
        let theme = DefaultTheme::default();
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());
        state.force_pending_runtime_ids = HashSet::from([42]);
        let state = Mutex::new(state);
        let cx = StepViewCtx {
            display_id: 1,
            runtime_id: Some(42),
            proxy_runtime_ids: &[7],
            successor_runtime_ids: &[],
            kind: StepKind::Step,
            label: "JoinL",
            metadata: None,
            phase: Phase::Live(RuntimeState::Completed),
        };

        let _ = theme.view_step(&state, &cx);

        let guard = state
            .try_lock()
            .expect("theme state lock should be available");
        assert!(guard.node_visuals.is_empty());
    }

    #[test]
    fn edge_style_forced_pending_overrides_stale_proxy_completion() {
        let theme = DefaultTheme::default();
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());
        state.force_pending_runtime_ids = HashSet::from([42]);
        let state = Mutex::new(state);

        let style = theme
            .edge_style(
                &state,
                EdgeStyleCtx {
                    edge_index: 0,
                    source_display_id: 1,
                    target_display_id: 2,
                    source_runtime_id: Some(42),
                    target_runtime_id: Some(99),
                    source_has_proxy_runtime: true,
                    target_has_proxy_runtime: false,
                    source_phase: Phase::Live(RuntimeState::Completed),
                    target_phase: Phase::Live(RuntimeState::Pending),
                    extent: 1.0,
                },
            )
            .expect("default theme should always provide an edge style");

        assert_eq!(style.start, runtime_color(RuntimeState::Pending));
        assert_eq!(style.end, runtime_color(RuntimeState::Pending));
    }

    #[test]
    fn graph_model_uses_unique_display_node_ids() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::While {
                label: "Loop",
                metadata: "",
                body: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step { label: "A1" },
                    JourneyAst::Conditional {
                        label: "Branch",
                        metadata: "",
                        left: Box::new(JourneyAst::Step { label: "A2" }),
                        right: Box::new(JourneyAst::Step { label: "A3" }),
                    },
                ])),
            },
            JourneyAst::Select {
                label: "Select",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "A4" }),
                right: Box::new(JourneyAst::Step { label: "A5" }),
            },
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "A6" }),
                right: Box::new(JourneyAst::Step { label: "A7" }),
            },
        ]);

        let model = GraphModel::from_ast(ast);
        let ids = model.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        let unique = ids.iter().copied().collect::<HashSet<_>>();

        assert_eq!(
            ids.len(),
            unique.len(),
            "display node IDs must be unique for iced-sugiyama indexing"
        );

        let max_id = ids.iter().copied().max().unwrap_or(0);
        assert_eq!(
            max_id as usize + 1,
            ids.len(),
            "display node IDs should be dense for stable layout indexing"
        );

        for (from, to) in &model.edges {
            assert!(
                unique.contains(from),
                "edge source must reference a known node"
            );
            assert!(
                unique.contains(to),
                "edge destination must reference a known node"
            );
        }

        for cluster in &model.while_clusters {
            for node in &cluster.nodes {
                assert!(
                    unique.contains(node),
                    "cluster node must reference a known node"
                );
            }
        }
    }

    #[test]
    fn graph_model_control_flow_edges_match_runtime_shape() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::While {
                label: "flow::LoopCondition",
                metadata: "",
                body: Box::new(JourneyAst::Conditional {
                    label: "flow::Branch",
                    metadata: "",
                    left: Box::new(JourneyAst::Step { label: "LoopL" }),
                    right: Box::new(JourneyAst::Step { label: "LoopR" }),
                }),
            },
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "JoinL" }),
                right: Box::new(JourneyAst::Step { label: "JoinR" }),
            },
            JourneyAst::Select {
                label: "Select",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "SelL" }),
                right: Box::new(JourneyAst::Step { label: "SelR" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]);

        let model = GraphModel::from_ast(ast);

        let id_for = |label: &str| -> u32 {
            let mut matches = model
                .nodes
                .iter()
                .filter(|node| node.label == label)
                .map(|node| node.id);
            let id = matches
                .next()
                .unwrap_or_else(|| panic!("missing node with label {label}"));
            assert!(
                matches.next().is_none(),
                "expected unique node label in test: {label}"
            );
            id
        };

        let branch_id = id_for("Branch");
        let loop_l_id = id_for("LoopL");
        let loop_r_id = id_for("LoopR");
        let join_l_id = id_for("JoinL");
        let join_r_id = id_for("JoinR");
        let sel_l_id = id_for("SelL");
        let sel_r_id = id_for("SelR");
        let tail_id = id_for("Tail");

        assert!(
            model.nodes.iter().all(|node| node.label != "LoopCondition"),
            "while loops should not render as standalone nodes"
        );
        assert!(
            model.nodes.iter().all(|node| node.label != "Select"),
            "select steps should not render as standalone nodes"
        );
        assert!(
            model.nodes.iter().all(|node| node.label != "Join"),
            "join steps should not render as standalone nodes"
        );

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();

        assert!(edges.contains(&(branch_id, loop_l_id)));
        assert!(edges.contains(&(branch_id, loop_r_id)));
        assert!(edges.contains(&(loop_l_id, branch_id)));
        assert!(edges.contains(&(loop_r_id, branch_id)));
        assert!(edges.contains(&(loop_l_id, join_l_id)));
        assert!(edges.contains(&(loop_l_id, join_r_id)));
        assert!(edges.contains(&(loop_r_id, join_l_id)));
        assert!(edges.contains(&(loop_r_id, join_r_id)));
        assert!(!edges.contains(&(branch_id, join_l_id)));
        assert!(!edges.contains(&(branch_id, join_r_id)));

        assert!(edges.contains(&(join_l_id, sel_l_id)));
        assert!(edges.contains(&(join_l_id, sel_r_id)));
        assert!(edges.contains(&(join_r_id, sel_l_id)));
        assert!(edges.contains(&(join_r_id, sel_r_id)));

        assert!(edges.contains(&(sel_l_id, tail_id)));
        assert!(edges.contains(&(sel_r_id, tail_id)));
        assert!(!edges.contains(&(join_l_id, tail_id)));
        assert!(!edges.contains(&(join_r_id, tail_id)));
    }

    #[test]
    fn while_cluster_stays_scoped_to_loop_body() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::While {
                label: "flow::LoopCondition",
                metadata: "",
                body: Box::new(JourneyAst::Conditional {
                    label: "flow::StaticCondition",
                    metadata: "",
                    left: Box::new(JourneyAst::Step { label: "InLoopL" }),
                    right: Box::new(JourneyAst::Step { label: "InLoopR" }),
                }),
            },
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "OutJoinL" }),
                right: Box::new(JourneyAst::Step { label: "OutJoinR" }),
            },
            JourneyAst::Select {
                label: "Select",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "OutSelL" }),
                right: Box::new(JourneyAst::Step { label: "OutSelR" }),
            },
        ]);

        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };

        let cond_id = id_for("StaticCondition");
        let in_l_id = id_for("InLoopL");
        let in_r_id = id_for("InLoopR");
        let out_join_l_id = id_for("OutJoinL");
        let out_join_r_id = id_for("OutJoinR");
        assert!(
            model.nodes.iter().all(|node| node.label != "Select"),
            "select steps should not render as standalone nodes"
        );
        assert!(
            model.nodes.iter().all(|node| node.label != "Join"),
            "join steps should not render as standalone nodes"
        );

        assert_eq!(model.while_clusters.len(), 1);
        let cluster = &model.while_clusters[0];
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        assert!(cluster_nodes.contains(&cond_id));
        assert!(cluster_nodes.contains(&in_l_id));
        assert!(cluster_nodes.contains(&in_r_id));
        assert!(!cluster_nodes.contains(&out_join_l_id));
        assert!(!cluster_nodes.contains(&out_join_r_id));
        assert_eq!(model.while_cluster_labels, vec!["while: LoopCondition"]);

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();
        assert!(edges.contains(&(cond_id, in_l_id)));
        assert!(edges.contains(&(cond_id, in_r_id)));
        assert!(edges.contains(&(in_l_id, cond_id)));
        assert!(edges.contains(&(in_r_id, cond_id)));
        assert!(edges.contains(&(in_l_id, out_join_l_id)));
        assert!(edges.contains(&(in_l_id, out_join_r_id)));
        assert!(edges.contains(&(in_r_id, out_join_l_id)));
        assert!(edges.contains(&(in_r_id, out_join_r_id)));
        assert!(!edges.contains(&(cond_id, out_join_l_id)));
        assert!(!edges.contains(&(cond_id, out_join_r_id)));
    }

    #[test]
    fn while_loop_exit_edges_do_not_attach_to_loop_entry_step() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step {
                label: "GorillaBirthday",
            },
            JourneyAst::While {
                label: "flow::GorillaDaylightRemaining",
                metadata: "",
                body: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step {
                        label: "EvaluateActivityWindow",
                    },
                    JourneyAst::Conditional {
                        label: "flow::GorillaIsActiveNow",
                        metadata: "",
                        left: Box::new(JourneyAst::Step { label: "DoDay" }),
                        right: Box::new(JourneyAst::Step { label: "RestDay" }),
                    },
                    JourneyAst::Step {
                        label: "TickPerceivedTime",
                    },
                ])),
            },
            JourneyAst::Step {
                label: "AdvanceAge",
            },
        ]);

        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };

        let birthday_id = id_for("GorillaBirthday");
        let evaluate_id = id_for("EvaluateActivityWindow");
        let active_now_id = id_for("GorillaIsActiveNow");
        let tick_id = id_for("TickPerceivedTime");
        let advance_age_id = id_for("AdvanceAge");

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();
        assert!(edges.contains(&(birthday_id, evaluate_id)));
        assert!(edges.contains(&(evaluate_id, active_now_id)));
        assert!(edges.contains(&(tick_id, advance_age_id)));
        assert!(!edges.contains(&(evaluate_id, advance_age_id)));
    }

    #[test]
    fn nested_while_clusters_use_parent_relationship() {
        let ast = JourneyAst::While {
            label: "flow::OuterLoop",
            metadata: "",
            body: Box::new(JourneyAst::While {
                label: "flow::InnerLoop",
                metadata: "",
                body: Box::new(JourneyAst::Step { label: "LoopStep" }),
            }),
        };

        let model = GraphModel::from_ast(ast);

        assert_eq!(model.while_clusters.len(), 2);
        assert_eq!(
            model.while_cluster_labels,
            vec!["while: OuterLoop", "while: InnerLoop"]
        );
        assert_eq!(model.while_clusters[0].parent, None);
        assert_eq!(model.while_clusters[1].parent, Some(0));
        assert!(!model.while_clusters[0].nodes.is_empty());
        assert!(!model.while_clusters[1].nodes.is_empty());
    }

    #[test]
    fn transparent_cluster_scopes_body_and_preserves_direct_sequence_edges() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "Start" },
            JourneyAst::Transparent {
                label: "flow::Boundary",
                metadata: "section:gorilla/lifecycle",
                body: Box::new(JourneyAst::Conditional {
                    label: "flow::Gate",
                    metadata: "",
                    left: Box::new(JourneyAst::Step { label: "InL" }),
                    right: Box::new(JourneyAst::Step { label: "InR" }),
                }),
            },
            JourneyAst::Step { label: "Tail" },
        ]);

        let model = GraphModel::from_ast(ast);
        let id_for = |label: &str| -> u32 {
            model
                .nodes
                .iter()
                .find(|node| node.label == label)
                .map(|node| node.id)
                .unwrap_or_else(|| panic!("missing node with label {label}"))
        };

        let start_id = id_for("Start");
        let gate_id = id_for("Gate");
        let in_l_id = id_for("InL");
        let in_r_id = id_for("InR");
        let tail_id = id_for("Tail");

        assert!(
            model
                .nodes
                .iter()
                .all(|node| !node.label.contains("Boundary")),
            "transparent boundaries should not render as standalone nodes"
        );

        assert_eq!(model.while_clusters.len(), 1);
        assert_eq!(
            model.while_cluster_labels,
            vec!["transparent: Boundary :: section:gorilla/lifecycle"]
        );
        let cluster = &model.while_clusters[0];
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        assert!(cluster_nodes.contains(&gate_id));
        assert!(cluster_nodes.contains(&in_l_id));
        assert!(cluster_nodes.contains(&in_r_id));
        assert!(!cluster_nodes.contains(&start_id));
        assert!(!cluster_nodes.contains(&tail_id));

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();
        assert!(edges.contains(&(start_id, gate_id)));
        assert!(edges.contains(&(gate_id, in_l_id)));
        assert!(edges.contains(&(gate_id, in_r_id)));
        assert!(edges.contains(&(in_l_id, tail_id)));
        assert!(edges.contains(&(in_r_id, tail_id)));
    }

    #[test]
    fn cluster_successors_include_downstream_runtime_after_direct_exit() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Transparent {
                label: "flow::Boundary",
                metadata: "",
                body: Box::new(JourneyAst::Step { label: "Inside" }),
            },
            JourneyAst::Step {
                label: "DirectSuccessor",
            },
            JourneyAst::Step {
                label: "DownstreamSuccessor",
            },
        ]);
        let model = GraphModel::from_ast(ast);
        let successor_runtime_ids = cluster_successor_runtime_ids(&model);
        assert_eq!(successor_runtime_ids.len(), 1);
        assert_eq!(successor_runtime_ids[0], vec![1, 2]);
    }

    #[test]
    fn completed_cluster_border_allows_downstream_successor_trigger() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 9,
            cluster_index: 0,
            kind: ClusterKind::Transparent,
            label: "transparent: section",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[18],
            member_runtime_ids: &[18, 19],
            successor_runtime_ids: &[32, 33],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx);

        let entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(18, entry));
        let border_state = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Running);

        let downstream_successor = entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(33, downstream_successor));
        let border_state = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Completed);
    }

    #[test]
    fn while_cluster_border_resets_on_reentry() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 9,
            cluster_index: 0,
            kind: ClusterKind::While,
            label: "while: flow::GorillaDaylightRemaining",
            metadata: None,
            parent_cluster_id: Some(1),
            depth: 1,
            member_display_ids: &[],
            entry_runtime_ids: &[18],
            member_runtime_ids: &[18, 19],
            successor_runtime_ids: &[32],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx);

        let first_entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(18, first_entry));
        let border_state = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Running);

        let first_exit = first_entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(32, first_exit));
        let border_state = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Completed);

        let second_entry = first_exit + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(18, second_entry));
        let border_state = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Running);
    }

    #[test]
    fn while_cluster_reentry_via_non_entry_member_resets_previous_children_to_pending() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 14,
            cluster_index: 0,
            kind: ClusterKind::While,
            label: "while: flow::LyrebirdLoopForever",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[10],
            member_runtime_ids: &[10, 11, 12],
            successor_runtime_ids: &[20],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx);

        let first_entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(10, first_entry));
        assert!(state.update_node_state(10, RuntimeState::Completed));
        assert!(state.update_node_state(11, RuntimeState::Completed));
        assert!(state.update_node_state(12, RuntimeState::Completed));

        let first_exit = first_entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(20, first_exit));
        assert_eq!(
            state
                .cluster_visuals
                .get(&14)
                .expect("cluster visual should exist")
                .border_state,
            RuntimeState::Completed
        );

        let second_member = first_exit + Duration::from_millis(1);
        assert!(state.update_node_state(11, RuntimeState::Running));
        assert!(state.update_clusters_for_effect_input(11, second_member));

        assert_eq!(
            state
                .node_visuals
                .get(&10)
                .expect("entry node should exist")
                .state,
            RuntimeState::Pending
        );
        assert_eq!(
            state
                .node_visuals
                .get(&11)
                .expect("running node should exist")
                .state,
            RuntimeState::Running
        );
        assert_eq!(
            state
                .node_visuals
                .get(&12)
                .expect("other child node should exist")
                .state,
            RuntimeState::Pending
        );
        assert_eq!(
            state
                .cluster_visuals
                .get(&14)
                .expect("cluster visual should exist")
                .border_state,
            RuntimeState::Running
        );
    }

    #[test]
    fn completed_cluster_recollapses_when_successor_returns_to_pending() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 12,
            cluster_index: 0,
            kind: ClusterKind::Transparent,
            label: "transparent: section",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[70],
            member_runtime_ids: &[70, 71],
            successor_runtime_ids: &[95],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx);

        let entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(70, entry));
        assert!(
            state
                .cluster_visuals
                .get(&12)
                .expect("cluster visual should exist")
                .expanded
        );

        let exit = entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(95, exit));
        let border_state = state
            .cluster_visuals
            .get(&12)
            .expect("cluster visual should exist")
            .border_state;
        assert_eq!(border_state, RuntimeState::Completed);

        let successor_completed = exit + Duration::from_millis(1);
        assert!(state.update_node_state(95, RuntimeState::Completed));
        let successor_pending = successor_completed + Duration::from_millis(1);
        assert!(state.update_node_state(95, RuntimeState::Pending));

        let collapse_cx = ClusterViewCtx {
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: true,
            }),
            ..cx.clone()
        };
        assert!(
            !state.maybe_collapse_completed_cluster_for_pending_successor(
                &collapse_cx,
                successor_pending + Duration::from_millis(1)
            )
        );
        assert!(
            state.maybe_collapse_completed_cluster_for_pending_successor(
                &collapse_cx,
                successor_pending + CLUSTER_RECOLLAPSE_DELAY + Duration::from_millis(1)
            )
        );
        let visual = state
            .cluster_visuals
            .get(&12)
            .expect("cluster visual should exist");
        assert!(!visual.expanded);
        assert_eq!(visual.border_state, RuntimeState::Pending);
    }

    #[test]
    fn completed_cluster_does_not_recollapse_while_still_running() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 33,
            cluster_index: 0,
            kind: ClusterKind::While,
            label: "while: loop",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[10],
            member_runtime_ids: &[10, 11],
            successor_runtime_ids: &[22],
            phase: Phase::Live(ClusterLive {
                has_running: true,
                has_failed: false,
                has_completed: true,
            }),
        };
        state.register_cluster(&cx);
        let initial_visual = state
            .cluster_visuals
            .get(&33)
            .expect("cluster visual should exist");
        assert!(initial_visual.expanded);
        assert_eq!(initial_visual.border_state, RuntimeState::Running);

        assert!(!state.update_clusters_for_effect_input(10, started_at + Duration::from_millis(1)));
        assert!(state.update_clusters_for_effect_input(22, started_at + Duration::from_millis(2)));

        assert!(
            !state.maybe_collapse_completed_cluster_for_pending_successor(
                &cx,
                started_at + Duration::from_millis(3)
            )
        );
        assert!(
            state
                .cluster_visuals
                .get(&33)
                .expect("cluster visual should exist")
                .expanded
        );
    }

    #[test]
    fn running_cluster_registers_as_expanded_without_prior_effect_input() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig::default());

        let cx = ClusterViewCtx {
            cluster_id: 77,
            cluster_index: 0,
            kind: ClusterKind::While,
            label: "while: loop",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[10],
            member_runtime_ids: &[10, 11],
            successor_runtime_ids: &[22],
            phase: Phase::Live(ClusterLive {
                has_running: true,
                has_failed: false,
                has_completed: false,
            }),
        };

        state.register_cluster(&cx);

        let visual = state
            .cluster_visuals
            .get(&77)
            .expect("cluster visual should exist");
        assert!(visual.expanded);
        assert_eq!(visual.border_state, RuntimeState::Running);
    }

    #[test]
    fn always_expanded_cluster_config_keeps_cluster_open() {
        let mut state = DefaultThemeState::new(ClusterExpansionConfig {
            while_clusters: ClusterExpansionMode::AlwaysExpanded,
            transparent_clusters: ClusterExpansionMode::AlwaysExpanded,
        });

        let started_at = Instant::now();
        let cx = ClusterViewCtx {
            cluster_id: 55,
            cluster_index: 0,
            kind: ClusterKind::Transparent,
            label: "transparent: section",
            metadata: None,
            parent_cluster_id: None,
            depth: 0,
            member_display_ids: &[],
            entry_runtime_ids: &[70],
            member_runtime_ids: &[70, 71],
            successor_runtime_ids: &[95],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };

        state.register_cluster(&cx);
        assert!(
            state
                .cluster_visuals
                .get(&55)
                .expect("cluster visual should exist")
                .expanded
        );

        let entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(70, entry));
        let exit = entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(95, exit));

        let collapse_cx = ClusterViewCtx {
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: true,
            }),
            ..cx.clone()
        };
        assert!(
            !state.maybe_collapse_completed_cluster_for_pending_successor(
                &collapse_cx,
                exit + CLUSTER_RECOLLAPSE_DELAY + Duration::from_millis(1)
            )
        );
        let visual = state
            .cluster_visuals
            .get(&55)
            .expect("cluster visual should exist");
        assert!(visual.expanded);
        assert_eq!(visual.border_state, RuntimeState::Completed);
    }
}
