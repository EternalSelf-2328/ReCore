//! The options a theme exposes: the custom properties the window can change
//! and the controls Appearance offers for them. The names are the theme's own,
//! so a Vencord theme's `--background-primary` or ReCord's
//! `--rl-scanline-opacity` both become a control with a readable label.

use crate::{Rgba, css::Vars};

/// A control Appearance can show for one of a theme's custom properties.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeOption {
    /// The custom property, such as `--rl-accent`.
    pub name: String,
    pub label: String,
    pub description: Option<String>,
    /// The heading it belongs under.
    pub group: String,
    pub kind: OptionKind,
    /// The value as the theme writes it now, overrides included.
    pub value: String,
}

/// What sort of value a theme option holds.
#[derive(Clone, Debug, PartialEq)]
pub enum OptionKind {
    Color {
        /// The resolved colour, which the swatch shows.
        color: Rgba,
    },
    /// A percentage, shown from 0 to 100.
    Percent {
        value: f32,
        /// Whether the theme writes `25%` rather than `0.25`.
        percent: bool,
    },
    /// A length, a duration or another number with a unit.
    Number {
        value: f32,
        unit: String,
    },
    Toggle {
        on: bool,
        on_value: String,
        off_value: String,
    },
    Choice {
        choices: Vec<&'static str>,
    },
    /// A picture, as a `url(...)` or `none`.
    Image {
        url: Option<String>,
        embedded: bool,
    },
    Font,
    Text,
}

/// Reads every custom property worth offering from the resolved variables.
pub fn detect(vars: &Vars, dark: bool) -> Vec<ThemeOption> {
    let mut options = Vec::new();
    for (name, raw) in vars.iter() {
        if !name.starts_with("--") || name.starts_with("--rl-asset-") {
            continue;
        }
        // A data URI is a picture the theme carries, not something to type
        // into a field; skip the ones too large to be useful as text.
        if raw.len() > 2_000 {
            continue;
        }
        let Some(kind) = classify(vars, name, raw, dark) else {
            continue;
        };
        let (label, group, description) = describe(name);
        // A colour's field and swatch show hex, not the `var()` or
        // `color-mix()` that produced it.
        let value = match &kind {
            OptionKind::Color { color } => color.hex(),
            _ => raw.to_owned(),
        };
        options.push(ThemeOption {
            name: name.to_owned(),
            label,
            description,
            group,
            kind,
            value,
        });
    }
    options.sort_by(|a, b| {
        group_order(&a.group)
            .cmp(&group_order(&b.group))
            .then_with(|| a.label.cmp(&b.label))
    });
    options
}

fn classify(vars: &Vars, name: &str, raw: &str, dark: bool) -> Option<OptionKind> {
    let value = raw.trim();
    let lower = value.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();

    if let Some(choices) = choices(name) {
        return Some(OptionKind::Choice { choices });
    }
    if name_lower.contains("flicker") || name_lower.contains("beam") {
        let on = vars
            .resolve(vars.get(name)?)?
            .trim()
            .trim_end_matches(['s', 'm'])
            .parse::<f32>()
            .is_ok_and(|value| value > 0.0);
        return Some(OptionKind::Toggle {
            on,
            on_value: if name_lower.contains("flicker") {
                "6s".into()
            } else {
                "0.5".into()
            },
            off_value: "0".into(),
        });
    }
    if matches!(
        lower.as_str(),
        "on" | "off" | "true" | "false" | "yes" | "no"
    ) {
        let (on_value, off_value) = match lower.as_str() {
            "true" | "false" => ("true".to_owned(), "false".to_owned()),
            "yes" | "no" => ("yes".to_owned(), "no".to_owned()),
            _ => ("on".to_owned(), "off".to_owned()),
        };
        return Some(OptionKind::Toggle {
            on: matches!(lower.as_str(), "on" | "true" | "yes"),
            on_value,
            off_value,
        });
    }
    if name_lower.contains("font") {
        return Some(OptionKind::Font);
    }
    if image_name(&name_lower) {
        let url = value.find("url(").and_then(|start| {
            let rest = &value[start + 4..];
            rest.find(')')
                .map(|end| rest[..end].trim().trim_matches(['"', '\'']).to_owned())
        });
        if let Some(url) = url {
            return Some(OptionKind::Image {
                embedded: url.starts_with("data:"),
                url: Some(url),
            });
        }
        if lower == "none" {
            return Some(OptionKind::Image {
                url: None,
                embedded: false,
            });
        }
    }
    if let Some(color) = vars.color(name, dark) {
        return Some(OptionKind::Color { color });
    }
    if name_lower.contains("opacity")
        || name_lower.contains("surface")
        || name_lower.contains("dim")
        || name_lower.contains("alpha")
    {
        return percent(vars, name).map(|(value, percent)| OptionKind::Percent { value, percent });
    }
    if let Some((value, unit)) = number(vars, name) {
        return Some(OptionKind::Number { value, unit });
    }
    Some(OptionKind::Text)
}

