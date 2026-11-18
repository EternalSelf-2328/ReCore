//! ReCord's CRT layer: scanlines, film grain, a vignette and the corner
//! brackets, drawn over the window. The canvas takes no pointer events, so
//! the window under it behaves as if it weren't there.

use std::sync::OnceLock;

use iced::{
    Color, Point, Rectangle, Size, Theme, mouse,
    widget::canvas::{self, Frame, Image},
};

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
    /// How far through the beam's and flicker's cycles this frame is.
    pub phase: f32,
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

    /// The layer at a moment of its cycle: 0 at the start, 1 after 9 s.
    pub fn at_phase(mut self, phase: f32) -> Self {
        self.phase = phase.fract();
        self
    }
}

/// The pictures the layer tiles or stretches, made once.
#[derive(Default)]
pub struct Mask {
    grain: OnceLock<Image>,
    vignette: OnceLock<Image>,
}

impl Mask {
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

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = bounds.size();
        let mut frame = Frame::new(renderer, size);

        if self.scanlines > 0.0 {
            let ink = Color::from_rgba(0.0, 0.0, 0.0, self.scanlines * 0.5);
            let mut y = 1.0;
            while y < size.height {
                frame.fill_rectangle(Point::new(0.0, y), Size::new(size.width, 1.0), ink);
                y += 3.0;
            }
        }

        if self.vignette > 0.0 {
            frame.draw_image(
                Rectangle::new(Point::ORIGIN, size),
                state
                    .vignette()
                    .clone()
                    .opacity(self.vignette.min(1.0) * 0.9),
            );
        }

        if self.grain > 0.0 {
            let tile = state.grain().clone().opacity(self.grain.min(1.0) * 0.45);
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

        if self.frame > 0.0 {
            let alpha = self.frame.clamp(0.0, 1.0) * 0.8;
            let ink = Color {
                a: alpha,
                ..self.accent
            };
            let soft = Color {
                a: alpha * 0.55,
                ..self.line
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

        if self.flicker > 0.0 {
            let wave = ((self.phase * std::f32::consts::TAU * 1.5).sin() * 0.5 + 0.5) * 0.035;
            frame.fill_rectangle(
                Point::ORIGIN,
                size,
                Color::from_rgba(1.0, 1.0, 1.0, self.flicker * wave),
            );
        }

        if self.beam > 0.0 {
            let band = 120.0f32.min(size.height / 3.0);
            let travel = size.height + band;
            let y = self.phase * travel - band;
            let strips = 30;
            for step in 0..strips {
                let offset = step as f32 / strips as f32;
                let edge = (1.0 - (offset * 2.0 - 1.0).abs()).powf(2.0);
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

        vec![frame.into_geometry()]
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
