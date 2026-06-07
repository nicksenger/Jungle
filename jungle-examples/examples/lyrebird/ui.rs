use crate::{Lyrebird, LyrebirdInstrument, LyrebirdInstrumentState, LyrebirdState};
use iced::widget::{button, column, container, image as iced_image, row, stack, text};
use iced::window;
use iced::{
    alignment, clipboard, window::Screenshot, ContentFit, Element, Font, Length, Subscription, Task,
};
use jungle_sdk::JungleClient;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1840.0;
const WINDOW_HEIGHT: f32 = 860.0;
const DAG_HEADER_HEIGHT: f32 = SPECTROGRAM_OVERLAY_HEIGHT;
const DAG_HEADER_HORIZONTAL_PADDING: u16 = 8;
const SECTION_HORIZONTAL_PADDING: u16 = 0;
const SNAPSHOT_ROW_VERTICAL_PADDING: u16 = 0;
const SNAPSHOT_GAP: f32 = 0.0;
const SPECTROGRAM_OVERLAY_HEIGHT: f32 = 30.0;
const HEADER_LABEL_TEXT_SIZE: f32 = 13.0;
const HEADER_LABEL_VERTICAL_PADDING: u16 = 4;
const HEADER_LABEL_HORIZONTAL_PADDING: u16 = 8;
const SPECTROGRAM_HUE_ROTATION_DEGREES: i32 = -100;

