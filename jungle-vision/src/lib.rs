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
#[cfg(test)]
use jungle_core::dag::NodeDisplay;
use jungle_core::dag::{Dag as GraphModel, DagProjection, DagSnapshot, LiveDagState as LiveData};
#[cfg(test)]
use jungle_types::JourneyAst;
use jungle_types::{
    Animal, JourneyAstSource, JourneyUpdateEvent, NodeLifecyclePhase, RunnerUpdateOut,
};
use std::collections::{HashMap, HashSet};
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
const INITIAL_LIVE_BATCH_WINDOW: Duration = Duration::from_millis(20);

static CLUSTER_FILL_COLORS: OnceLock<RwLock<Vec<Color>>> = OnceLock::new();
static VISION_LIVE_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static VISION_APPLY_EVENT_COUNT: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_APPLY_QUEUE_DELAY_MS: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_END_TO_END_EVENT_AGE_MS: AtomicUsize = AtomicUsize::new(0);
static VISION_MAX_APPLY_ELAPSED_MS: AtomicUsize = AtomicUsize::new(0);

fn graph_refresh_task<Message: Clone + Send + 'static>(
    graph_widget_id: iced_sugiyama::Id,
    invalidate_layout: bool,
) -> Task<Message> {
    if invalidate_layout {
        iced_sugiyama::invalidate::<Message>(graph_widget_id)
    } else {
        // Some live events only advance sequence/activation-path state. Those still affect
        // repaired node phases, so the widget must review its cached node rendering.
        iced_sugiyama::force_review::<Message>(graph_widget_id)
    }
}

