//! A picture under the pointer's wheel and drag: zooming in and out around
//! its middle and moving it while it's larger than its box, both kept inside
//! it. The picture itself is untouched, so a moving one keeps moving; the
//! renderer scales and clips its drawing. A new picture (a `key` of its own)
//! starts unzoomed.

use iced::{
    Element, Length, Point, Rectangle, Size, Transformation, Vector,
    advanced::{
        Layout, Widget, layout, mouse, renderer,
        widget::{Tree, tree},
    },
};

use super::Renderer;

/// How far it zooms: 1x to this.
const MOST: f32 = 6.0;
/// One wheel line's zoom, and one pixel's.
const LINE: f32 = 0.12;
const PIXEL: f32 = 0.0015;

/// `content`, fitted to `size`, under the pointer's zoom and drag; `key`
/// names the picture, so another one starts unzoomed.
pub struct Zoom<'a, M> {
    content: Element<'a, M>,
    size: Size,
    key: String,
}

impl<'a, M> Zoom<'a, M> {
    pub fn new(content: impl Into<Element<'a, M>>, size: Size, key: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size,
            key: key.into(),
        }
    }
}

struct State {
    zoom: f32,
    pan: Vector,
    /// Where the pointer held it from, while it does.
    from: Option<Point>,
    key: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vector::ZERO,
            from: None,
            key: String::new(),
        }
    }
}

impl State {
    /// Holds the picture inside its box: what's larger than the box moves
    /// no further than its edge.
    fn held(&mut self, size: Size) {
        let room = |extent: f32| (extent * (self.zoom - 1.0) / 2.0).max(0.0);
        self.pan = Vector::new(
            self.pan.x.clamp(-room(size.width), room(size.width)),
            self.pan.y.clamp(-room(size.height), room(size.height)),
        );
    }
}

impl<M> Widget<M, iced::Theme, Renderer> for Zoom<'_, M> {
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.size.width),
            Length::Fixed(self.size.height),
        )
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
        let state = tree.state.downcast_mut::<State>();
        if state.key != self.key {
            *state = State {
                key: self.key.clone(),
                ..State::default()
            };
        }
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let node = self.content.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, self.size)
                .max_width(self.size.width)
                .max_height(self.size.height),
        );
        layout::Node::with_children(self.size, vec![node])
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let state = tree.state.downcast_mut::<State>();
        if state.key != self.key {
            let key = self.key.clone();
            *state = State {
                key,
                ..State::default()
            };
        }
        match event {
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let step = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * LINE,
                    mouse::ScrollDelta::Pixels { y, .. } => y * PIXEL,
                };
                state.zoom = (state.zoom * (1.0 + step)).clamp(1.0, MOST);
                if state.zoom == 1.0 {
                    state.pan = Vector::ZERO;
                }
                state.held(self.size);
                shell.capture_event();
                shell.request_redraw();
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(bounds) =>
            {
                state.from = cursor.position();
                shell.capture_event();
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.from.take().is_some() {
                    shell.capture_event();
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let (Some(from), Some(to)) = (state.from, cursor.position()) {
                    state.pan += to - from;
                    state.from = Some(to);
                    state.held(self.size);
                    shell.capture_event();
                    shell.request_redraw();
                }
            }
            _ => {}
        }
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout.children().next().unwrap_or(layout),
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
        renderer: &mut Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let state = tree.state.downcast_ref::<State>();
        let child = layout.children().next().unwrap_or(layout);
        let content = move |renderer: &mut Renderer| {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                child,
                cursor,
                viewport,
            );
        };
        if state.zoom <= 1.0 && state.pan == Vector::ZERO {
            content(renderer);
            return;
        }
        let (x, y) = (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        let transformation = Transformation::translate(x + state.pan.x, y + state.pan.y)
            * Transformation::scale(state.zoom)
            * Transformation::translate(-x, -y);
        renderer.with_layer(bounds, |renderer| {
            renderer.with_transformation(transformation, |renderer| content(renderer));
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(layout.bounds()) {
            let state = tree.state.downcast_ref::<State>();
            if state.from.is_some() {
                return mouse::Interaction::Grabbing;
            }
            if state.zoom > 1.0 {
                return mouse::Interaction::Grab;
            }
            return mouse::Interaction::ZoomIn;
        }
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout.children().next().unwrap_or(layout),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, M, iced::Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout.children().next().unwrap_or(layout),
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<Zoom<'a, M>> for Element<'a, M> {
    fn from(zoom: Zoom<'a, M>) -> Self {
        Element::new(zoom)
    }
}
