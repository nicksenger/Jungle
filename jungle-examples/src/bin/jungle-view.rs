use iced::widget::{button, column, container, text};
use iced::{Color, Element, Length, Task};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::{JungleClient, LocalClient};
use jungle_sdk::types::RunnerUpdateOut;
use jungle_viewer::{
    AnyAnimal, ClusterView, ClusterViewCtx, JunglePanelTheme, Phase, RuntimeState, StepKind,
    StepViewCtx, ViewerEvent,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const NODE_ANIMATION_DURATION: Duration = Duration::from_millis(320);
const ANIMATION_TICK: Duration = Duration::from_millis(16);

#[derive(Clone, Copy)]
struct ExampleTheme;

#[derive(Debug, Clone, Copy)]
struct NodeVisual {
    from: RuntimeState,
    to: RuntimeState,
    started_at: Instant,
}

#[derive(Debug, Clone)]
struct ClusterRuntimeIndex {
    member_runtime_ids: HashSet<u32>,
    successor_runtime_ids: HashSet<u32>,
}

#[derive(Debug, Clone, Copy)]
struct ClusterVisual {
    expanded: bool,
}

#[derive(Debug)]
struct ExampleThemeState {
    node_visuals: HashMap<u32, NodeVisual>,
    cluster_index: HashMap<u32, ClusterRuntimeIndex>,
    cluster_visuals: HashMap<u32, ClusterVisual>,
}

impl ExampleThemeState {
    fn register_cluster(&mut self, cx: &ClusterViewCtx<'_>) {
        let index = ClusterRuntimeIndex {
            member_runtime_ids: cx.member_runtime_ids.iter().copied().collect(),
            successor_runtime_ids: cx.successor_runtime_ids.iter().copied().collect(),
        };
        self.cluster_index.insert(cx.cluster_id, index);
        self.cluster_visuals.entry(cx.cluster_id).or_insert(ClusterVisual {
            expanded: false,
        });
    }

    fn cluster_is_expanded(&self, cluster_id: u32) -> bool {
        self.cluster_visuals
            .get(&cluster_id)
            .map(|visual| visual.expanded)
            .unwrap_or(false)
    }

    fn update_node_state(&mut self, runtime_id: u32, to: RuntimeState, now: Instant) -> bool {
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
            changed |= self.update_node_state(member_id, RuntimeState::Pending, now);
        }
        changed
    }

    fn update_clusters_for_action_input(&mut self, runtime_id: u32, now: Instant) -> bool {
        let mut changed = false;
        let cluster_ids = self.cluster_index.keys().copied().collect::<Vec<_>>();
        for cluster_id in cluster_ids {
            let Some(index) = self.cluster_index.get(&cluster_id) else {
                continue;
            };
            let contains_member = index.member_runtime_ids.contains(&runtime_id);
            let contains_successor = index.successor_runtime_ids.contains(&runtime_id);

            let mut just_opened = false;
            if let Some(visual) = self.cluster_visuals.get_mut(&cluster_id) {
                if !visual.expanded && contains_member {
                    visual.expanded = true;
                    changed = true;
                    just_opened = true;
                } else if visual.expanded && contains_successor {
                    visual.expanded = false;
                    changed = true;
                }
            }

            if just_opened {
                changed |= self.reset_cluster_members_to_pending(cluster_id, runtime_id, now);
            }
        }
        changed
    }

    fn has_running_animations(&self, now: Instant) -> bool {
        self.node_visuals.values().any(|visual| {
            visual.from != visual.to && now.duration_since(visual.started_at) < NODE_ANIMATION_DURATION
        })
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
        changed
    }
}

impl JunglePanelTheme<AnyAnimal> for ExampleTheme {
    type State = Mutex<ExampleThemeState>;
    type Message = ();

