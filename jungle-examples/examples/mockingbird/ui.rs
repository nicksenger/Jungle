use crate::{MockingBird, MockingBirdInstrument, MockingBirdInstrumentState, MockingBirdState};
use iced::widget::{button, column, container, image, row, stack, text};
use iced::{alignment, clipboard, ContentFit, Element, Font, Length, Subscription, Task};
use jungle_sdk::JungleClient;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1840.0;
const WINDOW_HEIGHT: f32 = 1060.0;
const PANEL_HEADER_HEIGHT: f32 = 52.0;
const SNAPSHOT_ROW_HEIGHT: f32 = (WINDOW_HEIGHT - PANEL_HEADER_HEIGHT) / 5.0;
const WINDOW_HEADER_HORIZONTAL_PADDING: u16 = 12;
const WINDOW_HEADER_CELL_HORIZONTAL_PADDING: u16 = 10;
const SECTION_HORIZONTAL_PADDING: u16 = 0;
const HEADER_VERTICAL_PADDING: u16 = 14;
const SNAPSHOT_ROW_VERTICAL_PADDING: u16 = 0;
const SNAPSHOT_GAP: f32 = 0.0;
const SPECTROGRAM_OVERLAY_HEIGHT: f32 = 30.0;

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

#[derive(Debug, Clone, Copy)]
enum SnapshotKind {
    Initial,
    Current,
    Best,
    Target,
}

#[derive(Debug, Clone)]
enum Message {
    Viewer(jungle_vision::EjectedViewerMessage),
    SnapshotLoaded(Result<Option<MockingBirdState>, String>),
    ActivateSpectrogram(MockingBirdInstrument, SnapshotKind),
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
            Message::ActivateSpectrogram(instrument, kind) => {
                let Some(snapshot) = self.snapshot.as_ref() else {
                    return Task::none();
                };
                let instrument_state = snapshot.instrument_state(instrument);
                let (audio_path, source) = spectrogram_action_payload(instrument_state, kind);

                self.snapshot_error = None;
                if let Some(path) = audio_path.as_deref() {
                    if let Err(err) = self.audio.play(path) {
                        self.snapshot_error = Some(err);
                    }
                }

                source.map_or_else(Task::none, clipboard::write)
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.viewer.subscription().map(Message::Viewer)
    }

