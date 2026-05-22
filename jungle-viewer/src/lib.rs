use iced::futures::{self, Stream, StreamExt};
use iced::widget::{button, column, container, row, text, Space};
use iced::window;
use iced::window::Screenshot;
use iced::{Color, Element, Font, Length, Subscription, Task};
use iced_sugiyama::{Cluster, Graph, OutgoingEdgeStyle, Sugiyama};
use jungle_client::JungleClient;
use jungle_types::{Animal, JourneyAst, JourneyAstSource, JourneyUpdateEvent, RunnerUpdateOut};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1360.0;
const WINDOW_HEIGHT: f32 = 900.0;
const NODE_WIDTH: f64 = 240.0;
const NODE_HEIGHT: f64 = 80.0;
const GRAPH_WIDGET_ID: &str = "jungle-viewer";
const DEFAULT_CLUSTER_FILL: Color = Color::TRANSPARENT;
const NODE_ANIMATION_DURATION: Duration = Duration::from_millis(320);
const CLUSTER_BORDER_ANIMATION_DURATION: Duration = Duration::from_millis(320);
const ANIMATION_TICK: Duration = Duration::from_millis(16);

static CLUSTER_FILL_COLORS: OnceLock<RwLock<Vec<Color>>> = OnceLock::new();

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

