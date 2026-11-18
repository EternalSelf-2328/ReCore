//! ReCore's reader for Discord CSS themes: Vencord, Vesktop, Equicord,
//! BetterDiscord, Legcord and any other client that loads a `.theme.css`
//! file of custom properties.
//!
//! The interpreter reads the theme's palette, accent, fonts, radius,
//! background picture and ReCord's CRT options into [`ThemeFile`], which the
//! window turns into its own colours. CSS that styles Discord's web DOM has no
//! nodes in a native window; those rules are read without error and simply
//! not painted. [`theme_folders`] lists the places the common clients keep
//! local themes, and [`scan`] reads them.
//!
//! ```
//! use recore_css::{Mode, Options, load_str};
//!
//! let css = ":root { --background-primary: #101014; --brand-experiment: #e94221; }";
//! let theme = load_str(css, None, &Options::with_mode(Mode::Dark)).unwrap();
//! assert_eq!(theme.palette.accent.hex(), "#e94221");
//! ```

mod base64;
pub mod color;
pub mod css;
mod options;
mod theme;

pub use color::Rgba;
pub use css::{Meta, ParseError, Stylesheet};
pub use options::{OptionKind, ThemeOption};
pub use theme::{
    Effects, Font, Image, Look, Mode, Motion, Options, Palette, Problem, Scan, ThemeFile, load,
    load_str, load_with, palette, scan, theme_folders,
};

/// The version of the interpreter, shown beside a loaded theme.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_with_no_colours_still_gets_a_usable_palette() {
        let theme = load_str(
            ":root { --font-primary: monospace; }",
            None,
            &Options::default(),
        )
        .expect("readable");
        assert!(theme.palette.chat.contrast(theme.palette.text) > 4.0);
        assert_eq!(theme.look.font_family, None);
    }

    #[test]
    fn the_cache_of_theme_folders_is_only_paths() {
        for folder in theme_folders() {
            assert!(folder.file_name().is_some());
        }
    }
}
