use crate::BackoffBeetle;
use iced::widget::{column, container, row, text};
use iced::window;
use iced::{window::Screenshot, Element, Font, Length, Subscription, Task};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1520.0;
const WINDOW_HEIGHT: f32 = 920.0;
const SIDEBAR_WIDTH: f32 = 360.0;

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
    image_dump: Option<ImageDumpConfig>,
) -> Result<(), iced::Error>
where
    C: jungle_sdk::JungleClient + Clone + 'static,
{
    let title = "Backoff Loop";
    iced::application(
        move || BackoffUi::new(client.clone(), journey_id, image_dump.clone()),
        BackoffUi::update,
        BackoffUi::view,
    )
    .title(move |_app: &BackoffUi| title.to_string())
    .subscription(BackoffUi::subscription)
    .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
    .default_font(Font::with_name("Iosevka"))
    .antialiasing(true)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    AppStarted,
    Viewer(jungle_vision::EjectedViewerMessage),
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

struct BackoffUi {
    journey_id: Uuid,
    viewer: jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
    image_dump: Option<ImageDumpConfig>,
}

impl BackoffUi {
    fn new<C>(
        client: C,
        journey_id: Uuid,
        image_dump: Option<ImageDumpConfig>,
    ) -> (Self, Task<Message>)
    where
        C: jungle_sdk::JungleClient + Clone + 'static,
    {
        let viewer = jungle_vision::JungleViewerBuilder::new()
            .title("Backoff Journey")
            .eject_live_animal_with_theme::<BackoffBeetle, _, _, jungle_vision::AnyAnimal>(
                client,
                journey_id,
                jungle_vision::DefaultTheme::default().with_cluster_expansion_config(
                    jungle_vision::ClusterExpansionConfig {
                        while_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                        transparent_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                    },
                ),
            );

        (
            Self {
                journey_id,
                viewer,
                image_dump,
            },
            Task::done(Message::AppStarted),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppStarted => self
                .image_dump
                .as_ref()
                .map(|image_dump| schedule_capture(image_dump.delay))
                .unwrap_or_else(Task::none),
            Message::Viewer(event) => self.viewer.update(event).map(Message::Viewer),
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
        self.viewer.subscription().map(Message::Viewer)
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(self.summary_panel())
            .width(Length::Fixed(SIDEBAR_WIDTH))
            .height(Length::Fill)
            .padding(16);
        let graph = container(self.viewer.view().map(Message::Viewer))
            .width(Length::Fill)
            .height(Length::Fill);

        row![sidebar, graph]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn summary_panel(&self) -> Element<'_, Message> {
        let journey_id = self.journey_id.to_string();
        let short_journey_id = &journey_id[..8];

        let body = column![
            text("Backoff").size(28),
            text("An example usage of a generic backoff implementation.").size(14),
            text(format!("Journey {short_journey_id}")).size(16),
        ]
        .spacing(10);

        container(body.spacing(12)).into()
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
