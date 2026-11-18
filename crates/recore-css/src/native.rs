//! CSS rules for Discord's web window, matched against ReCore's own window.
//!
//! A native window has no `.messageContent_...` nodes, so a selector cannot
//! be painted as it would be in a browser. It can still describe a *role*:
//! the sidebar, the chat, the member list, the composer, a selected channel.
//! This module gives every role a small synthetic node (with the class
//! fragments Discord themes commonly target), runs the theme's selectors
//! against those nodes, and keeps the winning colours and shape values.
//!
//! It is deliberately not a browser: it understands the selectors and
//! properties a theme uses to skin those regions, not layout, pseudo-elements
//! or DOM animations. The window applies the result on top of the palette it
//! already reads from the theme's root variables.

use crate::{
    Rgba,
    color::{self, parse as parse_color},
    css::{Declaration, Rule, Vars, selector_compounds},
};

/// The parts of a Discord window a theme can address. A node may match more
/// than one selector fragment (for example `sidebar` and `sidebarList_`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Root,
    Rail,
    Sidebar,
    Chat,
    Members,
    Header,
    Messages,
    Message,
    Composer,
    Member,
    Card,
}

/// One synthetic node in the native window.
struct Node {
    role: Role,
    tags: &'static [&'static str],
    id: Option<&'static str>,
    classes: &'static [&'static str],
    parent: Option<usize>,
}

const ROOT: usize = 0;
const CHAT: usize = 3;
const MEMBERS: usize = 4;
const MESSAGES: usize = 6;

/// The tree ReCore shows, with the class names themes tend to mention. The
/// class fragments end in `_` because Discord's generated class names do.
const NODES: [Node; 11] = [
    Node {
        role: Role::Root,
        tags: &["html", "body"],
        id: Some("app-mount"),
        classes: &["app", "appMount", "theme-dark", "theme-light"],
        parent: None,
    },
    Node {
        role: Role::Rail,
        tags: &["nav", "div"],
        id: None,
        classes: &["guilds", "guilds_", "guildsnav___", "guildRail", "rail"],
        parent: Some(ROOT),
    },
    Node {
        role: Role::Sidebar,
        tags: &["div", "nav"],
        id: None,
        classes: &[
            "sidebar",
            "sidebar_",
            "sidebarList_",
            "sidebarRegion",
            "channels",
            "channelList",
            "privateChannels",
        ],
        parent: Some(ROOT),
    },
    Node {
        role: Role::Chat,
        tags: &["main", "div"],
        id: None,
        classes: &[
            "chat",
            "chat_",
            "chatContent_",
            "messagesWrapper",
            "content",
        ],
        parent: Some(ROOT),
    },
    Node {
        role: Role::Members,
        tags: &["aside", "div"],
        id: None,
        classes: &[
            "members",
            "members_",
            "membersWrap_",
            "memberList",
            "membersList",
        ],
        parent: Some(ROOT),
    },
    Node {
        role: Role::Header,
        tags: &["header", "div"],
        id: None,
        classes: &["header", "header_", "title_", "channelHeader", "toolbar"],
        parent: Some(CHAT),
    },
    Node {
        role: Role::Messages,
        tags: &["div"],
        id: None,
        classes: &["messages", "messages_", "scroller", "messageList"],
        parent: Some(CHAT),
    },
    Node {
        role: Role::Message,
        tags: &["div", "li"],
        id: None,
        classes: &[
            "message",
            "message_",
            "messageContent",
            "messageContent_",
            "messageListItem",
        ],
        parent: Some(MESSAGES),
    },
    Node {
        role: Role::Composer,
        tags: &["form", "div"],
        id: None,
        classes: &[
            "channelTextArea",
            "channelTextArea_",
            "textArea",
            "textArea_",
            "composer",
            "form",
        ],
        parent: Some(CHAT),
    },
    Node {
        role: Role::Member,
        tags: &["div", "li"],
        id: None,
        classes: &["member", "member_", "memberInner", "memberRow"],
        parent: Some(MEMBERS),
    },
    Node {
        role: Role::Card,
        tags: &["div", "section"],
        id: None,
        classes: &[
            "card",
            "card_",
            "modal_",
            "popout_",
            "menu_",
            "userPopout_",
            "profilePanel",
        ],
        parent: Some(ROOT),
    },
];

