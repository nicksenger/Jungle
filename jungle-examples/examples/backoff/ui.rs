use crate::{ActionBackoffAppearance, BackoffAnimal, BackoffAppearance, SubflowBackoffAppearance};
use iced::widget::{column, container, row, text};
use iced::window;
use iced::{window::Screenshot, Element, Font, Length, Subscription, Task};
use jungle_sdk::JungleClient;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
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
    C: JungleClient + Clone + 'static,
{
    let title = "Backoff Demo - Joined Backoff Loops";
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
    SnapshotLoaded(Result<Option<BackoffAppearance>, String>),
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

struct BackoffUi {
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
    viewer: jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
    snapshot: Option<BackoffAppearance>,
    snapshot_error: Option<String>,
    image_dump: Option<ImageDumpConfig>,
}

impl BackoffUi {
    fn new<C>(
        client: C,
        journey_id: Uuid,
        image_dump: Option<ImageDumpConfig>,
    ) -> (Self, Task<Message>)
    where
        C: JungleClient + Clone + 'static,
    {
        let viewer = jungle_vision::JungleViewerBuilder::new()
            .title("Backoff Journey")
            .eject_live_animal_with_theme::<BackoffAnimal, _, _, jungle_vision::AnyAnimal>(
                client.clone(),
                journey_id,
                jungle_vision::DefaultTheme::default().with_cluster_expansion_config(
                    jungle_vision::ClusterExpansionConfig {
                        while_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                        transparent_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                    },
                ),
            );
        let client: Arc<dyn JungleClient> = Arc::new(client);

        (
            Self {
                client: client.clone(),
                journey_id,
                viewer,
                snapshot: None,
                snapshot_error: None,
                image_dump,
            },
            Task::batch([
                Task::done(Message::AppStarted),
                Task::perform(load_snapshot(client, journey_id), Message::SnapshotLoaded),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AppStarted => self
                .image_dump
                .as_ref()
                .map(|image_dump| schedule_capture(image_dump.delay))
                .unwrap_or_else(Task::none),
            Message::Viewer(event) => {
                let should_refresh = matches!(
                    event,
                    jungle_vision::EjectedViewerMessage::ApplyLiveEvent { .. }
                );
                let viewer_task = self.viewer.update(event).map(Message::Viewer);
                if should_refresh {
                    Task::batch([
                        viewer_task,
                        Task::perform(
                            load_snapshot(self.client.clone(), self.journey_id),
                            Message::SnapshotLoaded,
                        ),
                    ])
                } else {
                    viewer_task
                }
            }
            Message::SnapshotLoaded(Ok(snapshot)) => {
                self.snapshot = snapshot;
                self.snapshot_error = None;
                Task::none()
            }
            Message::SnapshotLoaded(Err(err)) => {
                self.snapshot_error = Some(err);
                Task::none()
            }
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

        let mut body = column![
            text("Joined Backoffs").size(28),
            text("Left arm runs subflow backoff, right arm runs single-action backoff, and both loop forever under a Join.")
                .size(14),
            text(format!("Journey {short_journey_id}")).size(16),
        ]
        .spacing(10);

        if let Some(snapshot) = self.snapshot.as_ref() {
            body = body.push(
                text(format!(
                    "Pre-join stub steps completed: {}",
                    snapshot.before_join_steps_completed
                ))
                .size(16),
            );
            body = body.push(
                text(format!(
                    "Post-join stub steps completed: {}",
                    snapshot.after_join_steps_completed
                ))
                .size(16),
            );
            body = body.push(branch_panel("Subflow Backoff", &snapshot.subflow));
            body = body.push(branch_panel("Single-Action Backoff", &snapshot.action));
        } else {
            body = body.push(text("Loading appearance snapshot").size(16));
        }

        if let Some(error) = self.snapshot_error.as_deref() {
            body = body.push(text(format!("Snapshot error: {error}")).size(16));
        }

        container(body.spacing(12)).into()
    }
}

fn branch_panel<'a, Detail>(title: &'static str, detail: &Detail) -> Element<'a, Message>
where
    Detail: BackoffBranchSummary,
{
    let summary = detail.summary_lines();
    let mut body = column![text(title).size(20)].spacing(8);
    for line in summary {
        body = body.push(text(line).size(16));
    }
    container(body).into()
}

trait BackoffBranchSummary {
    fn summary_lines(&self) -> Vec<String>;
}

impl BackoffBranchSummary for SubflowBackoffAppearance {
    fn summary_lines(&self) -> Vec<String> {
        vec![
            format!("Attempts completed: {}", self.attempts),
            format!("Next delay: {} ms", self.next_delay_ms),
            format!(
                "Policy: start={} ms, multiplier={}, max={} ms",
                self.policy.initial_delay_ms, self.policy.multiplier, self.policy.max_delay_ms
            ),
            format!("Attempts started: {}", self.metrics.started_attempts),
            format!("Failures recorded: {}", self.metrics.failed_attempts),
            format!(
                "Last result: {}",
                self.last_result.as_deref().unwrap_or("pending")
            ),
            format!(
                "Last failure: {}",
                self.metrics
                    .last_failure_message
                    .as_deref()
                    .unwrap_or("waiting for first failure")
            ),
        ]
    }
}

impl BackoffBranchSummary for ActionBackoffAppearance {
    fn summary_lines(&self) -> Vec<String> {
        vec![
            format!("Attempts completed: {}", self.attempts),
            format!("Next delay: {} ms", self.next_delay_ms),
            format!(
                "Policy: start={} ms, multiplier={}, max={} ms",
                self.policy.initial_delay_ms, self.policy.multiplier, self.policy.max_delay_ms
            ),
            format!("Attempts started: {}", self.metrics.started_attempts),
            format!("Failures recorded: {}", self.metrics.failed_attempts),
            format!(
                "Last result: {}",
                self.last_result.as_deref().unwrap_or("pending")
            ),
            format!(
                "Last failure: {}",
                self.metrics
                    .last_failure_message
                    .as_deref()
                    .unwrap_or("waiting for first failure")
            ),
        ]
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

async fn load_snapshot(
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
) -> Result<Option<BackoffAppearance>, String> {
    let bytes = client
        .animal_appearance(journey_id)
        .await
        .map_err(|err| format!("failed to load backoff state: {err}"))?;
    bytes
        .map(|bytes| {
            postcard::from_bytes::<BackoffAppearance>(&bytes)
                .map_err(|err| format!("failed to decode backoff state snapshot: {err}"))
        })
        .transpose()
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
