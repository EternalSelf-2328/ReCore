//! A small CSS reader: comments and strings, rules and their declarations,
//! `@import`, `@font-face`, and the at-rules that wrap variable declarations
//! (`@media`, `@supports`, `@layer`, `@container`). It reads what themes use,
//! not every corner of the language, and keeps what it doesn't know as text.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};

use crate::color::{Rgba, parse as parse_color};

/// One declaration, and whether it carried `!important`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    pub name: String,
    pub value: String,
    pub important: bool,
}

/// One style rule: its selectors and declarations, under the at-rules around
/// it (whether they ask for a dark or a light colour scheme).
#[derive(Clone, Debug, Default)]
pub struct Rule {
    pub selectors: Vec<String>,
    pub declarations: Vec<Declaration>,
    /// `Some(true)` under `prefers-color-scheme: dark`, `Some(false)` under
    /// its light query, `None` under anything else.
    pub color_scheme: Option<bool>,
}

/// A font a theme carries or points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFace {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    /// Pulled out of a `data:` URI, or read from the path beside the theme.
    pub data: Option<Vec<u8>>,
    /// What the `src` asked for, for the diagnostics.
    pub source: String,
}

/// What a stylesheet holds.
#[derive(Clone, Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    pub fonts: Vec<FontFace>,
    pub warnings: Vec<String>,
}

/// A parse failure; the loader keeps what came before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ParseError {}

/// Reads a stylesheet, pulling its `@import`s in through `resolve`.
///
/// `resolve` gets the folder of the stylesheet that holds the import and the
/// path as written, and returns the imported file's text and path, so the
/// caller decides which files and which URLs it will read.
pub fn parse(
    text: &str,
    mut resolve: impl FnMut(Option<&Path>, &str) -> Option<(String, PathBuf)>,
) -> Stylesheet {
    parse_in(text, None, &mut resolve)
}

/// [`parse`] with the folder of `text` for its first imports.
pub fn parse_in(
    text: &str,
    base: Option<&Path>,
    resolve: &mut impl FnMut(Option<&Path>, &str) -> Option<(String, PathBuf)>,
) -> Stylesheet {
    let mut stylesheet = Stylesheet::default();
    let mut imported = Vec::new();
    expand(text, base, &mut stylesheet, resolve, &mut imported, 0);
    stylesheet
}