#[derive(Debug, Clone)]
pub struct ImageDumpConfig {
    output_path: PathBuf,
    delay: Duration,
    panel: Option<ImageDumpPanel>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ImageDumpPanel {
    Spectrograms,
    Graph,
}

impl ImageDumpConfig {
    pub fn new(output_path: PathBuf, delay: Duration, panel: Option<ImageDumpPanel>) -> Self {
        Self {
            output_path,
            delay,
            panel,
        }
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
    let title = "Lyrebird - Appetite for Deduction";
    iced::application(
        move || LyrebirdUi::new(client.clone(), journey_id, image_dump.clone()),
        LyrebirdUi::update,
        LyrebirdUi::view,
    )
    .title(move |_app: &LyrebirdUi| title.to_string())
    .subscription(LyrebirdUi::subscription)
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
    AppStarted,
    Viewer(jungle_vision::EjectedViewerMessage),
    SnapshotLoaded(Result<Option<LyrebirdState>, String>),
    ActivateSpectrogram(LyrebirdInstrument, SnapshotKind),
    CaptureView,
    ViewCaptured(Screenshot),
    ViewSaved(Result<PathBuf, String>),
}

struct LyrebirdUi {
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
    viewer: jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
    snapshot: Option<LyrebirdState>,
    spectrogram_handles: BTreeMap<PathBuf, iced_image::Handle>,
    snapshot_error: Option<String>,
    audio: AudioPlayer,
    image_dump: Option<ImageDumpConfig>,
}

impl LyrebirdUi {
    fn new<C>(
        client: C,
        journey_id: Uuid,
        image_dump: Option<ImageDumpConfig>,
    ) -> (Self, Task<Message>)
    where
        C: JungleClient + Clone + 'static,
    {
        let viewer = jungle_vision::JungleViewerBuilder::new()
            .title("Lyrebird Journey")
            .eject_live_animal_with_theme::<Lyrebird, _, _, jungle_vision::AnyAnimal>(
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
                spectrogram_handles: BTreeMap::new(),
                snapshot_error: None,
                audio: AudioPlayer::new(),
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
                self.spectrogram_handles = snapshot
                    .as_ref()
                    .map(build_spectrogram_handles)
                    .unwrap_or_default();
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
        let (image_section_fill_portion, dag_section_fill_portion) = match self
            .image_dump
            .as_ref()
            .and_then(|image_dump| image_dump.panel)
        {
            Some(ImageDumpPanel::Spectrograms) => (0, 1),
            Some(ImageDumpPanel::Graph) => (1, 0),
            None => (1, 1),
        };
        let image_section = container(self.snapshot_panel())
            .width(Length::FillPortion(image_section_fill_portion))
            .height(Length::Fill)
            .style(sidebar_style);
        let dag_section = container(column![
            self.dag_header(),
            container(self.viewer.view().map(Message::Viewer))
                .width(Length::Fill)
                .height(Length::Fill)
        ])
        .width(Length::FillPortion(dag_section_fill_portion))
        .height(Length::Fill);

        let body = row![
            image_section,
            divider_vertical(image_section_fill_portion > 0 && dag_section_fill_portion > 0),
            dag_section
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

    fn dag_header(&self) -> Element<'_, Message> {
        let journey_id = self.journey_id.to_string();
        let short_journey_id = &journey_id[..8];
        let summary = if let Some(snapshot) = self.snapshot.as_ref() {
            format!(
                "Journey {} ({} generations)",
                short_journey_id,
                snapshot.generation_count()
            )
        } else {
            format!("Journey {}", short_journey_id)
        };

        container(
            text(summary)
                .size(HEADER_LABEL_TEXT_SIZE)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Right),
        )
        .width(Length::Fill)
        .height(Length::Fixed(DAG_HEADER_HEIGHT))
        .padding([HEADER_LABEL_VERTICAL_PADDING, DAG_HEADER_HORIZONTAL_PADDING])
        .style(header_style)
        .into()
    }

    fn snapshot_panel(&self) -> Element<'_, Message> {
        if let Some(snapshot) = self.snapshot.as_ref() {
            let mut rows = column![].spacing(0).width(Length::Fill);
            for (index, instrument) in LyrebirdInstrument::ALL.into_iter().enumerate() {
                rows = rows.push(
                    container(self.snapshot_row(snapshot, instrument, index == 0))
                        .width(Length::Fill)
                        .height(Length::FillPortion(1)),
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
        snapshot: &'a LyrebirdState,
        instrument: LyrebirdInstrument,
        is_first_row: bool,
    ) -> Element<'a, Message> {
        let instrument_state = snapshot.instrument_state(instrument);
        let (
            initial_spectrogram_path,
            initial_overlay_label,
            initial_activate_message,
            initial_empty_label,
        ) = if instrument_state.disabled {
            (None, None, None, "Disabled")
        } else {
            (
                initial_spectrogram_path(instrument_state),
                initial_overlay_label(instrument_state),
                initial_spectrogram_path(instrument_state)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Initial)),
                "Waiting for spectrogram",
            )
        };
        let (
            current_spectrogram_path,
            current_overlay_label,
            current_activate_message,
            current_empty_label,
        ) = if instrument_state.disabled {
            (None, None, None, "Disabled")
        } else {
            (
                current_spectrogram_path(instrument_state),
                current_overlay_label(instrument_state),
                current_spectrogram_path(instrument_state)
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Current)),
                "Waiting for spectrogram",
            )
        };
        let (best_spectrogram_path, best_overlay_label, best_activate_message, best_empty_label) =
            if instrument_state.disabled {
                (None, None, None, "Disabled")
            } else {
                (
                    best_spectrogram_path(instrument_state),
                    best_overlay_label(instrument_state),
                    best_spectrogram_path(instrument_state)
                        .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Best)),
                    "Waiting for spectrogram",
                )
            };
        let cards = row![
            spectrogram_tile(
                self.spectrogram_handle(non_empty_path(&instrument_state.target_spectrogram_path)),
                Some(instrument.display_name().to_owned()),
                alignment::Horizontal::Left,
                existing_path(non_empty_path(&instrument_state.target_sample_path))
                    .map(|_| Message::ActivateSpectrogram(instrument, SnapshotKind::Target)),
                "Waiting for spectrogram",
            ),
            spectrogram_tile(
                self.spectrogram_handle(initial_spectrogram_path),
                initial_overlay_label,
                alignment::Horizontal::Right,
                initial_activate_message,
                initial_empty_label,
            ),
            spectrogram_tile(
                self.spectrogram_handle(current_spectrogram_path),
                current_overlay_label,
                alignment::Horizontal::Right,
                current_activate_message,
                current_empty_label,
            ),
            spectrogram_tile(
                self.spectrogram_handle(best_spectrogram_path),
                best_overlay_label,
                alignment::Horizontal::Right,
                best_activate_message,
                best_empty_label,
            ),
        ]
        .spacing(SNAPSHOT_GAP)
        .width(Length::Fill)
        .height(Length::Fill);

        let mut content = column![].width(Length::Fill).height(Length::Fill);
        if !is_first_row {
            content = content.push(divider_horizontal());
        }
        content = content.push(
            container(
                container(cards)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([SNAPSHOT_ROW_VERTICAL_PADDING, SECTION_HORIZONTAL_PADDING]),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .style(snapshot_row_style),
        );

        content.width(Length::Fill).height(Length::Fill).into()
    }

    fn spectrogram_handle(&self, path: Option<&Path>) -> Option<iced_image::Handle> {
        path.and_then(|path| self.spectrogram_handles.get(path))
            .cloned()
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

fn spectrogram_tile<'a>(
    spectrogram_handle: Option<iced_image::Handle>,
    overlay_label: Option<String>,
    overlay_alignment: alignment::Horizontal,
    activate_message: Option<Message>,
    empty_label: &'static str,
) -> Element<'a, Message> {
    let image_panel: Element<'a, Message> = match spectrogram_handle {
        Some(handle) => {
            let preview = iced_image(handle)
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
        None => container(text(empty_label).size(14))
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
                .size(HEADER_LABEL_TEXT_SIZE)
                .width(Length::Fill)
                .align_x(overlay_alignment),
        )
        .width(Length::Fill)
        .height(Length::Fixed(SPECTROGRAM_OVERLAY_HEIGHT))
        .padding([
            HEADER_LABEL_VERTICAL_PADDING,
            HEADER_LABEL_HORIZONTAL_PADDING,
        ])
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