/// Colours and shape values a theme's selectors give ReCore's regions.
///
/// `None` leaves the value the theme's root variables already picked. Every
/// field is a value that can be applied to both the window's palette and the
/// Appearance tile without introducing a second theme model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeStyle {
    pub rail: Option<Rgba>,
    pub sidebar: Option<Rgba>,
    pub chat: Option<Rgba>,
    pub panel: Option<Rgba>,
    pub raised: Option<Rgba>,
    pub field: Option<Rgba>,
    pub text: Option<Rgba>,
    pub secondary: Option<Rgba>,
    pub muted: Option<Rgba>,
    pub hover: Option<Rgba>,
    pub row_hover: Option<Rgba>,
    pub radius: Option<f32>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub letter_spacing: Option<f32>,
}

impl NativeStyle {
    /// Matches `rules` against the native window and returns the values that
    /// win the cascade. `vars` are the root custom properties, already
    /// resolved for `dark`.
    pub fn compute(rules: &[Rule], vars: &Vars, dark: bool) -> Self {
        let mut writes = Vec::new();
        for (order, rule) in rules.iter().enumerate() {
            if rule
                .color_scheme
                .is_some_and(|color_scheme| color_scheme != dark)
            {
                continue;
            }
            for selector in &rule.selectors {
                let Some((role, hover, specificity)) = match_selector(selector, dark) else {
                    continue;
                };
                for declaration in &rule.declarations {
                    writes.push(Write {
                        important: declaration.important,
                        specificity,
                        order,
                        role,
                        hover,
                        declaration,
                    });
                }
            }
        }
        // Normal declarations first, then specificity, then source order;
        // `!important` declarations come after all of them.
        writes.sort_by_key(|write| (write.important, write.specificity, write.order));

        let mut style = Self::default();
        for write in writes {
            let Some(value) = vars.resolve(&write.declaration.value) else {
                continue;
            };
            apply(&mut style, write, &value, vars, dark);
        }
        style
    }
}

/// One declaration that matched, with the cascade keys it needs.
struct Write<'a> {
    important: bool,
    specificity: u32,
    order: usize,
    role: Role,
    hover: bool,
    declaration: &'a Declaration,
}

fn apply(style: &mut NativeStyle, write: Write<'_>, value: &str, _vars: &Vars, dark: bool) {
    let property = write.declaration.name.trim().to_ascii_lowercase();
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    let role = write.role;
    match property.as_str() {
        "background" | "background-color" => {
            if let Some(color) = background_color(value, dark) {
                set_background(style, role, write.hover, color);
            }
        }
        "color" => {
            if let Some(color) = color_in(value, dark) {
                set_text(style, role, color);
            }
        }
        "border-radius" => {
            if let Some(radius) = length(value) {
                style.radius = Some(radius);
            }
        }
        "font-family" => {
            if let Some(family) = family(value) {
                style.font_family = Some(family);
            }
        }
        "font-size" => {
            if let Some(size) = length(value) {
                style.font_size = Some(size);
            }
        }
        "letter-spacing" => {
            if let Some(spacing) = length(value) {
                style.letter_spacing = Some(spacing);
            }
        }
        _ => {}
    }
}

fn set_background(style: &mut NativeStyle, role: Role, hover: bool, color: Rgba) {
    let slot = if hover {
        match role {
            Role::Message | Role::Messages => &mut style.row_hover,
            _ => &mut style.hover,
        }
    } else {
        match role {
            Role::Root | Role::Chat | Role::Messages => &mut style.chat,
            Role::Rail => &mut style.rail,
            Role::Sidebar => &mut style.sidebar,
            Role::Members | Role::Member => &mut style.panel,
            Role::Header | Role::Card => &mut style.raised,
            Role::Composer => &mut style.field,
            Role::Message => &mut style.row_hover,
        }
    };
    *slot = Some(color);
}

fn set_text(style: &mut NativeStyle, role: Role, color: Rgba) {
    match role {
        Role::Sidebar => {
            style.secondary = Some(color);
        }
        Role::Members | Role::Member => {
            style.muted = Some(color);
        }
        _ => style.text = Some(color),
    }
}

fn background_color(value: &str, dark: bool) -> Option<Rgba> {
    if value.starts_with("url(") || value.contains("gradient(") {
        return None;
    }
    color_in(value, dark)
}

fn color_in(value: &str, dark: bool) -> Option<Rgba> {
    if let Some(color) = parse_color(value, dark) {
        return Some(color);
    }
    // A shorthand such as `#fff 1px solid` or `1px solid #fff`.
    for token in value.split_whitespace() {
        if let Some(color) = parse_color(token, dark) {
            return Some(color);
        }
    }
    None
}

