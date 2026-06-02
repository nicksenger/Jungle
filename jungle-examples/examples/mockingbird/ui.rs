use crate::{MockingBird, MockingBirdState};
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{ContentFit, Element, Font, Length, Subscription, Task};
use jungle_sdk::JungleClient;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1840.0;
const WINDOW_HEIGHT: f32 = 860.0;
const SIDEBAR_WIDTH: f32 = 980.0;
const SPECTROGRAM_WIDTH: f32 = 300.0;
const SPECTROGRAM_HEIGHT: f32 = 220.0;

pub fn run_ui<C>(client: C, journey_id: Uuid) -> Result<(), iced::Error>
where
    C: JungleClient + Clone + 'static,
{
    let title = format!("Mockingbird {journey_id}");
    iced::application(
        move || MockingbirdUi::new(client.clone(), journey_id),
        MockingbirdUi::update,
        MockingbirdUi::view,
    )
    .title(move |_app: &MockingbirdUi| title.to_string())
    .subscription(MockingbirdUi::subscription)
    .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
    .default_font(Font::with_name("Iosevka"))
    .antialiasing(true)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    Viewer(jungle_vision::EjectedViewerMessage),
    SnapshotLoaded(Result<Option<MockingBirdState>, String>),
    PlayLatest,
    PlayBest,
}

struct MockingbirdUi {
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
    viewer: jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
    snapshot: Option<MockingBirdState>,
    snapshot_error: Option<String>,
    audio: AudioPlayer,
}

impl MockingbirdUi {
    fn new<C>(client: C, journey_id: Uuid) -> (Self, Task<Message>)
    where
        C: JungleClient + Clone + 'static,
    {
        let viewer = jungle_vision::JungleViewerBuilder::new()
            .title("Mockingbird Journey")
            .eject_live_animal::<MockingBird, _>(client.clone(), journey_id);
        let client: Arc<dyn JungleClient> = Arc::new(client);

        (
            Self {
                client: client.clone(),
                journey_id,
                viewer,
                snapshot: None,
                snapshot_error: None,
                audio: AudioPlayer::new(),
            },
            Task::perform(load_snapshot(client, journey_id), Message::SnapshotLoaded),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Viewer(event) => {
                let should_refresh =
                    matches!(event, jungle_vision::EjectedViewerMessage::ApplyLiveEvent { .. });
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
            Message::PlayLatest => {
                if let Some(path) = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.latest_generated_sample_path.as_deref())
                {
                    if let Err(err) = self.audio.play(path) {
                        self.snapshot_error = Some(err);
                    }
                }
                Task::none()
            }
            Message::PlayBest => {
                if let Some(path) = self
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.best_generated_sample_path.as_deref())
                {
                    if let Err(err) = self.audio.play(path) {
                        self.snapshot_error = Some(err);
                    }
                }
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.viewer.subscription().map(Message::Viewer)
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = container(
            column![
                self.snapshot_row(),
                self.status_line(),
                self.audio_status_line(),
                Space::new().height(Length::Fill)
            ]
            .spacing(16),
        )
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .padding(20)
        .style(sidebar_style);

        let body = row![
            sidebar,
            container(self.viewer.view().map(Message::Viewer))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(0)
        .height(Length::Fill)
        .width(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_style)
            .into()
    }

    fn snapshot_row(&self) -> Element<'_, Message> {
        let target = self
            .snapshot
            .as_ref()
            .map(|snapshot| spectrogram_card("Target Spectrogram", Some(&snapshot.target_spectrogram_path), None, None))
            .unwrap_or_else(|| spectrogram_card("Target Spectrogram", None, None, None));
        let latest = self.snapshot.as_ref().map_or_else(
            || spectrogram_card("Most Recent", None, None, None),
            |snapshot| {
                spectrogram_card(
                    "Most Recent",
                    snapshot.latest_generated_spectrogram_path.as_deref(),
                    snapshot.latest_generated_similarity,
                    snapshot
                        .latest_generated_sample_path
                        .as_deref()
                        .map(|_| Message::PlayLatest),
                )
            },
        );
        let best = self.snapshot.as_ref().map_or_else(
            || spectrogram_card("Best In Session", None, None, None),
            |snapshot| {
                spectrogram_card(
                    "Best In Session",
                    snapshot.best_generated_spectrogram_path.as_deref(),
                    snapshot.best_similarity,
                    snapshot
                        .best_generated_sample_path
                        .as_deref()
                        .map(|_| Message::PlayBest),
                )
            },
        );

        row![target, latest, best].spacing(16).into()
    }

    fn status_line(&self) -> Element<'_, Message> {
        let label = if let Some(snapshot) = self.snapshot.as_ref() {
            format!(
                "iteration {}  current {:.6}  best {}",
                snapshot.iteration,
                snapshot.last_similarity,
                snapshot
                    .best_similarity
                    .map(|score| format!("{score:.6}"))
                    .unwrap_or_else(|| "n/a".to_owned())
            )
        } else {
            "loading mockingbird session snapshot".to_owned()
        };

        text(label).size(14).into()
    }

