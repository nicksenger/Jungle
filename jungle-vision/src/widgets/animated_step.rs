use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::widget::{button, column, text};
use iced::window::RedrawRequest;
use iced::{Color, Element, Event, Length, Rectangle, Theme};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tracing::info;

const FRAME_DURATION: Duration = Duration::from_millis(16);
static TWEEN_CACHE: OnceLock<Mutex<HashMap<u64, TweenState>>> = OnceLock::new();

fn tween_cache() -> &'static Mutex<HashMap<u64, TweenState>> {
    TWEEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Copy)]
struct TweenState {
    from: Color,
    to: Color,
    started_at: Instant,
    initialized: bool,
}

impl Default for TweenState {
    fn default() -> Self {
        Self {
            from: Color::TRANSPARENT,
            to: Color::TRANSPARENT,
            started_at: Instant::now(),
            initialized: false,
        }
    }
}

pub struct AnimatedStepNode<Message>
where
    Message: Clone + 'static,
{
    cache_namespace: u64,
    step_id: u32,
    runtime_id: Option<u32>,
    role: String,
    label: String,
    metadata: Option<String>,
    target_fill: Color,
    duration: Duration,
    _marker: std::marker::PhantomData<Message>,
}

impl<Message> AnimatedStepNode<Message>
where
    Message: Clone + 'static,
{
    fn cache_key(&self) -> u64 {
        let runtime = u64::from(self.runtime_id.unwrap_or(u32::MAX));
        self.cache_namespace.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ ((runtime << 32) | u64::from(self.step_id))
    }

    pub fn new(
        cache_namespace: u64,
        step_id: u32,
        runtime_id: Option<u32>,
        role: impl Into<String>,
        label: impl Into<String>,
        metadata: Option<String>,
        target_fill: Color,
        duration: Duration,
    ) -> Self {
        Self {
            cache_namespace,
            step_id,
            runtime_id,
            role: role.into(),
            label: label.into(),
            metadata,
            target_fill,
            duration,
            _marker: std::marker::PhantomData,
        }
    }

    fn sync_target(&self, state: &mut TweenState, now: Instant, shell: &mut Shell<'_, Message>) {
        if !state.initialized {
            let cached = tween_cache()
                .lock()
                .ok()
                .and_then(|cache| cache.get(&self.cache_key()).copied());
            if let Some(cached) = cached {
                *state = cached;
            } else {
                state.from = self.target_fill;
                state.to = self.target_fill;
                state.started_at = now;
                state.initialized = true;
            }
        }

        if state.to != self.target_fill {
            state.from = sample_color(*state, now, self.duration);
            state.to = self.target_fill;
            state.started_at = now;
            shell.request_redraw();
        }

        self.persist_state(*state);
    }

    fn persist_state(&self, state: TweenState) {
        if let Ok(mut cache) = tween_cache().lock() {
            cache.insert(self.cache_key(), state);
        }
    }

    fn as_element(&self, fill: Color) -> Element<'_, Message> {
        let accent_border = vary_green_shade(Color::from_rgb8(58, 122, 86), self.step_id);
        let accent_role = vary_green_shade(Color::from_rgb8(168, 198, 181), self.step_id);
        let body = column![
            text(self.role.as_str()).size(10).color(accent_role),
            text(self.label.as_str())
                .size(13)
                .color(Color::from_rgb8(223, 245, 230))
        ]
        .spacing(4);

        button(body)
            .padding([8, 10])
            .width(Length::Shrink)
            .style(move |_theme, status| {
                let (styled_fill, border_color, border_width, shadow) = match status {
                    button::Status::Hovered => (
                        mix_color(fill, Color::from_rgb8(20, 74, 45), 0.62),
                        Color::from_rgb8(120, 214, 160),
                        2.2,
                        iced::Shadow {
                            color: Color::from_rgba8(7, 23, 14, 0.55),
                            offset: iced::Vector::new(0.0, 4.0),
                            blur_radius: 9.0,
                        },
                    ),
                    button::Status::Pressed => (
                        mix_color(fill, Color::from_rgb8(11, 52, 31), 0.8),
                        Color::from_rgb8(153, 235, 189),
                        2.8,
                        iced::Shadow {
                            color: Color::from_rgba8(5, 17, 10, 0.68),
                            offset: iced::Vector::new(0.0, 2.0),
                            blur_radius: 6.0,
                        },
                    ),
                    _ => (
                        fill,
                        accent_border,
                        1.0,
                        iced::Shadow {
                            color: Color::from_rgba8(7, 23, 14, 0.25),
                            offset: iced::Vector::new(0.0, 1.0),
                            blur_radius: 3.0,
                        },
                    ),
                };
                iced::widget::button::Style {
                    background: Some(iced::Background::Color(Color {
                        a: 0.8,
                        ..styled_fill
                    })),
                    text_color: Color::from_rgb8(223, 245, 230),
                    border: iced::border::rounded(10)
                        .color(border_color)
                        .width(border_width),
                    shadow,
                    ..Default::default()
                }
            })
            .into()
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for AnimatedStepNode<Message>
where
    Message: Clone + 'static,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<TweenState>()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.as_element(Color::TRANSPARENT).as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.as_element(Color::TRANSPARENT)]);
    }

    fn state(&self) -> tree::State {
        tree::State::new(TweenState::default())
    }

    fn size(&self) -> iced::Size<Length> {
        iced::Size::new(Length::Shrink, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut element = self.as_element(self.target_fill);
        element
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<TweenState>();
        let now = match event {
            Event::Window(iced::window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };
        self.sync_target(state, now, shell);
        if let Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) = event {
            if cursor.is_over(layout.bounds()) {
                info!(
                    step_display_id = self.step_id,
                    step_runtime_id = ?self.runtime_id,
                    step_role = %self.role,
                    step_label = %self.label,
                    step_metadata = %self.metadata.as_deref().unwrap_or(""),
                    "jungle-vision step node clicked"
                );
            }
        }

        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if is_animating(*state, *now, self.duration) {
                shell.request_redraw_at(RedrawRequest::At(*now + FRAME_DURATION));
            } else if state.from != state.to {
                state.from = state.to;
            }
            self.persist_state(*state);
        }

        let fill = sample_color(*state, now, self.duration);
        let mut element = self.as_element(fill);
        element.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<TweenState>();
        let fill = sample_color(*state, Instant::now(), self.duration);
        let element = self.as_element(fill);
        element.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            return mouse::Interaction::Pointer;
        }
        let state = tree.state.downcast_ref::<TweenState>();
        let fill = sample_color(*state, Instant::now(), self.duration);
        let element = self.as_element(fill);
        element
            .as_widget()
            .mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let mut element = self.as_element(self.target_fill);
        element
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &iced::Renderer,
        _viewport: &Rectangle,
        _translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, iced::Renderer>> {
        None
    }
}

