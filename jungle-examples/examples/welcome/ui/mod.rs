use crate::animals::{Bass, Drums, LeadGuitarist, LeadVocalist, RhythmGuitarist};
use crate::RuntimeClient;
use iced::widget::{column, container, text, Row};
use iced::{Color, Element, Font, Length, Subscription, Task};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct JourneyIds {
    pub lead_vocalist: Option<Uuid>,
    pub lead_guitarist: Option<Uuid>,
    pub rhythm_guitarist: Option<Uuid>,
    pub bass: Option<Uuid>,
    pub drums: Option<Uuid>,
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

pub fn run_ui(client: RuntimeClient, journeys: JourneyIds, shutdown: ShutdownFlag) -> iced::Result {
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
    Panel(Panel, jungle_vision::EjectedViewerMessage),
    Tick,
}

struct WelcomeUi {
    lead_vocalist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    lead_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    rhythm_guitarist:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    bass:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    drums:
        Option<jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>>,
    shutdown: ShutdownFlag,
}

impl WelcomeUi {
    fn new(
        client: RuntimeClient,
        journeys: JourneyIds,
        shutdown: ShutdownFlag,
    ) -> (Self, Task<Message>) {
        let lead_vocalist = journeys.lead_vocalist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Vocalist")
                .eject_live_animal::<LeadVocalist, _>(client.clone(), journey)
        });
        let lead_guitarist = journeys.lead_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Lead Guitarist")
                .eject_live_animal::<LeadGuitarist, _>(client.clone(), journey)
        });
        let rhythm_guitarist = journeys.rhythm_guitarist.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Rhythm Guitarist")
                .eject_live_animal::<RhythmGuitarist, _>(client.clone(), journey)
        });
        let bass = journeys.bass.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Bass")
                .eject_live_animal::<Bass, _>(client.clone(), journey)
        });
        let drums = journeys.drums.map(|journey| {
            jungle_vision::JungleViewerBuilder::new()
                .title("Welcome: Drums")
                .eject_live_animal::<Drums, _>(client, journey)
        });

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
                Panel::LeadVocalist => self.lead_vocalist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadVocalist, next))
                }),
                Panel::LeadGuitarist => self.lead_guitarist.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::LeadGuitarist, next))
                }),
                Panel::RhythmGuitarist => {
                    self.rhythm_guitarist.as_mut().map_or_else(Task::none, |v| {
                        v.update(event)
                            .map(move |next| Message::Panel(Panel::RhythmGuitarist, next))
                    })
                }
                Panel::Bass => self.bass.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::Bass, next))
                }),
                Panel::Drums => self.drums.as_mut().map_or_else(Task::none, |v| {
                    v.update(event)
                        .map(move |next| Message::Panel(Panel::Drums, next))
                }),
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = Vec::new();
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::LeadVocalist, event)),
            );
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::LeadGuitarist, event)),
            );
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::RhythmGuitarist, event)),
            );
        }
        if let Some(viewer) = self.bass.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::Bass, event)),
            );
        }
        if let Some(viewer) = self.drums.as_ref() {
            subscriptions.push(
                viewer
                    .subscription()
                    .map(|event| Message::Panel(Panel::Drums, event)),
            );
        }
        subscriptions
            .push(iced::time::every(std::time::Duration::from_millis(200)).map(|_| Message::Tick));
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<'_, Message> {
        let mut panels = Row::new()
            .spacing(12)
            .height(Length::Fill)
            .width(Length::Fill);
        let mut panel_count = 0usize;

        if let Some(viewer) = self.bass.as_ref() {
            panels = panels.push(panel("Bass", viewer.view(), Panel::Bass));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_guitarist.as_ref() {
            panels = panels.push(panel("Lead Guitarist", viewer.view(), Panel::LeadGuitarist));
            panel_count += 1;
        }
        if let Some(viewer) = self.lead_vocalist.as_ref() {
            panels = panels.push(panel("Lead Vocalist", viewer.view(), Panel::LeadVocalist));
            panel_count += 1;
        }
        if let Some(viewer) = self.rhythm_guitarist.as_ref() {
            panels = panels.push(panel(
                "Rhythm Guitarist",
                viewer.view(),
                Panel::RhythmGuitarist,
            ));
            panel_count += 1;
        }
        if let Some(viewer) = self.drums.as_ref() {
            panels = panels.push(panel("Drums", viewer.view(), Panel::Drums));
            panel_count += 1;
        }

        let content: Element<'_, Message> = if panel_count == 0 {
            text("No animals selected. Pass --animals with one or more names to show panels.")
                .size(16)
                .color(Color::from_rgb8(198, 229, 211))
                .into()
        } else {
            panels.into()
        };

        container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(app_background)
            .into()
    }
}

fn panel<'a>(
    label: &'a str,
    content: Element<'a, jungle_vision::EjectedViewerMessage>,
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
