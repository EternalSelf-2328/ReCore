//! The chosen CSS theme's page paint list, drawn once by style-engine's
//! tiny-skia sink. iced then shows the image under the native panels, so the
//! whole display list reaches the window: colours, every gradient kind,
//! background images, shadows, clips, transforms, opacity, filters and blend
//! modes. The native panels themselves keep their own paint path.

use iced::advanced::image::Handle;
use recore_css::Paint;
use recore_iced_kit::canvas::{self, Canvas};

/// The logical viewport the semantic pass lays a page out at.
const VIEWPORT: (f32, f32) = (1280.0, 720.0);

/// Paints `paint` at the window's `width` by `height`, scaled from the
/// semantic layer's reference viewport.
pub fn handle(paint: &Paint, width: u32, height: u32) -> Handle {
    let width = width.max(1);
    let height = height.max(1);
    let scale = width as f32 / VIEWPORT.0;
    let pixmap = style_engine_tiny::render(paint, width, height, [0.0; 4], scale);
    let mut canvas = Canvas::new(width, height);
    canvas.pixels = pixmap.take();
    // tiny-skia stores straight RGBA; the kit's canvas is premultiplied BGRA
    // until `canvas::handle` turns it into the renderer's format.
    for pixel in canvas.pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    canvas::handle(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(paint: &Paint) -> Vec<u8> {
        let Handle::Rgba {
            width,
            height,
            pixels,
            ..
        } = self::handle(paint, 96, 54)
        else {
            panic!("a page is drawn into pixels");
        };
        assert_eq!((width, height), (96, 54));
        pixels.as_ref().to_vec()
    }

    #[test]
    fn a_page_with_no_paint_is_a_transparent_picture() {
        let pixels = pixels(&Paint::default());
        assert!(pixels.iter().all(|channel| *channel == 0));
    }

    #[test]
    fn a_css_pages_radial_gradient_reaches_the_picture() {
        let theme = recore_css::load_str(
            "body { background-color: #101010; background-image: radial-gradient(#ff0000, #0000ff); }",
            None,
            &recore_css::Options {
                paint: true,
                ..recore_css::Options::default()
            },
        )
        .expect("readable CSS");
        let paint = theme.paint.expect("a page to paint");
        let pixels = pixels(&paint);
        let at = |x: usize, y: usize| pixels[(y * 96 + x) * 4..][..4].to_vec();
        assert!(at(48, 27)[3] > 0, "the centre is painted");
        assert_ne!(at(48, 27), at(0, 0), "the radial gradient has two ends");
    }
}
