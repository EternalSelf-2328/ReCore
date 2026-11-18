//! The colours a Discord theme's CSS names, read into RGBA without pulling in
//! a browser engine. Hex, `rgb()`, `rgba()`, `hsl()`, `hsla()`, named colours,
//! `transparent`, `color-mix(in srgb, ...)` and `light-dark()` are understood;
//! anything else is left to the caller's fallback.

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
        let source = u32::from(self.a);
        let rest = 255 - source;
        let mix = |front: u8, back: u8| {
            ((u32::from(front) * source + u32::from(back) * rest + 127) / 255) as u8
        };
        let alpha = source * 255 / 255;
        Self {
            r: mix(self.r, background.r),
            g: mix(self.g, background.g),
            b: mix(self.b, background.b),
            a: u8::try_from(alpha + u32::from(background.a) * rest / 255).unwrap_or(255),
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
        let weight = |color: Self| f32::from(color.a) / 255.0 * amount;
        let (first, second) = (weight(self), weight(other));
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

/// Parses the colour a CSS declaration uses, with `light_dark` choosing which
/// of `light-dark()`'s two colours applies.
pub fn parse(text: &str, light_dark: bool) -> Option<Rgba> {
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
    if lower.starts_with("color-mix(") {
        let inside = function_body(text)?;
        return parse_color_mix(inside, light_dark);
    }
    if lower.starts_with("light-dark(") {
        let inside = function_body(text)?;
        let parts = split_top(inside, ',');
        let chosen = if light_dark {
            parts.first()
        } else {
            parts.get(1)
        };
        return chosen.and_then(|part| parse(part, light_dark));
    }
    named(&lower).map(|(r, g, b)| Rgba::rgb(r, g, b))
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
    let end = text.rfind(')')?;
    (end > start).then(|| &text[start + 1..end])
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
    let hue: f32 = parts[0]
        .trim()
        .trim_end_matches("deg")
        .parse::<f32>()
        .ok()?
        .rem_euclid(360.0);
    let percent = |text: &str| -> Option<f32> {
        let value = if let Some(percent) = text.trim().strip_suffix('%') {
            percent.trim().parse::<f32>().ok()? / 100.0
        } else {
            text.trim().parse::<f32>().ok()?
        };
        Some(value.clamp(0.0, 1.0))
    };
    let saturation = percent(parts[1])?;
    let lightness = percent(parts[2])?;
    let a = match alpha {
        Some(alpha) => alpha_component(alpha)?,
        None if parts.len() >= 4 => alpha_component(parts[3])?,
        None => 255,
    };
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
    let channel = |value: f32| ((value + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Rgba::rgba(channel(r), channel(g), channel(b), a))
}

/// `in srgb, #111 30%, #fff` with one percentage or two, as CSS mixes.
fn parse_color_mix(inside: &str, light_dark: bool) -> Option<Rgba> {
    let args = split_top(inside, ',');
    if args.len() < 3 {
        return None;
    }
    let space = args[0].trim().to_ascii_lowercase();
    if !space.starts_with("in ") || !space.contains("srgb") {
        return None;
    }
    let mut colours: Vec<(Rgba, Option<f32>)> = Vec::new();
    for arg in &args[1..] {
        let (colour_text, percentage) = take_percentage(arg);
        let colour = parse(colour_text, light_dark)?;
        colours.push((colour, percentage));
    }
    let (first, second) = (*colours.first()?, *colours.get(1)?);
    let mut first_share = first.1;
    let mut second_share = second.1;
    match (first_share, second_share) {
        (Some(a), Some(b)) => {
            let total = a + b;
            if total > 0.0 {
                first_share = Some(a / total);
                second_share = Some(b / total);
            }
        }
        (Some(a), None) => {
            first_share = Some(a);
            second_share = Some((1.0 - a).max(0.0));
        }
        (None, Some(b)) => {
            second_share = Some(b);
            first_share = Some((1.0 - b).max(0.0));
        }
        (None, None) => {
            first_share = Some(0.5);
            second_share = Some(0.5);
        }
    }
    let a = first_share.unwrap_or(0.5);
    let b = second_share.unwrap_or(0.5);
    let total = a + b;
    if total > 0.0 {
        let first_weight = a / total;
        let second_weight = 1.0 - first_weight;
        let weight = |color: Rgba, share: f32| f32::from(color.a) / 255.0 * share;
        let (wa, wb) = (
            weight(first.0, first_weight),
            weight(second.0, second_weight),
        );
        let all = wa + wb;
        if all <= f32::EPSILON {
            return Some(Rgba::TRANSPARENT);
        }
        let channel = |a: u8, b: u8| {
            ((f32::from(a) * wa + f32::from(b) * wb) / all)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        return Some(Rgba::rgba(
            channel(first.0.r, second.0.r),
            channel(first.0.g, second.0.g),
            channel(first.0.b, second.0.b),
            ((all * 255.0).round().clamp(0.0, 255.0)) as u8,
        ));
    }
    Some(first.0)
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
    fn an_over_puts_a_translucent_layer_on_its_ground() {
        let red = Rgba::rgba(255, 0, 0, 128);
        let black = Rgba::rgb(0, 0, 0);
        let mixed = red.over(black);
        assert_eq!((mixed.r, mixed.g, mixed.b), (128, 0, 0));
    }
}
