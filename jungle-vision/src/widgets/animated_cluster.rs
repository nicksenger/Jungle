use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::widget::{button, container, text};
use iced::window::RedrawRequest;
use iced::{Color, Element, Event, Length, Rectangle, Theme};
use std::time::{Duration, Instant};

const FRAME_DURATION: Duration = Duration::from_millis(16);

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

#[derive(Debug, Clone, Copy)]
enum Mode {
    Overlay,
    Chip,
}

#[derive(Debug, Clone, Copy, Default)]
struct AnimatedClusterState {
    border: TweenState,
    fill: TweenState,
}

pub struct AnimatedClusterView<Message>
where
    Message: Clone + 'static,
{
    label: String,
    target_border: Color,
    target_fill: Color,
    duration: Duration,
    mode: Mode,
    _marker: std::marker::PhantomData<Message>,
}

impl<Message> AnimatedClusterView<Message>
where
    Message: Clone + 'static,
{
    pub fn overlay(
        label: impl Into<String>,
        target_border: Color,
        target_fill: Color,
        duration: Duration,
    ) -> Self {
        Self {
            label: label.into(),
            target_border,
            target_fill,
            duration,
            mode: Mode::Overlay,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn chip(
        label: impl Into<String>,
        target_border: Color,
        duration: Duration,
    ) -> Self {
        Self {
            label: label.into(),
            target_border,
            target_fill: Color::TRANSPARENT,
            duration,
            mode: Mode::Chip,
            _marker: std::marker::PhantomData,
        }
    }

    fn sync_target(
        &self,
        state: &mut AnimatedClusterState,
        now: Instant,
        shell: &mut Shell<'_, Message>,
    ) {
        sync_tween(&mut state.border, self.target_border, now, self.duration, shell);
        if matches!(self.mode, Mode::Overlay) {
            sync_tween(&mut state.fill, self.target_fill, now, self.duration, shell);
        }
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for AnimatedClusterView<Message>
where
    Message: Clone + 'static,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(self.as_element(Color::TRANSPARENT, Color::TRANSPARENT).as_widget())]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&[self.as_element(Color::TRANSPARENT, Color::TRANSPARENT)]);
    }

    fn state(&self) -> tree::State {
        tree::State::new(AnimatedClusterState::default())
    }

    fn size(&self) -> iced::Size<Length> {
        match self.mode {
            Mode::Overlay => iced::Size::new(Length::Fill, Length::Fill),
            Mode::Chip => iced::Size::new(Length::Shrink, Length::Shrink),
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut element = self.as_element(self.target_border, self.target_fill);
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
        let state = tree.state.downcast_mut::<AnimatedClusterState>();
        let now = match event {
            Event::Window(iced::window::Event::RedrawRequested(now)) => *now,
            _ => Instant::now(),
        };

        self.sync_target(state, now, shell);

        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let border_animating = is_animating(state.border, *now, self.duration);
            let fill_animating = matches!(self.mode, Mode::Overlay)
                && is_animating(state.fill, *now, self.duration);

            if border_animating || fill_animating {
                shell.request_redraw_at(RedrawRequest::At(*now + FRAME_DURATION));
            } else {
                if state.border.from != state.border.to {
                    state.border.from = state.border.to;
                }
                if state.fill.from != state.fill.to {
                    state.fill.from = state.fill.to;
                }
            }
        }

        let border = sample_color(state.border, now, self.duration);
        let fill = if matches!(self.mode, Mode::Overlay) {
            sample_color(state.fill, now, self.duration)
        } else {
            Color::TRANSPARENT
        };
        let mut element = self.as_element(border, fill);
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
        let state = tree.state.downcast_ref::<AnimatedClusterState>();
        let border = sample_color(state.border, Instant::now(), self.duration);
        let fill = if matches!(self.mode, Mode::Overlay) {
            sample_color(state.fill, Instant::now(), self.duration)
        } else {
            Color::TRANSPARENT
        };
        let element = self.as_element(border, fill);
        element
            .as_widget()
            .draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<AnimatedClusterState>();
        let border = sample_color(state.border, Instant::now(), self.duration);
        let fill = if matches!(self.mode, Mode::Overlay) {
            sample_color(state.fill, Instant::now(), self.duration)
        } else {
            Color::TRANSPARENT
        };
        let element = self.as_element(border, fill);
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
        let mut element = self.as_element(self.target_border, self.target_fill);
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

impl<Message> AnimatedClusterView<Message>
where
    Message: Clone + 'static,
{
    fn as_element(&self, border_color: Color, fill_color: Color) -> Element<'_, Message> {
        match self.mode {
            Mode::Overlay => container(
                container(text(self.label.as_str()).size(11).color(border_color))
                    .padding([4, 8])
                    .style(move |_theme| iced::widget::container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba8(20, 46, 30, 0.35))),
                        border: iced::border::rounded(6).color(border_color).width(1.0),
                        text_color: Some(border_color),
                        ..Default::default()
                    }),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top)
            .style(move |_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(fill_color)),
                border: iced::border::rounded(10).color(border_color).width(2.0),
                text_color: Some(border_color),
                ..Default::default()
            })
            .into(),
            Mode::Chip => button(text(self.label.as_str()).size(11).color(border_color))
                .padding([6, 10])
                .width(Length::Shrink)
                .style(move |_theme, _status| iced::widget::button::Style {
                    background: None,
                    text_color: border_color,
                    border: iced::border::rounded(8).color(border_color).width(1.4),
                    ..Default::default()
                })
                .into(),
        }
    }
}

fn sync_tween<Message>(
    tween: &mut TweenState,
    target: Color,
    now: Instant,
    duration: Duration,
    shell: &mut Shell<'_, Message>,
) {
    if !tween.initialized {
        tween.from = target;
        tween.to = target;
        tween.started_at = now;
        tween.initialized = true;
        return;
    }

    if tween.to != target {
        tween.from = sample_color(*tween, now, duration);
        tween.to = target;
        tween.started_at = now;
        shell.request_redraw();
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

impl<'a, Message> From<AnimatedClusterView<Message>> for Element<'a, Message>
where
    Message: Clone + 'static,
{
    fn from(widget: AnimatedClusterView<Message>) -> Self {
        Element::new(widget)
    }
}