    fn init(&self) -> Self::State {
        Mutex::new(ExampleThemeState {
            node_visuals: HashMap::new(),
            cluster_index: HashMap::new(),
            cluster_visuals: HashMap::new(),
        })
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: ViewerEvent<Self::Message>,
    ) -> Task<ViewerEvent<Self::Message>> {
        let now = Instant::now();
        let mut guard = state.lock().expect("example theme state mutex poisoned");
        let mut should_tick = false;

        match event {
            ViewerEvent::JourneyUpdate(update) => match update.event {
                RunnerUpdateOut::ActionInput { node_id, .. } => {
                    let node_changed = guard.update_node_state(node_id, RuntimeState::Running, now);
                    let cluster_changed = guard.update_clusters_for_action_input(node_id, now);
                    should_tick = node_changed || cluster_changed;
                }
                RunnerUpdateOut::ActionSuccessOutput { node_id, .. } => {
                    should_tick = guard.update_node_state(node_id, RuntimeState::Completed, now);
                }
                RunnerUpdateOut::ActionFailureOutput { node_id, .. } => {
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
            drop(guard);
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
            let mut guard = state.lock().expect("example theme state mutex poisoned");
            let visual = guard.node_visuals.entry(runtime_id).or_insert(NodeVisual {
                from: RuntimeState::Pending,
                to: RuntimeState::Pending,
                started_at: now,
            });

            let phase_target = match cx.phase {
                Phase::Live(target) => target,
                Phase::Static => RuntimeState::Pending,
            };
            if visual.to != phase_target {
                let blended = sampled_runtime_state(visual, now);
                visual.from = blended;
                visual.to = phase_target;
                visual.started_at = now;
            }
            blend_runtime_color(*visual, now)
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
        let mut guard = state.lock().expect("example theme state mutex poisoned");
        guard.register_cluster(cx);
        let expanded = guard.cluster_is_expanded(cx.cluster_id);
        drop(guard);

        let overlay = container(
            text(cx.label.to_string())
                .size(11)
                .color(Color::from_rgb8(145, 183, 157)),
        )
        .padding([4, 8])
        .style(|_theme| iced::widget::container::Style {
            background: None,
            border: iced::border::rounded(6)
                .color(Color::from_rgb8(54, 117, 78))
                .width(1.0),
            text_color: Some(Color::from_rgb8(145, 183, 157)),
            ..Default::default()
        })
        .into();

        if expanded {
            ClusterView::Expanded {
                overlay: Some(overlay),
                fill: Color::from_rgba(0.0, 0.0, 0.0, 0.0),
            }
        } else {
            ClusterView::Collapsed {
                element: button(
                    text(cx.label.to_string())
                        .size(11)
                        .color(Color::from_rgb8(145, 183, 157)),
                )
                .padding([6, 10])
                .width(Length::Shrink)
                .style(|_theme, _status| iced::widget::button::Style {
                    background: None,
                    text_color: Color::from_rgb8(145, 183, 157),
                    border: iced::border::rounded(8)
                        .color(Color::from_rgb8(54, 117, 78))
                        .width(1.0),
                    ..Default::default()
                })
                .into(),
                size: (240.0, 46.0),
            }
        }
    }
}

fn sampled_runtime_state(visual: &NodeVisual, now: Instant) -> RuntimeState {
    if now.duration_since(visual.started_at) >= NODE_ANIMATION_DURATION {
        return visual.to;
    }
    visual.from
}

fn blend_runtime_color(visual: NodeVisual, now: Instant) -> Color {
    if visual.from == visual.to {
        return runtime_color(visual.to);
    }

    let elapsed = now.saturating_duration_since(visual.started_at);
    let t = (elapsed.as_secs_f32() / NODE_ANIMATION_DURATION.as_secs_f32()).clamp(0.0, 1.0);
    lerp_color(runtime_color(visual.from), runtime_color(visual.to), ease_out_cubic(t))
}

fn runtime_color(state: RuntimeState) -> Color {
    match state {
        RuntimeState::Pending => Color::from_rgb8(120, 120, 120),
        RuntimeState::Running => Color::from_rgb8(212, 190, 68),
        RuntimeState::Completed => Color::from_rgb8(55, 144, 81),
        RuntimeState::Failed => Color::from_rgb8(165, 61, 61),
    }
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
            ()
        },
        ViewerEvent::Message,
    )
}

fn main() {
    let mut headless = false;
    let mut screenshot: Option<PathBuf> = None;
    let mut dump_graph = false;
    let mut live = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--headless" => headless = true,
            "--dump-graph" => dump_graph = true,
            "--live" => live = true,
            "--screenshot" => {
                let value = args
                    .next()
                    .unwrap_or_else(|| panic!("missing value for --screenshot"));
                screenshot = Some(PathBuf::from(value));
            }
            _ if arg.starts_with("--screenshot=") => {
                screenshot = Some(PathBuf::from(&arg["--screenshot=".len()..]));
            }
            _ => {}
        }
    }

    if dump_graph {
        let graph =
            jungle_viewer::debug_graph_for_animal::<jungle_zoo::animals::gorilla::Gorilla>();
        println!("nodes:");
        for node in &graph.nodes {
            println!("  {} {}", node.id, node.label);
        }
        println!("edges:");
        for (from, to) in &graph.edges {
            println!("  {} -> {}", from, to);
        }
        println!("while-clusters:");
        for (index, cluster) in graph.while_clusters.iter().enumerate() {
            println!("  #{index}: {:?}", cluster);
        }
    }

    let mut viewer = jungle_viewer::JungleViewerBuilder::new()
        .title("Jungle View Example (zoo::Gorilla)")
        .animation_duration(Duration::from_millis(280));
    if let Some(path) = screenshot {
        viewer = viewer.screenshot_path(path);
    }
    if headless {
        viewer = viewer.headless(true);
    }

    if live {
        let live_runtime = tokio::runtime::Runtime::new().expect("live runtime should start");

        let client = live_runtime
            .block_on(LocalClient::builder().build())
            .expect("local client should build");
        let worker_client = client.clone();

        let _worker_task = live_runtime.spawn(async move {
            let worker = JungleWorker::new(jungle_zoo::Zoo, worker_client);
            let _ = worker.spawn().await;
        });

        let seed = postcard::to_allocvec(&jungle_zoo::animals::gorilla::default_temporal_seed())
            .expect("gorilla seed should serialize");
        let journey_id = live_runtime
            .block_on(client.start_journey::<jungle_zoo::animals::gorilla::Gorilla>(seed))
            .expect("start_journey gorilla should succeed");

        viewer
            .view_live_animal_with_theme::<jungle_zoo::animals::gorilla::Gorilla, _, _, AnyAnimal>(
                client.clone(),
                journey_id,
                ExampleTheme,
            )
            .expect("jungle-view example should launch live viewer");
    } else {
        viewer
            .view_animal_with_theme::<jungle_zoo::animals::gorilla::Gorilla, _, AnyAnimal>(
                ExampleTheme,
            )
            .expect("jungle-view example should launch viewer");
    }
}
