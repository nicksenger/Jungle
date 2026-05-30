use iced::widget::stack;
use iced::{Element, Length, Subscription};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tracing::{info, warn};

mod duration_millis {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(u64::try_from(value.as_millis()).unwrap_or(u64::MAX))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        let duration = Duration::from_millis(millis);
        let round_trip = u64::try_from(duration.as_millis()).map_err(D::Error::custom)?;
        if round_trip != millis {
            return Err(D::Error::custom("duration milliseconds out of range"));
        }
        Ok(duration)
    }
}

const fn default_cover_offset() -> f32 {
    0.5
}

const fn default_video_fade_in() -> Duration {
    VIDEO_FADE_IN
}

const fn default_video_fade_out() -> Duration {
    VIDEO_FADE_OUT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    LeadVocalist,
    RhythmGuitarist,
    LeadGuitarist,
    Bass,
    Drums,
}

impl Panel {
    pub const ALL: [Self; 5] = [
        Self::LeadVocalist,
        Self::RhythmGuitarist,
        Self::LeadGuitarist,
        Self::Bass,
        Self::Drums,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::LeadVocalist => "lead_vocalist",
            Self::RhythmGuitarist => "rhythm_guitarist",
            Self::LeadGuitarist => "lead_guitarist",
            Self::Bass => "bass",
            Self::Drums => "drums",
        }
    }

    const fn video_region_name(self) -> &'static str {
        match self {
            Self::LeadVocalist => "lead vocalist panel overlay",
            Self::RhythmGuitarist => "rhythm guitarist panel overlay",
            Self::LeadGuitarist => "lead guitarist panel overlay",
            Self::Bass => "bass panel overlay",
            Self::Drums => "drums panel overlay",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VideoAsset {
    Chimp,
    Chimp2,
    Chimp3,
    Chimp4,
    Croc,
    Croc4,
    Elephants,
    Elephants2,
    Fungi,
    Fungi2,
    Frogs,
    Giraffe,
    Giraffe2,
    Hippo,
    Jackfruit,
    Jaguar,
    Jaguar2,
    Jungle,
    Jungle2,
    JungleDown,
    JungleDown2,
    Lions2,
    Monkey,
    Ostrich,
    Rhino,
    Serpentine,
    Tiger,
    Tiger2,
    Tiger3,
    Toucan,
    Toucan2,
    Zebra,
    Zebra2,
}

impl VideoAsset {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Chimp => "chimp.mkv",
            Self::Chimp2 => "chimp2.mkv",
            Self::Chimp3 => "chimp3.mkv",
            Self::Chimp4 => "chimp4.mkv",
            Self::Croc => "croc.mkv",
            Self::Croc4 => "croc4.mkv",
            Self::Elephants => "elephants.mkv",
            Self::Elephants2 => "elephants2.mkv",
            Self::Fungi => "fungi.mkv",
            Self::Fungi2 => "fungi2.mkv",
            Self::Frogs => "frogs.mkv",
            Self::Giraffe => "giraffe.mkv",
            Self::Giraffe2 => "giraffe2.mkv",
            Self::Hippo => "hippo.mkv",
            Self::Jackfruit => "jackfruit.mkv",
            Self::Jaguar => "jaguar.mkv",
            Self::Jaguar2 => "jaguar2.mkv",
            Self::Jungle => "jungle.mkv",
            Self::Jungle2 => "jungle2.mkv",
            Self::JungleDown => "jungledown.mkv",
            Self::JungleDown2 => "jungledown2.mkv",
            Self::Lions2 => "lions2.mkv",
            Self::Monkey => "monkey.mkv",
            Self::Ostrich => "ostrich.mkv",
            Self::Rhino => "rhino.mkv",
            Self::Serpentine => "serpentine.mkv",
            Self::Tiger => "tiger.mkv",
            Self::Tiger2 => "tiger2.mkv",
            Self::Tiger3 => "tiger3.mkv",
            Self::Toucan => "toucan.mkv",
            Self::Toucan2 => "toucan2.mkv",
            Self::Zebra => "zebra.mkv",
            Self::Zebra2 => "zebra2.mkv",
        }
    }

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::Chimp => include_bytes!("../../assets/chimp.mkv"),
            Self::Chimp2 => include_bytes!("../../assets/chimp2.mkv"),
            Self::Chimp3 => include_bytes!("../../assets/chimp3.mkv"),
            Self::Chimp4 => include_bytes!("../../assets/chimp4.mkv"),
            Self::Croc => include_bytes!("../../assets/croc.mkv"),
            Self::Croc4 => include_bytes!("../../assets/croc4.mkv"),
            Self::Elephants => include_bytes!("../../assets/elephants.mkv"),
            Self::Elephants2 => include_bytes!("../../assets/elephants2.mkv"),
            Self::Fungi => include_bytes!("../../assets/fungi.mkv"),
            Self::Fungi2 => include_bytes!("../../assets/fungi2.mkv"),
            Self::Frogs => include_bytes!("../../assets/frogs.mkv"),
            Self::Giraffe => include_bytes!("../../assets/giraffe.mkv"),
            Self::Giraffe2 => include_bytes!("../../assets/giraffe2.mkv"),
            Self::Hippo => include_bytes!("../../assets/hippo.mkv"),
            Self::Jackfruit => include_bytes!("../../assets/jackfruit.mkv"),
            Self::Jaguar => include_bytes!("../../assets/jaguar.mkv"),
            Self::Jaguar2 => include_bytes!("../../assets/jaguar2.mkv"),
            Self::Jungle => include_bytes!("../../assets/jungle.mkv"),
            Self::Jungle2 => include_bytes!("../../assets/jungle2.mkv"),
            Self::JungleDown => include_bytes!("../../assets/jungledown.mkv"),
            Self::JungleDown2 => include_bytes!("../../assets/jungledown2.mkv"),
            Self::Lions2 => include_bytes!("../../assets/lions2.mkv"),
            Self::Monkey => include_bytes!("../../assets/monkey.mkv"),
            Self::Ostrich => include_bytes!("../../assets/ostrich.mkv"),
            Self::Rhino => include_bytes!("../../assets/rhino.mkv"),
            Self::Serpentine => include_bytes!("../../assets/serpentine.mkv"),
            Self::Tiger => include_bytes!("../../assets/tiger.mkv"),
            Self::Tiger2 => include_bytes!("../../assets/tiger2.mkv"),
            Self::Tiger3 => include_bytes!("../../assets/tiger3.mkv"),
            Self::Toucan => include_bytes!("../../assets/toucan.mkv"),
            Self::Toucan2 => include_bytes!("../../assets/toucan2.mkv"),
            Self::Zebra => include_bytes!("../../assets/zebra.mkv"),
            Self::Zebra2 => include_bytes!("../../assets/zebra2.mkv"),
        }
    }
}

