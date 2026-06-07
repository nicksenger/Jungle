use crate::{Depth2, ReplayLifecycle};
use iced::widget::{container, text};
use iced::window;
use iced::{window::Screenshot, Element, Font, Length, Subscription, Task};
use jungle_sdk::JungleClient;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1520.0;
const WINDOW_HEIGHT: f32 = 920.0;
const UI_TICK_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct ImageDumpConfig {
    output_path: PathBuf,
    delay: Duration,
}

impl ImageDumpConfig {
    pub fn new(output_path: PathBuf, delay: Duration) -> Self {
        Self { output_path, delay }
    }
}

pub fn run_ui<C>(
    client: C,
    journey_id: Uuid,
    lifecycle: ReplayLifecycle,
    image_dump: Option<ImageDumpConfig>,
) -> Result<(), iced::Error>
where
    C: JungleClient + Clone + 'static,
{
    iced::application(
        move || {
            ReplayUi::new(
                client.clone(),
                journey_id,
                lifecycle.clone(),
                image_dump.clone(),
            )
        },
        ReplayUi::update,
        ReplayUi::view,
    )
    .title(ReplayUi::<C>::title)
    .subscription(ReplayUi::subscription)
    .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
    .default_font(Font::with_name("Iosevka"))
    .antialiasing(true)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    AppStarted,
    RebuildViewer,
    Viewer(jungle_vision::EjectedViewerMessage),
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

struct ReplayUi<C>
where
    C: JungleClient + Clone + 'static,
{
    client: C,
    journey_id: Uuid,
    lifecycle: ReplayLifecycle,
    replayed: bool,
    viewer_generation: u64,
    viewer:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    image_dump: Option<ImageDumpConfig>,
}

impl<C> ReplayUi<C>
where
    C: JungleClient + Clone + 'static,
{
    fn new(
        client: C,
        journey_id: Uuid,
        lifecycle: ReplayLifecycle,
        image_dump: Option<ImageDumpConfig>,
    ) -> (Self, Task<Message>) {
        let viewer = Self::build_viewer(client.clone(), journey_id);

        (
            Self {
                client,
                journey_id,
                lifecycle,
                replayed: false,
                viewer_generation: 0,
                viewer: Some(viewer),
                image_dump,
            },
            Task::done(Message::AppStarted),
        )
    }

    fn title(&self) -> String {
        if self.replayed {
            "Replay Example - Replayed Worker".to_owned()
        } else {
            "Replay Example - Initial Execution".to_owned()
        }
    }

    fn build_viewer(
        client: C,
        journey_id: Uuid,
    ) -> jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal> {
        jungle_vision::JungleViewerBuilder::new()
            .title("Replay Journey")
            .eject_live_animal_with_theme::<Depth2, _, _, jungle_vision::AnyAnimal>(
                client,
                journey_id,
                jungle_vision::DefaultTheme::default().with_cluster_expansion_config(
                    jungle_vision::ClusterExpansionConfig {
                        while_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                        transparent_clusters:
                            jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                    },
                ),
            )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.lifecycle.take_replay_viewer_request() {
                    // Drop the first viewer completely before constructing the replay viewer.
                    // This guarantees a brand-new JourneyEvents subscription and a fresh
                    // graph/live-state rebuild from that second stream.
                    self.viewer = None;
                    self.replayed = true;
                    self.viewer_generation = self.viewer_generation.saturating_add(1);
                    return Task::done(Message::RebuildViewer);
                }
                Task::none()
            }
            Message::AppStarted => self
                .image_dump
                .as_ref()
                .map(|image_dump| schedule_capture(image_dump.delay))
                .unwrap_or_else(Task::none),
            Message::RebuildViewer => {
                self.viewer = Some(Self::build_viewer(self.client.clone(), self.journey_id));
                Task::none()
            }
            Message::Viewer(event) => self
                .viewer
                .as_mut()
                .map(|viewer| viewer.update(event).map(Message::Viewer))
                .unwrap_or_else(Task::none),
            Message::CaptureView => window::latest().then(|id| match id {
                Some(id) => window::screenshot(id).map(Message::ViewCaptured),
                None => Task::none(),
            }),
            Message::ViewCaptured(screenshot) => {
                let Some(image_dump) = self.image_dump.clone() else {
                    return Task::none();
                };
                Task::perform(
                    save_screenshot_png(image_dump.output_path, screenshot),
                    Message::ViewSaved,
                )
            }
            Message::ViewSaved(result) => {
                match result {
                    Ok(path) => println!("Wrote {}", path.display()),
                    Err(error) => eprintln!("Failed to save screenshot: {error}"),
                }
                close_latest_window()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(UI_TICK_INTERVAL).map(|_| Message::Tick),
            self.viewer
                .as_ref()
                .map(|viewer| viewer.subscription().map(Message::Viewer))
                .unwrap_or_else(Subscription::none),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        if let Some(viewer) = self.viewer.as_ref() {
            container(viewer.view().map(Message::Viewer))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text(format!(
                "Rebuilding replay viewer from fresh JourneyEvents stream (generation {})",
                self.viewer_generation
            )))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        }
    }
}

fn schedule_capture(delay: Duration) -> Task<Message> {
    if delay.is_zero() {
        Task::done(Message::CaptureView)
    } else {
        Task::perform(
            async move {
                tokio::time::sleep(delay).await;
            },
            |_| Message::CaptureView,
        )
    }
}

fn close_latest_window() -> Task<Message> {
    window::latest().then(|id| match id {
        Some(id) => window::close(id),
        None => Task::none(),
    })
}

async fn save_screenshot_png(path: PathBuf, screenshot: Screenshot) -> Result<PathBuf, String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create screenshot directory {}: {error}",
                parent.display()
            )
        })?;
    }

    let image = image::RgbaImage::from_raw(
        screenshot.size.width,
        screenshot.size.height,
        screenshot.rgba.to_vec(),
    )
    .ok_or_else(|| "failed to build image buffer from screenshot".to_owned())?;

    image
        .save(&path)
        .map_err(|error| format!("failed to save screenshot to {}: {error}", path.display()))?;
    Ok(path)
}