fn length(value: &str) -> Option<f32> {
    let value = value.trim().trim_end_matches(';').trim();
    let first = value.split_whitespace().next()?;
    let number = |suffix: &str| {
        first
            .strip_suffix(suffix)
            .and_then(|number| number.trim().parse::<f32>().ok())
    };
    number("px")
        .or_else(|| number("rem").map(|number| number * 16.0))
        .or_else(|| number("em").map(|number| number * 16.0))
        .or_else(|| number("%").map(|number| number / 100.0 * 16.0))
        .or_else(|| first.parse::<f32>().ok())
}

fn family(value: &str) -> Option<String> {
    for family in color::split_top(value, ',') {
        let family = family.trim().trim_matches(['"', '\'']).trim();
        if family.is_empty() {
            continue;
        }
        if matches!(
            family.to_ascii_lowercase().as_str(),
            "inherit"
                | "initial"
                | "unset"
                | "revert"
                | "sans-serif"
                | "serif"
                | "monospace"
                | "system-ui"
        ) {
            continue;
        }
        return Some(family.to_owned());
    }
    None
}

/// Whether `selector` matches any native role, and whether it is a hover
/// rule. Every compound must match the role or one of its ancestors.
fn match_selector(selector: &str, dark: bool) -> Option<(Role, bool, u32)> {
    let selector = selector.trim();
    if selector.is_empty()
        || selector.contains("::")
        || selector.starts_with('@')
        || selector.contains(":has(")
        || selector.contains(":nth-")
        || selector.contains(":first-")
        || selector.contains(":last-")
        || selector.contains(":only-")
    {
        return None;
    }
    let compounds = selector_compounds(selector);
    if compounds.is_empty() {
        return None;
    }
    let specificity = specificity(selector);
    for (index, node) in NODES.iter().enumerate() {
        let mut current = Some(index);
        let mut hover = false;
        let mut matched = true;
        for compound in compounds.iter().rev() {
            let mut found = None;
            let mut walk = current;
            while let Some(index) = walk {
                if compound_matches(compound, index, dark, &mut hover) {
                    found = Some(index);
                    break;
                }
                walk = NODES[index].parent;
            }
            let Some(index) = found else {
                matched = false;
                break;
            };
            // Keep the node in the walk: a native region collapses several
            // Discord wrappers (`.message` and `.messageContent_`) into one
            // node, so a later compound may still match it.
            current = Some(index);
        }
        if matched {
            return Some((node.role, hover, specificity));
        }
    }
    None
}

/// A rough CSS specificity: ids, classes/attributes/pseudo-classes, tags.
fn specificity(selector: &str) -> u32 {
    let mut specificity = 0;
    for compound in selector_compounds(selector) {
        let bytes = compound.as_bytes();
        let mut index = 0;
        if bytes.first().is_some_and(u8::is_ascii_alphabetic) {
            specificity += 1;
        }
        while index < bytes.len() {
            match bytes[index] {
                b'#' => {
                    specificity += 100;
                    index += 1;
                }
                b'.' | b'[' => {
                    specificity += 10;
                    index += 1;
                }
                b':' if bytes.get(index + 1) != Some(&b':') => {
                    specificity += 10;
                    index += 1;
                }
                _ => index += 1,
            }
        }
    }
    specificity
}

