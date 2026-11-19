//! ReCord's CRT layer: scanlines, film grain, a vignette and the corner
//! brackets, drawn over the window. The canvas takes no pointer events, so
//! the window under it behaves as if it weren't there.

use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
    time::{Duration, Instant},
};

use iced::{
    Color, Point, Rectangle, Size, Theme, mouse,
    widget::canvas::{self, Action, Frame, Image},
    window,
};

/// How long a beam's pass and a flicker's cycle take, in seconds.
const CYCLE: f32 = 9.0;
/// How often the layer asks for a frame while its beam or flicker moves.
const FRAME: Duration = Duration::from_millis(100);

/// How strong each part of the layer is, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shade {
    pub scanlines: f32,
    pub grain: f32,
    pub vignette: f32,
    pub frame: f32,
    pub beam: f32,
    pub flicker: f32,
    pub line: Color,
    pub accent: Color,
}

impl Shade {
    /// Whether there is anything to draw.
    pub fn visible(&self) -> bool {
        self.scanlines > 0.0
            || self.grain > 0.0
            || self.vignette > 0.0
            || self.frame > 0.0
            || self.beam > 0.0
            || self.flicker > 0.0
    }

    /// Whether the beam or the flicker moves, which makes the layer ask for
    /// its own frames rather than the window's clock.
    fn animated(&self) -> bool {
        self.beam > 0.0 || self.flicker > 0.0
    }
}

/// The pictures the layer tiles or stretches, made once, and when its
/// beam and flicker started.
#[derive(Default)]
pub struct Mask {
    grain: OnceLock<Image>,
    vignette: OnceLock<Image>,
    started: Cell<Option<Instant>>,
    still: RefCell<Option<Still>>,
}

/// The layer's parts that stand still, and what they were made from: the
/// scanlines, the vignette, the grain and the corner brackets, drawn again
/// only when the size, a level or an ink changes.
struct Still {
    key: StillKey,
    cache: canvas::Cache,
}

#[derive(Clone, Copy, PartialEq)]
struct StillKey {
    width: u32,
    height: u32,
    scanlines: u32,
    grain: u32,
    vignette: u32,
    frame: u32,
    line: [u8; 4],
    accent: [u8; 4],
}

impl StillKey {
    fn of(shade: &Shade, size: Size) -> Self {
        Self {
            width: size.width.to_bits(),
            height: size.height.to_bits(),
            scanlines: shade.scanlines.to_bits(),
            grain: shade.grain.to_bits(),
            vignette: shade.vignette.to_bits(),
            frame: shade.frame.to_bits(),
            line: shade.line.into_rgba8(),
            accent: shade.accent.into_rgba8(),
        }
    }
}

impl Mask {
    /// How far through the beam's and flicker's cycles this frame is.
    fn phase(&self) -> f32 {
        let now = Instant::now();
        let start = *self.started.get().get_or_insert(now);
        self.started.set(Some(start));
        (now.saturating_duration_since(start).as_secs_f32() / CYCLE).fract()
    }

    fn grain(&self) -> &Image {
        self.grain.get_or_init(|| {
            const SIDE: u32 = 128;
            let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
            let mut seed = 0x9e37_79b9u32;
            for _ in 0..SIDE * SIDE {
                // A cheap deterministic noise, one byte per pixel.
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let value = (seed & 0xff) as u8;
                pixels.extend_from_slice(&[value, value, value, 22]);
            }
            Image::new(iced::widget::image::Handle::from_rgba(SIDE, SIDE, pixels))
                .filter_method(iced::widget::image::FilterMethod::Nearest)
        })
    }

    fn vignette(&self) -> &Image {
        self.vignette.get_or_init(|| {
            const SIDE: u32 = 256;
            let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
            let half = SIDE as f32 / 2.0;
            for y in 0..SIDE {
                for x in 0..SIDE {
                    let dx = (x as f32 + 0.5 - half) / half;
                    let dy = (y as f32 + 0.5 - half) / half;
                    let distance = (dx * dx + dy * dy).sqrt().min(1.0);
                    let alpha = (distance.powf(2.4) * 150.0) as u8;
                    pixels.extend_from_slice(&[0, 0, 0, alpha]);
                }
            }
            Image::new(iced::widget::image::Handle::from_rgba(SIDE, SIDE, pixels))
        })
    }
}

impl<Message> canvas::Program<Message> for Shade {
    type State = Mask;

    /// A moving beam or flicker asks for its next frame itself: the window
    /// draws the layer as the frames fall due, with no message and no view
    /// built again for each one.
    fn update(
        &self,
        state: &mut Mask,
        event: &canvas::Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if !self.animated() {
            return None;
        }
        let iced::Event::Window(window::Event::RedrawRequested(now)) = event else {
            return None;
        };
        if state.started.get().is_none() {
            state.started.set(Some(*now));
        }
        Some(Action::request_redraw_at(*now + FRAME))
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = bounds.size();
        let key = StillKey::of(self, size);
        {
            let mut still = state.still.borrow_mut();
            if still.as_ref().is_none_or(|still| still.key != key) {
                *still = Some(Still {
                    key,
                    cache: canvas::Cache::new(),
                });
            }
        }
        let mut geometries = Vec::with_capacity(2);
        if let Some(still) = state.still.borrow().as_ref() {
            geometries.push(still.cache.draw(renderer, size, |frame| {
                draw_still(self, state, frame, size);
            }));
        }

        // Only the beam and the flicker move: they're made for this frame,
        // over the parts kept above.
        if self.animated() {
            let phase = state.phase();
            let mut frame = Frame::new(renderer, size);
            if self.flicker > 0.0 {
                let wave = ((phase * std::f32::consts::TAU * 1.5).sin() * 0.5 + 0.5) * 0.035;
                frame.fill_rectangle(
                    Point::ORIGIN,
                    size,
                    Color::from_rgba(1.0, 1.0, 1.0, self.flicker * wave),
                );
            }
            if self.beam > 0.0 {
                let band = 120.0f32.min(size.height / 3.0);
                let travel = size.height + band;
                let y = phase * travel - band;
                let strips = 30;
                for step in 0..strips {
                    let offset = step as f32 / strips as f32;
                    let edge = 1.0 - (offset * 2.0 - 1.0).abs();
                    let edge = edge * edge;
                    frame.fill_rectangle(
                        Point::new(0.0, y + offset * band),
                        Size::new(size.width, band / strips as f32 + 1.0),
                        Color {
                            a: self.beam * edge * 0.045,
                            ..self.accent
                        },
                    );
                }
            }
            geometries.push(frame.into_geometry());
        }
        geometries
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        // Let every click, hover and scroll through to the window under it.
        mouse::Interaction::None
    }
}