const VIDEO_FADE_IN: Duration = Duration::from_millis(180);
const VIDEO_FADE_OUT: Duration = Duration::from_millis(220);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct VideoPlaybackRequest {
    video: VideoAsset,
    #[serde(with = "duration_millis")]
    offset: Duration,
    #[serde(with = "duration_millis")]
    duration: Duration,
    #[serde(default = "default_video_fade_in", with = "duration_millis")]
    fade_in: Duration,
    #[serde(default = "default_video_fade_out", with = "duration_millis")]
    fade_out: Duration,
    opacity: f32,
    #[serde(default = "default_cover_offset")]
    cover_offset: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TickPlaybackPlan {
    tick: u32,
    app_overlay: Option<VideoPlaybackRequest>,
    lead_vocalist_panel: Option<VideoPlaybackRequest>,
    rhythm_guitarist_panel: Option<VideoPlaybackRequest>,
    lead_guitarist_panel: Option<VideoPlaybackRequest>,
    bass_panel: Option<VideoPlaybackRequest>,
    drums_panel: Option<VideoPlaybackRequest>,
}

#[derive(Debug, Clone)]
pub struct VideoPlaybackPlan(Vec<TickPlaybackPlan>);

#[derive(Debug, Clone, Deserialize)]
struct VideoPlaybackPlanDocument {
    plans: Vec<TickPlaybackPlan>,
}

impl VideoPlaybackPlan {
    pub fn from_toml_str(toml_str: &str) -> Result<Self, String> {
        toml::from_str::<VideoPlaybackPlanDocument>(toml_str)
            .map(|doc| Self(doc.plans))
            .map_err(|err| format!("failed to deserialize video plan TOML: {err}"))
    }

    fn plans(&self) -> &[TickPlaybackPlan] {
        &self.0
    }
}

impl TickPlaybackPlan {
    fn panel_request(self, panel: Panel) -> Option<VideoPlaybackRequest> {
        match panel {
            Panel::LeadVocalist => self.lead_vocalist_panel,
            Panel::RhythmGuitarist => self.rhythm_guitarist_panel,
            Panel::LeadGuitarist => self.lead_guitarist_panel,
            Panel::Bass => self.bass_panel,
            Panel::Drums => self.drums_panel,
        }
    }
}

const VIDEO_PLAYBACK_PLAN_TOML: &str =
    include_str!("../../assets/default_video_playback_plan.toml");

fn default_video_playback_plan() -> Vec<TickPlaybackPlan> {
    toml::from_str::<VideoPlaybackPlanDocument>(VIDEO_PLAYBACK_PLAN_TOML)
        .map(|doc| doc.plans)
        .expect("default video playback plan TOML should deserialize")
}

#[derive(Debug, Clone)]
struct RegionPlayback {
    enabled: bool,
    visible_until: Option<Instant>,
    fade_out_at: Option<Instant>,
    fade_out_duration: Duration,
    fade_out_started: bool,
}

impl RegionPlayback {
    fn hidden() -> Self {
        Self {
            enabled: false,
            visible_until: None,
            fade_out_at: None,
            fade_out_duration: VIDEO_FADE_OUT,
            fade_out_started: false,
        }
    }
}

struct VideoOverlayLayer {
    id: u64,
    state: iced_av1::widget::State,
    playback: RegionPlayback,
}

#[derive(Debug, Clone)]
pub enum VideoMessage {
    AppLayer(u64, iced_av1::widget::Message),
    PanelLayer(Panel, u64, iced_av1::widget::Message),
}

pub struct VideoOverlayController {
    applied_ticks: HashSet<u32>,
    next_video_layer_id: u64,
    app_overlay_layers: Vec<VideoOverlayLayer>,
    lead_vocalist_panel_overlay_layers: Vec<VideoOverlayLayer>,
    rhythm_guitarist_panel_overlay_layers: Vec<VideoOverlayLayer>,
    lead_guitarist_panel_overlay_layers: Vec<VideoOverlayLayer>,
    bass_panel_overlay_layers: Vec<VideoOverlayLayer>,
    drums_panel_overlay_layers: Vec<VideoOverlayLayer>,
    video_playback_plan: Option<VideoPlaybackPlan>,
    default_video_playback_plan: Vec<TickPlaybackPlan>,
}

impl VideoOverlayController {
    pub fn new(video_playback_plan: Option<VideoPlaybackPlan>) -> Self {
        Self {
            applied_ticks: HashSet::new(),
            next_video_layer_id: 0,
            app_overlay_layers: Vec::new(),
            lead_vocalist_panel_overlay_layers: Vec::new(),
            rhythm_guitarist_panel_overlay_layers: Vec::new(),
            lead_guitarist_panel_overlay_layers: Vec::new(),
            bass_panel_overlay_layers: Vec::new(),
            drums_panel_overlay_layers: Vec::new(),
            video_playback_plan,
            default_video_playback_plan: default_video_playback_plan(),
        }
    }

    pub fn update(&mut self, message: VideoMessage) {
        match message {
            VideoMessage::AppLayer(layer_id, event) => {
                if let Some(layer) = self
                    .app_overlay_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.update(event);
                }
            }
            VideoMessage::PanelLayer(panel, layer_id, event) => {
                if let Some(layer) = self
                    .panel_overlay_layers_mut(panel)
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.update(event);
                }
            }
        }
    }

    pub fn apply_playback_plan(&mut self, rhythm_tick: u64) {
        let plans = self
            .video_playback_plan
            .as_ref()
            .map_or_else(
                || self.default_video_playback_plan.as_slice(),
                VideoPlaybackPlan::plans,
            )
            .to_vec();
        for plan in plans {
            if rhythm_tick < u64::from(plan.tick) {
                continue;
            }
            if !self.applied_ticks.insert(plan.tick) {
                continue;
            }

            let now = Instant::now();
            if let Some(request) = plan.app_overlay {
                info!(
                    tick = plan.tick,
                    video = request.video.name(),
                    "selected app overlay video from playback request"
                );
                self.push_overlay_layer(
                    None,
                    "app overlay",
                    iced_av1::ScaleMode::Stretch,
                    request,
                    now,
                );
            }

            for panel in Panel::ALL {
                if let Some(request) = plan.panel_request(panel) {
                    info!(
                        tick = plan.tick,
                        panel = panel.name(),
                        video = request.video.name(),
                        cover_offset = request.cover_offset,
                        "selected panel overlay video from playback request"
                    );
                    self.push_overlay_layer(
                        Some(panel),
                        panel.video_region_name(),
                        iced_av1::ScaleMode::Cover {
                            offset: request.cover_offset.clamp(-1.0, 1.0),
                        },
                        request,
                        now,
                    );
                }
            }
        }
    }

    pub fn update_playback_regions(&mut self) {
        let now = Instant::now();
        Self::tick_overlay_layers(&mut self.app_overlay_layers, now);
        for panel in Panel::ALL {
            Self::tick_overlay_layers(self.panel_overlay_layers_mut(panel), now);
        }
    }

    pub fn stack_app_overlays<'a, M>(
        &'a self,
        app: Element<'a, M>,
        map: impl Fn(VideoMessage) -> M + Copy + 'a,
    ) -> Element<'a, M>
    where
        M: 'a + Clone,
    {
        let mut composed = app;
        for layer in self
            .app_overlay_layers
            .iter()
            .filter(|layer| layer.playback.enabled)
        {
            let layer_id = layer.id;
            if let Some(overlay) = layer.state.overlay_view(map_video_message) {
                let overlay =
                    overlay.map(move |event| map(VideoMessage::AppLayer(layer_id, event)));
                composed = stack([composed, overlay])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
            }
        }
        composed
    }

    pub fn stack_panel_overlays<'a, M>(
        &'a self,
        panel: Element<'a, M>,
        panel_kind: Panel,
        map: impl Fn(VideoMessage) -> M + Copy + 'a,
    ) -> Element<'a, M>
    where
        M: 'a + Clone,
    {
        let mut composed = panel;
        for layer in self
            .panel_overlay_layers(panel_kind)
            .iter()
            .filter(|layer| layer.playback.enabled)
        {
            let layer_id = layer.id;
            if let Some(overlay) = layer.state.overlay_view(map_video_message) {
                let overlay = overlay
                    .map(move |event| map(VideoMessage::PanelLayer(panel_kind, layer_id, event)));
                composed = stack([composed, overlay])
                    .width(Length::FillPortion(1))
                    .height(Length::Fill)
                    .into();
            }
        }
        composed
    }

    pub fn subscriptions<M>(&self, map: fn(VideoMessage) -> M) -> Vec<Subscription<M>>
    where
        M: Clone + 'static,
    {
        let mut subscriptions = Vec::new();
        for layer in &self.app_overlay_layers {
            let layer_id = layer.id;
            subscriptions.push(
                layer
                    .state
                    .subscription(move |event| map(VideoMessage::AppLayer(layer_id, event))),
            );
        }
        for panel in Panel::ALL {
            for layer in self.panel_overlay_layers(panel) {
                let layer_id = layer.id;
                subscriptions.push(layer.state.subscription(move |event| {
                    map(VideoMessage::PanelLayer(panel, layer_id, event))
                }));
            }
        }
        subscriptions
    }

    fn push_overlay_layer(
        &mut self,
        panel: Option<Panel>,
        region: &str,
        scale_mode: iced_av1::ScaleMode,
        request: VideoPlaybackRequest,
        now: Instant,
    ) {
        let Some(state) = init_video_state(region, scale_mode, request.video.bytes()) else {
            return;
        };
        let mut layer = VideoOverlayLayer {
            id: self.allocate_video_layer_id(),
            state,
            playback: RegionPlayback::hidden(),
        };
        Self::start_region_playback(&layer.state, &mut layer.playback, request, now);
        match panel {
            None => self.app_overlay_layers.push(layer),
            Some(panel) => self.panel_overlay_layers_mut(panel).push(layer),
        }
    }

    fn tick_overlay_layers(layers: &mut Vec<VideoOverlayLayer>, now: Instant) {
        for layer in layers.iter_mut() {
            Self::tick_region(&layer.state, &mut layer.playback, now);
        }
        layers.retain(|layer| layer.playback.enabled);
    }

    fn tick_region(overlay: &iced_av1::widget::State, playback: &mut RegionPlayback, now: Instant) {
        if !playback.enabled {
            return;
        }

        if !playback.fade_out_started {
            if let Some(fade_out_at) = playback.fade_out_at {
                if now >= fade_out_at {
                    overlay.tween_to_opacity_with(
                        0.0,
                        iced_av1::OpacityTween {
                            duration: playback.fade_out_duration,
                        },
                    );
                    playback.fade_out_started = true;
                }
            }
        }

        if let Some(visible_until) = playback.visible_until {
            if now >= visible_until {
                overlay.set_opacity(0.0);
                if let Err(error) = overlay.pause() {
                    warn!(error = %error, "failed to pause AV overlay after visibility ended");
                }
                *playback = RegionPlayback::hidden();
            }
        }
    }

    fn start_region_playback(
        overlay: &iced_av1::widget::State,
        playback: &mut RegionPlayback,
        request: VideoPlaybackRequest,
        now: Instant,
    ) {
        if let Err(error) = overlay.resume() {
            warn!(error = %error, "failed to resume AV overlay before playback");
            return;
        }
        if let Err(error) = overlay.seek(duration_to_ns(request.offset)) {
            warn!(error = %error, "failed to seek AV overlay to requested offset");
            return;
        }
        overlay.set_opacity(0.0);
        overlay.tween_to_opacity_with(
            request.opacity,
            iced_av1::OpacityTween {
                duration: request.fade_in.min(request.duration),
            },
        );

        let visible_until = now + request.duration;
        let fade_out_duration = request.fade_out.min(request.duration);
        playback.enabled = true;
        playback.visible_until = Some(visible_until);
        playback.fade_out_at = Some(visible_until.checked_sub(fade_out_duration).unwrap_or(now));
        playback.fade_out_duration = fade_out_duration;
        playback.fade_out_started = false;
    }

    fn allocate_video_layer_id(&mut self) -> u64 {
        let id = self.next_video_layer_id;
        self.next_video_layer_id = self.next_video_layer_id.wrapping_add(1);
        id
    }

    fn panel_overlay_layers(&self, panel: Panel) -> &[VideoOverlayLayer] {
        match panel {
            Panel::LeadVocalist => &self.lead_vocalist_panel_overlay_layers,
            Panel::RhythmGuitarist => &self.rhythm_guitarist_panel_overlay_layers,
            Panel::LeadGuitarist => &self.lead_guitarist_panel_overlay_layers,
            Panel::Bass => &self.bass_panel_overlay_layers,
            Panel::Drums => &self.drums_panel_overlay_layers,
        }
    }

    fn panel_overlay_layers_mut(&mut self, panel: Panel) -> &mut Vec<VideoOverlayLayer> {
        match panel {
            Panel::LeadVocalist => &mut self.lead_vocalist_panel_overlay_layers,
            Panel::RhythmGuitarist => &mut self.rhythm_guitarist_panel_overlay_layers,
            Panel::LeadGuitarist => &mut self.lead_guitarist_panel_overlay_layers,
            Panel::Bass => &mut self.bass_panel_overlay_layers,
            Panel::Drums => &mut self.drums_panel_overlay_layers,
        }
    }
}

