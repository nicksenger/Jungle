use iced::widget::{button, column, container, row, text, Space};
use iced::window;
use iced::window::Screenshot;
use iced::{Color, Element, Font, Length, Subscription, Task};
use iced_sugiyama::{Cluster, Graph, Sugiyama};
use iced::futures::{self, Stream, StreamExt};
use jungle_client::JungleClient;
use jungle_types::{Animal, JourneyAst, JourneyAstSource, JourneyUpdateEvent, RunnerUpdateOut};
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1360.0;
const WINDOW_HEIGHT: f32 = 900.0;
const NODE_WIDTH: f64 = 240.0;
const NODE_HEIGHT: f64 = 80.0;
const GRAPH_WIDGET_ID: &str = "jungle-viewer";

#[derive(Clone)]
pub struct JungleViewerBuilder {
    title: String,
    width: f32,
    height: f32,
    screenshot_path: Option<PathBuf>,
    headless: bool,
}

impl Default for JungleViewerBuilder {
    fn default() -> Self {
        Self {
            title: "Jungle Viewer".to_string(),
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            screenshot_path: None,
            headless: false,
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

    pub fn view_animal<A>(self) -> iced::Result
    where
        A: Animal + 'static,
        A::Journey: JourneyAstSource,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
        let model = GraphModel::from_ast(ast);

        self.run(ViewMode::Static {
            journey_name,
            model,
        })
    }

    pub fn view_live_animal<A, C>(self, client: C, journey_id: Uuid) -> iced::Result
    where
        A: Animal + 'static,
        A::Journey: JourneyAstSource,
        C: JungleClient + 'static,
    {
        let ast = <A::Journey as JourneyAstSource>::journey_ast();
        let journey_name = short_type_name::<A::Journey>();
        let model = GraphModel::from_ast(ast);
        let client: Arc<dyn JungleClient> = Arc::new(client);

        self.run(ViewMode::Live {
            journey_name,
            model,
            client,
            journey_id,
        })
    }

    fn run(self, mode: ViewMode) -> iced::Result {
        let title = self.title.clone();
        let width = self.width;
        let height = self.height;
        let capture = self.screenshot_path.clone().map(|path| CaptureConfig {
            output_path: path,
            close_after_capture: self.headless,
        });
        iced::application(
            move || ViewerApp::new(mode.clone(), capture.clone()),
            ViewerApp::update,
            ViewerApp::view,
        )
        .title(move |_app: &ViewerApp| title.clone())
        .subscription(ViewerApp::subscription)
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
            .while_clusters
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

struct ViewerApp {
    mode: ViewMode,
    state: LiveState,
    live_generation: u64,
    capture: Option<CaptureConfig>,
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
    latest_event_count: usize,
}

#[derive(Debug, Clone)]
enum Message {
    AppStarted,
    LiveEvent(Result<JourneyUpdateEvent, String>),
    Retry,
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

impl ViewerApp {
    fn new(mode: ViewMode, capture: Option<CaptureConfig>) -> (Self, Task<Message>) {
        let state = match &mode {
            ViewMode::Live { .. } => LiveState::Loading,
            ViewMode::Static { .. } => LiveState::Idle,
        };

        (
            Self {
                mode,
                state,
                live_generation: 0,
                capture,
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
                        data.apply_update(update);
                    }
                    Err(error) => {
                        self.state = LiveState::Error(error);
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
            graph_panel(model, live_data)
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
    fn apply_update(&mut self, update: JourneyUpdateEvent) {
        self.latest_event_count = update.sequence_id as usize;
        match update.event {
            RunnerUpdateOut::ActionInput { node_id, .. } => {
                self.active_runtime_ids.insert(node_id);
            }
            RunnerUpdateOut::ActionSuccessOutput { node_id, .. } => {
                self.active_runtime_ids.remove(&node_id);
                self.finished_runtime_ids.insert(node_id);
            }
            RunnerUpdateOut::ActionFailureOutput { node_id, .. } => {
                self.active_runtime_ids.remove(&node_id);
                self.failed_runtime_ids.insert(node_id);
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
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
        text(format!("loops: {}", model.while_clusters.len()))
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
        text("Step: action request node")
            .size(12)
            .color(jungle_text_muted()),
        text("Conditional: branch fanout")
            .size(12)
            .color(jungle_text_muted()),
        text("While: clustered body + condition label")
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

fn graph_panel<'a>(model: &'a GraphModel, live_data: Option<&'a LiveData>) -> Element<'a, Message> {
    let nodes_for_view = model.node_map.clone();
    let highlights = live_data.cloned();

    let clusters = model.while_clusters.clone();
    let cluster_labels = model.while_cluster_labels.clone();

    let graph =
        Sugiyama::<Message, iced::Theme, iced::Renderer>::new(&model.graph, move |node_id| {
            let info = nodes_for_view
                .get(&node_id)
                .cloned()
                .unwrap_or_else(|| NodeDisplay::unknown(node_id));
            let live_state = highlights.as_ref().and_then(|live| {
                info.runtime_node_id
                    .and_then(|rid| live_state_for_node(rid, live))
            });

            let label = truncate_label(&info.label, 58);

            let badge = if info.is_conditional_branch {
                "condition"
            } else if info.is_join {
                "join"
            } else if info.is_select {
                "select"
            } else if info.is_transparent {
                "transparent"
            } else {
                "step"
            };

            let content = column![
                text(badge).size(10).color(jungle_text_muted()),
                text(label).size(13).color(jungle_text_base())
            ]
            .spacing(4);

            button(content)
                .padding([8, 10])
                .width(Length::Shrink)
                .style(move |_theme, status| node_button_style(status, &info, live_state))
                .into()
        })
        .id(iced_sugiyama::Id::new(GRAPH_WIDGET_ID))
        .edge_color(jungle_edge)
        .stroke_width(1.6)
        .edge_corner_radius(18.0)
        .node_size(move |_node_id| (NODE_WIDTH, NODE_HEIGHT))
        .clusters(clusters)
        .cluster_container(move |index, _| {
            let label = cluster_labels
                .get(index)
                .cloned()
                .unwrap_or_else(|| format!("while #{index}"));
            Some(
                container(text(label).size(11).color(jungle_text_muted()))
                    .padding([4, 8])
                    .style(loop_cluster_label)
                    .into(),
            )
        })
        .cluster_color(loop_cluster_color)
        .padding(24);

    container(container(graph).width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(graph_panel_style)
        .into()
}

fn live_state_for_node(runtime_id: u32, live: &LiveData) -> Option<RuntimeNodeState> {
    if live.failed_runtime_ids.contains(&runtime_id) {
        return Some(RuntimeNodeState::Failed);
    }
    if live.active_runtime_ids.contains(&runtime_id) {
        return Some(RuntimeNodeState::Active);
    }
    if live.finished_runtime_ids.contains(&runtime_id) {
        return Some(RuntimeNodeState::Finished);
    }
    None
}

#[derive(Clone, Copy)]
enum RuntimeNodeState {
    Active,
    Finished,
    Failed,
}

#[derive(Clone)]
struct GraphModel {
    graph: Graph,
    nodes: Vec<NodeDisplay>,
    node_map: HashMap<u32, NodeDisplay>,
    edges: Vec<(u32, u32)>,
    while_clusters: Vec<Cluster>,
    while_cluster_labels: Vec<String>,
}

impl GraphModel {
    fn from_ast(ast: JourneyAst) -> Self {
        let mut builder = GraphBuilder::default();
        builder.flatten(&ast);

        let graph = Graph::new(
            builder.nodes.iter().map(|node| node.id).collect(),
            builder.edges.clone(),
        );

        let node_map = builder
            .nodes
            .iter()
            .map(|node| (node.id, node.clone()))
            .collect::<HashMap<_, _>>();

        Self {
            graph,
            nodes: builder.nodes,
            node_map,
            edges: builder.edges,
            while_clusters: builder.clusters,
            while_cluster_labels: builder.cluster_labels,
        }
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<NodeDisplay>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<Cluster>,
    cluster_labels: Vec<String>,
    while_cluster_stack: Vec<usize>,
    runtime_next_id: u32,
    display_next_id: u32,
    label_occurrences: HashMap<String, u32>,
}

#[derive(Clone)]
struct NodeDisplay {
    id: u32,
    label: String,
    runtime_node_id: Option<u32>,
    is_conditional_branch: bool,
    is_select: bool,
    is_join: bool,
    is_transparent: bool,
}

impl NodeDisplay {
    fn unknown(id: u32) -> Self {
        Self {
            id,
            label: format!("node {id}"),
            runtime_node_id: None,
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
            is_transparent: false,
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
            JourneyAst::Conditional { label, left, right } => {
                let branch = self.push_layout_node(short_type_name_str(label), |node| {
                    node.is_conditional_branch = true;
                });
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
            JourneyAst::While { label, body } => {
                let parent_cluster = self.while_cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster = Cluster::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);
                self.cluster_labels
                    .push(format!("while: {}", short_type_name_str(label)));
                self.while_cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.while_cluster_stack.pop();

                for exit in &body_flow.exits {
                    for root in &body_flow.roots {
                        self.edges.push((*exit, *root));
                    }
                }

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes;
                }

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.roots,
                    members: body_flow.members,
                }
            }
            JourneyAst::Transparent {
                label,
                metadata,
                body,
            } => {
                let merged = if metadata.trim().is_empty() {
                    short_type_name_str(label)
                } else {
                    format!("{} :: {}", short_type_name_str(label), metadata)
                };
                let transparent = self.push_layout_node(merged, |node| {
                    node.is_transparent = true;
                });

                let body_flow = self.flatten(body);
                for target in &body_flow.roots {
                    self.edges.push((transparent, *target));
                }

                let mut members = vec![transparent];
                members.extend(body_flow.members.iter().copied());

                let exits = if body_flow.exits.is_empty() {
                    vec![transparent]
                } else {
                    body_flow.exits
                };

                Flattened {
                    roots: vec![transparent],
                    exits,
                    members,
                }
            }
            JourneyAst::Select { left, right, .. } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let label = self.unique_label("Select");
                let select = self.push_runtime_node(label, runtime_id);
                self.mark(select, |node| node.is_select = true);

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
            JourneyAst::Join { left, right, .. } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let label = self.unique_label("Join");
                let join = self.push_runtime_node(label, runtime_id);
                self.mark(join, |node| node.is_join = true);

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
            runtime_node_id: Some(runtime_id),
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
            is_transparent: false,
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
            runtime_node_id: None,
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
            is_transparent: false,
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

fn truncate_label(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let mut out = String::new();
    for (index, ch) in label.chars().enumerate() {
        if index + 1 >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
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

fn node_button_style(
    status: button::Status,
    node: &NodeDisplay,
    live_state: Option<RuntimeNodeState>,
) -> iced::widget::button::Style {
    let mut base = if node.is_conditional_branch {
        Color::from_rgb8(23, 71, 47)
    } else if node.is_select || node.is_join {
        Color::from_rgb8(19, 84, 58)
    } else if node.is_transparent {
        Color::from_rgb8(16, 57, 36)
    } else {
        Color::from_rgb8(25, 99, 65)
    };

    if let Some(state) = live_state {
        base = match state {
            RuntimeNodeState::Active => Color::from_rgb8(123, 166, 52),
            RuntimeNodeState::Finished => Color::from_rgb8(70, 166, 92),
            RuntimeNodeState::Failed => Color::from_rgb8(155, 57, 57),
        };
    }

    let mut border_color = jungle_accent_dark();
    if matches!(status, button::Status::Hovered) {
        border_color = jungle_accent_bright();
    }

    iced::widget::button::Style {
        background: Some(iced::Background::Color(base)),
        text_color: jungle_text_base(),
        border: iced::border::rounded(10).color(border_color).width(
            if matches!(status, button::Status::Hovered) {
                1.6
            } else {
                1.0
            },
        ),
        shadow: if matches!(status, button::Status::Hovered) {
            iced::Shadow {
                color: Color::from_rgba8(80, 220, 130, 0.28),
                offset: iced::Vector::new(0.0, 1.0),
                blur_radius: 8.0,
            }
        } else {
            iced::Shadow::default()
        },
        ..Default::default()
    }
}

fn jungle_edge(_index: usize) -> (Color, Color) {
    (
        Color::from_rgb8(64, 169, 104),
        Color::from_rgb8(40, 104, 67),
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

fn loop_cluster_label(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgba8(20, 46, 30, 0.35))),
        border: iced::border::rounded(6)
            .color(Color::from_rgb8(54, 117, 78))
            .width(1.0),
        text_color: Some(jungle_text_muted()),
        ..Default::default()
    }
}

fn loop_cluster_color(_index: usize) -> Color {
    Color::from_rgba8(30, 91, 53, 0.04)
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
    use std::collections::HashSet;

    #[test]
    fn graph_model_uses_unique_display_node_ids() {
        let ast = JourneyAst::Sequence(vec![
            JourneyAst::While {
                label: "Loop",
                body: Box::new(JourneyAst::Sequence(vec![
                    JourneyAst::Step { label: "A1" },
                    JourneyAst::Conditional {
                        label: "Branch",
                        left: Box::new(JourneyAst::Step { label: "A2" }),
                        right: Box::new(JourneyAst::Step { label: "A3" }),
                    },
                ])),
            },
            JourneyAst::Select {
                label: "Select",
                left: Box::new(JourneyAst::Step { label: "A4" }),
                right: Box::new(JourneyAst::Step { label: "A5" }),
            },
            JourneyAst::Join {
                label: "Join",
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
                body: Box::new(JourneyAst::Conditional {
                    label: "flow::Branch",
                    left: Box::new(JourneyAst::Step { label: "LoopL" }),
                    right: Box::new(JourneyAst::Step { label: "LoopR" }),
                }),
            },
            JourneyAst::Join {
                label: "Join",
                left: Box::new(JourneyAst::Step { label: "JoinL" }),
                right: Box::new(JourneyAst::Step { label: "JoinR" }),
            },
            JourneyAst::Select {
                label: "Select",
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
        assert!(edges.contains(&(branch_id, join_id)));

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
                body: Box::new(JourneyAst::Conditional {
                    label: "flow::StaticCondition",
                    left: Box::new(JourneyAst::Step { label: "InLoopL" }),
                    right: Box::new(JourneyAst::Step { label: "InLoopR" }),
                }),
            },
            JourneyAst::Join {
                label: "Join",
                left: Box::new(JourneyAst::Step { label: "OutJoinL" }),
                right: Box::new(JourneyAst::Step { label: "OutJoinR" }),
            },
            JourneyAst::Select {
                label: "Select",
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
        assert!(edges.contains(&(cond_id, join_id)));
    }

    #[test]
    fn nested_while_clusters_use_parent_relationship() {
        let ast = JourneyAst::While {
            label: "flow::OuterLoop",
            body: Box::new(JourneyAst::While {
                label: "flow::InnerLoop",
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
}
