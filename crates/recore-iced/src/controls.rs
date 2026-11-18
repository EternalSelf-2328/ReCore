//! The kit's controls with ReCord's interface feedback attached: a form's
//! action sends a hover tick as well as its own message.

pub use recore_iced_kit::controls::*;

use std::borrow::Cow;

use crate::{app::Message, theme::Theme};

impl recore_iced_kit::controls::HoverFeedback for Message {
    fn hover_feedback() -> Option<Self> {
        Some(Message::Interface(
            recore_core::sounds::InterfaceSound::Hover,
        ))
    }
}

/// A form button, as [`recore_iced_kit::controls::action`], with ReCord's
/// hover sound while it can be pressed.
pub fn action<'a>(
    theme: &Theme,
    label: impl Into<Cow<'a, str>>,
    kind: Kind,
    message: Option<Message>,
) -> recore_iced_kit::controls::Action<'a, Message> {
    let enabled = message.is_some();
    let button = recore_iced_kit::controls::action(theme, label, kind, message);
    if enabled {
        button.on_hover(Message::Interface(
            recore_core::sounds::InterfaceSound::Hover,
        ))
    } else {
        button
    }
}