fn divider_vertical<'a>(visible: bool) -> Element<'a, Message> {
    container(text(""))
        .width(if visible { 1 } else { 0 })
        .height(Length::Fill)
        .style(divider_style)
        .into()
}

fn current_spectrogram_path(instrument_state: &LyrebirdInstrumentState) -> Option<&Path> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| non_empty_path(&code.spectrogram_path))
        .or_else(|| {
            instrument_state
                .latest_generated_spectrogram_path
                .as_deref()
                .and_then(non_empty_path)
        })
}

fn current_sample_path(instrument_state: &LyrebirdInstrumentState) -> Option<&Path> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| non_empty_path(&code.sample_path))
        .or_else(|| {
            instrument_state
                .latest_generated_sample_path
                .as_deref()
                .and_then(non_empty_path)
        })
}

fn current_overlay_label(instrument_state: &LyrebirdInstrumentState) -> Option<String> {
    instrument_state
        .latest_rendered_code
        .as_ref()
        .and_then(|code| {
            current_spectrogram_path(instrument_state).map(|_| score_label("current", code.score()))
        })
        .or_else(|| {
            current_spectrogram_path(instrument_state)
                .map(|_| score_label("current", instrument_state.latest_generated_similarity))
        })
}

fn best_spectrogram_path(instrument_state: &LyrebirdInstrumentState) -> Option<&Path> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| non_empty_path(&code.spectrogram_path))
        .or_else(|| {
            instrument_state
                .best_generated_spectrogram_path
                .as_deref()
                .and_then(non_empty_path)
        })
}

fn best_sample_path(instrument_state: &LyrebirdInstrumentState) -> Option<&Path> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| non_empty_path(&code.sample_path))
        .or_else(|| {
            instrument_state
                .best_generated_sample_path
                .as_deref()
                .and_then(non_empty_path)
        })
}

fn best_overlay_label(instrument_state: &LyrebirdInstrumentState) -> Option<String> {
    instrument_state
        .best_generated_code
        .as_ref()
        .and_then(|code| {
            best_spectrogram_path(instrument_state).map(|_| score_label("best", code.score()))
        })
        .or_else(|| {
            best_spectrogram_path(instrument_state)
                .map(|_| score_label("best", instrument_state.best_similarity))
        })
}