#[derive(Debug, Clone)]
pub struct StepViewCtx<'a> {
    pub display_id: u32,
    pub runtime_id: Option<u32>,
    pub successor_runtime_ids: Vec<u32>,
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
    pub entry_runtime_ids: Vec<u32>,
    pub member_runtime_ids: Vec<u32>,
    pub successor_runtime_ids: Vec<u32>,
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
        A::Journey: JourneyAstSource,
    {
        self.view_animal_with_theme::<A, _, AnyAnimal>(DefaultTheme)
    }

    pub fn view_animal_with_theme<A, T, Scope>(self, theme: T) -> iced::Result
    where
        A: Animal + 'static,
        A::Journey: JourneyAstSource,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
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
        A::Journey: JourneyAstSource,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
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
        A::Journey: JourneyAstSource,
    {
        self.eject_animal_with_theme::<A, _, AnyAnimal>(DefaultTheme)
    }

    pub fn view_live_animal<A, C>(self, client: C, journey_id: Uuid) -> iced::Result
    where
        A: Animal + 'static,
        A::Journey: JourneyAstSource,
        C: JungleClient + 'static,
    {
        self.view_live_animal_with_theme::<A, C, _, AnyAnimal>(client, journey_id, DefaultTheme)
    }

    pub fn view_live_animal_with_theme<A, C, T, Scope>(
        self,
        client: C,
        journey_id: Uuid,
        theme: T,
    ) -> iced::Result
    where
        A: Animal + 'static,
        A::Journey: JourneyAstSource,
        C: JungleClient + 'static,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
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
        A::Journey: JourneyAstSource,
        C: JungleClient + 'static,
        T: JunglePanelTheme<Scope, Message = ()>,
        Scope: 'static,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
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
        A::Journey: JourneyAstSource,
        C: JungleClient + 'static,
    {
        self.eject_live_animal_with_theme::<A, C, _, AnyAnimal>(client, journey_id, DefaultTheme)
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
    A::Journey: JourneyAstSource,
{
    JungleViewerBuilder::new().view_animal::<A>()
}

pub fn view_live_animal<A, C>(client: C, journey_id: Uuid) -> iced::Result
where
    A: Animal + 'static,
    A::Journey: JourneyAstSource,
    C: JungleClient + 'static,
{
    JungleViewerBuilder::new().view_live_animal::<A, C>(client, journey_id)
}

#[derive(Debug, Clone)]
pub enum EjectedViewerMessage {
    LiveEvent(Result<JourneyUpdateEvent, String>),
    ApplyLiveEvent(JourneyUpdateEvent),
    Theme(ViewerEvent<()>),
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

    pub fn update(&mut self, message: EjectedViewerMessage) -> Task<EjectedViewerMessage> {
        match message {
            EjectedViewerMessage::LiveEvent(result) => {
                match result {
                    Ok(update) => {
                        return Task::done(EjectedViewerMessage::ApplyLiveEvent(update));
                    }
                    Err(error) => {
                        self.state = LiveState::Error(error);
                    }
                }
                Task::none()
            }
            EjectedViewerMessage::ApplyLiveEvent(update) => {
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
                let _ = data.apply_update(update);
                Task::batch(vec![
                    theme_task,
                    iced_sugiyama::force_review::<EjectedViewerMessage>(
                        self.graph_widget_id.clone(),
                    ),
                ])
            }
            EjectedViewerMessage::Theme(event) => {
                let theme_task = self
                    .theme
                    .update(&mut self.theme_state, event)
                    .map(EjectedViewerMessage::Theme);
                Task::batch(vec![
                    theme_task,
                    iced_sugiyama::force_review::<EjectedViewerMessage>(
                        self.graph_widget_id.clone(),
                    ),
                ])
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
        )
        .map(|message| match message {
            Message::Theme(event) => EjectedViewerMessage::Theme(event),
            Message::LiveEvent(result) => EjectedViewerMessage::LiveEvent(result),
            Message::ApplyLiveEvent(update) => EjectedViewerMessage::ApplyLiveEvent(update),
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
    A::Journey: JourneyAstSource,
{
    let ast = <A::Journey as JourneyAstSource>::journey_ast();
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
    ApplyLiveEvent(JourneyUpdateEvent),
    Theme(ViewerEvent<()>),
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
                let _ = data.apply_update(update);
                Task::batch(vec![
                    theme_task,
                    iced_sugiyama::force_review::<Message>(self.graph_widget_id.clone()),
                ])
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
        match client.subscribe_step_updates(journey_id, None).await {
            Ok(subscription) => subscription
                .map(|event| Message::LiveEvent(event.map_err(|err| err.to_string())))
                .left_stream(),
            Err(err) => futures::stream::once(async move {
                Message::LiveEvent(Err(format!("live update stream setup failed: {err}")))
            })
            .right_stream(),
        }
    })
    .flatten()
}

fn live_updates_stream_for_panel(
    config: &LiveSubscription,
) -> impl Stream<Item = EjectedViewerMessage> {
    let client = config.client.clone();
    let journey_id = config.journey_id;
    futures::stream::once(async move {
        match client.subscribe_step_updates(journey_id, None).await {
            Ok(subscription) => subscription
                .map(|event| EjectedViewerMessage::LiveEvent(event.map_err(|err| err.to_string())))
                .left_stream(),
            Err(err) => futures::stream::once(async move {
                EjectedViewerMessage::LiveEvent(Err(format!(
                    "live update stream setup failed: {err}"
                )))
            })
            .right_stream(),
        }
    })
    .flatten()
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

fn runtime_state_for_live_data(live: &LiveData, runtime_id: u32) -> RuntimeState {
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

fn infer_condition_runtime_state(live: &LiveData, successor_runtime_ids: &[u32]) -> RuntimeState {
    let mut newest: Option<(usize, RuntimeState)> = None;

    for runtime_id in successor_runtime_ids {
        let Some(sequence) = live.runtime_update_sequence.get(runtime_id).copied() else {
            continue;
        };
        if newest
            .map(|(best_sequence, _)| sequence > best_sequence)
            .unwrap_or(true)
        {
            newest = Some((sequence, runtime_state_for_live_data(live, *runtime_id)));
        }
    }

    newest
        .map(|(_, state)| state)
        .unwrap_or(RuntimeState::Pending)
}

fn node_phase_for_display(
    live_data: Option<&LiveData>,
    display_id: u32,
    runtime_id: Option<u32>,
    condition_successor_runtime_ids: &HashMap<u32, Vec<u32>>,
) -> Phase<RuntimeState> {
    let Some(live) = live_data else {
        return Phase::Static;
    };

    let state = match runtime_id {
        Some(id) => runtime_state_for_live_data(live, id),
        None => condition_successor_runtime_ids
            .get(&display_id)
            .map(|successors| infer_condition_runtime_state(live, successors))
            .unwrap_or(RuntimeState::Pending),
    };

    Phase::Live(state)
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
) -> Element<'a, Message>
where
    T: JunglePanelTheme<Scope, Message = ()>,
{
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisibleOwner {
        Node(u32),
        Cluster(usize),
    }

    let mut condition_successor_runtime_ids = HashMap::<u32, Vec<u32>>::new();
    let mut condition_successor_seen = HashMap::<u32, BTreeSet<u32>>::new();
    for (from, to) in &model.edges {
        let Some(source) = model.node_map.get(from) else {
            continue;
        };
        if !source.is_conditional_branch {
            continue;
        }
        let Some(target) = model.node_map.get(to) else {
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

    let cluster_phase = move |cluster: &ClusterInfo| -> Phase<ClusterLive> {
        if let Some(live) = live_data {
            let mut has_running = false;
            let mut has_failed = false;
            let mut has_completed = false;
            for node_id in &cluster.nodes {
                let Some(node) = model.node_map.get(node_id) else {
                    continue;
                };
                let Some(runtime_id) = node.runtime_node_id else {
                    continue;
                };
                if live.active_runtime_ids.contains(&runtime_id) {
                    has_running = true;
                }
                if live.failed_runtime_ids.contains(&runtime_id) {
                    has_failed = true;
                }
                if live.finished_runtime_ids.contains(&runtime_id) {
                    has_completed = true;
                }
            }
            Phase::Live(ClusterLive {
                has_running,
                has_failed,
                has_completed,
            })
        } else {
            Phase::Static
        }
    };

    let mut cluster_member_runtime_ids = vec![Vec::<u32>::new(); model.cluster_info.len()];
    for (index, cluster) in model.cluster_info.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for node_id in &cluster.nodes {
            let Some(node) = model.node_map.get(node_id) else {
                continue;
            };
            let Some(runtime_id) = node.runtime_node_id else {
                continue;
            };
            if seen.insert(runtime_id) {
                cluster_member_runtime_ids[index].push(runtime_id);
            }
        }
    }

    let mut cluster_successor_runtime_ids = vec![Vec::<u32>::new(); model.cluster_info.len()];
    for (index, cluster) in model.cluster_info.iter().enumerate() {
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        let mut seen = BTreeSet::new();
        for (from, to) in &model.edges {
            if !cluster_nodes.contains(from) || cluster_nodes.contains(to) {
                continue;
            }
            let Some(node) = model.node_map.get(to) else {
                continue;
            };
            let Some(runtime_id) = node.runtime_node_id else {
                continue;
            };
            if seen.insert(runtime_id) {
                cluster_successor_runtime_ids[index].push(runtime_id);
            }
        }
    }

    let mut cluster_entry_runtime_ids = vec![Vec::<u32>::new(); model.cluster_info.len()];
    for (index, cluster) in model.cluster_info.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for node_id in &cluster.root_nodes {
            let Some(node) = model.node_map.get(node_id) else {
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
            entry_runtime_ids: cluster_entry_runtime_ids[index].clone(),
            member_runtime_ids: cluster_member_runtime_ids[index].clone(),
            successor_runtime_ids: cluster_successor_runtime_ids[index].clone(),
            phase: cluster_phase(cluster),
        };
        if matches!(
            theme.view_cluster(theme_state, &cx),
            ClusterView::Collapsed { .. }
        ) {
            collapsed_clusters.insert(index);
        }
    }

    let mut memberships = HashMap::<u32, Vec<(usize, usize)>>::new();
    for (index, cluster) in model.cluster_info.iter().enumerate() {
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

    let max_node_id = model.nodes.iter().map(|node| node.id).max().unwrap_or(0);
    let cluster_node_id = |index: usize| -> Option<u32> {
        let offset = u32::try_from(index).ok()?;
        Some(max_node_id.saturating_add(1).saturating_add(offset))
    };

    let owner_to_display = |owner: VisibleOwner| -> Option<u32> {
        match owner {
            VisibleOwner::Node(node_id) => Some(node_id),
            VisibleOwner::Cluster(index) => cluster_node_id(index),
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
        let phase = node_phase_for_display(
            live_data,
            node.id,
            node.runtime_node_id,
            &condition_successor_runtime_ids,
        );
        let step_ctx = StepViewCtx {
            display_id: node.id,
            runtime_id: node.runtime_node_id,
            successor_runtime_ids: condition_successor_runtime_ids
                .get(&node.id)
                .cloned()
                .unwrap_or_default(),
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
        let Some(display_id) = cluster_node_id(index) else {
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
            entry_runtime_ids: cluster_entry_runtime_ids[index].clone(),
            member_runtime_ids: cluster_member_runtime_ids[index].clone(),
            successor_runtime_ids: cluster_successor_runtime_ids[index].clone(),
            phase: cluster_phase(cluster),
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
            entry_runtime_ids: cluster_entry_runtime_ids[source_index].clone(),
            member_runtime_ids: cluster_member_runtime_ids[source_index].clone(),
            successor_runtime_ids: cluster_successor_runtime_ids[source_index].clone(),
            phase: cluster_phase(cluster),
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
    let runtime_by_display_id = model
        .node_map
        .iter()
        .map(|(display_id, node)| (*display_id, node.runtime_node_id))
        .collect::<HashMap<_, _>>();

    let graph_widget = {
        let node_map = model.node_map.clone();
        let cluster_info_for_nodes = model.cluster_info.clone();
        let cluster_info_for_clusters = model.cluster_info.clone();
        let collapsed_display_map = collapsed_cluster_by_display.clone();
        let visible_nodes = visible_real_nodes.clone();
        let sizes_for_view = node_sizes.clone();
        let visible_cluster_sources = visible_cluster_source_indices.clone();
        let cluster_member_runtime_ids_for_nodes = cluster_member_runtime_ids.clone();
        let cluster_successor_runtime_ids_for_nodes = cluster_successor_runtime_ids.clone();
        let cluster_entry_runtime_ids_for_nodes = cluster_entry_runtime_ids.clone();
        let runtime_ids_for_edge_colors = runtime_by_display_id.clone();
        let runtime_ids_for_edge_strokes = runtime_by_display_id.clone();
        let condition_successors_for_nodes = condition_successor_runtime_ids.clone();
        let condition_successors_for_edge_colors = condition_successor_runtime_ids.clone();
        let condition_successors_for_edge_strokes = condition_successor_runtime_ids.clone();
        let mut widget = Sugiyama::<Message, iced::Theme, iced::Renderer>::new(
            std::borrow::Cow::Owned(graph.clone()),
            move |node_id| {
                if visible_nodes.contains(&node_id) {
                    if let Some(node) = node_map.get(&node_id) {
                        let phase = node_phase_for_display(
                            live_data,
                            node.id,
                            node.runtime_node_id,
                            &condition_successors_for_nodes,
                        );
                        let step_ctx = StepViewCtx {
                            display_id: node.id,
                            runtime_id: node.runtime_node_id,
                            successor_runtime_ids: condition_successors_for_nodes
                                .get(&node.id)
                                .cloned()
                                .unwrap_or_default(),
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
                            entry_runtime_ids: cluster_entry_runtime_ids_for_nodes[cluster_index]
                                .clone(),
                            member_runtime_ids: cluster_member_runtime_ids_for_nodes[cluster_index]
                                .clone(),
                            successor_runtime_ids: cluster_successor_runtime_ids_for_nodes
                                [cluster_index]
                                .clone(),
                            phase: cluster_phase(cluster),
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
            let style = theme
                .edge_style(
                    theme_state,
                    EdgeStyleCtx {
                        edge_index: ctx.edge_index,
                        source_display_id: ctx.edge.0,
                        target_display_id: ctx.edge.1,
                        source_runtime_id,
                        target_runtime_id,
                        source_phase: node_phase_for_display(
                            live_data,
                            ctx.edge.0,
                            source_runtime_id,
                            &condition_successors_for_edge_colors,
                        ),
                        target_phase: node_phase_for_display(
                            live_data,
                            ctx.edge.1,
                            target_runtime_id,
                            &condition_successors_for_edge_colors,
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
            let style = theme
                .edge_style(
                    theme_state,
                    EdgeStyleCtx {
                        edge_index: ctx.edge_index,
                        source_display_id: ctx.edge.0,
                        target_display_id: ctx.edge.1,
                        source_runtime_id,
                        target_runtime_id,
                        source_phase: node_phase_for_display(
                            live_data,
                            ctx.edge.0,
                            source_runtime_id,
                            &condition_successors_for_edge_strokes,
                        ),
                        target_phase: node_phase_for_display(
                            live_data,
                            ctx.edge.1,
                            target_runtime_id,
                            &condition_successors_for_edge_strokes,
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
                entry_runtime_ids: cluster_entry_runtime_ids[source_index].clone(),
                member_runtime_ids: cluster_member_runtime_ids[source_index].clone(),
                successor_runtime_ids: cluster_successor_runtime_ids[source_index].clone(),
                phase: cluster_phase(cluster),
            };
            match theme.view_cluster(theme_state, &cx) {
                ClusterView::Expanded { overlay, .. } => overlay
                    .map(|element| element.map(|_event| Message::Theme(ViewerEvent::Message(())))),
                ClusterView::Collapsed { .. } => None,
            }
        })
        .cluster_color(cluster_fill_color)
        .padding(24);
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
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(graph_panel_style)
    .into()
}

#[derive(Clone)]
struct GraphModel {
    nodes: Vec<NodeDisplay>,
    node_map: HashMap<u32, NodeDisplay>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<Cluster>,
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

        let node_map = builder
            .nodes
            .iter()
            .map(|node| (node.id, node.clone()))
            .collect::<HashMap<_, _>>();

        Self {
            nodes: builder.nodes,
            node_map,
            edges: builder.edges,
            clusters: builder.clusters.clone(),
            #[cfg(test)]
            while_clusters: builder.clusters,
            #[cfg(test)]
            while_cluster_labels: builder.cluster_labels,
            cluster_info: builder.cluster_info,
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
            JourneyAst::Select {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let select_label = if metadata.trim().is_empty() {
                    (*label).to_string()
                } else {
                    format!("{label} :: {metadata}")
                };
                let select_label = self.unique_label(select_label);
                let select = self.push_runtime_node(select_label, runtime_id);
                self.mark(select, |node| node.is_select = true);
                if !metadata.trim().is_empty() {
                    self.mark(select, |node| node.metadata = Some((*metadata).to_string()));
                }

                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                for target in &left_flow.roots {
                    self.edges.push((select, *target));
                }
                for target in &right_flow.roots {
                    self.edges.push((select, *target));
                }

                let mut members = vec![select];
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());

                Flattened {
                    roots: vec![select],
                    exits: vec![select],
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
                let join_label = if metadata.trim().is_empty() {
                    (*label).to_string()
                } else {
                    format!("{label} :: {metadata}")
                };
                let join_label = self.unique_label(join_label);
                let join = self.push_runtime_node(join_label, runtime_id);
                self.mark(join, |node| node.is_join = true);
                if !metadata.trim().is_empty() {
                    self.mark(join, |node| node.metadata = Some((*metadata).to_string()));
                }

                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                for target in &left_flow.roots {
                    self.edges.push((join, *target));
                }
                for target in &right_flow.roots {
                    self.edges.push((join, *target));
                }

                let mut members = vec![join];
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());

                Flattened {
                    roots: vec![join],
                    exits: vec![join],
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

#[derive(Clone, Copy)]
pub struct DefaultTheme;

#[derive(Debug, Clone, Copy)]
struct NodeVisual {
    from: RuntimeState,
    to: RuntimeState,
    started_at: Instant,
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
    border: ClusterBorderVisual,
}

#[derive(Debug, Clone, Copy)]
struct ClusterBorderVisual {
    from: Color,
    to: Color,
    started_at: Instant,
}

#[derive(Debug)]
pub struct DefaultThemeState {
    node_visuals: HashMap<u32, NodeVisual>,
    condition_visuals: HashMap<u32, NodeVisual>,
    cluster_index: HashMap<u32, ClusterRuntimeIndex>,
    cluster_visuals: HashMap<u32, ClusterVisual>,
    condition_successor_runtime_ids: HashMap<u32, Vec<u32>>,
    force_pending_runtime_ids: HashSet<u32>,
    runtime_update_counter: u64,
    runtime_update_order: HashMap<u32, u64>,
}

impl DefaultThemeState {
    fn register_cluster(&mut self, cx: &ClusterViewCtx<'_>, now: Instant) {
        let index = ClusterRuntimeIndex {
            kind: cx.kind,
            entry_runtime_ids: cx.entry_runtime_ids.iter().copied().collect(),
            member_runtime_ids: cx.member_runtime_ids.iter().copied().collect(),
            successor_runtime_ids: cx.successor_runtime_ids.iter().copied().collect(),
        };
        self.cluster_index.insert(cx.cluster_id, index);
        self.cluster_visuals
            .entry(cx.cluster_id)
            .or_insert(ClusterVisual {
                expanded: false,
                border: ClusterBorderVisual {
                    from: cluster_border_color_gray(),
                    to: cluster_border_color_gray(),
                    started_at: now,
                },
            });
    }

    fn cluster_is_expanded(&self, cluster_id: u32) -> bool {
        self.cluster_visuals
            .get(&cluster_id)
            .map(|visual| visual.expanded)
            .unwrap_or(false)
    }

    fn effective_node_target(&self, runtime_id: u32) -> RuntimeState {
        let mut target = self
            .node_visuals
            .get(&runtime_id)
            .map(|visual| visual.to)
            .unwrap_or(RuntimeState::Pending);
        if self.force_pending_runtime_ids.contains(&runtime_id)
            && !matches!(target, RuntimeState::Running)
        {
            target = RuntimeState::Pending;
        }
        target
    }

    fn note_runtime_update(&mut self, runtime_id: u32) {
        self.runtime_update_counter = self.runtime_update_counter.saturating_add(1);
        self.runtime_update_order
            .insert(runtime_id, self.runtime_update_counter);
    }

    fn infer_condition_target(
        &self,
        display_id: u32,
        fallback_phase: Phase<RuntimeState>,
    ) -> RuntimeState {
        let mut newest: Option<(u64, RuntimeState)> = None;
        if let Some(successors) = self.condition_successor_runtime_ids.get(&display_id) {
            for runtime_id in successors {
                let Some(order) = self.runtime_update_order.get(runtime_id).copied() else {
                    continue;
                };
                let state = self.effective_node_target(*runtime_id);
                if newest.map(|(best, _)| order > best).unwrap_or(true) {
                    newest = Some((order, state));
                }
            }
        }
        newest
            .map(|(_, state)| state)
            .unwrap_or(match fallback_phase {
                Phase::Live(state) => state,
                Phase::Static => RuntimeState::Pending,
            })
    }

    fn update_node_state(&mut self, runtime_id: u32, to: RuntimeState, now: Instant) -> bool {
        if !matches!(to, RuntimeState::Pending) {
            self.force_pending_runtime_ids.remove(&runtime_id);
        }
        let entry = self.node_visuals.entry(runtime_id).or_insert(NodeVisual {
            from: RuntimeState::Pending,
            to: RuntimeState::Pending,
            started_at: now,
        });

        if entry.to == to {
            return false;
        }

        let blended = sampled_runtime_state(entry, now);
        entry.from = blended;
        entry.to = to;
        entry.started_at = now;
        self.note_runtime_update(runtime_id);
        true
    }

    fn reset_cluster_members_to_pending(
        &mut self,
        cluster_id: u32,
        except_runtime_id: u32,
        now: Instant,
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
            changed |= self.update_node_state(member_id, RuntimeState::Pending, now);
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

            let mut just_opened = false;
            if let Some(visual) = self.cluster_visuals.get_mut(&cluster_id) {
                if (is_while_cluster && contains_entry) || (!visual.expanded && contains_member) {
                    visual.expanded = true;
                    changed |= update_cluster_border_visual(
                        &mut visual.border,
                        cluster_border_color_gray(),
                        cluster_border_color_running(),
                        now,
                    );
                    just_opened = true;
                } else if visual.expanded && contains_successor {
                    let current = current_cluster_border_color(visual.border, now);
                    changed |= update_cluster_border_visual(
                        &mut visual.border,
                        current,
                        cluster_border_color_completed(),
                        now,
                    );
                }
            }

            if just_opened || (is_while_cluster && contains_entry) {
                changed |= self.reset_cluster_members_to_pending(cluster_id, runtime_id, now);
            }
        }
        changed
    }

    fn maybe_collapse_completed_cluster_for_pending_successor(
        &mut self,
        cx: &ClusterViewCtx<'_>,
        now: Instant,
    ) -> bool {
        if cx.successor_runtime_ids.is_empty() {
            return false;
        }
        if !matches!(cx.kind, ClusterKind::While | ClusterKind::Transparent) {
            return false;
        }

        let should_collapse = self
            .cluster_visuals
            .get(&cx.cluster_id)
            .map(|visual| visual.expanded && visual.border.to == cluster_border_color_completed())
            .unwrap_or(false);
        if !should_collapse {
            return false;
        }

        let successor_pending = cx.successor_runtime_ids.iter().any(|runtime_id| {
            matches!(
                self.effective_node_target(*runtime_id),
                RuntimeState::Pending
            )
        });
        if !successor_pending {
            return false;
        }

        let Some(visual) = self.cluster_visuals.get_mut(&cx.cluster_id) else {
            return false;
        };
        visual.expanded = false;
        let current = current_cluster_border_color(visual.border, now);
        let mut changed = true;
        changed |= update_cluster_border_visual(
            &mut visual.border,
            current,
            cluster_border_color_gray(),
            now,
        );
        changed
    }

    fn has_running_cluster_animations(&self, now: Instant) -> bool {
        self.cluster_visuals.values().any(|visual| {
            visual.border.from != visual.border.to
                && now.duration_since(visual.border.started_at) < CLUSTER_BORDER_ANIMATION_DURATION
        })
    }

    fn cluster_border_color(&self, cluster_id: u32, now: Instant) -> Color {
        self.cluster_visuals
            .get(&cluster_id)
            .map(|visual| current_cluster_border_color(visual.border, now))
            .unwrap_or_else(cluster_border_color_gray)
    }

    fn settle_cluster_animations(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for visual in self.cluster_visuals.values_mut() {
            let border = &mut visual.border;
            if border.from == border.to {
                continue;
            }
            if now.duration_since(border.started_at) >= CLUSTER_BORDER_ANIMATION_DURATION {
                border.from = border.to;
                changed = true;
            }
        }
        changed
    }

    fn has_running_animations(&self, now: Instant) -> bool {
        self.node_visuals
            .values()
            .chain(self.condition_visuals.values())
            .any(|visual| {
                visual.from != visual.to
                    && now.duration_since(visual.started_at) < NODE_ANIMATION_DURATION
            })
            || self.has_running_cluster_animations(now)
    }

    fn settle_animations(&mut self, now: Instant) -> bool {
        let mut changed = false;
        for visual in self.node_visuals.values_mut() {
            if visual.from == visual.to {
                continue;
            }
            if now.duration_since(visual.started_at) >= NODE_ANIMATION_DURATION {
                visual.from = visual.to;
                changed = true;
            }
        }
        for visual in self.condition_visuals.values_mut() {
            if visual.from == visual.to {
                continue;
            }
            if now.duration_since(visual.started_at) >= NODE_ANIMATION_DURATION {
                visual.from = visual.to;
                changed = true;
            }
        }
        changed |= self.settle_cluster_animations(now);
        changed
    }
}

impl JunglePanelTheme<AnyAnimal> for DefaultTheme {
    type State = Mutex<DefaultThemeState>;
    type Message = ();

    fn init(&self) -> Self::State {
        Mutex::new(DefaultThemeState {
            node_visuals: HashMap::new(),
            condition_visuals: HashMap::new(),
            cluster_index: HashMap::new(),
            cluster_visuals: HashMap::new(),
            condition_successor_runtime_ids: HashMap::new(),
            force_pending_runtime_ids: HashSet::new(),
            runtime_update_counter: 0,
            runtime_update_order: HashMap::new(),
        })
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: ViewerEvent<Self::Message>,
    ) -> Task<ViewerEvent<Self::Message>> {
        let now = Instant::now();
        let guard = state.get_mut();
        let mut should_tick = false;

        match event {
            ViewerEvent::JourneyUpdate(update) => match update.event {
                RunnerUpdateOut::EffectInput { node_id, .. } => {
                    let node_changed = guard.update_node_state(node_id, RuntimeState::Running, now);
                    let cluster_changed = guard.update_clusters_for_effect_input(node_id, now);
                    should_tick = node_changed || cluster_changed;
                }
                RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => {
                    should_tick = guard.update_node_state(node_id, RuntimeState::Completed, now);
                }
                RunnerUpdateOut::EffectFailureOutput { node_id, .. } => {
                    should_tick = guard.update_node_state(node_id, RuntimeState::Failed, now);
                }
                RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
            },
            ViewerEvent::Message(()) => {
                let settled = guard.settle_animations(now);
                should_tick = settled || guard.has_running_animations(now);
            }
        }

        if should_tick {
            return next_tick();
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

        let now = Instant::now();
        let fill = if let Some(runtime_id) = cx.runtime_id {
            if let Ok(mut guard) = state.try_lock() {
                let forced_pending = guard.force_pending_runtime_ids.contains(&runtime_id);
                let visual = guard.node_visuals.entry(runtime_id).or_insert(NodeVisual {
                    from: RuntimeState::Pending,
                    to: RuntimeState::Pending,
                    started_at: now,
                });

                let mut phase_target = match cx.phase {
                    Phase::Live(target) => target,
                    Phase::Static => RuntimeState::Pending,
                };
                if forced_pending && !matches!(phase_target, RuntimeState::Running) {
                    phase_target = RuntimeState::Pending;
                }
                if visual.to != phase_target {
                    let blended = sampled_runtime_state(visual, now);
                    visual.from = blended;
                    visual.to = phase_target;
                    visual.started_at = now;
                }
                blend_runtime_color(*visual, now)
            } else {
                let phase_target = match cx.phase {
                    Phase::Live(target) => target,
                    Phase::Static => RuntimeState::Pending,
                };
                runtime_color(phase_target)
            }
        } else if matches!(cx.kind, StepKind::Conditional) {
            if let Ok(mut guard) = state.try_lock() {
                if !cx.successor_runtime_ids.is_empty() {
                    guard
                        .condition_successor_runtime_ids
                        .insert(cx.display_id, cx.successor_runtime_ids.clone());
                }
                let phase_target = guard.infer_condition_target(cx.display_id, cx.phase);
                let visual = guard
                    .condition_visuals
                    .entry(cx.display_id)
                    .or_insert(NodeVisual {
                        from: RuntimeState::Pending,
                        to: RuntimeState::Pending,
                        started_at: now,
                    });
                if visual.to != phase_target {
                    let blended = sampled_runtime_state(visual, now);
                    visual.from = blended;
                    visual.to = phase_target;
                    visual.started_at = now;
                }
                blend_runtime_color(*visual, now)
            } else {
                let phase_target = match cx.phase {
                    Phase::Live(target) => target,
                    Phase::Static => RuntimeState::Pending,
                };
                runtime_color(phase_target)
            }
        } else {
            Color::from_rgb8(120, 120, 120)
        };

        let body = column![
            text(role).size(10).color(Color::from_rgb8(168, 198, 181)),
            text(cx.label.to_string())
                .size(13)
                .color(Color::from_rgb8(223, 245, 230))
        ]
        .spacing(4);

        (
            button(body)
                .padding([8, 10])
                .width(Length::Shrink)
                .style(move |_theme, _status| iced::widget::button::Style {
                    background: Some(iced::Background::Color(fill)),
                    text_color: Color::from_rgb8(223, 245, 230),
                    border: iced::border::rounded(10)
                        .color(Color::from_rgb8(58, 122, 86))
                        .width(1.0),
                    ..Default::default()
                })
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
            guard.register_cluster(cx, now);
            guard.maybe_collapse_completed_cluster_for_pending_successor(cx, now);
            (
                guard.cluster_is_expanded(cx.cluster_id),
                guard.cluster_border_color(cx.cluster_id, now),
            )
        } else {
            (false, cluster_border_color_gray())
        };

        let overlay = container(
            container(text(cx.label.to_string()).size(11).color(border_color))
                .padding([4, 8])
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba8(20, 46, 30, 0.35))),
                    border: iced::border::rounded(6).color(border_color).width(1.0),
                    text_color: Some(border_color),
                    ..Default::default()
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .align_x(iced::alignment::Horizontal::Left)
        .align_y(iced::alignment::Vertical::Top)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::TRANSPARENT)),
            border: iced::border::rounded(10).color(border_color).width(2.0),
            text_color: Some(border_color),
            ..Default::default()
        })
        .into();

        if expanded {
            ClusterView::Expanded {
                overlay: Some(overlay),
                fill: Color::TRANSPARENT,
            }
        } else {
            ClusterView::Collapsed {
                element: button(text(cx.label.to_string()).size(11).color(border_color))
                    .padding([6, 10])
                    .width(Length::Shrink)
                    .style(move |_theme, _status| iced::widget::button::Style {
                        background: None,
                        text_color: border_color,
                        border: iced::border::rounded(8).color(border_color).width(1.4),
                        ..Default::default()
                    })
                    .into(),
                size: (240.0, 46.0),
            }
        }
    }

    fn edge_style(&self, state: &Self::State, cx: EdgeStyleCtx) -> Option<EdgeStyle> {
        let now = Instant::now();
        let (from_color, to_color) = if let Some(runtime_id) = cx.source_runtime_id {
            if let Ok(mut guard) = state.try_lock() {
                let forced_pending = guard.force_pending_runtime_ids.contains(&runtime_id);
                let visual = guard.node_visuals.entry(runtime_id).or_insert(NodeVisual {
                    from: RuntimeState::Pending,
                    to: RuntimeState::Pending,
                    started_at: now,
                });

                let mut phase_target = match cx.source_phase {
                    Phase::Live(target) => target,
                    Phase::Static => RuntimeState::Pending,
                };
                if forced_pending && !matches!(phase_target, RuntimeState::Running) {
                    phase_target = RuntimeState::Pending;
                }
                if visual.to != phase_target {
                    let blended = sampled_runtime_state(visual, now);
                    visual.from = blended;
                    visual.to = phase_target;
                    visual.started_at = now;
                }

                (runtime_color(visual.from), runtime_color(visual.to))
            } else {
                let phase_target = match cx.source_phase {
                    Phase::Live(target) => target,
                    Phase::Static => RuntimeState::Pending,
                };
                let color = runtime_color(phase_target);
                (color, color)
            }
        } else {
            let phase_target = match cx.source_phase {
                Phase::Live(target) => target,
                Phase::Static => RuntimeState::Pending,
            };
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

fn sampled_runtime_state(visual: &NodeVisual, now: Instant) -> RuntimeState {
    if now.duration_since(visual.started_at) >= NODE_ANIMATION_DURATION {
        return visual.to;
    }
    visual.from
}

fn update_cluster_border_visual(
    visual: &mut ClusterBorderVisual,
    from: Color,
    to: Color,
    now: Instant,
) -> bool {
    if visual.to == to && visual.from == from {
        return false;
    }
    visual.from = from;
    visual.to = to;
    visual.started_at = now;
    true
}

fn current_cluster_border_color(visual: ClusterBorderVisual, now: Instant) -> Color {
    if visual.from == visual.to {
        return visual.to;
    }
    let elapsed = now.saturating_duration_since(visual.started_at);
    let t =
        (elapsed.as_secs_f32() / CLUSTER_BORDER_ANIMATION_DURATION.as_secs_f32()).clamp(0.0, 1.0);
    lerp_color(visual.from, visual.to, ease_out_cubic(t))
}

fn blend_runtime_color(visual: NodeVisual, now: Instant) -> Color {
    if visual.from == visual.to {
        return runtime_color(visual.to);
    }

    let elapsed = now.saturating_duration_since(visual.started_at);
    let t = (elapsed.as_secs_f32() / NODE_ANIMATION_DURATION.as_secs_f32()).clamp(0.0, 1.0);
    lerp_color(
        runtime_color(visual.from),
        runtime_color(visual.to),
        ease_out_cubic(t),
    )
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

fn cluster_border_color_running() -> Color {
    runtime_color(RuntimeState::Running)
}

fn cluster_border_color_completed() -> Color {
    runtime_color(RuntimeState::Completed)
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

fn next_tick() -> Task<ViewerEvent<()>> {
    Task::perform(
        async move {
            tokio::time::sleep(ANIMATION_TICK).await;
        },
        ViewerEvent::Message,
    )
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
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.active_runtime_ids.contains(&9));

        assert!(!live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 3,
            event: RunnerUpdateOut::EffectSuccessOutput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(!live.active_runtime_ids.contains(&9));
        assert!(live.finished_runtime_ids.contains(&9));

        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 4,
            event: RunnerUpdateOut::EffectInput {
                node_id: 9,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.active_runtime_ids.contains(&9));
        assert!(!live.finished_runtime_ids.contains(&9));

        assert!(!live.apply_update(JourneyUpdateEvent {
            sequence_id: 5,
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
            event: RunnerUpdateOut::EffectInput {
                node_id: 11,
                uuid: Uuid::nil(),
            },
        }));
        assert!(live.apply_update(JourneyUpdateEvent {
            sequence_id: 2,
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
        let join_id = id_for("Join");
        let join_l_id = id_for("JoinL");
        let join_r_id = id_for("JoinR");
        let select_id = id_for("Select");
        let sel_l_id = id_for("SelL");
        let sel_r_id = id_for("SelR");
        let tail_id = id_for("Tail");

        assert!(
            model.nodes.iter().all(|node| node.label != "LoopCondition"),
            "while loops should not render as standalone nodes"
        );

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();

        assert!(edges.contains(&(branch_id, loop_l_id)));
        assert!(edges.contains(&(branch_id, loop_r_id)));
        assert!(edges.contains(&(loop_l_id, branch_id)));
        assert!(edges.contains(&(loop_r_id, branch_id)));
        assert!(edges.contains(&(loop_l_id, join_id)));
        assert!(edges.contains(&(loop_r_id, join_id)));
        assert!(!edges.contains(&(branch_id, join_id)));

        assert!(edges.contains(&(join_id, join_l_id)));
        assert!(edges.contains(&(join_id, join_r_id)));
        assert!(edges.contains(&(join_id, select_id)));
        assert!(!edges.contains(&(join_l_id, select_id)));
        assert!(!edges.contains(&(join_r_id, select_id)));

        assert!(edges.contains(&(select_id, sel_l_id)));
        assert!(edges.contains(&(select_id, sel_r_id)));
        assert!(edges.contains(&(select_id, tail_id)));
        assert!(!edges.contains(&(sel_l_id, tail_id)));
        assert!(!edges.contains(&(sel_r_id, tail_id)));
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
        let join_id = id_for("Join");
        let select_id = id_for("Select");

        assert_eq!(model.while_clusters.len(), 1);
        let cluster = &model.while_clusters[0];
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        assert!(cluster_nodes.contains(&cond_id));
        assert!(cluster_nodes.contains(&in_l_id));
        assert!(cluster_nodes.contains(&in_r_id));
        assert!(!cluster_nodes.contains(&join_id));
        assert!(!cluster_nodes.contains(&select_id));
        assert_eq!(model.while_cluster_labels, vec!["while: LoopCondition"]);

        let edges = model.edges.iter().copied().collect::<HashSet<_>>();
        assert!(edges.contains(&(cond_id, in_l_id)));
        assert!(edges.contains(&(cond_id, in_r_id)));
        assert!(edges.contains(&(in_l_id, cond_id)));
        assert!(edges.contains(&(in_r_id, cond_id)));
        assert!(edges.contains(&(in_l_id, join_id)));
        assert!(edges.contains(&(in_r_id, join_id)));
        assert!(!edges.contains(&(cond_id, join_id)));
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
    fn while_cluster_border_resets_on_reentry() {
        let mut state = DefaultThemeState {
            node_visuals: HashMap::new(),
            condition_visuals: HashMap::new(),
            cluster_index: HashMap::new(),
            cluster_visuals: HashMap::new(),
            condition_successor_runtime_ids: HashMap::new(),
            force_pending_runtime_ids: HashSet::new(),
            runtime_update_counter: 0,
            runtime_update_order: HashMap::new(),
        };

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
            entry_runtime_ids: vec![18],
            member_runtime_ids: vec![18, 19],
            successor_runtime_ids: vec![32],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx, started_at);

        let first_entry = started_at + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(18, first_entry));
        let border = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border;
        assert_eq!(border.from, cluster_border_color_gray());
        assert_eq!(border.to, cluster_border_color_running());

        let first_exit = first_entry + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(32, first_exit));
        let border = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border;
        assert_eq!(border.to, cluster_border_color_completed());

        let second_entry = first_exit + Duration::from_millis(1);
        assert!(state.update_clusters_for_effect_input(18, second_entry));
        let border = state
            .cluster_visuals
            .get(&9)
            .expect("cluster visual should exist")
            .border;
        assert_eq!(border.from, cluster_border_color_gray());
        assert_eq!(border.to, cluster_border_color_running());
    }

    #[test]
    fn completed_cluster_recollapses_when_successor_returns_to_pending() {
        let mut state = DefaultThemeState {
            node_visuals: HashMap::new(),
            condition_visuals: HashMap::new(),
            cluster_index: HashMap::new(),
            cluster_visuals: HashMap::new(),
            condition_successor_runtime_ids: HashMap::new(),
            force_pending_runtime_ids: HashSet::new(),
            runtime_update_counter: 0,
            runtime_update_order: HashMap::new(),
        };

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
            entry_runtime_ids: vec![70],
            member_runtime_ids: vec![70, 71],
            successor_runtime_ids: vec![95],
            phase: Phase::Live(ClusterLive {
                has_running: false,
                has_failed: false,
                has_completed: false,
            }),
        };
        state.register_cluster(&cx, started_at);

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
        let border = state
            .cluster_visuals
            .get(&12)
            .expect("cluster visual should exist")
            .border;
        assert_eq!(border.to, cluster_border_color_completed());

        let successor_completed = exit + Duration::from_millis(1);
        assert!(state.update_node_state(95, RuntimeState::Completed, successor_completed));
        let successor_pending = successor_completed + Duration::from_millis(1);
        assert!(state.update_node_state(95, RuntimeState::Pending, successor_pending));

        assert!(
            state.maybe_collapse_completed_cluster_for_pending_successor(
                &cx,
                successor_pending + Duration::from_millis(1)
            )
        );
        let visual = state
            .cluster_visuals
            .get(&12)
            .expect("cluster visual should exist");
        assert!(!visual.expanded);
        assert_eq!(visual.border.to, cluster_border_color_gray());
    }
}
