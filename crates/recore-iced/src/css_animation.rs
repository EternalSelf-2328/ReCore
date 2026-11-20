//! A quiet wash over the window for a CSS theme's animated ground. The
//! theme's `@keyframes` give the colour; the canvas asks for its own next
//! frame, so the window above is not rebuilt for every step. It takes no
//! pointer events.

use std::time::{Duration, Instant};

use iced::{
    Color, Point, Rectangle, Size, Theme, mouse,
    widget::canvas::{self, Action, Frame, Program},
};
use recore_css::Animation;

/// How often a moving ground is repainted.
const FRAME: Duration = Duration::from_millis(50);

/// The animated ground, with the window's clock started when it first drew.
pub struct Ground {
    pub animations: Vec<Animation>,
    pub started: Instant,
}

impl<Message> Program<Message> for Ground {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if self.animations.is_empty() {
            return None;
        }
        let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event else {
            return None;
        };
        Some(Action::request_redraw_at(*now + FRAME))
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let Some(colour) = colour(&self.animations, self.started.elapsed().as_secs_f32()) else {
            return Vec::new();
        };
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            Size::new(bounds.width, bounds.height),
            colour,
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        // The chat, rail and every row below keep their presses and hovers.
        mouse::Interaction::None
    }
}

/// The current wash colour for `animations` at `elapsed` seconds. The first
/// animation with a colour wins; backgrounds outrank text colours. The
/// opacity is kept quiet so panels and text remain legible.
pub fn colour(animations: &[Animation], elapsed: f32) -> Option<Color> {
    animations.iter().find_map(|animation| {
        let frame = animation.at(elapsed)?;
        let colour = frame.background.or(frame.color)?;
        let mut colour =
            Color::from_rgba8(colour.r, colour.g, colour.b, f32::from(colour.a) / 255.0);
        colour.a *= (0.08 * frame.opacity.unwrap_or(1.0)).clamp(0.0, 0.12);
        Some(colour)
    })
}