pub struct AnyAnimal;
pub use jungle_core::dag::{ClusterKind, ClusterLive, Phase, RuntimeState, StepKind};

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
            ClusterKind::Join => self.transparent_clusters,
            ClusterKind::Transparent => self.transparent_clusters,
            ClusterKind::Attempt => self.transparent_clusters,
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
                let model = match &self.mode {
                    ViewMode::Static { model, .. } | ViewMode::Live { model, .. } => model,
                };
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
                data.bind_model(model);
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
                graph_refresh_task(self.graph_widget_id.clone(), highlight_changed)
                    .chain(theme_task)
            }
            EjectedViewerMessage::ApplyLiveEvent {
                update,
                received_unix_ms,
            } => {
                let model = match &self.mode {
                    ViewMode::Static { model, .. } | ViewMode::Live { model, .. } => model,
                };
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
                data.bind_model(model);
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
                graph_refresh_task(self.graph_widget_id.clone(), highlight_changed)
                    .chain(theme_task)
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
pub struct DebugRenderStateNode {
    pub id: u32,
    pub label: String,
    pub runtime_id: Option<u32>,
    pub state: RuntimeState,
}

#[derive(Debug, Clone)]
pub struct DebugRuntimeDecisionNode {
    pub id: u32,
    pub label: String,
    pub runtime_id: Option<u32>,
    pub state: RuntimeState,
    pub sequence: Option<usize>,
    pub floor: Option<usize>,
    pub activation_path: Option<Vec<u64>>,
    pub required_prefix: Option<Vec<u64>>,
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

pub fn debug_plain_layout_for_animal<A>() -> String
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
{
    let ast = <A::Flow as JourneyAstSource>::journey_ast();
    let model = GraphModel::from_ast(ast);
    let nodes = model.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    let clusters = model
        .clusters
        .iter()
        .map(|cluster| {
            let mut converted = iced_sugiyama::Cluster::new(cluster.nodes.clone());
            if let Some(padding) = cluster.padding {
                converted = converted.padding(padding.into());
            }
            if let Some(parent) = cluster.parent {
                converted = converted.parent(parent);
            }
            converted
        })
        .collect::<Vec<_>>();
    iced_sugiyama::graphviz_plain_layout(
        &nodes,
        &model.edges,
        &rust_sugiyama::Config::default(),
        |_id| (NODE_WIDTH, NODE_HEIGHT),
        |id| {
            model
                .node_map
                .get(&id)
                .map(|node| node.label.clone())
                .unwrap_or_else(|| format!("node-{id}"))
        },
        |_edge_index, _edge| None,
        &clusters,
        &rust_sugiyama::RenderConfig::default(),
        |index, _cluster| {
            model
                .cluster_info
                .get(index)
                .map(|cluster| cluster.label.clone())
                .unwrap_or_else(|| format!("cluster-{index}"))
        },
    )
}

pub fn debug_render_states_for_animal<A>(
    updates: impl IntoIterator<Item = JourneyUpdateEvent>,
) -> Vec<DebugRenderStateNode>
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
{
    let ast = <A::Flow as JourneyAstSource>::journey_ast();
    let model = GraphModel::from_ast(ast);
    let mut live = LiveData::default();
    live.bind_model(&model);
    for update in updates {
        let _ = live.apply_update(update);
    }
    let snapshot = DagSnapshot::new(&model, Some(&live));
    model
        .nodes
        .iter()
        .map(|node| DebugRenderStateNode {
            id: node.id,
            label: node.label.clone(),
            runtime_id: node.runtime_node_id,
            state: snapshot
                .node_states
                .get(&node.id)
                .copied()
                .unwrap_or(RuntimeState::Pending),
        })
        .collect()
}

pub fn debug_runtime_decisions_for_animal<A>(
    updates: impl IntoIterator<Item = JourneyUpdateEvent>,
) -> Vec<DebugRuntimeDecisionNode>
where
    A: Animal + 'static,
    A::Flow: JourneyAstSource,
{
    let ast = <A::Flow as JourneyAstSource>::journey_ast();
    let model = GraphModel::from_ast(ast);
    let mut live = LiveData::default();
    live.bind_model(&model);
    for update in updates {
        let _ = live.apply_update(update);
    }
    let snapshot = DagSnapshot::new(&model, Some(&live));

    model
        .nodes
        .iter()
        .map(|node| DebugRuntimeDecisionNode {
            id: node.id,
            label: node.label.clone(),
            runtime_id: node.runtime_node_id,
            state: snapshot
                .node_states
                .get(&node.id)
                .copied()
                .unwrap_or(RuntimeState::Pending),
            sequence: node
                .runtime_node_id
                .and_then(|id| live.runtime_update_sequence.get(&id).copied()),
            floor: node
                .runtime_node_id
                .and_then(|id| snapshot.runtime_sequence_floors.get(&id).copied()),
            activation_path: node
                .runtime_node_id
                .and_then(|id| live.runtime_activation_paths.get(&id).cloned()),
            required_prefix: node
                .runtime_node_id
                .and_then(|id| snapshot.runtime_activation_prefixes.get(&id).cloned()),
        })
        .collect()
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
                let model = match &self.mode {
                    ViewMode::Static { model, .. } | ViewMode::Live { model, .. } => model,
                };
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
                data.bind_model(model);
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
                graph_refresh_task(self.graph_widget_id.clone(), highlight_changed)
                    .chain(theme_task)
            }
            Message::ApplyLiveEvent(update) => {
                let model = match &self.mode {
                    ViewMode::Static { model, .. } | ViewMode::Live { model, .. } => model,
                };
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
                data.bind_model(model);
                let highlight_changed = data.apply_update(update);
                graph_refresh_task(self.graph_widget_id.clone(), highlight_changed)
                    .chain(theme_task)
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

#[cfg(test)]
fn runtime_state_for_live_data(
    live: &LiveData,
    runtime_id: u32,
    runtime_sequence_floors: &HashMap<u32, usize>,
) -> RuntimeState {
    runtime_state_for_live_data_with_activation_prefixes(
        live,
        runtime_id,
        runtime_sequence_floors,
        &HashMap::new(),
    )
}

#[cfg(test)]
fn runtime_state_for_live_data_with_activation_prefixes(
    live: &LiveData,
    runtime_id: u32,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> RuntimeState {
    if live
        .runtime_update_sequence
        .get(&runtime_id)
        .copied()
        .zip(runtime_sequence_floors.get(&runtime_id).copied())
        .map(|(sequence, floor)| sequence < floor)
        .unwrap_or(false)
    {
        return RuntimeState::Pending;
    }
    if runtime_activation_prefixes
        .get(&runtime_id)
        .and_then(|required_prefix| {
            live.runtime_activation_paths
                .get(&runtime_id)
                .map(|path| !path.starts_with(required_prefix))
        })
        .unwrap_or(false)
    {
        return RuntimeState::Pending;
    }
    if live.failed_runtime_ids.contains(&runtime_id) {
        RuntimeState::Failed
    } else if live.active_runtime_ids.contains(&runtime_id) {
        RuntimeState::Running
    } else if live.finished_runtime_ids.contains(&runtime_id) {
        RuntimeState::Completed
    } else {
        RuntimeState::Pending
    }
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

#[cfg(test)]
fn runtime_activation_prefixes_for_display(
    model: &GraphModel,
    live: &LiveData,
) -> HashMap<u32, Vec<u64>> {
    DagSnapshot::new(model, Some(live)).runtime_activation_prefixes
}

#[cfg(test)]
fn repaired_live_states_for_display(
    model: &GraphModel,
    live_data: Option<&LiveData>,
    _condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> HashMap<u32, RuntimeState> {
    DagSnapshot::new(model, live_data).repaired_node_states
}

#[cfg(test)]
fn runtime_sequence_floors_for_display(model: &GraphModel, live: &LiveData) -> HashMap<u32, usize> {
    DagSnapshot::new(model, Some(live)).runtime_sequence_floors
}

#[cfg(test)]
fn live_states_for_display(
    model: &GraphModel,
    live_data: Option<&LiveData>,
    _condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> HashMap<u32, RuntimeState> {
    DagSnapshot::new(model, live_data).node_states
}

#[cfg(test)]
fn cluster_phase_for_display(
    live_data: Option<&LiveData>,
    model: &GraphModel,
    cluster_index: usize,
    _repaired_live_states: &HashMap<u32, RuntimeState>,
    _runtime_sequence_floors: &HashMap<u32, usize>,
    _runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> Phase<ClusterLive> {
    if live_data.is_none() {
        Phase::Static
    } else {
        DagSnapshot::new(model, live_data).cluster_phase(cluster_index)
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
        text("Green: completed").size(12).color(jungle_text_muted()),
        text("Yellow: running").size(12).color(jungle_text_muted()),
        text("Red: failed").size(12).color(jungle_text_muted()),
        text("Gray: pending").size(12).color(jungle_text_muted()),
    ]
    .spacing(2);

    container(column![info, Space::new().height(18), legend].spacing(0))
        .style(sidebar_style)
        .padding(16)
        .width(280)
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
    let condition_successor_runtime_ids = &model.derived.condition_successor_runtime_ids;
    let cluster_member_runtime_ids = &model.derived.cluster_member_runtime_ids;
    let cluster_successor_runtime_ids = &model.derived.cluster_successor_runtime_ids;
    let cluster_entry_runtime_ids = &model.derived.cluster_entry_runtime_ids;
    let runtime_by_display_id = &model.derived.runtime_by_display_id;
    let proxy_runtime_ids_by_display_id = &model.derived.proxy_runtime_ids_by_display_id;
    let snapshot = Arc::new(DagSnapshot::new(model, live_data));

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
            phase: snapshot.cluster_phase(index),
        };
        if matches!(
            theme.view_cluster(theme_state, &cx),
            ClusterView::Collapsed { .. }
        ) {
            collapsed_clusters.insert(index);
        }
    }

    let projection = DagProjection::new(model, &collapsed_clusters);
    let mut node_sizes = HashMap::<u32, (f64, f64)>::new();

    for node in &model.nodes {
        if !projection.visible_real_nodes.contains(&node.id) {
            continue;
        }
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
            phase: snapshot.node_phase(node.id),
        };
        let (element, size) = theme.view_step(theme_state, &step_ctx);
        let _ = element;
        node_sizes.insert(node.id, size);
    }

    for (display_id, index) in &projection.collapsed_cluster_by_display {
        let cluster = &model.cluster_info[*index];
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: *index,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: cluster
                .parent
                .and_then(|parent| model.cluster_info.get(parent).map(|info| info.id)),
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &cluster_entry_runtime_ids[*index],
            member_runtime_ids: &cluster_member_runtime_ids[*index],
            successor_runtime_ids: &cluster_successor_runtime_ids[*index],
            phase: snapshot.cluster_phase(*index),
        };
        if let ClusterView::Collapsed { element, size } = theme.view_cluster(theme_state, &cx) {
            let _ = element;
            node_sizes.insert(*display_id, size);
        }
    }

    let graph = Graph::new(projection.nodes.clone(), projection.edges.clone());

    let mut visible_clusters = Vec::<Cluster>::new();
    let mut visible_cluster_source_indices = Vec::<usize>::new();
    let mut visible_cluster_fills = Vec::<Color>::new();
    for projected_cluster in &projection.visible_clusters {
        let source_index = projected_cluster.source_index;
        let cluster = &model.cluster_info[source_index];
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
            phase: snapshot.cluster_phase(source_index),
        };
        let ClusterView::Expanded { overlay, fill } = theme.view_cluster(theme_state, &cx) else {
            continue;
        };
        let mut spec = Cluster::new(projected_cluster.member_nodes.clone())
            .padding(projected_cluster.padding.into());
        if let Some(parent_visible) = projected_cluster.parent_visible_index {
            spec = spec.parent(parent_visible);
        }
        visible_clusters.push(spec);
        visible_cluster_fills.push(fill);
        let _ = overlay;
        visible_cluster_source_indices.push(source_index);
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
        let collapsed_display_map = projection.collapsed_cluster_by_display.clone();
        let visible_nodes = projection.visible_real_nodes.clone();
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
        let snapshot_for_nodes = snapshot.clone();
        let snapshot_for_edge_colors = snapshot.clone();
        let snapshot_for_edge_strokes = snapshot.clone();
        let snapshot_for_cluster_chips = snapshot.clone();
        let snapshot_for_cluster_overlays = snapshot.clone();
        let mut widget = Sugiyama::<Message, iced::Theme, iced::Renderer>::new(
            std::borrow::Cow::Owned(graph.clone()),
            move |node_id| {
                if visible_nodes.contains(&node_id) {
                    if let Some(node) = node_map.get(&node_id) {
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
                            phase: snapshot_for_nodes.node_phase(node.id),
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
                            phase: snapshot_for_cluster_chips.cluster_phase(cluster_index),
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
        .layout_fn(iced_sugiyama::microdot_layout)
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
                        source_phase: snapshot_for_edge_colors.node_phase(ctx.edge.0),
                        target_phase: snapshot_for_edge_colors.node_phase(ctx.edge.1),
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
                        source_phase: snapshot_for_edge_strokes.node_phase(ctx.edge.0),
                        target_phase: snapshot_for_edge_strokes.node_phase(ctx.edge.1),
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
                phase: snapshot_for_cluster_overlays.cluster_phase(source_index),
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

#[cfg(test)]
fn cluster_successor_runtime_ids(model: &GraphModel) -> Vec<Vec<u32>> {
    model.derived.cluster_successor_runtime_ids.clone()
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
        let now = Instant::now();
        let expansion_mode = self.cluster_expansion.mode_for(cx.kind);
        let expanded = matches!(expansion_mode, ClusterExpansionMode::AlwaysExpanded)
            || matches!(cx.phase, Phase::Live(live) if live.has_running || live.has_failed);
        let border_state = match cx.phase {
            Phase::Live(live) if live.has_failed => RuntimeState::Failed,
            Phase::Live(live) if live.has_running => RuntimeState::Running,
            Phase::Live(live) if live.has_completed => RuntimeState::Completed,
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
        let visual = self
            .cluster_visuals
            .entry(cx.cluster_id)
            .or_insert(ClusterVisual {
                expanded,
                border_state,
                completed_at: None,
            });
        if matches!(expansion_mode, ClusterExpansionMode::AlwaysExpanded) {
            visual.expanded = true;
        } else if matches!(border_state, RuntimeState::Running | RuntimeState::Failed) {
            visual.expanded = true;
        }
        if visual.border_state != border_state {
            visual.completed_at = if matches!(border_state, RuntimeState::Completed) {
                Some(now)
            } else {
                None
            };
            visual.border_state = border_state;
        }
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

    fn apply_force_pending_override(
        &mut self,
        runtime_id: u32,
        phase_target: RuntimeState,
    ) -> RuntimeState {
        if !self.force_pending_runtime_ids.contains(&runtime_id) {
            return phase_target;
        }

        match phase_target {
            RuntimeState::Pending => RuntimeState::Pending,
            RuntimeState::Running => RuntimeState::Running,
            RuntimeState::Completed | RuntimeState::Failed => {
                self.force_pending_runtime_ids.remove(&runtime_id);
                phase_target
            }
        }
    }

    fn maybe_collapse_completed_cluster_for_pending_successor(
        &mut self,
        cx: &ClusterViewCtx<'_>,
        now: Instant,
    ) -> bool {
        if !matches!(
            cx.kind,
            ClusterKind::While
                | ClusterKind::Join
                | ClusterKind::Transparent
                | ClusterKind::Attempt
        ) {
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
        match event {
            ViewerEvent::JourneyUpdate(update) => {
                let guard = state.get_mut();
                let now = Instant::now();
                match update.event {
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
                    RunnerUpdateOut::NodeLifecycle(node) => match node.phase {
                        NodeLifecyclePhase::Entered => {
                            let _ = guard.update_node_state(node.node_id, RuntimeState::Running);
                            let _ = guard.update_clusters_for_effect_input(node.node_id, now);
                        }
                        NodeLifecyclePhase::Succeeded => {
                            let _ = guard.update_node_state(node.node_id, RuntimeState::Completed);
                        }
                        NodeLifecyclePhase::Failed => {
                            let _ = guard.update_node_state(node.node_id, RuntimeState::Failed);
                        }
                    },
                    RunnerUpdateOut::SleepScheduled { .. }
                    | RunnerUpdateOut::SleepFired { .. }
                    | RunnerUpdateOut::PerturbationApplied { .. } => {}
                }
            }
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
            let phase_target = match cx.phase {
                Phase::Live(target) => target,
                Phase::Static => RuntimeState::Pending,
            };
            let phase_target = if let Ok(mut guard) = state.try_lock() {
                guard.apply_force_pending_override(runtime_id, phase_target)
            } else {
                phase_target
            };
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
        let source_phase = match cx.source_phase {
            Phase::Live(target) => target,
            Phase::Static => RuntimeState::Pending,
        };
        let source_phase = if let Some(runtime_id) = cx.source_runtime_id {
            if let Ok(mut guard) = state.try_lock() {
                guard.apply_force_pending_override(runtime_id, source_phase)
            } else {
                source_phase
            }
        } else {
            source_phase
        };
        let target_phase = match cx.target_phase {
            Phase::Live(target) => target,
            Phase::Static => RuntimeState::Pending,
        };
        let target_phase = if let Some(runtime_id) = cx.target_runtime_id {
            if let Ok(mut guard) = state.try_lock() {
                guard.apply_force_pending_override(runtime_id, target_phase)
            } else {
                target_phase
            }
        } else {
            target_phase
        };
        let phase_target = match target_phase {
            RuntimeState::Pending => match source_phase {
                RuntimeState::Running | RuntimeState::Failed => source_phase,
                RuntimeState::Completed | RuntimeState::Pending => RuntimeState::Pending,
            },
            RuntimeState::Running | RuntimeState::Completed | RuntimeState::Failed => target_phase,
        };
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
        background: Some(iced::Background::Color(Color::from_rgba8(7, 17, 11, 0.0))),
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
    use uuid::Uuid;

    fn empty_model() -> GraphModel {
        GraphModel::from_ast(JourneyAst::Empty)
    }

    fn node_by_label<'a>(model: &'a GraphModel, label: &str) -> &'a NodeDisplay {
        model
            .nodes
            .iter()
            .find(|node| node.label == label)
            .unwrap_or_else(|| panic!("missing node with label {label}"))
    }

    fn runtime_id_for(model: &GraphModel, label: &str) -> u32 {
        node_by_label(model, label)
            .runtime_node_id
            .unwrap_or_else(|| panic!("missing runtime id for {label}"))
    }

    fn cluster_index_for_kind(model: &GraphModel, kind: ClusterKind) -> usize {
        model
            .cluster_info
            .iter()
            .position(|cluster| cluster.kind == kind)
            .unwrap_or_else(|| panic!("missing cluster for kind {kind:?}"))
    }

    #[test]
    fn live_data_apply_update_reports_runtime_highlight_changes() {
        let model = empty_model();
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

        live.bind_model(&model);
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.finished_runtime_ids.contains(&9));
        assert_eq!(live.latest_event_count, 3);
    }

    #[test]
    fn lifecycle_state_stays_authoritative_over_effect_outputs() {
        let model = GraphModel::from_ast(JourneyAst::Step { label: "A" });
        let runtime_id = runtime_id_for(&model, "A");
        let mut live = LiveData::default();
        live.bind_model(&model);

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(live.active_runtime_ids.contains(&runtime_id));
        assert!(!live.finished_runtime_ids.contains(&runtime_id));

        assert!(!live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: runtime_id,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.active_runtime_ids.contains(&runtime_id));
        assert!(!live.finished_runtime_ids.contains(&runtime_id));

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Succeeded,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(!live.active_runtime_ids.contains(&runtime_id));
        assert!(live.finished_runtime_ids.contains(&runtime_id));
    }

    #[test]
    fn runtime_sequence_floors_hide_stale_runtime_state() {
        let mut live = LiveData::default();
        live.finished_runtime_ids.insert(7);
        live.runtime_update_sequence.insert(7, 1);

        assert_eq!(
            runtime_state_for_live_data(&live, 7, &HashMap::from([(7, 2)])),
            RuntimeState::Pending
        );
        assert_eq!(
            runtime_state_for_live_data(&live, 7, &HashMap::from([(7, 1)])),
            RuntimeState::Completed
        );
    }

    #[test]
    fn while_runtime_entry_advances_runtime_floors_for_effect_only_members() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;

        let mut live = LiveData::default();
        live.finished_runtime_ids.insert(a_runtime_id);
        live.finished_runtime_ids.insert(b_runtime_id);
        live.runtime_update_sequence.insert(a_runtime_id, 1);
        live.runtime_update_sequence.insert(b_runtime_id, 2);
        live.active_runtime_ids.insert(loop_runtime_id);
        live.runtime_update_sequence.insert(loop_runtime_id, 3);

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(states.get(&a_id).copied(), Some(RuntimeState::Pending));
        assert_eq!(states.get(&b_id).copied(), Some(RuntimeState::Pending));
    }

    #[test]
    fn while_runtime_floors_hide_stale_members_with_activation_paths_after_reentry() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id, activation_path, phase) in [
            (1, loop_runtime_id, vec![0], NodeLifecyclePhase::Entered),
            (2, a_runtime_id, vec![0, 0], NodeLifecyclePhase::Succeeded),
            (3, b_runtime_id, vec![0, 0], NodeLifecyclePhase::Succeeded),
            (4, a_runtime_id, vec![0, 1], NodeLifecyclePhase::Entered),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path,
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }
        let floors = runtime_sequence_floors_for_display(&model, &live);
        assert_eq!(floors.get(&a_runtime_id).copied(), Some(4));
        assert_eq!(floors.get(&b_runtime_id).copied(), Some(4));
        assert_eq!(floors.get(&loop_runtime_id).copied(), Some(4));

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(states.get(&a_id).copied(), Some(RuntimeState::Running));
        assert_eq!(states.get(&b_id).copied(), Some(RuntimeState::Pending));
    }

    #[test]
    fn while_reentry_hides_stale_skip_branch_activity() {
        let model = GraphModel::from_ast(JourneyAst::While {
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
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let begin_runtime_id = runtime_id_for(&model, "Begin");
        let branch_runtime_id = runtime_id_for(&model, "Branch");
        let select_runtime_id = runtime_id_for(&model, "Select");
        let optimize_runtime_id = runtime_id_for(&model, "Optimize");
        let skip_runtime_id = runtime_id_for(&model, "Skip");

        let select_id = node_by_label(&model, "Select").id;
        let optimize_id = node_by_label(&model, "Optimize").id;
        let skip_id = node_by_label(&model, "Skip").id;
        let flatten_id = node_by_label(&model, "Flatten").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id, activation_path, phase) in [
            (1, loop_runtime_id, vec![0], NodeLifecyclePhase::Entered),
            (
                2,
                begin_runtime_id,
                vec![0, 0],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                3,
                branch_runtime_id,
                vec![0, 1],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                4,
                skip_runtime_id,
                vec![0, 2],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                5,
                begin_runtime_id,
                vec![1, 0],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                6,
                branch_runtime_id,
                vec![1, 1],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                7,
                select_runtime_id,
                vec![1, 2],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                8,
                optimize_runtime_id,
                vec![1, 3],
                NodeLifecyclePhase::Entered,
            ),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path,
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
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
    fn while_reentry_allows_effect_updates_after_stale_lifecycle_state_is_cleared() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: a_runtime_id,
                    activation_path: vec![0, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                3,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: b_runtime_id,
                    activation_path: vec![0, 1],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                4,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![1],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                5,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: a_runtime_id,
                    activation_path: vec![1, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                6,
                RunnerUpdateOut::EffectInput {
                    node_id: b_runtime_id,
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
        assert_eq!(states.get(&a_id).copied(), Some(RuntimeState::Completed));
        assert_eq!(states.get(&b_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn node_lifecycle_reentry_clears_stale_descendants() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let a_display_id = node_by_label(&model, "A").id;
        let b_display_id = node_by_label(&model, "B").id;
        let mut live = LiveData::default();
        live.bind_model(&model);

        for (sequence_id, node_id, activation_path, phase) in [
            (1, loop_runtime_id, vec![0], NodeLifecyclePhase::Entered),
            (2, a_runtime_id, vec![0, 0], NodeLifecyclePhase::Succeeded),
            (3, b_runtime_id, vec![0, 0], NodeLifecyclePhase::Succeeded),
            (4, loop_runtime_id, vec![1], NodeLifecyclePhase::Entered),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path,
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            states.get(&a_display_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            states.get(&b_display_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert!(live.active_runtime_ids.contains(&loop_runtime_id));
    }

    #[test]
    fn node_lifecycle_reentry_clears_stale_effect_only_descendants_without_paths() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Prompt" },
                JourneyAst::Step { label: "Finalize" },
                JourneyAst::Step { label: "Sleep" },
            ])),
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let prompt_runtime_id = runtime_id_for(&model, "Prompt");
        let finalize_runtime_id = runtime_id_for(&model, "Finalize");
        let sleep_runtime_id = runtime_id_for(&model, "Sleep");
        let prompt_id = node_by_label(&model, "Prompt").id;
        let finalize_id = node_by_label(&model, "Finalize").id;
        let sleep_id = node_by_label(&model, "Sleep").id;
        let mut live = LiveData::default();
        live.bind_model(&model);

        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: prompt_runtime_id,
                    activation_path: vec![0, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: finalize_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: sleep_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                5,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![1],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                6,
                RunnerUpdateOut::EffectInput {
                    node_id: prompt_runtime_id,
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

        assert!(!live.finished_runtime_ids.contains(&finalize_runtime_id));
        assert!(!live.finished_runtime_ids.contains(&sleep_runtime_id));
        assert!(!live
            .runtime_update_sequence
            .contains_key(&finalize_runtime_id));
        assert!(!live.runtime_update_sequence.contains_key(&sleep_runtime_id));

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(states.get(&prompt_id).copied(), Some(RuntimeState::Running));
        assert_eq!(
            states.get(&finalize_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(states.get(&sleep_id).copied(), Some(RuntimeState::Pending));
    }

    #[test]
    fn cluster_phase_uses_hidden_cluster_lifecycle_state() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Step { label: "A" }),
        });
        let mut live = LiveData::default();
        live.bind_model(&model);

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: model.cluster_info[0].runtime_node_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));

        let repaired = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let phase = cluster_phase_for_display(
            Some(&live),
            &model,
            0,
            &repaired,
            &runtime_sequence_floors_for_display(&model, &live),
            &runtime_activation_prefixes_for_display(&model, &live),
        );

        assert_eq!(
            phase,
            Phase::Live(ClusterLive {
                has_running: true,
                has_failed: false,
                has_completed: false,
            })
        );
    }

    #[test]
    fn attempt_cluster_contains_failure_without_turning_outer_clusters_red() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "OuterLoop",
            metadata: "",
            body: Box::new(JourneyAst::Join {
                label: "OuterJoin",
                metadata: "",
                left: Box::new(JourneyAst::Attempt {
                    label: "Attempt",
                    metadata: "",
                    body: Box::new(JourneyAst::Step { label: "FailStep" }),
                }),
                right: Box::new(JourneyAst::Step { label: "PassStep" }),
            }),
        });
        let while_index = cluster_index_for_kind(&model, ClusterKind::While);
        let join_index = cluster_index_for_kind(&model, ClusterKind::Join);
        let attempt_index = cluster_index_for_kind(&model, ClusterKind::Attempt);
        let fail_display_id = node_by_label(&model, "FailStep").id;
        assert!(model.cluster_info[attempt_index]
            .nodes
            .contains(&fail_display_id));

        let mut live = LiveData::default();
        live.bind_model(&model);

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: model.cluster_info[while_index].runtime_node_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: model.cluster_info[attempt_index].runtime_node_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: runtime_id_for(&model, "PassStep"),
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 4,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id_for(&model, "FailStep"),
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Failed,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: model.cluster_info[attempt_index].runtime_node_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Succeeded,
                uuid: Uuid::nil(),
            }),
        }));

        let repaired = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&fail_display_id).copied(),
            Some(RuntimeState::Failed)
        );
        let runtime_sequence_floors = runtime_sequence_floors_for_display(&model, &live);
        let runtime_activation_prefixes = runtime_activation_prefixes_for_display(&model, &live);

        let while_phase = cluster_phase_for_display(
            Some(&live),
            &model,
            while_index,
            &repaired,
            &runtime_sequence_floors,
            &runtime_activation_prefixes,
        );
        let join_phase = cluster_phase_for_display(
            Some(&live),
            &model,
            join_index,
            &repaired,
            &runtime_sequence_floors,
            &runtime_activation_prefixes,
        );
        let attempt_phase = cluster_phase_for_display(
            Some(&live),
            &model,
            attempt_index,
            &repaired,
            &runtime_sequence_floors,
            &runtime_activation_prefixes,
        );

        assert!(matches!(
            while_phase,
            Phase::Live(ClusterLive {
                has_failed: false,
                ..
            })
        ));
        assert!(matches!(
            join_phase,
            Phase::Live(ClusterLive {
                has_failed: false,
                ..
            })
        ));
        assert!(matches!(
            attempt_phase,
            Phase::Live(ClusterLive {
                has_failed: true,
                ..
            })
        ));
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
        assert_eq!(ids.len(), unique.len());
        assert_eq!(
            ids.iter().copied().max().unwrap_or(0) as usize + 1,
            ids.len()
        );
    }

    #[test]
    fn graph_model_hides_select_and_join_control_nodes() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "Start" },
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
        ]));

        assert!(model.nodes.iter().all(|node| node.label != "Join"));
        assert!(model.nodes.iter().all(|node| node.label != "Select"));

        let join_l = node_by_label(&model, "JoinL");
        let join_r = node_by_label(&model, "JoinR");
        let sel_l = node_by_label(&model, "SelL");
        let sel_r = node_by_label(&model, "SelR");

        let join_runtime_id = join_l
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("JoinL should carry hidden join runtime"));
        assert_eq!(join_r.proxy_runtime_ids, vec![join_runtime_id]);

        let select_runtime_id = sel_l
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("SelL should carry hidden select runtime"));
        assert_eq!(sel_r.proxy_runtime_ids, vec![select_runtime_id]);
    }

    #[test]
    fn graph_model_renders_join_as_cluster() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "Start" },
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "JoinL" }),
                right: Box::new(JourneyAst::Step { label: "JoinR" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));

        let join_cluster = model
            .cluster_info
            .iter()
            .find(|cluster| matches!(cluster.kind, ClusterKind::Join))
            .unwrap_or_else(|| panic!("expected join cluster to be emitted"));

        assert_eq!(join_cluster.label, "join: Join");
        assert_eq!(
            join_cluster.nodes,
            vec![
                node_by_label(&model, "JoinL").id,
                node_by_label(&model, "JoinR").id
            ]
        );
        assert_eq!(join_cluster.root_nodes, join_cluster.nodes);
        assert_eq!(
            join_cluster.root_runtime_ids,
            vec![
                runtime_id_for(&model, "JoinL"),
                runtime_id_for(&model, "JoinR")
            ]
        );
    }

    #[test]
    fn repaired_live_states_backfill_pending_chain_when_downstream_runs() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
            JourneyAst::Step { label: "C" },
        ]);
        let model = GraphModel::from_ast(ast);
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;
        let c_id = node_by_label(&model, "C").id;

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: runtime_id_for(&model, "C"),
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
    fn repaired_live_states_complete_running_predecessor_when_successor_runs() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
        ]);
        let model = GraphModel::from_ast(ast);
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id_for(&model, "A"),
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: runtime_id_for(&model, "B"),
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(repaired.get(&a_id).copied(), Some(RuntimeState::Completed));
        assert_eq!(repaired.get(&b_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_promote_single_ready_successor_when_no_node_is_active() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "A" },
            JourneyAst::Step { label: "B" },
            JourneyAst::Step { label: "C" },
        ]));
        let a_id = node_by_label(&model, "A").id;
        let b_id = node_by_label(&model, "B").id;
        let c_id = node_by_label(&model, "C").id;

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: runtime_id_for(&model, "A"),
                uuid: Uuid::nil(),
            },
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(repaired.get(&a_id).copied(), Some(RuntimeState::Completed));
        assert_eq!(repaired.get(&b_id).copied(), Some(RuntimeState::Running));
        assert_eq!(repaired.get(&c_id).copied(), Some(RuntimeState::Pending));
    }

    #[test]
    fn repaired_live_states_leave_ambiguous_branch_successors_pending() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Conditional {
                label: "Branch",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "Left" }),
                right: Box::new(JourneyAst::Step { label: "Right" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let left_id = node_by_label(&model, "Left").id;
        let right_id = node_by_label(&model, "Right").id;
        let tail_id = node_by_label(&model, "Tail").id;

        let mut live = LiveData::default();
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 1,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id_for(&model, "Branch"),
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Succeeded,
                uuid: Uuid::nil(),
            }),
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(repaired.get(&left_id).copied(), Some(RuntimeState::Pending));
        assert_eq!(
            repaired.get(&right_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Pending));
    }

    #[test]
    fn repaired_live_states_advance_past_untaken_conditional_branch() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Conditional {
                label: "Branch",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "Left" }),
                right: Box::new(JourneyAst::Step { label: "Right" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let left_id = node_by_label(&model, "Left").id;
        let right_id = node_by_label(&model, "Right").id;
        let tail_id = node_by_label(&model, "Tail").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id, phase) in [
            (
                1,
                runtime_id_for(&model, "Branch"),
                NodeLifecyclePhase::Succeeded,
            ),
            (
                2,
                runtime_id_for(&model, "Left"),
                NodeLifecyclePhase::Succeeded,
            ),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path: vec![0],
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&left_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&right_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_advance_across_sequential_conditionals() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Conditional {
                label: "Branch1",
                metadata: "",
                left: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step { label: "Set1" },
                    JourneyAst::Step { label: "Work1" },
                ])),
                right: Box::new(JourneyAst::Step { label: "Skip1" }),
            },
            JourneyAst::Step { label: "Flatten1" },
            JourneyAst::Conditional {
                label: "Branch2",
                metadata: "",
                left: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step { label: "Set2" },
                    JourneyAst::Step { label: "Work2" },
                ])),
                right: Box::new(JourneyAst::Step { label: "Skip2" }),
            },
            JourneyAst::Step { label: "Flatten2" },
        ]));
        let set1_id = node_by_label(&model, "Set1").id;
        let work1_id = node_by_label(&model, "Work1").id;
        let skip1_id = node_by_label(&model, "Skip1").id;
        let flatten1_id = node_by_label(&model, "Flatten1").id;
        let branch2_id = node_by_label(&model, "Branch2").id;
        let set2_id = node_by_label(&model, "Set2").id;
        let work2_id = node_by_label(&model, "Work2").id;
        let skip2_id = node_by_label(&model, "Skip2").id;
        let flatten2_id = node_by_label(&model, "Flatten2").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id) in [
            (1, runtime_id_for(&model, "Branch1")),
            (2, runtime_id_for(&model, "Set1")),
            (3, runtime_id_for(&model, "Work1")),
            (4, runtime_id_for(&model, "Flatten1")),
            (5, runtime_id_for(&model, "Branch2")),
            (6, runtime_id_for(&model, "Set2")),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            }));
        }
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 7,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: runtime_id_for(&model, "Work2"),
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&set1_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&work1_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&skip1_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&flatten1_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&branch2_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&set2_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&work2_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(
            repaired.get(&skip2_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&flatten2_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn repaired_live_states_prefer_direct_completed_state_over_running_join_proxy() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "Left" }),
                right: Box::new(JourneyAst::Step { label: "Right" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let left = node_by_label(&model, "Left");
        let right = node_by_label(&model, "Right");
        let tail_id = node_by_label(&model, "Tail").id;
        let join_runtime_id = left
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("Left should carry hidden join runtime"));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Left"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Right"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: join_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                4,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Tail"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&left.id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&right.id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_ignore_join_proxy_for_untaken_conditional_branch() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Conditional {
                    label: "Branch",
                    metadata: "",
                    left: Box::new(JourneyAst::Step { label: "Taken" }),
                    right: Box::new(JourneyAst::Step { label: "Skipped" }),
                }),
                right: Box::new(JourneyAst::Step { label: "Other" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let taken_id = node_by_label(&model, "Taken").id;
        let skipped_id = node_by_label(&model, "Skipped").id;
        let other_id = node_by_label(&model, "Other").id;
        let tail_id = node_by_label(&model, "Tail").id;
        let join_runtime_id = node_by_label(&model, "Taken")
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("Taken should carry hidden join runtime"));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Taken"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Other"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: join_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                5,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Tail"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&taken_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&skipped_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&other_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_keep_untaken_conditional_path_pending_after_branch_flatten_and_join() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Conditional {
                        label: "Branch",
                        metadata: "",
                        left: Box::new(JourneyAst::Sequence(vec![
                            JourneyAst::Step { label: "WouldRun1" },
                            JourneyAst::Step { label: "WouldRun2" },
                        ])),
                        right: Box::new(JourneyAst::Step { label: "Pass" }),
                    },
                    JourneyAst::Step { label: "Flatten" },
                ])),
                right: Box::new(JourneyAst::Step { label: "Other" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let would_run_1_id = node_by_label(&model, "WouldRun1").id;
        let would_run_2_id = node_by_label(&model, "WouldRun2").id;
        let pass_id = node_by_label(&model, "Pass").id;
        let flatten_id = node_by_label(&model, "Flatten").id;
        let other_id = node_by_label(&model, "Other").id;
        let tail_id = node_by_label(&model, "Tail").id;
        let join_runtime_id = node_by_label(&model, "Flatten")
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("Flatten should carry hidden join runtime"));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Pass"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Flatten"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Other"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                5,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: join_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                6,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Tail"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&would_run_1_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&would_run_2_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&pass_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&flatten_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&other_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_keep_untaken_conditional_path_pending_when_taken_side_is_no_effect() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Conditional {
                        label: "Branch",
                        metadata: "",
                        left: Box::new(JourneyAst::Sequence(vec![
                            JourneyAst::Step { label: "WouldRun1" },
                            JourneyAst::Step { label: "WouldRun2" },
                        ])),
                        right: Box::new(JourneyAst::Step { label: "Skip" }),
                    },
                    JourneyAst::Step { label: "Flatten" },
                ])),
                right: Box::new(JourneyAst::Step { label: "Other" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let would_run_1_id = node_by_label(&model, "WouldRun1").id;
        let would_run_2_id = node_by_label(&model, "WouldRun2").id;
        let skip_id = node_by_label(&model, "Skip").id;
        let flatten_id = node_by_label(&model, "Flatten").id;
        let other_id = node_by_label(&model, "Other").id;
        let tail_id = node_by_label(&model, "Tail").id;
        let join_runtime_id = node_by_label(&model, "Flatten")
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("Flatten should carry hidden join runtime"));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Skip"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                3,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Flatten"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                4,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Other"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                5,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: join_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                6,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Tail"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&would_run_1_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&would_run_2_id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            repaired.get(&skip_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&flatten_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&other_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(repaired.get(&tail_id).copied(), Some(RuntimeState::Running));
    }

    #[test]
    fn repaired_live_states_prefer_newest_conditional_branch_activity() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
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
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let select_id = node_by_label(&model, "Select").id;
        let optimize_id = node_by_label(&model, "Optimize").id;
        let skip_id = node_by_label(&model, "Skip").id;
        let flatten_id = node_by_label(&model, "Flatten").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Skip"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Select"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Optimize"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&select_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&optimize_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(repaired.get(&skip_id).copied(), Some(RuntimeState::Pending));
        assert_eq!(
            repaired.get(&flatten_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn repaired_live_states_prefer_running_branch_over_newer_completed_skip_branch() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
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
        ]));
        let branch_id = node_by_label(&model, "Branch").id;
        let select_id = node_by_label(&model, "Select").id;
        let optimize_id = node_by_label(&model, "Optimize").id;
        let skip_id = node_by_label(&model, "Skip").id;
        let flatten_id = node_by_label(&model, "Flatten").id;

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Select"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Optimize"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Skip"),
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

        let repaired = repaired_live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        assert_eq!(
            repaired.get(&branch_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&select_id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            repaired.get(&optimize_id).copied(),
            Some(RuntimeState::Running)
        );
        assert_eq!(repaired.get(&skip_id).copied(), Some(RuntimeState::Pending));
        assert_eq!(
            repaired.get(&flatten_id).copied(),
            Some(RuntimeState::Pending)
        );
    }

    #[test]
    fn live_states_prefer_branch_with_more_activity_over_later_skip_signal() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
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
        ]));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Select"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Optimize"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Skip"),
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
            states.get(&node_by_label(&model, "Branch").id).copied(),
            Some(RuntimeState::Completed)
        );
    }

    #[test]
    fn live_states_prefer_earlier_root_branch_signal_over_later_skip_signal() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Conditional {
                label: "Branch",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "Select" }),
                right: Box::new(JourneyAst::Step { label: "Skip" }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, "Branch"),
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Select"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: runtime_id_for(&model, "Skip"),
                    uuid: Uuid::nil(),
                },
            ),
            (
                4,
                RunnerUpdateOut::EffectInput {
                    node_id: runtime_id_for(&model, "Tail"),
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
            states.get(&node_by_label(&model, "Branch").id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            states.get(&node_by_label(&model, "Select").id).copied(),
            Some(RuntimeState::Completed)
        );
        assert_eq!(
            states.get(&node_by_label(&model, "Skip").id).copied(),
            Some(RuntimeState::Pending)
        );
        assert_eq!(
            states.get(&node_by_label(&model, "Tail").id).copied(),
            Some(RuntimeState::Running)
        );
    }

    #[test]
    fn repaired_live_states_handle_five_way_prompt_join_selection() {
        fn prompt_branch(
            branch_label: &'static str,
            select_label: &'static str,
            optimize_label: &'static str,
            skip_label: &'static str,
            flatten_label: &'static str,
        ) -> JourneyAst {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: branch_label,
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: select_label,
                        },
                        JourneyAst::Step {
                            label: optimize_label,
                        },
                    ])),
                    right: Box::new(JourneyAst::Step { label: skip_label }),
                },
                JourneyAst::Step {
                    label: flatten_label,
                },
            ])
        }

        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "PromptJoin",
                metadata: "",
                left: Box::new(JourneyAst::Join {
                    label: "PromptLeft",
                    metadata: "",
                    left: Box::new(prompt_branch(
                        "Branch1",
                        "Select1",
                        "Optimize1",
                        "Skip1",
                        "Flatten1",
                    )),
                    right: Box::new(prompt_branch(
                        "Branch2",
                        "Select2",
                        "Optimize2",
                        "Skip2",
                        "Flatten2",
                    )),
                }),
                right: Box::new(JourneyAst::Join {
                    label: "PromptRight",
                    metadata: "",
                    left: Box::new(JourneyAst::Join {
                        label: "PromptRightPair",
                        metadata: "",
                        left: Box::new(prompt_branch(
                            "Branch3",
                            "Select3",
                            "Optimize3",
                            "Skip3",
                            "Flatten3",
                        )),
                        right: Box::new(prompt_branch(
                            "Branch4",
                            "Select4",
                            "Optimize4",
                            "Skip4",
                            "Flatten4",
                        )),
                    }),
                    right: Box::new(prompt_branch(
                        "Branch5",
                        "Select5",
                        "Optimize5",
                        "Skip5",
                        "Flatten5",
                    )),
                }),
            },
            JourneyAst::Step { label: "Tail" },
        ]));

        let mut live = LiveData::default();
        live.bind_model(&model);
        let mut sequence_id = 0_u64;
        let mut push_success = |label: &str| {
            sequence_id += 1;
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: runtime_id_for(&model, label),
                    activation_path: vec![0, sequence_id],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            }));
        };

        for label in [
            "Branch1",
            "Skip1",
            "Flatten1",
            "Branch2",
            "Select2",
            "Optimize2",
            "Flatten2",
            "Branch3",
            "Skip3",
            "Flatten3",
            "Branch4",
            "Select4",
            "Optimize4",
            "Flatten4",
            "Branch5",
            "Skip5",
            "Flatten5",
        ] {
            push_success(label);
        }

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: runtime_id_for(&model, "Tail"),
                uuid: Uuid::nil(),
            },
        }));

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            states.get(&id).copied().unwrap_or(RuntimeState::Pending)
        };

        assert_eq!(state_for("Branch1"), RuntimeState::Pending);
        assert_eq!(state_for("Branch2"), RuntimeState::Completed);
        assert_eq!(state_for("Branch3"), RuntimeState::Pending);
        assert_eq!(state_for("Branch4"), RuntimeState::Completed);
        assert_eq!(state_for("Branch5"), RuntimeState::Pending);
        assert_eq!(state_for("Skip1"), RuntimeState::Completed);
        assert_eq!(state_for("Select2"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize2"), RuntimeState::Completed);
        assert_eq!(state_for("Skip3"), RuntimeState::Completed);
        assert_eq!(state_for("Select4"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize4"), RuntimeState::Completed);
        assert_eq!(state_for("Skip5"), RuntimeState::Completed);

        assert_eq!(state_for("Select1"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize1"), RuntimeState::Pending);
        assert_eq!(state_for("Skip2"), RuntimeState::Pending);
        assert_eq!(state_for("Select3"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize3"), RuntimeState::Pending);
        assert_eq!(state_for("Skip4"), RuntimeState::Pending);
        assert_eq!(state_for("Select5"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize5"), RuntimeState::Pending);
        assert_eq!(state_for("Tail"), RuntimeState::Running);
    }

    #[test]
    fn while_reentry_hides_stale_skip_activity_in_five_way_prompt_join() {
        fn prompt_branch(
            branch_label: &'static str,
            select_label: &'static str,
            optimize_label: &'static str,
            skip_label: &'static str,
            flatten_label: &'static str,
        ) -> JourneyAst {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: branch_label,
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: select_label,
                        },
                        JourneyAst::Step {
                            label: optimize_label,
                        },
                    ])),
                    right: Box::new(JourneyAst::Step { label: skip_label }),
                },
                JourneyAst::Step {
                    label: flatten_label,
                },
            ])
        }

        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Begin" },
                JourneyAst::Join {
                    label: "PromptJoin",
                    metadata: "",
                    left: Box::new(JourneyAst::Join {
                        label: "PromptLeft",
                        metadata: "",
                        left: Box::new(prompt_branch(
                            "Branch1",
                            "Select1",
                            "Optimize1",
                            "Skip1",
                            "Flatten1",
                        )),
                        right: Box::new(prompt_branch(
                            "Branch2",
                            "Select2",
                            "Optimize2",
                            "Skip2",
                            "Flatten2",
                        )),
                    }),
                    right: Box::new(JourneyAst::Join {
                        label: "PromptRight",
                        metadata: "",
                        left: Box::new(JourneyAst::Join {
                            label: "PromptRightPair",
                            metadata: "",
                            left: Box::new(prompt_branch(
                                "Branch3",
                                "Select3",
                                "Optimize3",
                                "Skip3",
                                "Flatten3",
                            )),
                            right: Box::new(prompt_branch(
                                "Branch4",
                                "Select4",
                                "Optimize4",
                                "Skip4",
                                "Flatten4",
                            )),
                        }),
                        right: Box::new(prompt_branch(
                            "Branch5",
                            "Select5",
                            "Optimize5",
                            "Skip5",
                            "Flatten5",
                        )),
                    }),
                },
                JourneyAst::Step { label: "Tail" },
            ])),
        });

        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let tail_runtime_id = runtime_id_for(&model, "Tail");
        let mut live = LiveData::default();
        live.bind_model(&model);
        let mut sequence_id = 0_u64;
        let push_success =
            |live: &mut LiveData, sequence_id: &mut u64, label: &str, activation_path: Vec<u64>| {
                *sequence_id += 1;
                assert!(live.apply_update(JourneyUpdateEvent {
                    sequence_id: *sequence_id,
                    event_unix_ms: 0,
                    event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                        node_id: runtime_id_for(&model, label),
                        activation_path,
                        phase: NodeLifecyclePhase::Succeeded,
                        uuid: Uuid::nil(),
                    }),
                }));
            };

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: loop_runtime_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));

        push_success(&mut live, &mut sequence_id, "Begin", vec![0, 0]);
        for (offset, label) in [
            "Branch1", "Skip1", "Flatten1", "Branch2", "Skip2", "Flatten2", "Branch3", "Skip3",
            "Flatten3", "Branch4", "Skip4", "Flatten4", "Branch5", "Skip5", "Flatten5",
        ]
        .into_iter()
        .enumerate()
        {
            push_success(
                &mut live,
                &mut sequence_id,
                label,
                vec![0, 1 + offset as u64],
            );
        }

        push_success(&mut live, &mut sequence_id, "Begin", vec![1, 0]);
        for (offset, label) in [
            "Branch1",
            "Skip1",
            "Flatten1",
            "Branch2",
            "Select2",
            "Optimize2",
            "Flatten2",
            "Branch3",
            "Skip3",
            "Flatten3",
            "Branch4",
            "Select4",
            "Optimize4",
            "Flatten4",
            "Branch5",
            "Skip5",
            "Flatten5",
        ]
        .into_iter()
        .enumerate()
        {
            push_success(
                &mut live,
                &mut sequence_id,
                label,
                vec![1, 1 + offset as u64],
            );
        }

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: tail_runtime_id,
                uuid: Uuid::nil(),
            },
        }));

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            states.get(&id).copied().unwrap_or(RuntimeState::Pending)
        };

        assert_eq!(state_for("Branch1"), RuntimeState::Pending);
        assert_eq!(state_for("Branch2"), RuntimeState::Completed);
        assert_eq!(state_for("Branch3"), RuntimeState::Pending);
        assert_eq!(state_for("Branch4"), RuntimeState::Completed);
        assert_eq!(state_for("Branch5"), RuntimeState::Pending);
        assert_eq!(state_for("Skip1"), RuntimeState::Completed);
        assert_eq!(state_for("Select2"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize2"), RuntimeState::Completed);
        assert_eq!(state_for("Skip3"), RuntimeState::Completed);
        assert_eq!(state_for("Select4"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize4"), RuntimeState::Completed);
        assert_eq!(state_for("Skip5"), RuntimeState::Completed);

        assert_eq!(state_for("Select1"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize1"), RuntimeState::Pending);
        assert_eq!(state_for("Skip2"), RuntimeState::Pending);
        assert_eq!(state_for("Select3"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize3"), RuntimeState::Pending);
        assert_eq!(state_for("Skip4"), RuntimeState::Pending);
        assert_eq!(state_for("Select5"), RuntimeState::Pending);
        assert_eq!(state_for("Optimize5"), RuntimeState::Pending);
        assert_eq!(state_for("Tail"), RuntimeState::Running);
    }

    #[test]
    fn while_reentry_hides_late_stale_skip_activity_in_five_way_prompt_join() {
        fn prompt_branch(
            branch_label: &'static str,
            select_label: &'static str,
            optimize_label: &'static str,
            skip_label: &'static str,
            flatten_label: &'static str,
        ) -> JourneyAst {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: branch_label,
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: select_label,
                        },
                        JourneyAst::Step {
                            label: optimize_label,
                        },
                    ])),
                    right: Box::new(JourneyAst::Step { label: skip_label }),
                },
                JourneyAst::Step {
                    label: flatten_label,
                },
            ])
        }

        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Begin" },
                JourneyAst::Join {
                    label: "PromptJoin",
                    metadata: "",
                    left: Box::new(JourneyAst::Join {
                        label: "PromptLeft",
                        metadata: "",
                        left: Box::new(prompt_branch(
                            "Branch1",
                            "Select1",
                            "Optimize1",
                            "Skip1",
                            "Flatten1",
                        )),
                        right: Box::new(prompt_branch(
                            "Branch2",
                            "Select2",
                            "Optimize2",
                            "Skip2",
                            "Flatten2",
                        )),
                    }),
                    right: Box::new(JourneyAst::Join {
                        label: "PromptRight",
                        metadata: "",
                        left: Box::new(JourneyAst::Join {
                            label: "PromptRightPair",
                            metadata: "",
                            left: Box::new(prompt_branch(
                                "Branch3",
                                "Select3",
                                "Optimize3",
                                "Skip3",
                                "Flatten3",
                            )),
                            right: Box::new(prompt_branch(
                                "Branch4",
                                "Select4",
                                "Optimize4",
                                "Skip4",
                                "Flatten4",
                            )),
                        }),
                        right: Box::new(prompt_branch(
                            "Branch5",
                            "Select5",
                            "Optimize5",
                            "Skip5",
                            "Flatten5",
                        )),
                    }),
                },
                JourneyAst::Step { label: "Tail" },
            ])),
        });

        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let begin_runtime_id = runtime_id_for(&model, "Begin");
        let branch2_runtime_id = runtime_id_for(&model, "Branch2");
        let select2_runtime_id = runtime_id_for(&model, "Select2");
        let optimize2_runtime_id = runtime_id_for(&model, "Optimize2");
        let skip2_runtime_id = runtime_id_for(&model, "Skip2");
        let branch4_runtime_id = runtime_id_for(&model, "Branch4");
        let select4_runtime_id = runtime_id_for(&model, "Select4");
        let optimize4_runtime_id = runtime_id_for(&model, "Optimize4");

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id, activation_path, phase) in [
            (1, loop_runtime_id, vec![0], NodeLifecyclePhase::Entered),
            (
                2,
                begin_runtime_id,
                vec![0, 0],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                3,
                branch2_runtime_id,
                vec![0, 1],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                4,
                skip2_runtime_id,
                vec![0, 2],
                NodeLifecyclePhase::Succeeded,
            ),
            (5, loop_runtime_id, vec![1], NodeLifecyclePhase::Entered),
            (
                6,
                begin_runtime_id,
                vec![1, 0],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                7,
                branch2_runtime_id,
                vec![1, 1],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                8,
                select2_runtime_id,
                vec![1, 2],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                9,
                optimize2_runtime_id,
                vec![1, 3],
                NodeLifecyclePhase::Entered,
            ),
            (
                10,
                branch4_runtime_id,
                vec![1, 4],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                11,
                select4_runtime_id,
                vec![1, 5],
                NodeLifecyclePhase::Succeeded,
            ),
            (
                12,
                optimize4_runtime_id,
                vec![1, 6],
                NodeLifecyclePhase::Entered,
            ),
            (
                13,
                skip2_runtime_id,
                vec![0, 2],
                NodeLifecyclePhase::Succeeded,
            ),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path,
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            states.get(&id).copied().unwrap_or(RuntimeState::Pending)
        };

        assert_eq!(state_for("Skip2"), RuntimeState::Pending);
        assert_eq!(state_for("Select2"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize2"), RuntimeState::Running);
        assert_eq!(state_for("Skip4"), RuntimeState::Pending);
        assert_eq!(state_for("Select4"), RuntimeState::Completed);
        assert_eq!(state_for("Optimize4"), RuntimeState::Running);
        assert_eq!(state_for("Tail"), RuntimeState::Pending);
    }

    #[test]
    fn while_reentry_keeps_post_join_chain_highlighted_through_sleep() {
        fn prompt_branch(
            branch_label: &'static str,
            select_label: &'static str,
            optimize_label: &'static str,
            skip_label: &'static str,
            flatten_label: &'static str,
        ) -> JourneyAst {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: branch_label,
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: select_label,
                        },
                        JourneyAst::Step {
                            label: optimize_label,
                        },
                    ])),
                    right: Box::new(JourneyAst::Step { label: skip_label }),
                },
                JourneyAst::Step {
                    label: flatten_label,
                },
            ])
        }

        fn submit_branch(
            branch_label: &'static str,
            set_taken_label: &'static str,
            submit_label: &'static str,
            set_skipped_label: &'static str,
            skip_label: &'static str,
            flatten_label: &'static str,
        ) -> JourneyAst {
            JourneyAst::Sequence(vec![
                JourneyAst::Conditional {
                    label: branch_label,
                    metadata: "",
                    left: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: set_taken_label,
                        },
                        JourneyAst::Step {
                            label: submit_label,
                        },
                    ])),
                    right: Box::new(JourneyAst::Sequence(vec![
                        JourneyAst::Step {
                            label: set_skipped_label,
                        },
                        JourneyAst::Step { label: skip_label },
                    ])),
                },
                JourneyAst::Step {
                    label: flatten_label,
                },
            ])
        }

        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "Begin" },
                JourneyAst::Join {
                    label: "PromptJoin",
                    metadata: "",
                    left: Box::new(JourneyAst::Join {
                        label: "PromptLeft",
                        metadata: "",
                        left: Box::new(prompt_branch(
                            "PromptBranch1",
                            "PromptSelect1",
                            "PromptOptimize1",
                            "PromptSkip1",
                            "PromptFlatten1",
                        )),
                        right: Box::new(prompt_branch(
                            "PromptBranch2",
                            "PromptSelect2",
                            "PromptOptimize2",
                            "PromptSkip2",
                            "PromptFlatten2",
                        )),
                    }),
                    right: Box::new(JourneyAst::Join {
                        label: "PromptRight",
                        metadata: "",
                        left: Box::new(JourneyAst::Join {
                            label: "PromptRightPair",
                            metadata: "",
                            left: Box::new(prompt_branch(
                                "PromptBranch3",
                                "PromptSelect3",
                                "PromptOptimize3",
                                "PromptSkip3",
                                "PromptFlatten3",
                            )),
                            right: Box::new(prompt_branch(
                                "PromptBranch4",
                                "PromptSelect4",
                                "PromptOptimize4",
                                "PromptSkip4",
                                "PromptFlatten4",
                            )),
                        }),
                        right: Box::new(prompt_branch(
                            "PromptBranch5",
                            "PromptSelect5",
                            "PromptOptimize5",
                            "PromptSkip5",
                            "PromptFlatten5",
                        )),
                    }),
                },
                JourneyAst::Step {
                    label: "FlattenPromptPhase",
                },
                JourneyAst::Step { label: "Finalize" },
                submit_branch(
                    "SubmitBranch1",
                    "SetSubmit1",
                    "Submit1",
                    "SetSkip1",
                    "SkipSubmit1",
                    "SubmitFlatten1",
                ),
                submit_branch(
                    "SubmitBranch2",
                    "SetSubmit2",
                    "Submit2",
                    "SetSkip2",
                    "SkipSubmit2",
                    "SubmitFlatten2",
                ),
                submit_branch(
                    "SubmitBranch3",
                    "SetSubmit3",
                    "Submit3",
                    "SetSkip3",
                    "SkipSubmit3",
                    "SubmitFlatten3",
                ),
                submit_branch(
                    "SubmitBranch4",
                    "SetSubmit4",
                    "Submit4",
                    "SetSkip4",
                    "SkipSubmit4",
                    "SubmitFlatten4",
                ),
                submit_branch(
                    "SubmitBranch5",
                    "SetSubmit5",
                    "Submit5",
                    "SetSkip5",
                    "SkipSubmit5",
                    "SubmitFlatten5",
                ),
                JourneyAst::Step { label: "Sleep" },
            ])),
        });

        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let sleep_runtime_id = runtime_id_for(&model, "Sleep");
        let mut live = LiveData::default();
        live.bind_model(&model);
        let mut sequence_id = 0_u64;
        let push_success =
            |live: &mut LiveData, sequence_id: &mut u64, label: &str, activation_path: Vec<u64>| {
                *sequence_id += 1;
                assert!(live.apply_update(JourneyUpdateEvent {
                    sequence_id: *sequence_id,
                    event_unix_ms: 0,
                    event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                        node_id: runtime_id_for(&model, label),
                        activation_path,
                        phase: NodeLifecyclePhase::Succeeded,
                        uuid: Uuid::nil(),
                    }),
                }));
            };

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: loop_runtime_id,
                activation_path: vec![0],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));

        push_success(&mut live, &mut sequence_id, "Begin", vec![0, 0]);
        for (offset, label) in [
            "PromptBranch1",
            "PromptSkip1",
            "PromptFlatten1",
            "PromptBranch2",
            "PromptSkip2",
            "PromptFlatten2",
            "PromptBranch3",
            "PromptSkip3",
            "PromptFlatten3",
            "PromptBranch4",
            "PromptSkip4",
            "PromptFlatten4",
            "PromptBranch5",
            "PromptSkip5",
            "PromptFlatten5",
            "FlattenPromptPhase",
            "Finalize",
            "SubmitBranch1",
            "SetSkip1",
            "SkipSubmit1",
            "SubmitFlatten1",
            "SubmitBranch2",
            "SetSkip2",
            "SkipSubmit2",
            "SubmitFlatten2",
            "SubmitBranch3",
            "SetSkip3",
            "SkipSubmit3",
            "SubmitFlatten3",
            "SubmitBranch4",
            "SetSkip4",
            "SkipSubmit4",
            "SubmitFlatten4",
            "SubmitBranch5",
            "SetSkip5",
            "SkipSubmit5",
            "SubmitFlatten5",
        ]
        .into_iter()
        .enumerate()
        {
            push_success(
                &mut live,
                &mut sequence_id,
                label,
                vec![0, 1 + offset as u64],
            );
        }

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                node_id: loop_runtime_id,
                activation_path: vec![1],
                phase: NodeLifecyclePhase::Entered,
                uuid: Uuid::nil(),
            }),
        }));

        push_success(&mut live, &mut sequence_id, "Begin", vec![1, 0]);
        for (offset, label) in [
            "PromptBranch1",
            "PromptSkip1",
            "PromptFlatten1",
            "PromptBranch2",
            "PromptSelect2",
            "PromptOptimize2",
            "PromptFlatten2",
            "PromptBranch3",
            "PromptSkip3",
            "PromptFlatten3",
            "PromptBranch4",
            "PromptSelect4",
            "PromptOptimize4",
            "PromptFlatten4",
            "PromptBranch5",
            "PromptSkip5",
            "PromptFlatten5",
            "FlattenPromptPhase",
            "Finalize",
            "SubmitBranch1",
            "SetSkip1",
            "SkipSubmit1",
            "SubmitFlatten1",
            "SubmitBranch2",
            "SetSubmit2",
            "Submit2",
            "SubmitFlatten2",
            "SubmitBranch3",
            "SetSkip3",
            "SkipSubmit3",
            "SubmitFlatten3",
            "SubmitBranch4",
            "SetSubmit4",
            "Submit4",
            "SubmitFlatten4",
            "SubmitBranch5",
            "SetSkip5",
            "SkipSubmit5",
            "SubmitFlatten5",
        ]
        .into_iter()
        .enumerate()
        {
            push_success(
                &mut live,
                &mut sequence_id,
                label,
                vec![1, 1 + offset as u64],
            );
        }

        sequence_id += 1;
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id,
            event_unix_ms: 0,
            event: RunnerUpdateOut::EffectInput {
                node_id: sleep_runtime_id,
                uuid: Uuid::nil(),
            },
        }));

        let states = live_states_for_display(
            &model,
            Some(&live),
            &model.derived.condition_successor_runtime_ids,
        );
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            states.get(&id).copied().unwrap_or(RuntimeState::Pending)
        };

        assert_eq!(state_for("FlattenPromptPhase"), RuntimeState::Completed);
        assert_eq!(state_for("Finalize"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitBranch1"), RuntimeState::Pending);
        assert_eq!(state_for("SetSkip1"), RuntimeState::Completed);
        assert_eq!(state_for("SkipSubmit1"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitFlatten1"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitBranch2"), RuntimeState::Completed);
        assert_eq!(state_for("SetSubmit2"), RuntimeState::Completed);
        assert_eq!(state_for("Submit2"), RuntimeState::Completed);
        assert_eq!(state_for("SetSkip2"), RuntimeState::Pending);
        assert_eq!(state_for("SkipSubmit2"), RuntimeState::Pending);
        assert_eq!(state_for("SubmitFlatten2"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitBranch3"), RuntimeState::Pending);
        assert_eq!(state_for("SkipSubmit3"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitBranch4"), RuntimeState::Completed);
        assert_eq!(state_for("SetSubmit4"), RuntimeState::Completed);
        assert_eq!(state_for("Submit4"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitFlatten4"), RuntimeState::Completed);
        assert_eq!(state_for("SubmitBranch5"), RuntimeState::Pending);
        assert_eq!(state_for("SkipSubmit5"), RuntimeState::Completed);
        assert_eq!(state_for("Sleep"), RuntimeState::Running);
    }

    #[test]
    fn attempt_failure_stays_visible_when_sleep_is_inferred_running() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Step { label: "WithErr" },
            JourneyAst::Step { label: "InitIter" },
            JourneyAst::While {
                label: "Loop",
                metadata: "",
                body: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Join {
                        label: "Join",
                        metadata: "",
                        left: Box::new(JourneyAst::Sequence(vec![
                            JourneyAst::Step { label: "CloneOver" },
                            JourneyAst::Attempt {
                                label: "Attempt",
                                metadata: "",
                                body: Box::new(JourneyAst::Sequence(vec![
                                    JourneyAst::Step {
                                        label: "AnnounceFailure",
                                    },
                                    JourneyAst::Step { label: "Fail" },
                                ])),
                            },
                        ])),
                        right: Box::new(JourneyAst::Step {
                            label: "Passthrough",
                        }),
                    },
                    JourneyAst::Step { label: "IncIter" },
                    JourneyAst::Step { label: "SleepMult" },
                ])),
            },
        ]));

        let loop_index = cluster_index_for_kind(&model, ClusterKind::While);
        let attempt_index = cluster_index_for_kind(&model, ClusterKind::Attempt);
        let loop_runtime_id = model.cluster_info[loop_index].runtime_node_id;
        let attempt_runtime_id = model.cluster_info[attempt_index].runtime_node_id;
        let with_err_runtime_id = runtime_id_for(&model, "WithErr");
        let init_iter_runtime_id = runtime_id_for(&model, "InitIter");
        let clone_over_runtime_id = runtime_id_for(&model, "CloneOver");
        let passthrough_runtime_id = runtime_id_for(&model, "Passthrough");
        let announce_failure_runtime_id = runtime_id_for(&model, "AnnounceFailure");
        let fail_runtime_id = runtime_id_for(&model, "Fail");
        let inc_iter_runtime_id = runtime_id_for(&model, "IncIter");

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: with_err_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                2,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: init_iter_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                3,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                4,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: attempt_runtime_id,
                    activation_path: vec![0, 0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                5,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: clone_over_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                6,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: announce_failure_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                7,
                RunnerUpdateOut::EffectFailureOutput {
                    node_id: fail_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                8,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: attempt_runtime_id,
                    activation_path: vec![0, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                9,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: passthrough_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                10,
                RunnerUpdateOut::EffectInput {
                    node_id: inc_iter_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                11,
                RunnerUpdateOut::EffectSuccessOutput {
                    node_id: inc_iter_runtime_id,
                    uuid: Uuid::nil(),
                },
            ),
            (
                12,
                RunnerUpdateOut::SleepScheduled {
                    uuid: Uuid::nil(),
                    timer_id: Uuid::new_v4(),
                    wake_at_unix_ms: 1_000,
                },
            ),
        ] {
            let _ = live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: sequence_id as i64,
                event,
            });
        }

        let snapshot = DagSnapshot::new(&model, Some(&live));
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            snapshot
                .node_states
                .get(&id)
                .copied()
                .unwrap_or(RuntimeState::Pending)
        };
        assert_eq!(state_for("WithErr"), RuntimeState::Completed);
        assert_eq!(state_for("InitIter"), RuntimeState::Completed);
        assert_eq!(state_for("CloneOver"), RuntimeState::Completed);
        assert_eq!(state_for("Passthrough"), RuntimeState::Completed);
        assert_eq!(state_for("AnnounceFailure"), RuntimeState::Completed);
        assert_eq!(state_for("Fail"), RuntimeState::Failed);
        assert_eq!(state_for("IncIter"), RuntimeState::Completed);
        assert_eq!(state_for("SleepMult"), RuntimeState::Running);

        let attempt_phase = snapshot.cluster_phase(attempt_index);
        assert!(matches!(
            attempt_phase,
            Phase::Live(ClusterLive {
                has_running: false,
                has_failed: true,
                has_completed: true,
            })
        ));
    }

    #[test]
    fn attempt_member_running_after_attempt_completion_is_treated_as_failed() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Attempt {
                label: "Attempt",
                metadata: "",
                body: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step { label: "Announce" },
                    JourneyAst::Step { label: "Fail" },
                ])),
            },
            JourneyAst::Step { label: "Tail" },
        ]));

        let attempt_index = cluster_index_for_kind(&model, ClusterKind::Attempt);
        let attempt_runtime_id = model.cluster_info[attempt_index].runtime_node_id;
        let announce_runtime_id = runtime_id_for(&model, "Announce");
        let fail_runtime_id = runtime_id_for(&model, "Fail");
        let tail_runtime_id = runtime_id_for(&model, "Tail");

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, node_id, activation_path, phase) in [
            (1, attempt_runtime_id, vec![0], NodeLifecyclePhase::Entered),
            (
                2,
                announce_runtime_id,
                vec![0, 0],
                NodeLifecyclePhase::Entered,
            ),
            (
                3,
                announce_runtime_id,
                vec![0, 0],
                NodeLifecyclePhase::Succeeded,
            ),
            // Fail entered, but no terminal lifecycle update was emitted before attempt completed.
            (4, fail_runtime_id, vec![0, 1], NodeLifecyclePhase::Entered),
            (
                5,
                attempt_runtime_id,
                vec![0],
                NodeLifecyclePhase::Succeeded,
            ),
            (6, tail_runtime_id, vec![1], NodeLifecyclePhase::Entered),
        ] {
            assert!(live.apply_update(JourneyUpdateEvent {
                sequence_id,
                event_unix_ms: 0,
                event: RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id,
                    activation_path,
                    phase,
                    uuid: Uuid::nil(),
                }),
            }));
        }

        let snapshot = DagSnapshot::new(&model, Some(&live));
        let state_for = |label: &str| {
            let id = node_by_label(&model, label).id;
            snapshot
                .node_states
                .get(&id)
                .copied()
                .unwrap_or(RuntimeState::Pending)
        };

        assert_eq!(state_for("Announce"), RuntimeState::Completed);
        assert_eq!(state_for("Fail"), RuntimeState::Failed);
        assert_eq!(state_for("Tail"), RuntimeState::Running);

        let attempt_phase = snapshot.cluster_phase(attempt_index);
        assert!(matches!(
            attempt_phase,
            Phase::Live(ClusterLive {
                has_running: false,
                has_failed: true,
                has_completed: true,
            })
        ));
    }

    #[test]
    fn graph_model_does_not_register_join_or_select_for_descendant_clearing() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
            JourneyAst::Join {
                label: "Join",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "JoinL" }),
                right: Box::new(JourneyAst::Step { label: "JoinR" }),
            },
            JourneyAst::Select {
                label: "Select",
                metadata: "",
                left: Box::new(JourneyAst::Step { label: "SelectL" }),
                right: Box::new(JourneyAst::Step { label: "SelectR" }),
            },
        ]));
        let join_runtime_id = node_by_label(&model, "JoinL")
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("JoinL should carry hidden join runtime"));
        let select_runtime_id = node_by_label(&model, "SelectL")
            .proxy_runtime_ids
            .first()
            .copied()
            .unwrap_or_else(|| panic!("SelectL should carry hidden select runtime"));

        assert!(!model
            .derived
            .descendant_runtime_ids_by_runtime_id
            .contains_key(&join_runtime_id));
        assert!(!model
            .derived
            .descendant_runtime_ids_by_runtime_id
            .contains_key(&select_runtime_id));
    }

    #[test]
    fn theme_state_forces_while_members_back_to_pending_on_reentry() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let cluster = &model.cluster_info[0];
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: 0,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: None,
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &model.derived.cluster_entry_runtime_ids[0],
            member_runtime_ids: &model.derived.cluster_member_runtime_ids[0],
            successor_runtime_ids: &model.derived.cluster_successor_runtime_ids[0],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: true,
            }),
        };

        let mut state = DefaultThemeState::new(ClusterExpansionConfig {
            while_clusters: ClusterExpansionMode::AlwaysExpanded,
            transparent_clusters: ClusterExpansionMode::AlwaysExpanded,
        });
        state.register_cluster(&cx);

        let loop_runtime_id = cluster.runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let _ = state.update_node_state(a_runtime_id, RuntimeState::Completed);
        let _ = state.update_node_state(b_runtime_id, RuntimeState::Completed);

        assert!(state.update_clusters_for_effect_input(loop_runtime_id, Instant::now()));
        assert!(state.force_pending_runtime_ids.contains(&a_runtime_id));
        assert!(state.force_pending_runtime_ids.contains(&b_runtime_id));

        let _ = state.update_node_state(a_runtime_id, RuntimeState::Completed);
        assert!(!state.force_pending_runtime_ids.contains(&a_runtime_id));
        assert!(state.force_pending_runtime_ids.contains(&b_runtime_id));
    }

    #[test]
    fn theme_state_releases_force_pending_for_repaired_completed_state() {
        let model = GraphModel::from_ast(JourneyAst::While {
            label: "Loop",
            metadata: "",
            body: Box::new(JourneyAst::Sequence(vec![
                JourneyAst::Step { label: "A" },
                JourneyAst::Step { label: "B" },
            ])),
        });
        let cluster = &model.cluster_info[0];
        let cx = ClusterViewCtx {
            cluster_id: cluster.id,
            cluster_index: 0,
            kind: cluster.kind,
            label: &cluster.label,
            metadata: cluster.metadata.as_deref(),
            parent_cluster_id: None,
            depth: cluster.depth,
            member_display_ids: &cluster.nodes,
            entry_runtime_ids: &model.derived.cluster_entry_runtime_ids[0],
            member_runtime_ids: &model.derived.cluster_member_runtime_ids[0],
            successor_runtime_ids: &model.derived.cluster_successor_runtime_ids[0],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: true,
            }),
        };

        let mut state = DefaultThemeState::new(ClusterExpansionConfig {
            while_clusters: ClusterExpansionMode::AlwaysExpanded,
            transparent_clusters: ClusterExpansionMode::AlwaysExpanded,
        });
        state.register_cluster(&cx);

        let loop_runtime_id = cluster.runtime_node_id;
        let a_runtime_id = runtime_id_for(&model, "A");
        let b_runtime_id = runtime_id_for(&model, "B");
        let _ = state.update_node_state(a_runtime_id, RuntimeState::Completed);
        let _ = state.update_node_state(b_runtime_id, RuntimeState::Completed);

        assert!(state.update_clusters_for_effect_input(loop_runtime_id, Instant::now()));
        assert!(state.force_pending_runtime_ids.contains(&a_runtime_id));
        assert!(state.force_pending_runtime_ids.contains(&b_runtime_id));

        assert_eq!(
            state.apply_force_pending_override(a_runtime_id, RuntimeState::Completed),
            RuntimeState::Completed
        );
        assert!(!state.force_pending_runtime_ids.contains(&a_runtime_id));
        assert!(state.force_pending_runtime_ids.contains(&b_runtime_id));
        assert_eq!(
            state.apply_force_pending_override(b_runtime_id, RuntimeState::Pending),
            RuntimeState::Pending
        );
        assert!(state.force_pending_runtime_ids.contains(&b_runtime_id));
    }

    #[test]
    fn edge_style_keeps_completed_to_pending_branch_gray() {
        let theme = DefaultTheme::default();
        let state = theme.init();
        let style = theme
            .edge_style(
                &state,
                EdgeStyleCtx {
                    edge_index: 0,
                    source_display_id: 1,
                    target_display_id: 2,
                    source_runtime_id: None,
                    target_runtime_id: None,
                    source_has_proxy_runtime: false,
                    target_has_proxy_runtime: false,
                    source_phase: Phase::Live(RuntimeState::Completed),
                    target_phase: Phase::Live(RuntimeState::Pending),
                    extent: 1.0,
                },
            )
            .expect("default theme should provide an edge style");

        assert_eq!(style.start, runtime_color(RuntimeState::Pending));
        assert_eq!(style.end, runtime_color(RuntimeState::Pending));
    }

    #[test]
    fn edge_style_uses_active_target_state() {
        let theme = DefaultTheme::default();
        let state = theme.init();
        let style = theme
            .edge_style(
                &state,
                EdgeStyleCtx {
                    edge_index: 0,
                    source_display_id: 1,
                    target_display_id: 2,
                    source_runtime_id: None,
                    target_runtime_id: None,
                    source_has_proxy_runtime: false,
                    target_has_proxy_runtime: false,
                    source_phase: Phase::Live(RuntimeState::Pending),
                    target_phase: Phase::Live(RuntimeState::Running),
                    extent: 1.0,
                },
            )
            .expect("default theme should provide an edge style");

        assert_eq!(style.start, runtime_color(RuntimeState::Running));
        assert_eq!(style.end, runtime_color(RuntimeState::Running));
    }

    #[test]
    fn while_loop_current_iteration_running_node_keeps_ancestors_completed() {
        let model = GraphModel::from_ast(JourneyAst::While {
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
        });
        let loop_runtime_id = model.cluster_info[0].runtime_node_id;
        let begin_id = node_by_label(&model, "Begin").id;
        let select_id = node_by_label(&model, "Select").id;
        let optimize_id = node_by_label(&model, "Optimize").id;
        let skip_id = node_by_label(&model, "Skip").id;
        let flatten_id = node_by_label(&model, "Flatten").id;
        let begin_runtime_id = runtime_id_for(&model, "Begin");
        let select_runtime_id = runtime_id_for(&model, "Select");
        let optimize_runtime_id = runtime_id_for(&model, "Optimize");
        let flatten_runtime_id = runtime_id_for(&model, "Flatten");

        let mut live = LiveData::default();
        live.bind_model(&model);
        for (sequence_id, event) in [
            (
                1,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                2,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: begin_runtime_id,
                    activation_path: vec![0, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                3,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: select_runtime_id,
                    activation_path: vec![0, 1],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                4,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: optimize_runtime_id,
                    activation_path: vec![0, 2],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                5,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: flatten_runtime_id,
                    activation_path: vec![0, 3],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                6,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: loop_runtime_id,
                    activation_path: vec![1],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                7,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: begin_runtime_id,
                    activation_path: vec![1, 0],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                8,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: begin_runtime_id,
                    activation_path: vec![1, 0],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                9,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: select_runtime_id,
                    activation_path: vec![1, 1],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                10,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: select_runtime_id,
                    activation_path: vec![1, 1],
                    phase: NodeLifecyclePhase::Succeeded,
                    uuid: Uuid::nil(),
                }),
            ),
            (
                11,
                RunnerUpdateOut::NodeLifecycle(jungle_types::NodeLifecycle {
                    node_id: optimize_runtime_id,
                    activation_path: vec![1, 2],
                    phase: NodeLifecyclePhase::Entered,
                    uuid: Uuid::nil(),
                }),
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
    fn cluster_successors_include_downstream_runtime_after_direct_exit() {
        let model = GraphModel::from_ast(JourneyAst::Sequence(vec![
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
        ]));

        let successor_runtime_ids = cluster_successor_runtime_ids(&model);
        assert_eq!(successor_runtime_ids.len(), 1);
        assert_eq!(
            successor_runtime_ids[0],
            vec![
                runtime_id_for(&model, "DirectSuccessor"),
                runtime_id_for(&model, "DownstreamSuccessor"),
            ]
        );
    }
}