fn choices(name: &str) -> Option<Vec<&'static str>> {
    match name {
        "--rl-motion" => Some(vec!["full", "subtle", "off"]),
        "--rl-background-size" => Some(vec!["cover", "contain", "auto"]),
        _ => None,
    }
}

fn image_name(name: &str) -> bool {
    name.contains("image")
        || name.contains("background")
        || name.contains("logo")
        || name.contains("avatar")
        || name.contains("mark")
        || name.contains("picture")
}

fn percent(vars: &Vars, name: &str) -> Option<(f32, bool)> {
    let value = vars.resolve(vars.get(name)?)?;
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return Some((percent.trim().parse::<f32>().ok()?.clamp(0.0, 100.0), true));
    }
    let number = value.parse::<f32>().ok()?;
    Some(((number * 100.0).clamp(0.0, 100.0), false))
}

fn number(vars: &Vars, name: &str) -> Option<(f32, String)> {
    let value = vars.resolve(vars.get(name)?)?;
    let value = value.trim();
    for unit in ["px", "rem", "em", "ms", "s"] {
        if let Some(number) = value.strip_suffix(unit) {
            return Some((number.trim().parse::<f32>().ok()?, unit.to_owned()));
        }
    }
    None
}

/// A readable label, group and description for a custom property.
fn describe(name: &str) -> (String, String, Option<String>) {
    if let Some((label, group, description)) = record(name) {
        return (
            label.to_owned(),
            group.to_owned(),
            description.map(str::to_owned),
        );
    }
    let label = pretty(name);
    let group = if name.contains("font") || name.contains("text-size") {
        "Typography"
    } else if name.contains("radius")
        || name.contains("border-width")
        || name.contains("spacing")
        || name.contains("padding")
    {
        "Shape"
    } else if image_name(name) {
        "Pictures"
    } else if name.contains("opacity")
        || name.contains("shadow")
        || name.contains("blur")
        || name.contains("animation")
        || name.contains("motion")
    {
        "Effects"
    } else if name.contains("background")
        || name.contains("color")
        || name.contains("text")
        || name.contains("brand")
        || name.contains("accent")
        || name.contains("border")
        || name.contains("status")
        || name.contains("icon")
        || name.contains("interactive")
    {
        "Colours"
    } else {
        "Other"
    };
    (label, group.to_owned(), None)
}

/// ReCord's own options, with their names and what each does.
fn record(name: &str) -> Option<(&'static str, &'static str, Option<&'static str>)> {
    let option = match name {
        "--rl-accent" | "--rl-signal" => (
            "Accent",
            "ReCore look",
            Some("The signal colour every ReCord shade derives from."),
        ),
        "--rl-font" => (
            "Typeface",
            "ReCore look",
            Some("The font family the window draws with."),
        ),
        "--rl-radius" => (
            "Corner radius",
            "ReCore look",
            Some("Square panels at 0, softer corners above."),
        ),
        "--rl-motion" => (
            "Animations",
            "ReCore look",
            Some("Full, subtle (menus and dialogs) or off."),
        ),
        "--rl-icons" => (
            "Pixel icons",
            "ReCore look",
            Some("ReCord's 16 px pixel icons instead of Discord's."),
        ),
        "--rl-scanline-opacity" => (
            "Scanlines",
            "ReCord effects",
            Some("The horizontal CRT lines over the window."),
        ),
        "--rl-grain-opacity" => (
            "Film grain",
            "ReCord effects",
            Some("The static over the window."),
        ),
        "--rl-vignette-opacity" => (
            "Vignette",
            "ReCord effects",
            Some("The darkening towards the window's edges."),
        ),
        "--rl-flicker-duration" => (
            "CRT flicker",
            "ReCord effects",
            Some("A slow brightness flicker."),
        ),
        "--rl-beam-opacity" => (
            "Rolling scan beam",
            "ReCord effects",
            Some("The pale band that sweeps down the window."),
        ),
        "--rl-frame-opacity" => (
            "Corner frame",
            "ReCord effects",
            Some("The brackets in the window's corners."),
        ),
        "--rl-background-image" => (
            "Background image",
            "Background",
            Some("A picture behind the interface."),
        ),
        "--rl-background-dim" => (
            "Background darkening",
            "Background",
            Some("How far the picture is dimmed behind the panels."),
        ),
        "--rl-surface" => (
            "Panel opacity",
            "Background",
            Some("How see-through panels are over the picture."),
        ),
        "--rl-background-size" => (
            "Background fit",
            "Background",
            Some("How the picture fills the window."),
        ),
        "--rl-avatar-filter" => (
            "Avatar filter",
            "Pictures",
            Some("The CSS filter over people's avatars."),
        ),
        "--rl-avatar-scanline" => (
            "Avatar scanlines",
            "Pictures",
            Some("The scanline mask over avatars."),
        ),
        "--rl-tracking" => (
            "Letter spacing",
            "Typography",
            Some("The spacing between letters."),
        ),
        "--rl-lite" => (
            "Low power",
            "ReCord effects",
            Some("Drops the grain and the constant repaints."),
        ),
        "--rl-void" => ("Void", "ReCord palette", None),
        "--rl-ink" => ("Ink", "ReCord palette", None),
        "--rl-coal" => ("Coal", "ReCord palette", None),
        "--rl-panel" => ("Panel", "ReCord palette", None),
        "--rl-raised" => ("Raised", "ReCord palette", None),
        "--rl-high" => ("High", "ReCord palette", None),
        "--rl-higher" => ("Higher", "ReCord palette", None),
        "--rl-line" => ("Line", "ReCord palette", None),
        "--rl-paper" => ("Paper", "ReCord palette", None),
        "--rl-cream" => ("Cream", "ReCord palette", None),
        "--rl-bone" => ("Bone", "ReCord palette", None),
        "--rl-smoke" => ("Smoke", "ReCord palette", None),
        "--rl-ash" => ("Ash", "ReCord palette", None),
        "--rl-faint" => ("Faint", "ReCord palette", None),
        "--rl-teal" => ("Teal", "ReCord palette", None),
        "--rl-green" => ("Green", "ReCord palette", None),
        "--rl-amber" => ("Amber", "ReCord palette", None),
        "--rl-red" => ("Red", "ReCord palette", None),
        "--rl-purple" => ("Purple", "ReCord palette", None),
        "--rl-offline" => ("Offline", "ReCord palette", None),
        _ => return None,
    };
    Some(option)
}