    fn view(&self) -> Element<'_, Message> {
        let image_section = container(column![self.snapshot_header(), self.snapshot_panel()])
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(sidebar_style);
        let dag_section = container(column![
            self.dag_header(),
            container(self.viewer.view().map(Message::Viewer))
                .width(Length::Fill)
                .height(Length::Fill)
        ])
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        let body = row![image_section, divider_vertical(), dag_section]
            .spacing(0)
            .height(Length::Fill)
            .width(Length::Fill);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_style)
            .into()
    }

    fn snapshot_header(&self) -> Element<'_, Message> {
        let header = row![
            column_header("Target"),
            column_header("Initial"),
            column_header("Current"),
            column_header("Best"),
        ]
        .spacing(SNAPSHOT_GAP);

        container(header)
            .width(Length::Fill)
            .height(Length::Fixed(PANEL_HEADER_HEIGHT))
            .padding([HEADER_VERTICAL_PADDING, WINDOW_HEADER_HORIZONTAL_PADDING])
            .style(header_style)
            .into()
    }

    fn dag_header(&self) -> Element<'_, Message> {
        let journey_id = self.journey_id.to_string();
        let short_journey_id = &journey_id[..8];
        let summary = if let Some(snapshot) = self.snapshot.as_ref() {
            format!(
                "Iteration {}  Journey {}",
                snapshot.iteration, short_journey_id
            )
        } else {
            format!("Journey {}", short_journey_id)
        };

        let header = column![text(summary).size(15)].spacing(6);

        container(header)
            .width(Length::Fill)
            .height(Length::Fixed(PANEL_HEADER_HEIGHT))
            .padding([HEADER_VERTICAL_PADDING, WINDOW_HEADER_HORIZONTAL_PADDING])
            .style(header_style)
            .into()
    }

    fn snapshot_panel(&self) -> Element<'_, Message> {
        if let Some(snapshot) = self.snapshot.as_ref() {
            let mut rows = column![].spacing(0).width(Length::Fill);
            for instrument in MockingBirdInstrument::ALL {
                rows = rows.push(
                    container(self.snapshot_row(snapshot, instrument))
                        .width(Length::Fill)
                        .height(Length::Fixed(SNAPSHOT_ROW_HEIGHT)),
                );
            }

            container(rows)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("Loading instrument snapshots").size(16))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        }
    }

    fn snapshot_row<'a>(
        &self,
        snapshot: &'a MockingBirdState,
        instrument: MockingBirdInstrument,
    ) -> Element<'a, Message> {
        let instrument_state = snapshot.instrument_state(instrument);
        let cards = row![
            spectrogram_tile(
                Some(&instrument_state.target_spectrogram_path),
                Some(instrument.display_name().to_owned()),
                alignment::Horizontal::Left,
                sibling_audio_path(&instrument_state.target_spectrogram_path)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Target)),
            ),
            spectrogram_tile(
                initial_spectrogram_path(instrument_state),
                initial_overlay_label(instrument_state),
                alignment::Horizontal::Right,
                initial_spectrogram_path(instrument_state)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Initial)),
            ),
            spectrogram_tile(
                current_spectrogram_path(instrument_state),
                current_overlay_label(instrument_state),
                alignment::Horizontal::Right,
                current_spectrogram_path(instrument_state)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Current)),
            ),
            spectrogram_tile(
                best_spectrogram_path(instrument_state),
                best_overlay_label(instrument_state),
                alignment::Horizontal::Right,
                best_spectrogram_path(instrument_state)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Best)),
            ),
        ]
        .spacing(SNAPSHOT_GAP)
        .width(Length::Fill)
        .height(Length::Fill);

        column![
            divider_horizontal(),
            container(
                container(cards)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([SNAPSHOT_ROW_VERTICAL_PADDING, SECTION_HORIZONTAL_PADDING]),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(snapshot_row_style)
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn column_header(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(15))
        .width(Length::FillPortion(1))
        .padding([0, WINDOW_HEADER_CELL_HORIZONTAL_PADDING])
        .into()
}

fn sibling_audio_path(image_path: &str) -> Option<String> {
    let audio_path = Path::new(image_path).with_extension("wav");
    audio_path
        .exists()
        .then(|| audio_path.display().to_string())
}

fn spectrogram_tile<'a>(
    spectrogram_path: Option<&'a str>,
    overlay_label: Option<String>,
    overlay_alignment: alignment::Horizontal,
    activate_message: Option<Message>,
) -> Element<'a, Message> {
    let image_panel: Element<'a, Message> = match spectrogram_path.filter(|path| image_exists(path))
    {
        Some(path) => {
            let preview = image(image::Handle::from_path(path))
                .content_fit(ContentFit::Fill)
                .width(Length::Fill)
                .height(Length::Fill);
            match activate_message {
                Some(message) => button(preview)
                    .padding(0)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(image_button_style)
                    .on_press(message)
                    .into(),
                None => preview.into(),
            }
        }
        None => container(text("Waiting for spectrogram").size(14))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(image_placeholder_style)
            .into(),
    };

    let overlay = container(
        container(
            text(overlay_label.unwrap_or_default())
                .size(13)
                .width(Length::Fill)
                .align_x(overlay_alignment),
        )
        .width(Length::Fill)
        .height(Length::Fixed(SPECTROGRAM_OVERLAY_HEIGHT))
        .padding([4, 8])
        .style(spectrogram_overlay_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_top(Length::Fill);

    container(
        stack([
            container(image_panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            overlay.into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .into()
}

fn divider_horizontal<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(1)
        .style(divider_style)
        .into()
}

fn divider_vertical<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(1)
        .height(Length::Fill)
        .style(divider_style)
        .into()
}

fn current_spectrogram_path(instrument_state: &MockingBirdInstrumentState) -> Option<&str> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| non_empty(&code.spectrogram_path))
        .or_else(|| {
            instrument_state
                .latest_generated_spectrogram_path
                .as_deref()
                .and_then(non_empty)
        })
}

fn current_sample_path(instrument_state: &MockingBirdInstrumentState) -> Option<&str> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| non_empty(&code.sample_path))
        .or_else(|| {
            instrument_state
                .latest_generated_sample_path
                .as_deref()
                .and_then(non_empty)
        })
}

