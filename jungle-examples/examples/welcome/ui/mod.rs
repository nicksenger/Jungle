use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use iced::widget::{column, container, row, text};
use iced::{Color, Element, Font, Length, Subscription, Task};
use jungle_sdk::LocalClient;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct JourneyIds {
    pub lead_vocalist: Uuid,
    pub lead_guitarist: Uuid,
    pub rhythm_guitarist: Uuid,
    pub bass: Uuid,
    pub drums: Uuid,
}

#[derive(Clone)]
pub struct ShutdownFlag(Arc<AtomicBool>);

impl ShutdownFlag {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn request_shutdown(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn should_shutdown(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

pub fn spawn_ui(
    client: LocalClient,
    journeys: JourneyIds,
    shutdown: ShutdownFlag,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        if let Err(err) = run_ui(client, journeys, shutdown) {
            eprintln!("welcome UI exited with error: {err}");
        }
    })
}

fn run_ui(client: LocalClient, journeys: JourneyIds, shutdown: ShutdownFlag) -> iced::Result {
    let title = "Welcome Example";
    iced::application(
        move || WelcomeUi::new(client.clone(), journeys, shutdown.clone()),
        WelcomeUi::update,
        WelcomeUi::view,
    )
    .title(move |_app: &WelcomeUi| title.to_string())
    .subscription(WelcomeUi::subscription)
    .window_size((1800.0, 700.0))
    .antialiasing(true)
    .default_font(Font::with_name("Iosevka"))
    .run()
}

#[derive(Debug, Clone, Copy)]
enum Panel {
    LeadVocalist,
    LeadGuitarist,
    RhythmGuitarist,
    Bass,
    Drums,
}

#[derive(Debug, Clone)]
enum Message {
    Panel(Panel, jungle_viewer::EjectedViewerMessage),
    Tick,
}

struct WelcomeUi {
    lead_vocalist:
        jungle_viewer::EjectedViewer<jungle_viewer::DefaultTheme, jungle_viewer::AnyAnimal>,
    lead_guitarist:
        jungle_viewer::EjectedViewer<jungle_viewer::DefaultTheme, jungle_viewer::AnyAnimal>,
    rhythm_guitarist:
        jungle_viewer::EjectedViewer<jungle_viewer::DefaultTheme, jungle_viewer::AnyAnimal>,
    bass: jungle_viewer::EjectedViewer<jungle_viewer::DefaultTheme, jungle_viewer::AnyAnimal>,
    drums: jungle_viewer::EjectedViewer<jungle_viewer::DefaultTheme, jungle_viewer::AnyAnimal>,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: LocalClient,
        journeys: JourneyIds,
        shutdown: ShutdownFlag,
    ) -> (Self, Task<Message>) {
        let lead_vocalist = jungle_viewer::JungleViewerBuilder::new()
            .title("Welcome: Lead Vocalist")
            .eject_live_animal::<LeadVocalist, _>(client.clone(), journeys.lead_vocalist);
        let lead_guitarist = jungle_viewer::JungleViewerBuilder::new()
            .title("Welcome: Lead Guitarist")
            .eject_live_animal::<LeadGuitarist, _>(client.clone(), journeys.lead_guitarist);
        let rhythm_guitarist = jungle_viewer::JungleViewerBuilder::new()
            .title("Welcome: Rhythm Guitarist")
            .eject_live_animal::<RhythmGuitarist, _>(client.clone(), journeys.rhythm_guitarist);
        let bass = jungle_viewer::JungleViewerBuilder::new()
            .title("Welcome: Bass")
            .eject_live_animal::<Bass, _>(client.clone(), journeys.bass);
        let drums = jungle_viewer::JungleViewerBuilder::new()
            .title("Welcome: Drums")
            .eject_live_animal::<Drums, _>(client, journeys.drums);

        (
            Self {
                lead_vocalist,
                lead_guitarist,
                rhythm_guitarist,
                bass,
                drums,
                shutdown,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.shutdown.should_shutdown() {
                    return iced::exit();
                }
                Task::none()
            }
            Message::Panel(panel, event) => match panel {
                Panel::LeadVocalist => self
                    .lead_vocalist
                    .update(event)
                    .map(move |next| Message::Panel(Panel::LeadVocalist, next)),
                Panel::LeadGuitarist => self
                    .lead_guitarist
                    .update(event)
                    .map(move |next| Message::Panel(Panel::LeadGuitarist, next)),
                Panel::RhythmGuitarist => self
                    .rhythm_guitarist
                    .update(event)
                    .map(move |next| Message::Panel(Panel::RhythmGuitarist, next)),
                Panel::Bass => self
                    .bass
                    .update(event)
                    .map(move |next| Message::Panel(Panel::Bass, next)),
                Panel::Drums => self
                    .drums
                    .update(event)
                    .map(move |next| Message::Panel(Panel::Drums, next)),
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            self.lead_vocalist
                .subscription()
                .map(|event| Message::Panel(Panel::LeadVocalist, event)),
            self.lead_guitarist
                .subscription()
                .map(|event| Message::Panel(Panel::LeadGuitarist, event)),
            self.rhythm_guitarist
                .subscription()
                .map(|event| Message::Panel(Panel::RhythmGuitarist, event)),
            self.bass
                .subscription()
                .map(|event| Message::Panel(Panel::Bass, event)),
            self.drums
                .subscription()
                .map(|event| Message::Panel(Panel::Drums, event)),
            iced::time::every(std::time::Duration::from_millis(200)).map(|_| Message::Tick),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let panels = row![
            panel(
                "Lead Vocalist",
                self.lead_vocalist.view(),
                Panel::LeadVocalist
            ),
            panel(
                "Lead Guitarist",
                self.lead_guitarist.view(),
                Panel::LeadGuitarist
            ),
            panel(
                "Rhythm Guitarist",
                self.rhythm_guitarist.view(),
                Panel::RhythmGuitarist
            ),
            panel("Bass", self.bass.view(), Panel::Bass),
            panel("Drums", self.drums.view(), Panel::Drums),
        ]
        .spacing(12)
        .height(Length::Fill)
        .width(Length::Fill);

        container(panels)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background)
            .into()
    }
}

fn panel<'a>(
    label: &'a str,
    content: Element<'a, jungle_viewer::EjectedViewerMessage>,
    target: Panel,
) -> Element<'a, Message> {
    container(
        column![
            text(label).size(13).color(Color::from_rgb8(198, 229, 211)),
            container(content.map(move |event| Message::Panel(target, event)))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .spacing(8),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .height(Length::Fill)
    .style(panel_style)
    .into()
}

fn app_background(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(8, 19, 13))),
        ..Default::default()
    }
}

fn panel_style(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(10, 26, 17))),
        border: iced::border::rounded(8)
            .color(Color::from_rgb8(24, 63, 43))
            .width(1.0),
        ..Default::default()
    }
}
