use crate::{Depth1, ShutdownFlag};
use iced::widget::container;
use iced::{Element, Font, Length, Subscription, Task};
use jungle_sdk::JungleClient;
use std::time::Duration;
use uuid::Uuid;

const WINDOW_WIDTH: f32 = 1520.0;
const WINDOW_HEIGHT: f32 = 920.0;
const UI_TICK_INTERVAL: Duration = Duration::from_millis(100);

pub fn run_ui<C>(
    client: C,
    journey_id: Uuid,
    shutdown: ShutdownFlag,
    title: &'static str,
) -> Result<(), iced::Error>
where
    C: JungleClient + Clone + 'static,
{
    iced::application(
        move || ReplayUi::new(client.clone(), journey_id, shutdown.clone()),
        ReplayUi::update,
        ReplayUi::view,
    )
    .title(move |_app: &ReplayUi| title.to_string())
    .subscription(ReplayUi::subscription)
    .window_size((WINDOW_WIDTH, WINDOW_HEIGHT))
    .default_font(Font::with_name("Iosevka"))
    .antialiasing(true)
    .run()
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    Viewer(jungle_vision::EjectedViewerMessage),
}

struct ReplayUi {
    shutdown: ShutdownFlag,
    viewer: jungle_vision::EjectedViewer<jungle_vision::DefaultTheme, jungle_vision::AnyAnimal>,
}

impl ReplayUi {
    fn new<C>(client: C, journey_id: Uuid, shutdown: ShutdownFlag) -> (Self, Task<Message>)
    where
        C: JungleClient + Clone + 'static,
    {
        let viewer = jungle_vision::JungleViewerBuilder::new()
            .title("Replay Journey")
            .eject_live_animal_with_theme::<Depth1, _, _, jungle_vision::AnyAnimal>(
                client,
                journey_id,
                jungle_vision::DefaultTheme::default().with_cluster_expansion_config(
                    jungle_vision::ClusterExpansionConfig {
                        while_clusters: jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                        transparent_clusters:
                            jungle_vision::ClusterExpansionMode::AlwaysExpanded,
                    },
                ),
            );

        (Self { shutdown, viewer }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if self.shutdown.should_shutdown() {
                    return iced::exit();
                }
                Task::none()
            }
            Message::Viewer(event) => self.viewer.update(event).map(Message::Viewer),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::time::every(UI_TICK_INTERVAL).map(|_| Message::Tick),
            self.viewer.subscription().map(Message::Viewer),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        container(self.viewer.view().map(Message::Viewer))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