    fn audio_status_line(&self) -> Element<'_, Message> {
        let label = self
            .snapshot_error
            .clone()
            .or_else(|| self.audio.last_error.clone())
            .unwrap_or_else(|| format!("journey {}", self.journey_id));
        text(label).size(13).into()
    }
}

fn spectrogram_card<'a>(
    title: &'a str,
    spectrogram_path: Option<&'a str>,
    similarity: Option<f32>,
    play_message: Option<Message>,
) -> Element<'a, Message> {
    let header = if let Some(similarity) = similarity {
        format!("{title}  {similarity:.6}")
    } else {
        title.to_owned()
    };

    let image_panel: Element<'a, Message> = match spectrogram_path.filter(|path| !path.is_empty()) {
        Some(path) if Path::new(path).exists() => image(image::Handle::from_path(path))
            .content_fit(ContentFit::Contain)
            .width(Length::Fill)
            .height(Length::Fixed(SPECTROGRAM_HEIGHT))
            .into(),
        _ => container(text("Waiting for spectrogram").size(14))
            .width(Length::Fill)
            .height(Length::Fixed(SPECTROGRAM_HEIGHT))
            .center_x(Length::Fill)
            .center_y(Length::Fixed(SPECTROGRAM_HEIGHT))
            .style(image_placeholder_style)
            .into(),
    };

    let mut card = column![text(header).size(15), image_panel].spacing(10);
    if let Some(message) = play_message {
        card = card.push(button("Play WAV").on_press(message));
    } else {
        card = card.push(Space::new().height(Length::Shrink));
    }

    container(card)
        .width(Length::Fixed(SPECTROGRAM_WIDTH))
        .padding(14)
        .style(card_style)
        .into()
}

async fn load_snapshot(
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
) -> Result<Option<MockingBirdState>, String> {
    let bytes = client
        .animal_appearance(journey_id)
        .await
        .map_err(|err| format!("failed to load mockingbird state: {err}"))?;
    bytes
        .map(|bytes| {
            postcard::from_bytes::<MockingBirdState>(&bytes)
                .map_err(|err| format!("failed to decode mockingbird state snapshot: {err}"))
        })
        .transpose()
}

struct AudioPlayer {
    _stream: Option<OutputStream>,
    handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    last_error: Option<String>,
}

impl AudioPlayer {
    fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Some(stream),
                handle: Some(handle),
                sink: None,
                last_error: None,
            },
            Err(err) => Self {
                _stream: None,
                handle: None,
                sink: None,
                last_error: Some(format!("audio output unavailable: {err}")),
            },
        }
    }

    fn play(&mut self, path: &str) -> Result<(), String> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| "audio output is unavailable".to_owned())?;
        let file = File::open(path).map_err(|err| format!("failed to open wav file {path}: {err}"))?;
        let decoder = Decoder::new(BufReader::new(file))
            .map_err(|err| format!("failed to decode wav file {path}: {err}"))?;
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        let sink =
            Sink::try_new(handle).map_err(|err| format!("failed to open audio sink: {err}"))?;
        sink.append(decoder);
        sink.play();
        self.last_error = None;
        self.sink = Some(sink);
        Ok(())
    }
}

fn app_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(9, 15, 12))),
        text_color: Some(iced::Color::from_rgb8(225, 238, 231)),
        ..Default::default()
    }
}

fn sidebar_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(16, 27, 21))),
        text_color: Some(iced::Color::from_rgb8(225, 238, 231)),
        border: iced::border::Border {
            width: 1.0,
            color: iced::Color::from_rgb8(34, 58, 46),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn card_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(24, 39, 31))),
        text_color: Some(iced::Color::from_rgb8(225, 238, 231)),
        border: iced::border::Border {
            width: 1.0,
            color: iced::Color::from_rgb8(46, 76, 60),
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn image_placeholder_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(18, 28, 23))),
        text_color: Some(iced::Color::from_rgb8(173, 191, 180)),
        border: iced::border::Border {
            width: 1.0,
            color: iced::Color::from_rgb8(38, 63, 50),
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}
