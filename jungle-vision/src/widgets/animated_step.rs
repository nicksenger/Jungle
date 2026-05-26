use iced::advanced::layout;
use iced::advanced::mouse;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Layout, Shell};
use iced::widget::{button, column, text};
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

pub struct AnimatedStepNode<Message>
where
    Message: Clone + 'static,
{
    role: String,
    label: String,
    target_fill: Color,
    duration: Duration,
    _marker: std::marker::PhantomData<Message>,
}

impl<Message> AnimatedStepNode<Message>
where
    Message: Clone + 'static,
{
    pub fn new(
        role: impl Into<String>,
        label: impl Into<String>,
        target_fill: Color,
        duration: Duration,
    ) -> Self {
        Self {
            role: role.into(),
            label: label.into(),
            target_fill,
            duration,
            _marker: std::marker::PhantomData,
        }
    }

    fn sync_target(&self, state: &mut TweenState, now: Instant, shell: &mut Shell<'_, Message>) {
        if !state.initialized {
            state.from = self.target_fill;
            state.to = self.target_fill;
            state.started_at = now;
            state.initialized = true;
            return;
        }

        if state.to != self.target_fill {
            state.from = sample_color(*state, now, self.duration);
            state.to = self.target_fill;
            state.started_at = now;
            shell.request_redraw();
        }
    }

    fn as_element(&self, fill: Color) -> Element<'_, Message> {
        let body = column![
            text(self.role.as_str())
                .size(10)
                .color(Color::from_rgb8(168, 198, 181)),
            text(self.label.as_str())
                .size(13)
                .color(Color::from_rgb8(223, 245, 230))
        ]
        .spacing(4);

        button(body)
            .padding([8, 10])
            .width(Length::Shrink)
            .style(move |_theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(fill)),
                text_color: Color::from_rgb8(223, 245, 230),
                border: iced::border::rounded(10)
                    .color(Color::from_rgb8(58, 122, 86))
                    .width(1.0),
                ..Default::default()
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

        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if is_animating(*state, *now, self.duration) {
                shell.request_redraw_at(RedrawRequest::At(*now + FRAME_DURATION));
            } else if state.from != state.to {
                state.from = state.to;
            }
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

impl<'a, Message> From<AnimatedStepNode<Message>> for Element<'a, Message>
where
    Message: Clone + 'static,
{
    fn from(widget: AnimatedStepNode<Message>) -> Self {
        Element::new(widget)
    }
}