/// Reads a stylesheet, inlining each top-level `@import` where it appears so
/// the rules after it override the ones it brought in, as CSS orders them.
fn expand(
    text: &str,
    base: Option<&Path>,
    stylesheet: &mut Stylesheet,
    resolve: &mut impl FnMut(Option<&Path>, &str) -> Option<(String, PathBuf)>,
    imported: &mut Vec<PathBuf>,
    depth: usize,
) {
    if depth > 16 {
        stylesheet
            .warnings
            .push("more than 16 nested imports; stopped".into());
        return;
    }
    let source = strip_comments(text);
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut chunk_start = 0;
    while index < bytes.len() {
        let start = index;
        let mut depth_in_rule = 0u32;
        let mut quote: Option<u8> = None;
        let mut escaped = false;
        let mut statement_end = None;
        let mut block = false;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            match byte {
                b'\\' => escaped = true,
                b'"' | b'\'' if quote == Some(byte) => quote = None,
                b'"' | b'\'' if quote.is_none() => quote = Some(byte),
                b'(' if quote.is_none() => depth_in_rule += 1,
                b')' if quote.is_none() => depth_in_rule = depth_in_rule.saturating_sub(1),
                b';' if quote.is_none() && depth_in_rule == 0 => {
                    statement_end = Some(index);
                    break;
                }
                b'{' if quote.is_none() && depth_in_rule == 0 => {
                    block = true;
                    break;
                }
                _ => {}
            }
            index += 1;
        }
        if block {
            // Skip the whole block, keeping its text in the chunk.
            let Some(close) = matching_brace(&source, index) else {
                break;
            };
            index = close + 1;
            continue;
        }
        let Some(end) = statement_end else {
            break;
        };
        let statement = source[start..end].trim();
        let is_import = statement
            .strip_prefix('@')
            .is_some_and(|rest| rest.trim_start().starts_with("import"));
        if is_import {
            // Parse what came before, then the imported sheet, then go on.
            let before = &source[chunk_start..start];
            if !before.trim().is_empty() {
                let mut at_rules = Vec::new();
                parse_block(before, stylesheet, &mut at_rules, None);
            }
            let rest = statement
                .trim_start_matches('@')
                .trim_start_matches("import")
                .trim();
            match import_path(rest) {
                Some(path)
                    if path.starts_with("http://")
                        || path.starts_with("https://")
                        || path.starts_with("data:") =>
                {
                    stylesheet.warnings.push(format!(
                        "@import {path} is not a local file, so it is skipped"
                    ));
                }
                Some(path) => {
                    if imported.iter().any(|seen| seen == &path) {
                        stylesheet
                            .warnings
                            .push(format!("@import {path} already read; skipped the loop"));
                    } else {
                        match resolve(base, &path) {
                            Some((imported_text, full)) => {
                                imported.push(full.clone());
                                expand(
                                    &imported_text,
                                    full.parent(),
                                    stylesheet,
                                    resolve,
                                    imported,
                                    depth + 1,
                                );
                                imported.pop();
                            }
                            None => stylesheet
                                .warnings
                                .push(format!("could not read the @import {path}")),
                        }
                    }
                }
                None => stylesheet
                    .warnings
                    .push(format!("an @import has no readable path: {rest}")),
            }
            index = end + 1;
            chunk_start = index;
            continue;
        }
        index = end + 1;
    }
    let rest = &source[chunk_start..];
    if !rest.trim().is_empty() {
        let mut at_rules = Vec::new();
        parse_block(rest, stylesheet, &mut at_rules, None);
    }
}

/// Reads a block's rules into `stylesheet`.
fn parse_block(
    text: &str,
    stylesheet: &mut Stylesheet,
    color_scheme: &mut Vec<Option<bool>>,
    current: Option<bool>,
) {
    let mut index = 0;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        if bytes[index] == b'}' {
            index += 1;
            continue;
        }
        let start = index;
        let mut depth = 0u32;
        let mut quote: Option<u8> = None;
        let mut escaped = false;
        let mut open = None;
        while index < bytes.len() {
            let byte = bytes[index];
            if escaped {
                escaped = false;
                index += 1;
                continue;
            }
            match byte {
                b'\\' => escaped = true,
                b'"' | b'\'' if quote == Some(byte) => quote = None,
                b'"' | b'\'' if quote.is_none() => quote = Some(byte),
                b'(' if quote.is_none() => depth += 1,
                b')' if quote.is_none() => depth = depth.saturating_sub(1),
                b';' if quote.is_none() && depth == 0 => {
                    let prelude = text[start..index].trim();
                    if prelude.starts_with('@') {
                        stylesheet
                            .warnings
                            .push(format!("a statement inside a block is skipped: {prelude}"));
                    }
                    index += 1;
                    open = None;
                    break;
                }
                b'{' if quote.is_none() && depth == 0 => {
                    open = Some(index);
                    break;
                }
                b'}' if quote.is_none() && depth == 0 => {
                    index += 1;
                    open = None;
                    break;
                }
                _ => {}
            }
            index += 1;
        }
        let Some(brace) = open else {
            continue;
        };
        let prelude = text[start..brace].trim();
        let Some(close) = matching_brace(text, brace) else {
            stylesheet
                .warnings
                .push(format!("a rule '{prelude}' has no closing brace"));
            break;
        };
        let body = &text[brace + 1..close];
        index = close + 1;
        if let Some(name) = prelude.strip_prefix('@') {
            let name = name
                .split(|character: char| character.is_whitespace() || character == '(')
                .next()
                .unwrap_or("");
            let lower = name.to_ascii_lowercase();
            match lower.as_str() {
                "font-face" => {
                    stylesheet.fonts.push(font_face(body));
                }
                "media" | "supports" | "layer" | "container" | "scope" | "document" => {
                    let scheme = if lower == "media" {
                        media_scheme(prelude)
                    } else {
                        None
                    };
                    let applies = scheme.or(current);
                    color_scheme.push(applies);
                    parse_block(body, stylesheet, color_scheme, applies);
                    color_scheme.pop();
                }
                "keyframes"
                | "-webkit-keyframes"
                | "property"
                | "counter-style"
                | "font-feature-values" => {}
                _ => {
                    // Unknown at-rules with blocks (@page, @starting-style) may
                    // still hold declarations for the root; read their rules.
                    parse_block(body, stylesheet, color_scheme, current);
                }
            }
            continue;
        }
        let selectors: Vec<String> = crate::color::split_top(prelude, ',')
            .into_iter()
            .map(str::to_owned)
            .collect();
        if selectors.is_empty() || body.trim().is_empty() {
            continue;
        }
        stylesheet.rules.push(Rule {
            selectors,
            declarations: declarations(body),
            color_scheme: color_scheme.last().copied().flatten(),
        });
    }
}