/// A human label from a custom property's name.
fn pretty(name: &str) -> String {
    let name = name.trim_start_matches("--");
    let name = name
        .strip_prefix("rl-")
        .or_else(|| name.strip_prefix("discord-"))
        .unwrap_or(name);
    let mut words = name.split('-').filter(|word| !word.is_empty());
    let Some(first) = words.next() else {
        return name.to_owned();
    };
    let mut label = String::with_capacity(name.len());
    let mut chars = first.chars();
    if let Some(first) = chars.next() {
        label.extend(first.to_uppercase());
        label.push_str(chars.as_str());
    }
    for word in words {
        label.push(' ');
        label.push_str(word);
    }
    label
}

fn group_order(group: &str) -> u8 {
    match group {
        "ReCore look" => 0,
        "ReCord effects" => 1,
        "Background" => 2,
        "ReCord palette" => 3,
        "Colours" => 4,
        "Typography" => 5,
        "Shape" => 6,
        "Pictures" => 7,
        "Effects" => 8,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{Declaration, Vars};

    fn vars(entries: &[(&str, &str)]) -> Vars {
        let mut vars = Vars::default();
        for (name, value) in entries {
            vars.insert(&Declaration {
                name: (*name).into(),
                value: (*value).into(),
                important: false,
            });
        }
        vars
    }

    #[test]
    fn record_options_become_controls() {
        let vars = vars(&[
            ("--rl-accent", "#e94221"),
            ("--rl-scanline-opacity", "0.25"),
            ("--rl-motion", "full"),
            ("--rl-icons", "on"),
            ("--rl-background-image", "url(\"bg.png\")"),
            ("--rl-font", "\"Pixel Operator\", monospace"),
        ]);
        let options = detect(&vars, true);
        let find = |name: &str| options.iter().find(|option| option.name == name).unwrap();
        assert!(matches!(find("--rl-accent").kind, OptionKind::Color { .. }));
        assert!(matches!(
            find("--rl-scanline-opacity").kind,
            OptionKind::Percent {
                value: 25.0,
                percent: false
            }
        ));
        assert!(matches!(
            find("--rl-motion").kind,
            OptionKind::Choice { .. }
        ));
        assert!(matches!(
            find("--rl-icons").kind,
            OptionKind::Toggle { on: true, .. }
        ));
        assert!(matches!(
            find("--rl-background-image").kind,
            OptionKind::Image { .. }
        ));
        assert!(matches!(find("--rl-font").kind, OptionKind::Font));
    }

    #[test]
    fn generic_discord_tokens_are_readable() {
        let vars = vars(&[
            ("--background-primary", "#313338"),
            ("--text-normal", "#dbdee1"),
            ("--radius-md", "8px"),
            ("--font-primary", "\"gg sans\", sans-serif"),
        ]);
        let options = detect(&vars, true);
        assert!(options.iter().any(|option| option.group == "Colours"));
        assert!(options.iter().any(|option| option.group == "Shape"));
        assert!(options.iter().any(|option| option.group == "Typography"));
        assert_eq!(
            options
                .iter()
                .find(|option| option.name == "--background-primary")
                .unwrap()
                .label,
            "Background primary"
        );
    }
}