fn compound_matches(compound: &str, node: usize, dark: bool, hover: &mut bool) -> bool {
    let mut rest = compound.trim();
    if rest.is_empty() {
        return false;
    }
    if rest == "*" {
        return true;
    }
    let mut saw_anything = false;

    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let (class, after) = take_identifier(after);
            if class.is_empty() {
                return false;
            }
            if !class_matches(node, class, dark) {
                return false;
            }
            rest = after;
            saw_anything = true;
            continue;
        }
        if let Some(after) = rest.strip_prefix('#') {
            let (id, after) = take_identifier(after);
            if NODES[node].id != Some(id) {
                return false;
            }
            rest = after;
            saw_anything = true;
            continue;
        }
        if rest.starts_with('[') {
            let Some(end) = rest.find(']') else {
                return false;
            };
            if !attribute_matches(&rest[1..end], node, dark) {
                return false;
            }
            rest = rest[end + 1..].trim_start();
            saw_anything = true;
            continue;
        }
        if rest.starts_with(':') {
            let after = &rest[1..];
            let (name, after) = take_identifier(after);
            if name.is_empty() {
                return false;
            }
            if let Some(after_paren) = skip_arguments(after) {
                let arguments = after_paren.0;
                rest = after_paren.1;
                match name.to_ascii_lowercase().as_str() {
                    "not" => {
                        let mut ignored = false;
                        let mut any = false;
                        for inner in split_selectors(arguments) {
                            if compound_matches(inner, node, dark, &mut ignored) {
                                any = true;
                                break;
                            }
                        }
                        if any {
                            return false;
                        }
                    }
                    "is" | "where" => {
                        let mut any = false;
                        for inner in split_selectors(arguments) {
                            let mut ignored = false;
                            if compound_matches(inner, node, dark, &mut ignored) {
                                any = true;
                                break;
                            }
                        }
                        if !any {
                            return false;
                        }
                    }
                    "has" => return false,
                    _ => return false,
                }
                saw_anything = true;
                continue;
            }
            match name.to_ascii_lowercase().as_str() {
                "root" => {
                    if NODES[node].role != Role::Root {
                        return false;
                    }
                }
                "hover" | "focus" | "focus-visible" | "focus-within" | "active" => {
                    *hover = true;
                }
                "link" | "visited" | "disabled" | "checked" | "empty" => return false,
                _ => return false,
            }
            rest = after.trim_start();
            saw_anything = true;
            continue;
        }
        // A type selector: a tag name at the start of the compound.
        let (tag, after) = take_identifier(rest);
        if tag.is_empty() {
            return false;
        }
        if !NODES[node]
            .tags
            .iter()
            .any(|known| known.eq_ignore_ascii_case(tag))
        {
            return false;
        }
        rest = after.trim_start();
        saw_anything = true;
    }
    saw_anything
}

fn class_matches(node: usize, class: &str, dark: bool) -> bool {
    if let Some(theme) = theme_class(class) {
        return NODES[node].role == Role::Root && theme == dark;
    }
    NODES[node]
        .classes
        .iter()
        .any(|known| known.eq_ignore_ascii_case(class))
}

fn theme_class(class: &str) -> Option<bool> {
    match class.to_ascii_lowercase().as_str() {
        "theme-dark" | "theme-darker" | "theme-midnight" | "theme-onxy" | "theme-ash" => Some(true),
        "theme-light" => Some(false),
        _ => None,
    }
}

fn attribute_matches(attribute: &str, node: usize, dark: bool) -> bool {
    let attribute = attribute.trim();
    let (name, operator, expected) = split_attribute(attribute);
    let name = name.trim().to_ascii_lowercase();
    let expected = expected.trim().trim_matches(['"', '\'']);
    let (actual, any) = match name.as_str() {
        "class" => (
            if NODES[node].role == Role::Root {
                format!("app appMount theme-{}", if dark { "dark" } else { "light" })
            } else {
                NODES[node].classes.join(" ")
            },
            true,
        ),
        "id" => (NODES[node].id.unwrap_or("").to_owned(), false),
        "data-theme" => (
            if NODES[node].role == Role::Root {
                if dark { "dark" } else { "light" }.to_owned()
            } else {
                String::new()
            },
            false,
        ),
        _ => return false,
    };
    let values: Vec<&str> = if any {
        actual.split_whitespace().collect()
    } else {
        vec![actual.as_str()]
    };
    match operator {
        "=" => values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(expected)),
        "~=" => {
            any && values
                .iter()
                .any(|value| value.eq_ignore_ascii_case(expected))
        }
        "|=" => values.iter().any(|value| {
            value.eq_ignore_ascii_case(expected) || value.starts_with(&format!("{expected}-"))
        }),
        "^=" => values.iter().any(|value| {
            value
                .to_ascii_lowercase()
                .starts_with(&expected.to_ascii_lowercase())
        }),
        "$=" => values.iter().any(|value| {
            value
                .to_ascii_lowercase()
                .ends_with(&expected.to_ascii_lowercase())
        }),
        "*=" => values.iter().any(|value| {
            value
                .to_ascii_lowercase()
                .contains(&expected.to_ascii_lowercase())
        }),
        _ => false,
    }
}

fn split_attribute(attribute: &str) -> (&str, &str, &str) {
    for operator in ["~=", "|=", "^=", "$=", "*=", "="] {
        if let Some(index) = attribute.find(operator) {
            return (
                &attribute[..index],
                operator,
                &attribute[index + operator.len()..],
            );
        }
    }
    (attribute, "", "")
}