/// The path in `@import "x"`, `@import url(x)` or `@import url("x")`.
fn import_path(rest: &str) -> Option<String> {
    let rest = rest.trim();
    if let Some(inside) = rest.strip_prefix("url(") {
        let end = inside.find(')')?;
        return Some(unquote(inside[..end].trim()));
    }
    let first = rest.split_whitespace().next()?;
    if first.starts_with('"') || first.starts_with('\'') {
        let quote = first.chars().next()?;
        let inner = first.strip_prefix(quote)?.strip_suffix(quote)?;
        return Some(inner.to_owned());
    }
    None
}

/// Everything between `{` and its `}`.
fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0u32;
    let mut quote: Option<u8> = None;
    let mut escaped = false;
    for (offset, &byte) in bytes.iter().enumerate().skip(open) {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'"' | b'\'' if quote == Some(byte) => quote = None,
            b'"' | b'\'' if quote.is_none() => quote = Some(byte),
            b'{' if quote.is_none() => depth += 1,
            b'}' if quote.is_none() => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
    }
    None
}

/// The declarations of a block, each name and value as written.
fn declarations(body: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for piece in crate::color::split_top(body, ';') {
        if piece.trim().is_empty() {
            continue;
        }
        let Some((name, value)) = piece.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let mut value = value.trim();
        let mut important = false;
        if let Some(stripped) = value.strip_suffix("!important") {
            value = stripped.trim_end();
            important = true;
        } else if let Some(stripped) = value.strip_suffix("! important") {
            value = stripped.trim_end();
            important = true;
        }
        declarations.push(Declaration {
            name: name.to_owned(),
            value: value.to_owned(),
            important,
        });
    }
    declarations
}

fn font_face(body: &str) -> FontFace {
    let mut family = String::new();
    let mut weight = 400;
    let mut italic = false;
    let mut data = None;
    let mut source = String::new();
    for declaration in declarations(body) {
        match declaration.name.to_ascii_lowercase().as_str() {
            "font-family" => family = unquote(declaration.value.trim()),
            "font-weight" => {
                if let Ok(value) = declaration.value.trim().parse::<u16>() {
                    weight = value;
                }
            }
            "font-style" => {
                italic = declaration.value.trim().eq_ignore_ascii_case("italic")
                    || declaration.value.trim().eq_ignore_ascii_case("oblique");
            }
            "src" => {
                for candidate in crate::color::split_top(&declaration.value, ',') {
                    let candidate = candidate.trim();
                    if let Some(bytes) = data_uri(candidate) {
                        data = Some(bytes);
                        candidate.clone_into(&mut source);
                        break;
                    }
                    if source.is_empty() {
                        candidate.clone_into(&mut source);
                    }
                }
            }
            _ => {}
        }
    }
    FontFace {
        family,
        weight,
        italic,
        data,
        source,
    }
}