fn current_overlay_label(instrument_state: &MockingBirdInstrumentState) -> Option<String> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| {
            current_spectrogram_path(instrument_state)
                .map(|_| similarity_label("current", code.similarity))
        })
        .or_else(|| {
            current_spectrogram_path(instrument_state)
                .map(|_| similarity_label("current", instrument_state.latest_generated_similarity))
        })
}

fn best_spectrogram_path(instrument_state: &MockingBirdInstrumentState) -> Option<&str> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| non_empty(&code.spectrogram_path))
        .or_else(|| {
            instrument_state
                .best_generated_spectrogram_path
                .as_deref()
                .and_then(non_empty)
        })
}

fn best_sample_path(instrument_state: &MockingBirdInstrumentState) -> Option<&str> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| non_empty(&code.sample_path))
        .or_else(|| {
            instrument_state
                .best_generated_sample_path
                .as_deref()
                .and_then(non_empty)
        })
}

fn best_overlay_label(instrument_state: &MockingBirdInstrumentState) -> Option<String> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| {
            best_spectrogram_path(instrument_state)
                .map(|_| similarity_label("best", code.similarity))
        })
        .or_else(|| {
            best_spectrogram_path(instrument_state)
                .map(|_| similarity_label("best", instrument_state.best_similarity))
        })
}

fn initial_spectrogram_path(instrument_state: &MockingBirdInstrumentState) -> Option<&str> {
    non_empty(&instrument_state.initial_dsp_code.spectrogram_path)
}

fn initial_overlay_label(instrument_state: &MockingBirdInstrumentState) -> Option<String> {
    initial_spectrogram_path(instrument_state)
        .map(|_| score_only_label(instrument_state.initial_dsp_code.similarity))
}

fn spectrogram_action_payload(
    instrument_state: &MockingBirdInstrumentState,
    kind: SnapshotKind,
) -> (Option<String>, Option<String>) {
    match kind {
        SnapshotKind::Initial => (
            existing_path(non_empty(&instrument_state.initial_dsp_code.sample_path)),
            non_empty(&instrument_state.initial_dsp_code.source).map(ToOwned::to_owned),
        ),
        SnapshotKind::Current => (
            existing_path(current_sample_path(instrument_state)),
            instrument_state
                .latest_rendered_code
                .as_ref()
                .map(|code| code.source.clone()),
        ),
        SnapshotKind::Best => (
            existing_path(best_sample_path(instrument_state)),
            instrument_state
                .best_generated_code
                .as_ref()
                .map(|code| code.source.clone()),
        ),
        SnapshotKind::Target => (
            sibling_audio_path(&instrument_state.target_spectrogram_path),
            None,
        ),
    }
}

fn score_only_label(similarity: Option<f32>) -> String {
    similarity
        .map(|similarity| format!("{similarity:.6}"))
        .unwrap_or_default()
}

fn similarity_label(label: &str, similarity: Option<f32>) -> String {
    similarity
        .map(|similarity| format!("{label}  {similarity:.6}"))
        .unwrap_or_else(|| label.to_owned())
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn existing_path(path: Option<&str>) -> Option<String> {
    path.filter(|path| Path::new(path).exists())
        .map(ToOwned::to_owned)
}

fn image_exists(path: &str) -> bool {
    Path::new(path).exists()
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
        let file =
            File::open(path).map_err(|err| format!("failed to open wav file {path}: {err}"))?;
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
        ..Default::default()
    }
}

fn header_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(14, 24, 19))),
        text_color: Some(iced::Color::from_rgb8(225, 238, 231)),
        ..Default::default()
    }
}

fn snapshot_row_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(16, 27, 21))),
        text_color: Some(iced::Color::from_rgb8(225, 238, 231)),
        ..Default::default()
    }
}

fn spectrogram_overlay_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgba8(
            9, 15, 12, 0.34,
        ))),
        text_color: Some(iced::Color::from_rgb8(237, 246, 241)),
        ..Default::default()
    }
}

fn divider_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(34, 58, 46))),
        ..Default::default()
    }
}

fn image_placeholder_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb8(18, 28, 23))),
        text_color: Some(iced::Color::from_rgb8(173, 191, 180)),
        ..Default::default()
    }
}

fn image_button_style(
    _theme: &iced::Theme,
    _status: button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: None,
        text_color: iced::Color::WHITE,
        ..Default::default()
    }
}