/// The layer's parts that stand still: its scanlines, its vignette, its
/// grain and its corner brackets.
fn draw_still(shade: &Shade, state: &Mask, frame: &mut Frame, size: Size) {
    if shade.scanlines > 0.0 {
        let ink = Color::from_rgba(0.0, 0.0, 0.0, shade.scanlines * 0.5);
        let mut y = 1.0;
        while y < size.height {
            frame.fill_rectangle(Point::new(0.0, y), Size::new(size.width, 1.0), ink);
            y += 3.0;
        }
    }
    if shade.vignette > 0.0 {
        frame.draw_image(
            Rectangle::new(Point::ORIGIN, size),
            state
                .vignette()
                .clone()
                .opacity(shade.vignette.min(1.0) * 0.9),
        );
    }
    if shade.grain > 0.0 {
        let tile = state.grain().clone().opacity(shade.grain.min(1.0) * 0.45);
        let side: f32 = 128.0;
        let mut y = 0.0;
        while y < size.height {
            let mut x = 0.0;
            while x < size.width {
                frame.draw_image(
                    Rectangle::new(
                        Point::new(x, y),
                        Size::new(side.min(size.width - x), side.min(size.height - y)),
                    ),
                    tile.clone(),
                );
                x += side;
            }
            y += side;
        }
    }
    if shade.frame > 0.0 {
        let alpha = shade.frame.clamp(0.0, 1.0) * 0.8;
        let ink = Color {
            a: alpha,
            ..shade.accent
        };
        let soft = Color {
            a: alpha * 0.55,
            ..shade.line
        };
        let inset = 12.0;
        let length = 18.0;
        let thickness = 2.0;
        for (x, y, dx, dy) in [
            (inset, inset, 1.0, 1.0),
            (size.width - inset, inset, -1.0, 1.0),
            (inset, size.height - inset, 1.0, -1.0),
            (size.width - inset, size.height - inset, -1.0, -1.0),
        ] {
            let right = if dx > 0.0 { x } else { x - length };
            frame.fill_rectangle(
                Point::new(right, y - thickness / 2.0),
                Size::new(length, thickness),
                ink,
            );
            let down = if dy > 0.0 { y } else { y - length };
            frame.fill_rectangle(
                Point::new(x - thickness / 2.0, down),
                Size::new(thickness, length),
                ink,
            );
            // A quieter inner pair, as ReCord's frame has.
            let right = if dx > 0.0 {
                x + 4.0
            } else {
                x - 4.0 - length / 2.0
            };
            frame.fill_rectangle(
                Point::new(right, y + 5.0),
                Size::new(length / 2.0, 1.0),
                soft,
            );
            let down = if dy > 0.0 {
                y + 4.0
            } else {
                y - 4.0 - length / 2.0
            };
            frame.fill_rectangle(
                Point::new(x + 5.0, down),
                Size::new(1.0, length / 2.0),
                soft,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use iced::widget::canvas::Program as _;

    use super::*;

    fn shade(beam: f32, flicker: f32) -> Shade {
        Shade {
            scanlines: 0.0,
            grain: 0.0,
            vignette: 0.0,
            frame: 0.0,
            beam,
            flicker,
            line: Color::BLACK,
            accent: Color::WHITE,
        }
    }

    #[test]
    fn a_moving_layer_asks_for_its_next_frame_without_a_message() {
        let mut state = Mask::default();
        let now = Instant::now();
        let action: canvas::Action<()> = shade(0.5, 0.0)
            .update(
                &mut state,
                &iced::Event::Window(window::Event::RedrawRequested(now)),
                Rectangle::with_size(Size::new(100.0, 100.0)),
                mouse::Cursor::Unavailable,
            )
            .expect("a frame asked for");
        let (message, redraw, _) = action.into_inner();
        assert!(message.is_none(), "no view built again");
        assert_eq!(redraw, window::RedrawRequest::At(now + FRAME));
        assert_eq!(state.started.get(), Some(now));
    }

    #[test]
    fn a_still_layer_asks_for_nothing() {
        let mut state = Mask::default();
        let action: Option<canvas::Action<()>> = shade(0.0, 0.0).update(
            &mut state,
            &iced::Event::Window(window::Event::RedrawRequested(Instant::now())),
            Rectangle::with_size(Size::new(100.0, 100.0)),
            mouse::Cursor::Unavailable,
        );
        assert!(action.is_none());
    }

    #[test]
    fn the_phase_is_the_time_since_the_layer_started() {
        let state = Mask::default();
        state
            .started
            .set(Some(Instant::now() - Duration::from_secs_f32(CYCLE / 3.0)));
        assert!((state.phase() - 1.0 / 3.0).abs() < 0.01);
    }
}