/// The bytes of a `data:` URI, when it carries them.
fn data_uri(text: &str) -> Option<Vec<u8>> {
    let start = text.find("url(")?;
    let rest = &text[start + 4..];
    let end = rest.find(')')?;
    let inside = unquote(&rest[..end]);
    let inside = inside.strip_prefix("data:")?;
    let (meta, data) = inside.split_once(',')?;
    if meta.contains(";base64") {
        crate::base64::decode(data.trim())
    } else {
        Some(percent_decode(data))
    }
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if (bytes[0] == b'"' && bytes[text.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[text.len() - 1] == b'\'')
        {
            return text[1..text.len() - 1]
                .replace("\\\"", "\"")
                .replace("\\'", "'");
        }
    }
    text.to_owned()
}

fn percent_decode(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            decoded.push(((high << 4) | low) as u8);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    decoded
}

/// Removes comments, keeping line breaks and string contents.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        if let Some(active) = quote {
            out.push(bytes[index] as char);
            if bytes[index] == b'\\' && index + 1 < bytes.len() {
                out.push(bytes[index + 1] as char);
                index += 2;
                continue;
            }
            if bytes[index] == active {
                quote = None;
            }
            index += 1;
            continue;
        }
        match bytes[index] {
            b'"' | b'\'' => {
                quote = Some(bytes[index]);
                out.push(bytes[index] as char);
                index += 1;
            }
            b'/' if index + 1 < bytes.len() && bytes[index + 1] == b'*' => {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    if bytes[index] == b'\n' {
                        out.push('\n');
                    }
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            }
            byte => {
                out.push(byte as char);
                index += 1;
            }
        }
    }
    out
}

/// `prefers-color-scheme: dark` and its light sibling in a media query.
fn media_scheme(prelude: &str) -> Option<bool> {
    let lower = prelude.to_ascii_lowercase();
    if !lower.contains("prefers-color-scheme") {
        return None;
    }
    if lower.contains("prefers-color-scheme: dark") || lower.contains("prefers-color-scheme:dark") {
        return Some(true);
    }
    if lower.contains("prefers-color-scheme: light") || lower.contains("prefers-color-scheme:light")
    {
        return Some(false);
    }
    None
}

/// Resolves custom properties and the colours that use them.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    values: BTreeMap<String, String>,
    important: BTreeMap<String, bool>,
}

impl Vars {
    pub fn insert(&mut self, declaration: &Declaration) {
        let replace = !matches!(
            (
                self.important.get(&declaration.name).copied(),
                declaration.important
            ),
            (Some(true), false)
        );
        if replace {
            self.values
                .insert(declaration.name.clone(), declaration.value.clone());
            self.important
                .insert(declaration.name.clone(), declaration.important);
        }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// Every custom property, by name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
    }

    /// `value` with every `var()` replaced, or `None` when one has neither a
    /// value nor a fallback.
    pub fn resolve(&self, value: &str) -> Option<String> {
        self.resolve_depth(value, 0)
    }

    /// The colour a declaration names, after its `var()`s are filled in.
    pub fn color(&self, name: &str, light_dark: bool) -> Option<Rgba> {
        let value = self.resolve(self.get(name)?)?;
        parse_color(&value, light_dark)
    }

    /// The colour of the first of `names` that resolves, after filling in
    /// `var()`s.
    pub fn first_color(&self, names: &[&str], light_dark: bool) -> Option<Rgba> {
        names.iter().find_map(|name| self.color(name, light_dark))
    }

    fn resolve_depth(&self, value: &str, depth: usize) -> Option<String> {
        if depth > 32 {
            return None;
        }
        let mut out = String::with_capacity(value.len());
        let mut rest = value;
        while let Some(start) = rest.find("var(") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 4..];
            let Some(end) = matching_paren(after) else {
                // Not a `var()` we can read; keep it as text.
                out.push_str(&rest[start..]);
                return Some(out);
            };
            let inside = &after[..end];
            let arguments = crate::color::split_top(inside, ',');
            let name = arguments.first()?.trim();
            let resolved = self
                .values
                .get(name)
                .and_then(|value| self.resolve_depth(value, depth + 1))
                .or_else(|| {
                    arguments
                        .get(1)
                        .and_then(|fallback| self.resolve_depth(fallback, depth + 1))
                });
            out.push_str(&resolved?);
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        Some(out)
    }
}

