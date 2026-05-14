use iced::widget::{button, column, container, text};
use iced::{Color, Element, Length, Task};
use jungle_sdk::core::JungleWorker;
use jungle_sdk::server::ServerBuilder;
use jungle_sdk::JungleClient;
use jungle_viewer::{
    AnyAnimal, ClusterView, ClusterViewCtx, JunglePanelTheme, Phase, RuntimeState, StepKind,
    StepViewCtx, ViewerEvent,
};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Copy)]
struct ExampleTheme;

impl JunglePanelTheme<AnyAnimal> for ExampleTheme {
    type State = ();
    type Message = ();

    fn init(&self) -> Self::State {}

    fn update(
        &self,
        _state: &mut Self::State,
        _event: ViewerEvent<Self::Message>,
    ) -> Task<ViewerEvent<Self::Message>> {
        Task::none()
    }

    fn view_step(
        &self,
        _state: &Self::State,
        cx: &StepViewCtx<'_>,
    ) -> (Element<'static, ViewerEvent<Self::Message>>, (f64, f64)) {
        let role = match cx.kind {
            StepKind::Conditional => "condition",
            StepKind::Select => "select",
            StepKind::Join => "join",
            StepKind::Step => "step",
        };

        let mut fill = match cx.kind {
            StepKind::Conditional => Color::from_rgb8(28, 54, 105),
            StepKind::Select | StepKind::Join => Color::from_rgb8(20, 84, 76),
            StepKind::Step => Color::from_rgb8(23, 92, 58),
        };
        if let Phase::Live(state) = cx.phase {
            fill = match state {
                RuntimeState::Running => Color::from_rgb8(146, 158, 40),
                RuntimeState::Completed => Color::from_rgb8(55, 144, 81),
                RuntimeState::Failed => Color::from_rgb8(150, 58, 58),
                RuntimeState::Pending => fill,
            };
        }

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
        _state: &Self::State,
        cx: &ClusterViewCtx<'_>,
    ) -> ClusterView<Self::Message> {
        ClusterView::Expanded {
            overlay: Some(
                container(
                    text(cx.label.to_string())
                        .size(11)
                        .color(Color::from_rgb8(145, 183, 157)),
                )
                .padding([4, 8])
                .style(|_theme| iced::widget::container::Style {
                    background: Some(iced::Background::Color(Color::from_rgba8(20, 46, 30, 0.35))),
                    border: iced::border::rounded(6)
                        .color(Color::from_rgb8(54, 117, 78))
                        .width(1.0),
                    text_color: Some(Color::from_rgb8(145, 183, 157)),
                    ..Default::default()
                })
                .into(),
            ),
            fill: Color::from_rgba8(30, 91, 53, 0.04),
        }
    }
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

    let mut viewer =
        jungle_viewer::JungleViewerBuilder::new().title("Jungle View Example (zoo::Gorilla)");
    if let Some(path) = screenshot {
        viewer = viewer.screenshot_path(path);
    }
    if headless {
        viewer = viewer.headless(true);
    }

    if live {
        let listen_addr = jungle_examples::reserve_local_addr();
        let db_path =
            std::env::temp_dir().join(format!("jungle-view-example-{}.redb", Uuid::new_v4()));

        let live_runtime = tokio::runtime::Runtime::new().expect("live runtime should start");

        let _server_task = live_runtime.spawn({
            let db_path = db_path.clone();
            async move {
                let _ = ServerBuilder::new()
                    .listen(listen_addr)
                    .redb_path(db_path)
                    .run()
                    .await;
            }
        });

        let client = live_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));
        let worker_client =
            live_runtime.block_on(jungle_examples::connect_client_with_retry(listen_addr));

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