fn initial_spectrogram_path(instrument_state: &LyrebirdInstrumentState) -> Option<&Path> {
    non_empty_path(&instrument_state.initial_dsp_code.spectrogram_path)
}

fn initial_overlay_label(instrument_state: &LyrebirdInstrumentState) -> Option<String> {
    initial_spectrogram_path(instrument_state)
        .map(|_| score_label("initial", instrument_state.initial_dsp_code.score()))
}

fn score_label(label: &str, score: Option<f32>) -> String {
    score
        .map(|score| format!("{label}  {score:.6}"))
        .unwrap_or_else(|| label.to_owned())
}

fn spectrogram_action_payload(
    instrument_state: &LyrebirdInstrumentState,
    kind: SnapshotKind,
) -> (Option<PathBuf>, Option<String>) {
    match kind {
        SnapshotKind::Initial => (
            existing_path(non_empty_path(
                &instrument_state.initial_dsp_code.sample_path,
            )),
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
            existing_path(non_empty_path(&instrument_state.target_sample_path)),
            None,
        ),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

fn non_empty_path(path: &Path) -> Option<&Path> {
    (!path.as_os_str().is_empty()).then_some(path)
}

fn existing_path(path: Option<&Path>) -> Option<PathBuf> {
    path.filter(|path| path.exists()).map(Path::to_path_buf)
}

async fn load_snapshot(
    client: Arc<dyn JungleClient>,
    journey_id: Uuid,
) -> Result<Option<LyrebirdState>, String> {
    let bytes = client
        .animal_appearance(journey_id)
        .await
        .map_err(|err| format!("failed to load lyrebird state: {err}"))?;
    bytes
        .map(|bytes| {
            postcard::from_bytes::<LyrebirdState>(&bytes)
                .map(|state| state.normalized_for_observation())
                .map_err(|err| format!("failed to decode lyrebird state snapshot: {err}"))
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

fn build_spectrogram_handles(snapshot: &LyrebirdState) -> BTreeMap<PathBuf, iced_image::Handle> {
    let mut handles = BTreeMap::new();

    for instrument in LyrebirdInstrument::ALL {
        let instrument_state = snapshot.instrument_state(instrument);
        for path in [
            existing_path(non_empty_path(&instrument_state.target_spectrogram_path)),
            existing_path(initial_spectrogram_path(instrument_state)),
            existing_path(current_spectrogram_path(instrument_state)),
            existing_path(best_spectrogram_path(instrument_state)),
        ]
        .into_iter()
        .flatten()
        {
            if handles.contains_key(&path) {
                continue;
            }

            if let Some(handle) = load_spectrogram_handle(&path) {
                handles.insert(path, handle);
            }
        }
    }

    handles
}

fn load_spectrogram_handle(path: &Path) -> Option<iced_image::Handle> {
    let decoded = match ::image::ImageReader::open(path) {
        Ok(reader) => match reader.decode() {
            Ok(image) => image,
            Err(err) => {
                warn!(path = %path.display(), error = %err, "failed to decode spectrogram preview");
                return None;
            }
        },
        Err(err) => {
            warn!(path = %path.display(), error = %err, "failed to open spectrogram preview");
            return None;
        }
    };

    let rgba = decoded
        .huerotate(SPECTROGRAM_HUE_ROTATION_DEGREES)
        .to_rgba8();
    let (width, height) = rgba.dimensions();

    Some(iced_image::Handle::from_rgba(
        width,
        height,
        rgba.into_raw(),
    ))
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

    fn play(&mut self, path: &Path) -> Result<(), String> {
        let handle = self
            .handle
            .as_ref()
            .ok_or_else(|| "audio output is unavailable".to_owned())?;
        let file = File::open(path)
            .map_err(|err| format!("failed to open wav file {}: {err}", path.display()))?;
        let decoder = Decoder::new(BufReader::new(file))
            .map_err(|err| format!("failed to decode wav file {}: {err}", path.display()))?;
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