/// The closing parenthesis of `var(`'s arguments.
fn matching_paren(text: &str) -> Option<usize> {
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
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
            ')' if quote.is_none() => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Whether a selector writes variables on the page's root, and for which
/// colour scheme. Descendant selectors that style a part of Discord's DOM
/// don't set the window's palette.
pub fn root_selector(selector: &str, theme_dark: bool) -> Option<bool> {
    let selector = selector.trim();
    if selector.is_empty() || selector.contains("::") {
        return None;
    }
    let mut dark = theme_dark;
    let mut saw_root = false;
    for compound in selector_compounds(selector) {
        let lower = compound.to_ascii_lowercase();
        if lower.contains("theme-light")
            || lower.contains("data-rl-mode=\"light\"")
            || lower.contains("data-rl-mode='light'")
        {
            dark = false;
            saw_root = true;
        } else if lower.contains("theme-dark")
            || lower.contains("theme-darker")
            || lower.contains("theme-midnight")
            || lower.contains("theme-onxy")
            || lower.contains("theme-ash")
            || lower.contains("data-rl-mode=\"dark\"")
            || lower.contains("data-rl-mode='dark'")
        {
            dark = true;
            saw_root = true;
        } else if lower.contains(":root")
            || lower.starts_with("html")
            || lower.starts_with("body")
            || lower.starts_with("#app-mount")
            || lower.starts_with(":where(html")
            || lower.starts_with(":where(:root")
        {
            saw_root = true;
        } else {
            return None;
        }
    }
    saw_root.then_some(dark)
}

/// Splits a selector at its combinators, keeping the pieces that matter.
pub(crate) fn selector_compounds(selector: &str) -> Vec<String> {
    let mut compounds = Vec::new();
    let mut current = String::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for character in selector.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' => {
                current.push(character);
                escaped = true;
            }
            '"' | '\'' if quote == Some(character) => {
                quote = None;
                current.push(character);
            }
            '"' | '\'' if quote.is_none() => {
                quote = Some(character);
                current.push(character);
            }
            '(' if quote.is_none() => {
                depth += 1;
                current.push(character);
            }
            ')' if quote.is_none() => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            '>' | '+' | '~' if quote.is_none() && depth == 0 => {
                if !current.trim().is_empty() {
                    compounds.push(current.trim().to_owned());
                }
                current.clear();
            }
            character if character.is_whitespace() && quote.is_none() && depth == 0 => {
                if !current.trim().is_empty() {
                    compounds.push(current.trim().to_owned());
                }
                current.clear();
            }
            _ => current.push(character),
        }
    }
    if !current.trim().is_empty() {
        compounds.push(current.trim().to_owned());
    }
    compounds
}

/// The theme's header comment, which Vencord themes use as metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub source: Option<String>,
    pub website: Option<String>,
}