fn init_video_state(
    region: &str,
    scale_mode: iced_av1::ScaleMode,
    video_bytes: &[u8],
) -> Option<iced_av1::widget::State> {
    let playback_options = iced_av1::PlaybackOptions::default();
    let opacity_options = iced_av1::OpacityOptions {
        opacity: 0.0,
        tween: iced_av1::OpacityTween {
            duration: VIDEO_FADE_IN,
        },
    };
    let source = iced_av1::MediaSource::from_bytes(video_bytes.to_vec());
    match iced_av1::widget::State::new_with_media_source_and_opacity_options(
        source,
        playback_options,
        opacity_options,
    ) {
        Ok(mut state) => {
            state.set_scale_mode(scale_mode);
            if let Err(error) = state.pause() {
                warn!(
                    error = %error,
                    region,
                    "failed to pause AV overlay state at initialization"
                );
            }
            Some(state)
        }
        Err(error) => {
            warn!(error = %error, region, "failed to initialize AV overlay state");
            None
        }
    }
}

fn map_video_message(message: iced_av1::widget::Message) -> iced_av1::widget::Message {
    message
}

fn duration_to_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{VideoAsset, VideoPlaybackPlan};
    use std::time::Duration;

    #[test]
    fn parse_video_plan_toml_array_of_tables() {
        let toml_str = r#"
[[plans]]
tick = 6700
app_overlay = { video = "Jungle2", offset = 0, duration = 3000, opacity = 0.3, cover_offset = 0.5 }

[[plans]]
tick = 88000
lead_vocalist_panel = { video = "Jackfruit", offset = 0, duration = 2000, opacity = 0.3, cover_offset = 0.5 }
rhythm_guitarist_panel = { video = "Jackfruit", offset = 0, duration = 2000, opacity = 0.15, cover_offset = 0.5 }
lead_guitarist_panel = { video = "Jackfruit", offset = 0, duration = 2000, opacity = 0.15, cover_offset = 0.5 }
"#;

        let plan = VideoPlaybackPlan::from_toml_str(toml_str).expect("plan should parse");
        let plans = plan.plans();
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].tick, 6700);
        let first_overlay = plans[0]
            .app_overlay
            .as_ref()
            .expect("first plan should set app_overlay");
        assert!(matches!(first_overlay.video, VideoAsset::Jungle2));
        assert_eq!(first_overlay.offset, Duration::from_millis(0));
        assert_eq!(first_overlay.duration, Duration::from_millis(3000));
        assert!((first_overlay.opacity - 0.3).abs() < f32::EPSILON);
        assert!((first_overlay.cover_offset - 0.5).abs() < f32::EPSILON);
        assert!(plans[0].lead_vocalist_panel.is_none());

        assert_eq!(plans[1].tick, 88000);
        assert!(plans[1].app_overlay.is_none());
        assert!(plans[1].lead_vocalist_panel.is_some());
        assert!(plans[1].rhythm_guitarist_panel.is_some());
        assert!(plans[1].lead_guitarist_panel.is_some());
        assert!(plans[1].bass_panel.is_none());
        assert!(plans[1].drums_panel.is_none());
    }
}
