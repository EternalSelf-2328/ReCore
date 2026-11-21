//! The colours a Discord theme's CSS names, read into RGBA without pulling in
//! a browser engine. Hex, `rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`,
//! `oklab()`, `oklch()`, named colours, `transparent`, `color-mix()` and
//! `light-dark()` are understood; anything else is left to the caller's
//! fallback.

/// A colour with straight (not premultiplied) alpha, as CSS works them out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[allow(clippy::self_named_constructors, reason = "rgba is the CSS name")]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// The colour's brightness, 0 (black) to 1 (white), as WCAG weights it.
    pub fn luminance(self) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// Whether the colour reads dark on a screen.
    pub fn is_dark(self) -> bool {
        self.luminance() < 0.5
    }

    /// `self` over `background`, as a browser paints a translucent layer.
    pub fn over(self, background: Self) -> Self {
        if self.a == 255 {
            return self;
        }
        if self.a == 0 {
            return background;
        }
        let source = f32::from(self.a) / 255.0;
        let back = f32::from(background.a) / 255.0;
        let alpha = source + back * (1.0 - source);
        if alpha <= f32::EPSILON {
            return Self::TRANSPARENT;
        }
        let mix = |front: u8, behind: u8| {
            let value =
                (f32::from(front) * source + f32::from(behind) * back * (1.0 - source)) / alpha;
            value.round().clamp(0.0, 255.0) as u8
        };
        Self {
            r: mix(self.r, background.r),
            g: mix(self.g, background.g),
            b: mix(self.b, background.b),
            a: (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        }
    }

    /// The WCAG contrast ratio between two colours, for picking text that reads.
    pub fn contrast(self, other: Self) -> f32 {
        let (a, b) = (self.luminance(), other.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Black or white, whichever reads better on this colour.
    pub fn ink(self) -> Self {
        if self.luminance() > 0.45 {
            Self::rgb(0, 0, 0)
        } else {
            Self::rgb(255, 255, 255)
        }
    }

    /// `self` with `amount` (0..1) of `other` mixed in, as CSS `color-mix`
    /// mixes in sRGB with premultiplied alpha.
    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        if amount <= 0.0 {
            return self;
        }
        if amount >= 1.0 {
            return other;
        }
        // Opaque colours, as most gradients are, skip the alpha weighting
        // and the division per channel.
        if self.a == 255 && other.a == 255 {
            let channel = |a: u8, b: u8| {
                (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8
            };
            return Self {
                r: channel(self.r, other.r),
                g: channel(self.g, other.g),
                b: channel(self.b, other.b),
                a: 255,
            };
        }
        let first = f32::from(self.a) / 255.0 * (1.0 - amount);
        let second = f32::from(other.a) / 255.0 * amount;
        let total = first + second;
        if total <= f32::EPSILON {
            return Self::TRANSPARENT;
        }
        let channel = |a: u8, b: u8| {
            ((f32::from(a) * first + f32::from(b) * second) / total)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Self {
            r: channel(self.r, other.r),
            g: channel(self.g, other.g),
            b: channel(self.b, other.b),
            a: ((total * 255.0).round().clamp(0.0, 255.0)) as u8,
        }
    }

    /// The colour as `#rrggbb`, or `#rrggbbaa` when it isn't opaque.
    pub fn hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

/// Parses the colour a CSS declaration uses, with `light` choosing which of
/// `light-dark()`'s two colours applies: its first in light mode, its second
/// in dark mode.
pub fn parse(text: &str, light: bool) -> Option<Rgba> {
    parse_depth(text, light, 0)
}

/// How deep a colour expression may nest `color-mix()` or `light-dark()`.
/// A damaged theme can repeat them thousands of times; the window reads it
/// as a colour it can't use instead of running out of stack.
const MAX_DEPTH: usize = 32;

fn parse_depth(text: &str, light: bool, depth: usize) -> Option<Rgba> {
    if depth > MAX_DEPTH {
        return None;
    }
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "transparent" => return Some(Rgba::TRANSPARENT),
        "black" => return Some(Rgba::rgb(0, 0, 0)),
        "white" => return Some(Rgba::rgb(255, 255, 255)),
        _ => {}
    }
    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }
    if lower.starts_with("rgb(") || lower.starts_with("rgba(") {
        let inside = function_body(text)?;
        return parse_rgb(inside);
    }
    if lower.starts_with("hsl(") || lower.starts_with("hsla(") {
        let inside = function_body(text)?;
        return parse_hsl(inside);
    }
    if lower.starts_with("hwb(") {
        let inside = function_body(text)?;
        return parse_hwb(inside);
    }
    if lower.starts_with("oklab(") {
        let inside = function_body(text)?;
        return parse_oklab(inside);
    }
    if lower.starts_with("oklch(") {
        let inside = function_body(text)?;
        return parse_oklch(inside);
    }
    if lower.starts_with("lab(") {
        let inside = function_body(text)?;
        return parse_lab(inside);
    }
    if lower.starts_with("lch(") {
        let inside = function_body(text)?;
        return parse_lch(inside);
    }
    if lower.starts_with("color(") {
        let inside = function_body(text)?;
        return parse_color(inside);
    }
    if lower.starts_with("color-mix(") {
        let inside = function_body(text)?;
        return parse_color_mix(inside, light, depth + 1);
    }
    if lower.starts_with("light-dark(") {
        let inside = function_body(text)?;
        let parts = split_top(inside, ',');
        let chosen = if light { parts.first() } else { parts.get(1) };
        return chosen.and_then(|part| parse_depth(part, light, depth + 1));
    }
    if let Some(rgba) = parse_triplet(&lower) {
        return Some(rgba);
    }
    named(&lower).map(|(r, g, b)| Rgba::rgb(r, g, b))
}

/// `30,30,30` or `30, 30, 30, 0.5`: DiscordRecolor keeps colours as the
/// three numbers a `rgb()` would take, and themes use them through
/// `rgb(var(--name))`.
fn parse_triplet(text: &str) -> Option<Rgba> {
    if !text.contains(',') {
        return None;
    }
    let parts = split_top(text, ',');
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let channel = |text: &str| -> Option<u8> {
        let value: f32 = text.trim().parse().ok()?;
        Some(value.clamp(0.0, 255.0).round() as u8)
    };
    let alpha = match parts.get(3) {
        Some(alpha) => {
            let value: f32 = alpha.trim().parse().ok()?;
            if value <= 1.0 {
                (value.clamp(0.0, 1.0) * 255.0).round() as u8
            } else {
                value.clamp(0.0, 255.0).round() as u8
            }
        }
        None => 255,
    };
    Some(Rgba::rgba(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        alpha,
    ))
}

fn parse_hex(digits: &str) -> Option<Rgba> {
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        3 => {
            let nibble = |shift: u32| ((value >> shift) & 0xf) * 0x11;
            Some(Rgba::rgb(nibble(8) as u8, nibble(4) as u8, nibble(0) as u8))
        }
        4 => {
            let nibble = |shift: u32| ((value >> shift) & 0xf) * 0x11;
            Some(Rgba::rgba(
                nibble(12) as u8,
                nibble(8) as u8,
                nibble(4) as u8,
                nibble(0) as u8,
            ))
        }
        6 => Some(Rgba::rgb(
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )),
        8 => Some(Rgba::rgba(
            ((value >> 24) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )),
        _ => None,
    }
}

/// The text between a function's name and its closing parenthesis.
fn function_body(text: &str) -> Option<&str> {
    let start = text.find('(')?;
    let end = matching_close(text, start)?;
    (end > start).then(|| &text[start + 1..end])
}

/// The closing parenthesis of the one at `open`, ignoring parentheses inside
/// strings.
fn matching_close(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in text.char_indices().skip_while(|(index, _)| *index < open) {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' | '\'' if quote == Some(character) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(character),
            '(' if quote.is_none() => depth += 1,
            ')' if quote.is_none() => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits at `separator`, ignoring separators inside parentheses or strings.
pub fn split_top(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' | '\'' if quote == Some(character) => quote = None,
            '"' | '\'' if quote.is_none() => quote = Some(character),
            '(' if quote.is_none() => depth += 1,
            ')' if quote.is_none() => depth = depth.saturating_sub(1),
            '{' if quote.is_none() => depth += 1,
            '}' if quote.is_none() => depth = depth.saturating_sub(1),
            _ if character == separator && quote.is_none() && depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

/// `255`, `50%` or `1` (alpha), as an rgb component.
fn component(text: &str, percent_is_full: bool) -> Option<f32> {
    let text = text.trim();
    if let Some(percent) = text.strip_suffix('%') {
        let value: f32 = percent.trim().parse().ok()?;
        return Some(if percent_is_full {
            value / 100.0
        } else {
            value * 2.55
        });
    }
    text.parse().ok()
}

fn alpha_component(text: &str) -> Option<u8> {
    let value = component(text, true)?;
    Some((value.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `rgb(1 2 3 / 50%)`, `rgba(1, 2, 3, .5)` and the percentages between.
fn parse_rgb(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = match inside.split_once('/') {
        Some((components, alpha)) => (components.trim(), Some(alpha.trim())),
        None => (inside, None),
    };
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let channel = |text: &str| -> Option<u8> {
        Some(component(text, false)?.clamp(0.0, 255.0).round() as u8)
    };
    let a = match alpha {
        Some(alpha) => alpha_component(alpha)?,
        None if parts.len() >= 4 => alpha_component(parts[3])?,
        None => 255,
    };
    Some(Rgba::rgba(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        a,
    ))
}

/// `hsl(120 100% 25% / 50%)` and the comma syntax.
fn parse_hsl(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let hue = hue(parts[0])?;
    let saturation = percent(parts[1])?;
    let lightness = percent(parts[2])?;
    let a = alpha_value(alpha, parts.get(3).copied())?;
    let (r, g, b) = hsl_rgb(hue, saturation, lightness);
    Some(Rgba::rgba(channel(r), channel(g), channel(b), a))
}

/// `hwb(120 10% 20% / 50%)`.
fn parse_hwb(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let hue = hue(parts[0])?;
    let whiteness = percent(parts[1])?;
    let blackness = percent(parts[2])?;
    let a = alpha_value(alpha, parts.get(3).copied())?;
    let (mut r, mut g, mut b) = hsl_rgb(hue, 1.0, 0.5);
    let sum = whiteness + blackness;
    if sum >= 1.0 {
        let gray = whiteness / sum;
        (r, g, b) = (gray, gray, gray);
    } else {
        let factor = 1.0 - sum;
        r = r * factor + whiteness;
        g = g * factor + whiteness;
        b = b * factor + whiteness;
    }
    Some(Rgba::rgba(channel(r), channel(g), channel(b), a))
}

/// `oklab(0.62 0.04 -0.03 / 50%)`, as Material themes write colours.
fn parse_oklab(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let lightness = number_or_percent(parts[0], 1.0)?;
    let a = signed(parts[1], 0.4)?;
    let b = signed(parts[2], 0.4)?;
    let alpha = alpha_value(alpha, parts.get(3).copied())?;
    Some(oklab_rgba(lightness, a, b, alpha))
}

/// `oklch(0.62 0.12 30 / 50%)`.
fn parse_oklch(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let lightness = number_or_percent(parts[0], 1.0)?;
    let chroma = number_or_percent(parts[1], 0.4)?;
    let hue = hue(parts[2])?;
    let alpha = alpha_value(alpha, parts.get(3).copied())?;
    let (a, b) = (
        chroma * (hue.to_radians()).cos(),
        chroma * (hue.to_radians()).sin(),
    );
    Some(oklab_rgba(lightness, a, b, alpha))
}

fn oklab_rgba(lightness: f32, a: f32, b: f32, alpha: u8) -> Rgba {
    let l = (lightness + 0.396_337_78 * a + 0.215_803_76 * b).powi(3);
    let m = (lightness - 0.105_561_346 * a - 0.063_854_17 * b).powi(3);
    let s = (lightness - 0.089_484_18 * a - 1.291_485_5 * b).powi(3);
    let red = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_93 * s;
    let green = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_4 * s;
    let blue = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;
    Rgba::rgba(
        channel(srgb_encode(red)),
        channel(srgb_encode(green)),
        channel(srgb_encode(blue)),
        alpha,
    )
}

fn srgb_encode(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_decode(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// `lab(53.24 80.09 67.2 / 50%)`, CIELAB with a D65 white point.
fn parse_lab(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let lightness = number_or_percent(parts[0], 100.0)?;
    let a = signed(parts[1], 125.0)?;
    let b = signed(parts[2], 125.0)?;
    let alpha = alpha_value(alpha, parts.get(3).copied())?;
    Some(lab_rgba(lightness, a, b, alpha))
}

/// `lch(53.24 104.55 40 / 50%)`, CIELAB's cylindrical form.
fn parse_lch(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = if components.contains(',') {
        split_top(components, ',')
    } else {
        components.split_whitespace().collect()
    };
    if parts.len() < 3 {
        return None;
    }
    let lightness = number_or_percent(parts[0], 100.0)?;
    let chroma = number_or_percent(parts[1], 150.0)?;
    let hue = hue(parts[2])?;
    let alpha = alpha_value(alpha, parts.get(3).copied())?;
    Some(lab_rgba(
        lightness,
        chroma * hue.to_radians().cos(),
        chroma * hue.to_radians().sin(),
        alpha,
    ))
}

fn lab_rgba(lightness: f32, a: f32, b: f32, alpha: u8) -> Rgba {
    let fy = (lightness + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let finv = |value: f32| {
        if value.powi(3) > 0.008_856 {
            value.powi(3)
        } else {
            (116.0 * value - 16.0) / 903.3
        }
    };
    let x = 0.950_47 * finv(fx);
    let y = finv(fy);
    let z = 1.088_83 * finv(fz);
    let red = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let green = -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z;
    let blue = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    Rgba::rgba(
        channel(srgb_encode(red)),
        channel(srgb_encode(green)),
        channel(srgb_encode(blue)),
        alpha,
    )
}

/// `color(srgb 0.5 0.2 0.1 / 50%)`, `color(display-p3 ...)` and the linear
/// forms.
fn parse_color(inside: &str) -> Option<Rgba> {
    let inside = inside.trim();
    let (components, alpha) = split_alpha(inside);
    let parts = components.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 4 {
        return None;
    }
    let space = parts[0].to_ascii_lowercase();
    let red = component(parts[1], true)?;
    let green = component(parts[2], true)?;
    let blue = component(parts[3], true)?;
    let alpha = alpha_value(alpha, None)?;
    let (red, green, blue) = match space.as_str() {
        "srgb" => (srgb_decode(red), srgb_decode(green), srgb_decode(blue)),
        "srgb-linear" => (red, green, blue),
        "display-p3" => {
            let (red, green, blue) = (srgb_decode(red), srgb_decode(green), srgb_decode(blue));
            let x = 0.486_570_9 * red + 0.265_667_7 * green + 0.198_217_3 * blue;
            let y = 0.228_974_6 * red + 0.691_738_5 * green + 0.079_286_9 * blue;
            let z = 0.045_113_4 * green + 1.043_944_4 * blue;
            (
                3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z,
                -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z,
                0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z,
            )
        }
        _ => (srgb_decode(red), srgb_decode(green), srgb_decode(blue)),
    };
    Some(Rgba::rgba(
        channel(srgb_encode(red)),
        channel(srgb_encode(green)),
        channel(srgb_encode(blue)),
        alpha,
    ))
}

fn split_alpha(inside: &str) -> (&str, Option<&str>) {
    if let Some((components, alpha)) = inside.split_once('/') {
        (components.trim(), Some(alpha.trim()))
    } else {
        (inside, None)
    }
}

fn alpha_value(alpha: Option<&str>, comma_alpha: Option<&str>) -> Option<u8> {
    if let Some(alpha) = alpha {
        return alpha_component(alpha);
    }
    comma_alpha.map(alpha_component).unwrap_or(Some(255))
}

fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn percent(text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(percent) = text.strip_suffix('%') {
        percent
            .trim()
            .parse::<f32>()
            .ok()
            .map(|value| value / 100.0)
    } else {
        text.parse::<f32>().ok().map(|value| value / 100.0)
    }
}

fn number_or_percent(text: &str, full: f32) -> Option<f32> {
    let text = text.trim();
    if let Some(percent) = text.strip_suffix('%') {
        Some(percent.trim().parse::<f32>().ok()? / 100.0 * full)
    } else {
        text.parse::<f32>().ok()
    }
}

fn signed(text: &str, full: f32) -> Option<f32> {
    let text = text.trim();
    if let Some(percent) = text.strip_suffix('%') {
        Some(percent.trim().parse::<f32>().ok()? / 100.0 * full)
    } else {
        text.parse::<f32>().ok()
    }
}

/// A hue in degrees, radians, grades or turns.
fn hue(text: &str) -> Option<f32> {
    let text = text.trim().to_ascii_lowercase();
    let parse = |suffix: &str, scale: f32| {
        text.strip_suffix(suffix)
            .and_then(|value| value.trim().parse::<f32>().ok())
            .map(|value| value * scale)
    };
    parse("deg", 1.0)
        .or_else(|| parse("grad", 0.9))
        .or_else(|| parse("rad", 180.0 / std::f32::consts::PI))
        .or_else(|| parse("turn", 360.0))
        .or_else(|| text.parse::<f32>().ok())
        .map(|value| value.rem_euclid(360.0))
}

fn hsl_rgb(hue: f32, saturation: f32, lightness: f32) -> (f32, f32, f32) {
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = lightness - chroma / 2.0;
    (r + m, g + m, b + m)
}

/// `in srgb, #111 30%, #fff` with any number of colours and percentages, as
/// CSS mixes them. The common sRGB form is exact. `srgb-linear` mixes through
/// linear light; other spaces (`oklab`, `oklch`, `hsl`) are read and mixed in
/// sRGB, which keeps their colours usable without a full colour-management
/// engine.
fn parse_color_mix(inside: &str, light: bool, depth: usize) -> Option<Rgba> {
    let args = split_top(inside, ',');
    if args.len() < 3 {
        return None;
    }
    let space = args[0].trim().to_ascii_lowercase();
    if !space.starts_with("in ") {
        return None;
    }
    let linear = space.contains("srgb-linear");
    let mut colours: Vec<(Rgba, Option<f32>)> = Vec::new();
    for arg in &args[1..] {
        let (colour_text, percentage) = take_percentage(arg);
        let colour = parse_depth(colour_text, light, depth)?;
        colours.push((colour, percentage));
    }
    if colours.len() < 2 {
        return None;
    }
    let specified: f32 = colours.iter().filter_map(|(_, share)| *share).sum();
    let unspecified = colours.iter().filter(|(_, share)| share.is_none()).count();
    let weights: Vec<f32> = if unspecified == 0 {
        if specified <= f32::EPSILON {
            return Some(Rgba::TRANSPARENT);
        }
        colours
            .iter()
            .map(|(_, share)| share.unwrap_or(0.0) / specified)
            .collect()
    } else if specified >= 1.0 {
        let share_of_unspecified = 0.0;
        colours
            .iter()
            .map(|(_, share)| match share {
                Some(share) => share / specified,
                None => share_of_unspecified,
            })
            .collect()
    } else {
        let each = (1.0 - specified.max(0.0)) / unspecified as f32;
        colours
            .iter()
            .map(|(_, share)| share.unwrap_or(each))
            .collect()
    };
    let total: f32 = weights.iter().sum();
    if total <= f32::EPSILON {
        return Some(Rgba::TRANSPARENT);
    }
    // With no unspecified colour, percentages below 100% scale the result's
    // alpha, as CSS color-mix does; above 100% they are normalised back.
    let alpha_multiplier = if unspecified == 0 {
        specified.min(1.0)
    } else {
        1.0
    };
    let decode = |value: u8| {
        let value = f32::from(value) / 255.0;
        if linear {
            if value <= 0.040_45 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        } else {
            value
        }
    };
    let encode = |value: f32| {
        let value = value.clamp(0.0, 1.0);
        let value = if linear {
            if value <= 0.003_130_8 {
                value * 12.92
            } else {
                1.055 * value.powf(1.0 / 2.4) - 0.055
            }
        } else {
            value
        };
        (value * 255.0).round().clamp(0.0, 255.0) as u8
    };
    let mut alpha = 0.0f32;
    let mut channels = [0.0f32; 3];
    for ((colour, _), weight) in colours.iter().zip(&weights) {
        let share = f32::from(colour.a) / 255.0 * weight;
        alpha += share;
        channels[0] += decode(colour.r) * share;
        channels[1] += decode(colour.g) * share;
        channels[2] += decode(colour.b) * share;
    }
    if alpha <= f32::EPSILON {
        return Some(Rgba::TRANSPARENT);
    }
    Some(Rgba::rgba(
        encode(channels[0] / alpha),
        encode(channels[1] / alpha),
        encode(channels[2] / alpha),
        (alpha * alpha_multiplier * 255.0).round().clamp(0.0, 255.0) as u8,
    ))
}

/// Splits a trailing `30%` from a colour, as `color-mix` writes it.
fn take_percentage(text: &str) -> (&str, Option<f32>) {
    let text = text.trim();
    let Some(space) = text.rfind(char::is_whitespace) else {
        return (text, None);
    };
    let (colour, percent) = text.split_at(space);
    let percent = percent.trim();
    match percent.strip_suffix('%') {
        Some(value) => match value.trim().parse::<f32>() {
            Ok(value) => (colour.trim(), Some((value / 100.0).clamp(0.0, 1.0))),
            Err(_) => (text, None),
        },
        None => (text, None),
    }
}

/// CSS's named colours, the ones Discord themes use by name.
fn named(name: &str) -> Option<(u8, u8, u8)> {
    let value = match name {
        "aliceblue" => 0xf0f8ff,
        "antiquewhite" => 0xfaebd7,
        "aqua" | "cyan" => 0x00ffff,
        "aquamarine" => 0x7fffd4,
        "azure" => 0xf0ffff,
        "beige" => 0xf5f5dc,
        "bisque" => 0xffe4c4,
        "blanchedalmond" => 0xffebcd,
        "blue" => 0x0000ff,
        "blueviolet" => 0x8a2be2,
        "brown" => 0xa52a2a,
        "burlywood" => 0xdeb887,
        "cadetblue" => 0x5f9ea0,
        "chartreuse" => 0x7fff00,
        "chocolate" => 0xd2691e,
        "coral" => 0xff7f50,
        "cornflowerblue" => 0x6495ed,
        "cornsilk" => 0xfff8dc,
        "crimson" => 0xdc143c,
        "darkblue" => 0x00008b,
        "darkcyan" => 0x008b8b,
        "darkgoldenrod" => 0xb8860b,
        "darkgray" | "darkgrey" => 0xa9a9a9,
        "darkgreen" => 0x006400,
        "darkkhaki" => 0xbdb76b,
        "darkmagenta" => 0x8b008b,
        "darkolivegreen" => 0x556b2f,
        "darkorange" => 0xff8c00,
        "darkorchid" => 0x9932cc,
        "darkred" => 0x8b0000,
        "darksalmon" => 0xe9967a,
        "darkseagreen" => 0x8fbc8f,
        "darkslateblue" => 0x483d8b,
        "darkslategray" | "darkslategrey" => 0x2f4f4f,
        "darkturquoise" => 0x00ced1,
        "darkviolet" => 0x9400d3,
        "deeppink" => 0xff1493,
        "deepskyblue" => 0x00bfff,
        "dimgray" | "dimgrey" => 0x696969,
        "dodgerblue" => 0x1e90ff,
        "firebrick" => 0xb22222,
        "floralwhite" => 0xfffaf0,
        "forestgreen" => 0x228b22,
        "fuchsia" | "magenta" => 0xff00ff,
        "gainsboro" => 0xdcdcdc,
        "ghostwhite" => 0xf8f8ff,
        "gold" => 0xffd700,
        "goldenrod" => 0xdaa520,
        "gray" | "grey" => 0x808080,
        "green" => 0x008000,
        "greenyellow" => 0xadff2f,
        "honeydew" => 0xf0fff0,
        "hotpink" => 0xff69b4,
        "indianred" => 0xcd5c5c,
        "indigo" => 0x4b0082,
        "ivory" => 0xfffff0,
        "khaki" => 0xf0e68c,
        "lavender" => 0xe6e6fa,
        "lavenderblush" => 0xfff0f5,
        "lawngreen" => 0x7cfc00,
        "lemonchiffon" => 0xfffacd,
        "lightblue" => 0xadd8e6,
        "lightcoral" => 0xf08080,
        "lightcyan" => 0xe0ffff,
        "lightgoldenrodyellow" => 0xfafad2,
        "lightgray" | "lightgrey" => 0xd3d3d3,
        "lightgreen" => 0x90ee90,
        "lightpink" => 0xffb6c1,
        "lightsalmon" => 0xffa07a,
        "lightseagreen" => 0x20b2aa,
        "lightskyblue" => 0x87cefa,
        "lightslategray" | "lightslategrey" => 0x778899,
        "lightsteelblue" => 0xb0c4de,
        "lightyellow" => 0xffffe0,
        "lime" => 0x00ff00,
        "limegreen" => 0x32cd32,
        "linen" => 0xfaf0e6,
        "maroon" => 0x800000,
        "mediumaquamarine" => 0x66cdaa,
        "mediumblue" => 0x0000cd,
        "mediumorchid" => 0xba55d3,
        "mediumpurple" => 0x9370db,
        "mediumseagreen" => 0x3cb371,
        "mediumslateblue" => 0x7b68ee,
        "mediumspringgreen" => 0x00fa9a,
        "mediumturquoise" => 0x48d1cc,
        "mediumvioletred" => 0xc71585,
        "midnightblue" => 0x191970,
        "mintcream" => 0xf5fffa,
        "mistyrose" => 0xffe4e1,
        "moccasin" => 0xffe4b5,
        "navajowhite" => 0xffdead,
        "navy" => 0x000080,
        "oldlace" => 0xfdf5e6,
        "olive" => 0x808000,
        "olivedrab" => 0x6b8e23,
        "orange" => 0xffa500,
        "orangered" => 0xff4500,
        "orchid" => 0xda70d6,
        "palegoldenrod" => 0xeee8aa,
        "palegreen" => 0x98fb98,
        "paleturquoise" => 0xafeeee,
        "palevioletred" => 0xdb7093,
        "papayawhip" => 0xffefd5,
        "peachpuff" => 0xffdab9,
        "peru" => 0xcd853f,
        "pink" => 0xffc0cb,
        "plum" => 0xdda0dd,
        "powderblue" => 0xb0e0e6,
        "purple" => 0x800080,
        "rebeccapurple" => 0x663399,
        "red" => 0xff0000,
        "rosybrown" => 0xbc8f8f,
        "royalblue" => 0x4169e1,
        "saddlebrown" => 0x8b4513,
        "salmon" => 0xfa8072,
        "sandybrown" => 0xf4a460,
        "seagreen" => 0x2e8b57,
        "seashell" => 0xfff5ee,
        "sienna" => 0xa0522d,
        "silver" => 0xc0c0c0,
        "skyblue" => 0x87ceeb,
        "slateblue" => 0x6a5acd,
        "slategray" | "slategrey" => 0x708090,
        "snow" => 0xfffafa,
        "springgreen" => 0x00ff7f,
        "steelblue" => 0x4682b4,
        "tan" => 0xd2b48c,
        "teal" => 0x008080,
        "thistle" => 0xd8bfd8,
        "tomato" => 0xff6347,
        "turquoise" => 0x40e0d0,
        "violet" => 0xee82ee,
        "wheat" => 0xf5deb3,
        "whitesmoke" => 0xf5f5f5,
        "yellow" => 0xffff00,
        "yellowgreen" => 0x9acd32,
        _ => return None,
    };
    Some((
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_come_in_every_length() {
        assert_eq!(parse("#fff", true), Some(Rgba::rgb(255, 255, 255)));
        assert_eq!(parse("#e94221", true), Some(Rgba::rgb(0xe9, 0x42, 0x21)));
        assert_eq!(
            parse("#e9422129", true),
            Some(Rgba::rgba(0xe9, 0x42, 0x21, 0x29))
        );
        assert_eq!(parse("#ggg", true), None);
    }

    #[test]
    fn rgb_and_hsl_take_both_syntaxes() {
        assert_eq!(
            parse("rgb(233, 66, 33)", true),
            Some(Rgba::rgb(233, 66, 33))
        );
        assert_eq!(
            parse("rgba(233 66 33 / 50%)", true),
            Some(Rgba::rgba(233, 66, 33, 128))
        );
        assert_eq!(parse("hsl(0 0% 0%)", true), Some(Rgba::rgb(0, 0, 0)));
        assert_eq!(parse("hsl(0, 100%, 50%)", true), Some(Rgba::rgb(255, 0, 0)));
    }

    #[test]
    fn color_mix_mixes_in_srgb() {
        let mixed = parse("color-mix(in srgb, #e94221 80%, #fff3d6)", true).unwrap();
        assert_eq!(mixed, Rgba::rgb(0xed, 0x65, 0x45));
        let halfway = parse("color-mix(in srgb, white, black)", true).unwrap();
        assert_eq!(halfway, Rgba::rgb(128, 128, 128));
        let clear = parse("color-mix(in srgb, #e94221 34%, transparent)", true).unwrap();
        assert_eq!(clear, Rgba::rgba(0xe9, 0x42, 0x21, 87));
    }

    #[test]
    fn light_dark_picks_by_mode() {
        assert_eq!(
            parse("light-dark(#fff, #000)", true),
            Some(Rgba::rgb(255, 255, 255))
        );
        assert_eq!(
            parse("light-dark(#fff, #000)", false),
            Some(Rgba::rgb(0, 0, 0))
        );
    }

    #[test]
    fn lab_lch_and_colour_spaces_read_as_colours() {
        let lab = parse("lab(53.24 80.09 67.2)", true).unwrap();
        assert!(lab.r > 220 && lab.g < 80 && lab.b < 80, "{lab:?}");
        let lch = parse("lch(53.24 104.55 40)", true).unwrap();
        assert!(lch.r > 220 && lch.g < 90 && lch.b < 90, "{lch:?}");
        let srgb = parse("color(srgb 0.5 0.5 0.5)", true).unwrap();
        assert_eq!(srgb, Rgba::rgb(128, 128, 128));
        let p3 = parse("color(display-p3 1 0 0)", true).unwrap();
        assert!(p3.r > 240 && p3.g < 20 && p3.b < 20, "{p3:?}");
        assert_eq!(
            parse("color(srgb 1 1 1 / 50%)", true),
            Some(Rgba::rgba(255, 255, 255, 128))
        );
    }

    #[test]
    fn oklab_and_oklch_read_as_colours() {
        let lab = parse("oklab(0.6279 0.2249 0.1258)", true).unwrap();
        assert!(lab.r > 200 && lab.g < 100 && lab.b < 100, "{lab:?}");
        let lch = parse("oklch(0.6279 0.2577 29.23)", true).unwrap();
        assert!(lch.r > 200 && lch.g < 120, "{lch:?}");
        assert_eq!(parse("hwb(0 0% 0%)", true), Some(Rgba::rgb(255, 0, 0)));
    }

    #[test]
    fn color_mix_takes_more_than_two_colours() {
        let mixed = parse("color-mix(in srgb, #ff0000, #00ff00, #0000ff)", true).unwrap();
        assert_eq!(mixed, Rgba::rgb(85, 85, 85));
        let weighted = parse(
            "color-mix(in srgb, #ff0000 50%, #00ff00 25%, #0000ff)",
            true,
        )
        .unwrap();
        // The last colour takes the remaining 25%.
        assert_eq!(weighted, Rgba::rgb(128, 64, 64));
        let short = parse("color-mix(in srgb, #ff0000 30%, #0000ff 30%)", true).unwrap();
        assert_eq!(short, Rgba::rgba(128, 0, 128, 153));
    }

    #[test]
    fn mixing_takes_the_share_of_the_other_colour() {
        let mixed = Rgba::rgb(0, 0, 0).mix(Rgba::rgb(255, 255, 255), 0.25);
        assert_eq!(mixed, Rgba::rgb(64, 64, 64));
        let clear = Rgba::rgb(255, 0, 0).mix(Rgba::TRANSPARENT, 0.5);
        assert_eq!(clear, Rgba::rgba(255, 0, 0, 128));
    }

    #[test]
    fn an_over_puts_a_translucent_layer_on_its_ground() {
        let red = Rgba::rgba(255, 0, 0, 128);
        let black = Rgba::rgb(0, 0, 0);
        let mixed = red.over(black);
        assert_eq!((mixed.r, mixed.g, mixed.b), (128, 0, 0));
    }
}