impl Meta {
    pub fn read(css: &str) -> Self {
        let mut meta = Self::default();
        // Only the opening comment is metadata, not a comment deep in the file.
        let Some(start) = css.find("/*") else {
            return meta;
        };
        let Some(end) = css[start + 2..].find("*/") else {
            return meta;
        };
        let header = &css[start + 2..start + 2 + end];
        for line in header.lines() {
            let line = line.trim().trim_start_matches('*').trim();
            // A theme may write several `@key value` pairs on one line; each
            // pair runs to the next ` @`.
            let mut pieces: Vec<String> = line
                .split(" @")
                .map(|piece| piece.trim().to_owned())
                .collect();
            for piece in pieces.iter_mut().skip(1) {
                piece.insert(0, '@');
            }
            for piece in pieces {
                let Some((key, value)) = piece.split_once(char::is_whitespace) else {
                    continue;
                };
                let key = key.trim_start_matches('@').to_ascii_lowercase();
                let value = value.trim().to_owned();
                if value.is_empty() {
                    continue;
                }
                match key.as_str() {
                    "name" => meta.name = Some(value),
                    "author" => meta.author = Some(value),
                    "description" => meta.description = Some(value),
                    "version" => meta.version = Some(value),
                    "license" => meta.license = Some(value),
                    "source" => meta.source = Some(value),
                    "website" => meta.website = Some(value),
                    _ => {}
                }
            }
        }
        meta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_resolve_through_each_other_and_take_fallbacks() {
        let mut vars = Vars::default();
        vars.insert(&Declaration {
            name: "--a".into(),
            value: "color-mix(in srgb, var(--b) 50%, transparent)".into(),
            important: false,
        });
        vars.insert(&Declaration {
            name: "--b".into(),
            value: "#e94221".into(),
            important: false,
        });
        assert_eq!(
            vars.color("--a", true),
            Some(Rgba::rgba(0xe9, 0x42, 0x21, 128))
        );
        assert_eq!(
            vars.resolve("var(--missing, 12px)").as_deref(),
            Some("12px")
        );
        assert_eq!(vars.resolve("var(--missing)"), None);
    }

    #[test]
    fn important_declarations_win_over_later_ones() {
        let mut vars = Vars::default();
        vars.insert(&Declaration {
            name: "--x".into(),
            value: "red".into(),
            important: true,
        });
        vars.insert(&Declaration {
            name: "--x".into(),
            value: "blue".into(),
            important: false,
        });
        assert_eq!(vars.get("--x"), Some("red"));
    }

    #[test]
    fn rules_keep_their_media_colour_scheme() {
        let sheet = parse(
            "@media (prefers-color-scheme: light) { :root { --x: #fff; } } :root { --y: #000; }",
            |_, _| None,
        );
        assert_eq!(sheet.rules.len(), 2);
        assert_eq!(sheet.rules[0].color_scheme, Some(false));
        assert_eq!(sheet.rules[1].color_scheme, None);
    }

    #[test]
    fn font_faces_carry_their_data_uri() {
        let sheet = parse(
            r#"@font-face { font-family: "Test"; font-weight: 700; src: url("data:font/ttf;base64,AAEC") format("truetype"); }"#,
            |_, _| None,
        );
        assert_eq!(sheet.fonts.len(), 1);
        assert_eq!(sheet.fonts[0].family, "Test");
        assert_eq!(sheet.fonts[0].weight, 700);
        assert_eq!(sheet.fonts[0].data.as_deref(), Some(&[0, 1, 2][..]));
    }

    #[test]
    fn only_root_selectors_count_for_the_palette() {
        assert_eq!(root_selector(":root", true), Some(true));
        assert_eq!(
            root_selector("html.theme-light:not([data-x])", true),
            Some(false)
        );
        assert_eq!(root_selector(".theme-dark", false), Some(true));
        assert_eq!(root_selector(".theme-dark .container", true), None);
        assert_eq!(root_selector("body", true), Some(true));
        assert_eq!(root_selector("#app-mount", true), Some(true));
        assert_eq!(root_selector(".container", true), None);
    }

    #[test]
    fn header_comments_are_metadata() {
        let meta = Meta::read(
            "/**\n * @name ReCord\n * @author leriart\n * @description Terminal\n */\n:root{}",
        );
        assert_eq!(meta.name.as_deref(), Some("ReCord"));
        assert_eq!(meta.author.as_deref(), Some("leriart"));
        assert_eq!(meta.description.as_deref(), Some("Terminal"));
    }

    #[test]
    fn comments_are_stripped_but_strings_are_not() {
        let text = strip_comments("a{content:\"/*x*/\";/*y*/color:red}");
        assert!(text.contains("\"/*x*/\""));
        assert!(!text.contains("/*y*/"));
    }
}