fn take_identifier(text: &str) -> (&str, &str) {
    let mut end = 0;
    for (index, character) in text.char_indices() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '\\') {
            end = index + character.len_utf8();
        } else {
            break;
        }
    }
    (&text[..end], &text[end..])
}

/// Skips a balanced `(...)` after a pseudo-class and returns its inside and
/// what follows it.
fn skip_arguments(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    let inside = text.strip_prefix('(')?;
    let mut depth = 1u32;
    let mut quote = None;
    let bytes = inside.as_bytes();
    for (index, &byte) in bytes.iter().enumerate() {
        match byte {
            b'"' | b'\'' if quote == Some(byte) => quote = None,
            b'"' | b'\'' if quote.is_none() => quote = Some(byte),
            b'(' if quote.is_none() => depth += 1,
            b')' if quote.is_none() => {
                depth -= 1;
                if depth == 0 {
                    return Some((&inside[..index], &inside[index + 1..]));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_selectors(text: &str) -> Vec<&str> {
    color::split_top(text, ',')
        .into_iter()
        .map(str::trim)
        .filter(|selector| !selector.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mode, Options, load_str};

    fn dark(css: &str) -> crate::ThemeFile {
        load_str(css, None, &Options::with_mode(Mode::Dark)).expect("theme")
    }

    #[test]
    fn a_sidebar_rule_reaches_the_native_sidebar() {
        let theme = dark(
            r#"
            :root { --panel: #112233; }
            .theme-dark [class*="sidebarList_"] {
                background: var(--panel);
                color: #abcdef;
                border-radius: 3px;
            }
            "#,
        );
        assert_eq!(theme.native.sidebar, Some(Rgba::rgb(0x11, 0x22, 0x33)));
        assert_eq!(theme.native.secondary, Some(Rgba::rgb(0xab, 0xcd, 0xef)));
        assert_eq!(theme.look.radius, Some(3.0));
    }

    #[test]
    fn the_light_and_dark_themes_choose_their_own_rules() {
        let theme = dark(
            r#"
            .theme-light [class*="guilds_"] { background: #ffffff; }
            .theme-dark [class*="guilds_"] { background: #010203; }
            "#,
        );
        assert_eq!(theme.native.rail, Some(Rgba::rgb(0x01, 0x02, 0x03)));
    }

    #[test]
    fn roles_map_to_the_regions_they_name() {
        let theme = dark(
            r#"
            [class*="guilds_"] { background-color: #010203; }
            [class*="chat_"] { background: #040506; }
            [class*="membersWrap_"] { background: #070809; }
            [class*="channelTextArea_"] { background: #0a0b0c; }
            [class*="message_"]:hover { background: #0d0e0f; }
            "#,
        );
        assert_eq!(theme.native.rail, Some(Rgba::rgb(1, 2, 3)));
        assert_eq!(theme.native.chat, Some(Rgba::rgb(4, 5, 6)));
        assert_eq!(theme.native.panel, Some(Rgba::rgb(7, 8, 9)));
        assert_eq!(theme.native.field, Some(Rgba::rgb(10, 11, 12)));
        assert_eq!(theme.native.row_hover, Some(Rgba::rgb(13, 14, 15)));
    }

    #[test]
    fn specificity_and_source_order_pick_the_winner() {
        let theme = dark(
            r#"
            .theme-dark [class*="sidebar"] { background: #111111; }
            #app-mount [class*="sidebar"] { background: #222222; }
            .theme-dark [class*="sidebar"] { background: #333333; }
            "#,
        );
        assert_eq!(theme.native.sidebar, Some(Rgba::rgb(0x22, 0x22, 0x22)));
    }

    #[test]
    fn a_collapsed_message_wrapper_matches_both_compounds() {
        let theme = dark(
            r#"
            .theme-dark .message .messageContent { color: #123456; }
            "#,
        );
        assert_eq!(theme.native.text, Some(Rgba::rgb(0x12, 0x34, 0x56)));
    }

    #[test]
    fn variables_and_extra_properties_resolve() {
        let theme = dark(
            r#"
            :root { --line: #445566; --font: "Pixel Operator", monospace; }
            [class*="header_"] {
                font-family: var(--font);
                font-size: 18px;
                letter-spacing: 1px;
            }
            "#,
        );
        assert_eq!(theme.native.font_family.as_deref(), Some("Pixel Operator"));
        assert_eq!(theme.look.font_size, Some(18.0));
        assert_eq!(theme.look.letter_spacing, Some(1.0));
    }
}