fn is_animating(state: TweenState, now: Instant, duration: Duration) -> bool {
    state.from != state.to && progress(state, now, duration) < 1.0
}

fn sample_color(state: TweenState, now: Instant, duration: Duration) -> Color {
    if state.from == state.to {
        return state.to;
    }
    let t = ease_out_cubic(progress(state, now, duration));
    lerp_color(state.from, state.to, t)
}

fn progress(state: TweenState, now: Instant, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    let elapsed = now.saturating_duration_since(state.started_at);
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn vary_green_shade(base: Color, step_id: u32) -> Color {
    // Deterministic tiny variation in [-0.035, +0.035] keeps shades close to the base green.
    let noise = (step_id as u64)
        .wrapping_mul(1_103_515_245)
        .wrapping_add(12_345)
        & 0xFFFF;
    let unit = (noise as f32) / 65_535.0;
    let delta = (unit - 0.5) * 0.07;
    Color {
        r: (base.r + delta).clamp(0.0, 1.0),
        g: (base.g + delta).clamp(0.0, 1.0),
        b: (base.b + delta).clamp(0.0, 1.0),
        a: base.a,
    }
}

fn mix_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

impl<'a, Message> From<AnimatedStepNode<Message>> for Element<'a, Message>
where
    Message: Clone + 'static,
{
    fn from(widget: AnimatedStepNode<Message>) -> Self {
        Element::new(widget)
    }
}
