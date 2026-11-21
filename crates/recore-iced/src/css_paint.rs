//! The chosen CSS theme's page paint list, drawn once by the kit's software
//! canvas. iced then shows the image under the native panels, so a theme's
//! gradients, shadows and `::before` decorations reach the window without a
//! GPU pass of its own.

use iced::advanced::image::Handle;
use recore_css::Paint;
use recore_css::paint_engine::{
    AlphaColor, PaintOp, Srgb,
    css::{
        property::{ColorStop, Gradient, GradientKind},
        value::{ColorValue, LengthPercentage},
    },
};
use recore_iced_kit::canvas::{self, Canvas, Ink};

const VIEWPORT: (f32, f32) = (1280.0, 720.0);

/// Paints `paint` at the window's `width` by `height`, scaled from the
/// semantic layer's reference viewport.
pub fn handle(paint: &Paint, width: u32, height: u32) -> Handle {
    let mut canvas = Canvas::new(width.max(1), height.max(1));
    let sx = canvas.width as f32 / VIEWPORT.0;
    let sy = canvas.height as f32 / VIEWPORT.1;
    for op in &paint.ops {
        draw(&mut canvas, op, sx, sy);
    }
    canvas::handle(canvas)
}

fn draw(canvas: &mut Canvas, op: &PaintOp, sx: f32, sy: f32) {
    match op {
        PaintOp::FillRect {
            x,
            y,
            width,
            height,
            radius,
            color,
        } => rounded(canvas, *x, *y, *width, *height, radius, ink(*color), sx, sy),
        PaintOp::Gradient {
            x,
            y,
            width,
            height,
            radius,
            gradient,
            linear,
            ..
        } => {
            let Some(stops) = stops(gradient) else {
                return;
            };
            let Some((start, end)) = line(*x, *y, *width, *height, gradient, linear.as_ref())
            else {
                if let Some((_, color)) = stops.first() {
                    rounded(canvas, *x, *y, *width, *height, radius, *color, sx, sy);
                }
                return;
            };
            let (x0, y0, x1, y1) = bounds(*x, *y, *width, *height, sx, sy);
            let line = (end.0 - start.0, end.1 - start.1);
            let length = line.0 * line.0 + line.1 * line.1;
            if length <= f32::EPSILON {
                return;
            }
            for py in y0..y1 {
                for px in x0..x1 {
                    let at = (px as f32 / sx, py as f32 / sy);
                    let t = (((at.0 - start.0) * line.0 + (at.1 - start.1) * line.1) / length)
                        .clamp(0.0, 1.0);
                    let color = sample(&stops, t);
                    canvas.over(px, py, color, 1.0);
                }
            }
        }
        PaintOp::Shadow {
            x,
            y,
            width,
            height,
            radius,
            color,
            offset_x,
            offset_y,
            blur,
            ..
        } => {
            let ink = ink(*color);
            let steps = 3;
            for step in 0..steps {
                let grow = blur * (step as f32 + 1.0) / steps as f32;
                let alpha = ink.1 / steps as f32;
                rounded(
                    canvas,
                    x + offset_x - grow,
                    y + offset_y - grow,
                    width + grow * 2.0,
                    height + grow * 2.0,
                    radius,
                    Ink(ink.0, alpha),
                    sx,
                    sy,
                );
            }
        }
        PaintOp::Border {
            x,
            y,
            width,
            height,
            sides,
            ..
        } => {
            for (index, side) in sides.iter().enumerate() {
                if side.width <= 0.0
                    || matches!(
                        side.style,
                        recore_css::paint_engine::css::property::BorderStyle::None
                    )
                {
                    continue;
                }
                let color = ink(side.color);
                if color.1 <= 0.0 {
                    continue;
                }
                let (bx, by, bw, bh) = match index {
                    0 => (*x, *y, *width, side.width),
                    1 => (*x + *width - side.width, *y, side.width, *height),
                    2 => (*x, *y + *height - side.width, *width, side.width),
                    _ => (*x, *y, side.width, *height),
                };
                rounded(canvas, bx, by, bw, bh, &[0.0; 8], color, sx, sy);
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments, reason = "a paint box's fields")]
fn rounded(
    canvas: &mut Canvas,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: &[f32; 8],
    ink: Ink,
    sx: f32,
    sy: f32,
) {
    if ink.1 <= 0.0 || width <= 0.0 || height <= 0.0 {
        return;
    }
    let radius = (radius[0].max(radius[2]).max(radius[4]).max(radius[6])) * sx.min(sy);
    canvas.rounded(x * sx, y * sy, width * sx, height * sy, radius, ink);
}

fn bounds(x: f32, y: f32, width: f32, height: f32, sx: f32, sy: f32) -> (u32, u32, u32, u32) {
    (
        (x * sx).floor().max(0.0) as u32,
        (y * sy).floor().max(0.0) as u32,
        ((x + width) * sx).ceil().min((sx * VIEWPORT.0).max(0.0)) as u32,
        ((y + height) * sy).ceil().min((sy * VIEWPORT.1).max(0.0)) as u32,
    )
}

fn ink(color: AlphaColor<Srgb>) -> Ink {
    let [r, g, b, a] = color.components;
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    Ink(
        (channel(r) << 16) | (channel(g) << 8) | channel(b),
        a.clamp(0.0, 1.0),
    )
}

fn stops(gradient: &Gradient) -> Option<Vec<(f32, Ink)>> {
    let mut points: Vec<(Option<f32>, Ink)> = gradient
        .stops
        .iter()
        .filter_map(|stop| Some((position(stop), absolute(stop.color)?)))
        .collect();
    if points.is_empty() {
        return None;
    }
    let known: Vec<usize> = points
        .iter()
        .enumerate()
        .filter_map(|(index, (at, _))| at.is_some().then_some(index))
        .collect();
    if known.is_empty() {
        let last = points.len().saturating_sub(1).max(1);
        for (index, (at, _)) in points.iter_mut().enumerate() {
            *at = Some(index as f32 / last as f32);
        }
    } else {
        let first = known[0];
        let last = *known.last().unwrap_or(&first);
        for (at, _) in points.iter_mut().take(first) {
            *at = Some(0.0);
        }
        for (at, _) in points.iter_mut().skip(last + 1) {
            *at = Some(1.0);
        }
        for pair in known.windows(2) {
            let (start, start_at) = (pair[0], points[pair[0]].0.unwrap_or(0.0));
            let (end, end_at) = (pair[1], points[pair[1]].0.unwrap_or(1.0));
            let gap = (end - start) as f32;
            for (index, point) in points.iter_mut().enumerate().take(end).skip(start + 1) {
                let share = (index - start) as f32 / gap;
                point.0 = Some(start_at + (end_at - start_at) * share);
            }
        }
    }
    let mut previous = 0.0;
    Some(
        points
            .into_iter()
            .map(|(at, color)| {
                let at = at.unwrap_or(previous).clamp(previous, 1.0);
                previous = at;
                (at, color)
            })
            .collect(),
    )
}

fn position(stop: &ColorStop) -> Option<f32> {
    match stop.position.as_ref() {
        Some(LengthPercentage::Percent(value)) => Some(*value),
        _ => None,
    }
}
fn absolute(color: ColorValue) -> Option<Ink> {
    match color {
        ColorValue::Absolute(color) => Some(ink(color)),
        ColorValue::LightDark(a, _) => Some(ink(a)),
        _ => None,
    }
}

fn sample(stops: &[(f32, Ink)], t: f32) -> Ink {
    if stops.is_empty() {
        return Ink(0, 0.0);
    }
    if t <= stops[0].0 {
        return stops[0].1;
    }
    for pair in stops.windows(2) {
        let (start, first) = pair[0];
        let (end, second) = pair[1];
        if t <= end {
            let span = (end - start).max(f32::EPSILON);
            return mix(first, second, (t - start) / span);
        }
    }
    stops[stops.len() - 1].1
}

fn mix(first: Ink, second: Ink, amount: f32) -> Ink {
    let channel = |shift: u32| {
        let a = ((first.0 >> shift) & 0xff) as f32;
        let b = ((second.0 >> shift) & 0xff) as f32;
        ((a + (b - a) * amount).round() as u32) << shift
    };
    Ink(
        channel(16) | channel(8) | channel(0),
        first.1 + (second.1 - first.1) * amount,
    )
}

fn line(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    gradient: &Gradient,
    linear: Option<&recore_css::paint_engine::paint::LinearGeom>,
) -> Option<((f32, f32), (f32, f32))> {
    if let Some(linear) = linear {
        return Some((
            (x + linear.start[0], y + linear.start[1]),
            (x + linear.end[0], y + linear.end[1]),
        ));
    }
    let GradientKind::Linear(angle) = &gradient.kind else {
        return None;
    };
    let (cx, cy) = (x + width / 2.0, y + height / 2.0);
    let radians = (angle.0 - 90.0).to_radians();
    let (sin, cos) = radians.sin_cos();
    let length = (width * cos.abs() + height * sin.abs()).max(1.0);
    Some((
        (cx - cos * length / 2.0, cy - sin * length / 2.0),
        (cx + cos * length / 2.0, cy + sin * length / 2.0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use recore_css::paint_engine::css::{
        property::{ColorStop, GradientKind},
        value::Angle,
    };

    fn stop(red: f32) -> ColorStop {
        ColorStop {
            color: ColorValue::Absolute(AlphaColor::new([red, 0.0, 0.0, 1.0])),
            position: None,
        }
    }

    #[test]
    fn unpositioned_stops_become_finite_positions() {
        let gradient = Gradient {
            kind: GradientKind::Linear(Angle(180.0)),
            repeating: false,
            stops: vec![stop(1.0), stop(0.5), stop(0.0)],
            hints: Vec::new(),
        };
        let stops = stops(&gradient).expect("three stops");
        assert!(stops.iter().all(|(at, _)| at.is_finite()));
        assert_eq!(stops.first().unwrap().0, 0.0);
        assert_eq!(stops.last().unwrap().0, 1.0);
    }
}
