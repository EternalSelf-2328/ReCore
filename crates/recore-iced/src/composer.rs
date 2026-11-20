//! The composer under the timeline: the message request's answer in its
//! place, the reply's bar over it (with whether the reply mentions its
//! author), the field (`field::Field`) between the + and the emoji button,
//! suggestions over it (`suggest.rs`), and the hint under it. The draft is
//! the core's, per channel (`Action::SetDraft`), which tells the room this
//! account is typing; the field follows it when the core changes it, as
//! when a sent message empties it. A message of one's own is edited in its
//! row (`edit.rs`), in a field of its own beside the draft.

use iced::{
    Alignment, Color, Element, Length, Padding, Rectangle, Size, Task,
    advanced::mouse,
    keyboard,
    widget::{Id, Space, button, column, container, operation, row, stack, text, tooltip},
};
use recore_core::{
    application::{Action, Compose},
    network::Network,
};

use crate::{
    app::{App, Message, NOBODY},
    controls::{self, lit_style},
    field::{self, Field, MenuEdit},
    fonts::BOLD,
    menus::{Menu, MenuAct, MenuRow},
    picker::{Tab, Target},
    pictures::Icon,
    popover::{Popover, Side},
    style::{corners, faded, rounded},
};

type Renderer = iced::Renderer;

/// The composer's field and the inline edit's, for the app's focus tasks.
pub const COMPOSER: &str = "composer";
pub const EDIT: &str = "edit";
/// The reply bar's height.
pub const BAR: f32 = 36.0;
/// The composer's least height (`ClientStyle.composer-height`), the room
/// over and under its text, beside it, and under its buttons.
pub const HEIGHT: f32 = 58.0;
pub const PAD: f32 = 17.0;
pub const GUTTER: f32 = 8.0;
pub const BUTTON_INSET: f32 = 13.0;
/// The hint's line under the composer.
const HINT: f32 = 22.0;

/// Which field is written in: the room's draft, or a message's edit.
/// The composer, the edit in its row and the suggestions over them.
#[derive(Default)]
pub struct Composer {
    /// The composer's text and history: the core's draft for the open room.
    pub draft: crate::field::Draft,
    /// The draft the core last had, to tell its own changes from ours.
    pub draft_known: String,
    /// Whether the composer's field has the keys, which its outline shows.
    pub focused: bool,
    /// The message being edited in its row (the core's `edit`), and the
    /// edit's own field.
    pub editing: Option<String>,
    pub edit: crate::field::Draft,
    /// The suggestions over the field typed in last, and the spelling
    /// suggestion under it.
    pub suggest: crate::suggest::Suggest,
    pub spelling: Option<crate::spelling::Suggestion>,
    /// The composer written in last, the channel's or the thread's.
    pub writing_last: Writing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Writing {
    #[default]
    Draft,
    Edit,
    /// The thread beside the channel's composer, the New Thread form's
    /// first message, and a new forum post's (`thread.rs`, `forum.rs`).
    Aside,
    NewThread,
    NewPost,
}

/// What the composer's parts send.
#[derive(Clone, Debug)]
pub enum Event {
    /// A field's keys and text.
    Field(Writing, field::Event),
    /// The clipboard's text, for a field's paste.
    Pasted(Writing, Option<String>),
    /// A suggestion clicked, by its place.
    Suggestion(usize),
    /// The spelling suggestion under the composer, taken or dismissed.
    Spelling,
    DismissSpelling,
    CancelReply,
    TogglePing,
    /// The message request's Accept (true) or Ignore.
    Answer(bool),
    /// The +.
    Plus,
    /// The edit's hint: cancel or save; its emoji button.
    CancelEdit,
    SaveEdit,
    EditEmoji,
    /// A message of one's own that didn't go: send it again, or not; an
    /// answer only this account sees, dismissed.
    Retry(String),
    CancelSend(String),
    Dismiss(String),
    /// The silent-message toggle: Discord's `@silent` at the start.
    Silent,
    /// Translate messages before sending, turned on or off.
    ToggleTranslate,
    /// Files to go with the message.
    Uploads(crate::uploads::Event),
    /// An app command's pill clicked: its option, or a file option's file;
    /// the file chosen.
    CommandOption(usize),
    CommandFile(usize),
    CommandFileChosen(usize, String),
    /// The commands' list scrolled to this far down.
    CommandsScrolled(f32),
}

/// What the composer shows for the open room, from the core's state.
pub struct Look {
    pub enabled: bool,
    /// Discord's one bar in place of the composer, where this account may
    /// not write.
    pub locked: bool,
    /// The line under it: why it can't send, or nothing; with Matrix,
    /// whether the room is end-to-end encrypted, under a lock if it is.
    pub hint: String,
    pub encrypted: bool,
    /// A message request's title, answered in place of the composer.
    pub request: Option<String>,
    pub attach: bool,
    /// The + opens a menu (threads or polls can start here).
    pub plus: bool,
    /// A Discord room takes a silent message.
    pub discord: bool,
    /// The draft starts with Discord's `@silent`.
    pub silent: bool,
}

/// A reply's bar over a composer: "Replying to" its author and its first
/// line, and whether it mentions them.
pub fn reply_label(compose: Option<&Compose>) -> Option<(String, bool)> {
    let Compose::Reply {
        author, body, ping, ..
    } = compose?;
    Some((
        format!(
            "Replying to {author}: {}",
            body.lines().next().unwrap_or_default()
        ),
        *ping,
    ))
}

/// The composer box's top in the window, where what opens over it sits.
/// The top of the composer's buttons, which stay on its last line:
/// the expression picker opens 8 px over them.
pub fn composer_buttons_top(app: &App) -> f32 {
    app.window.height - composer_foot(app) - BUTTON_INSET - 32.0
}

/// The room under the composer's box: its hint, or the gutter.
fn composer_foot(app: &App) -> f32 {
    if composer_look(app.ctx()).hint.is_empty() {
        GUTTER
    } else {
        HINT
    }
}

pub fn composer_top(app: &App) -> f32 {
    let foot = composer_foot(app);
    let tray = if app.client.state().staged().is_empty() {
        0.0
    } else {
        crate::uploads::TRAY
    };
    app.window.height - foot - app.composer.draft.height().max(HEIGHT) - tray
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Field(which, event) => field_event(app, which, event),
        Event::Pasted(which, pasted) => {
            if pasted.is_some() {
                let draft = writing(app.ctx(), which);
                draft.act(MenuEdit::Paste, pasted);
                let (text, caret) = (draft.text(), draft.caret());
                edited(app, which, text, caret);
            }
            Task::none()
        }
        Event::Suggestion(index) => {
            // The keys go back to the field the suggestion was for: a
            // side composer's, the form's, the edit's or the channel's.
            let which = app.composer.suggest.field;
            app.composer.suggest.set_index(index);
            Task::batch([
                crate::suggest::choose_suggestion(app, which),
                operation::focus(crate::thread::field_id(which)),
            ])
        }
        Event::Spelling => {
            crate::spelling::apply(app);
            operation::focus(COMPOSER)
        }
        Event::DismissSpelling => {
            app.composer.spelling = None;
            operation::focus(COMPOSER)
        }
        Event::CancelReply => {
            app.client.dispatch(Action::CancelCompose);
            Task::batch([app.sync(), refocus(app)])
        }
        Event::TogglePing => {
            app.client
                .dispatch(Action::ToggleReplyPing { aside: false });
            Task::batch([app.sync(), refocus(app)])
        }
        Event::Answer(accept) => {
            app.client.dispatch(Action::AnswerRequest(accept));
            Task::batch([app.sync(), refocus(app)])
        }
        Event::Silent => {
            let text = toggle_silent(&app.composer.draft.text());
            app.composer.draft.reset(&text);
            app.client.dispatch(Action::SetDraft(text));
            Task::batch([app.sync(), operation::focus(COMPOSER)])
        }
        Event::ToggleTranslate => {
            let on = !app.layout.prefs.translate_send.unwrap_or(false);
            app.layout.prefs.translate_send = Some(on);
            crate::layout::store(app.client.root(), "translate_send", on);
            app.interface_sound(recore_core::sounds::InterfaceSound::Toggle);
            operation::focus(COMPOSER)
        }
        Event::Plus => plus(app),
        Event::CancelEdit => {
            app.client.dispatch(Action::CancelEdit);
            Task::batch([app.sync(), operation::focus(COMPOSER)])
        }
        Event::SaveEdit => crate::edit::save_edit(app),
        Event::EditEmoji => crate::picker::open_picker(app, Target::Edit, Tab::Emoji),
        Event::Retry(item) => {
            app.client.dispatch(Action::Retry(item));
            app.sync()
        }
        Event::CancelSend(item) => {
            app.client.dispatch(Action::CancelSend(item));
            app.sync()
        }
        Event::Dismiss(item) => {
            app.client.dispatch(Action::DismissEphemeral(item));
            app.sync()
        }
        Event::Uploads(event) => crate::uploads::update(app, event),
        Event::CommandOption(index) => {
            Task::batch([crate::commands::focus_option(app, index), refocus(app)])
        }
        Event::CommandFile(index) => crate::commands::command_file(index),
        Event::CommandFileChosen(index, value) => {
            app.client
                .dispatch(Action::SetCommandOption { index, value });
            app.sync()
        }
        Event::CommandsScrolled(offset) => {
            app.composer.suggest.command_offset = offset;
            Task::none()
        }
    }
}

/// The keys go back to the field being written in: the edit while one
/// is open, else the composer; not while a server's or a channel's
/// settings cover it, as after a member's profile from Members.
pub fn refocus(app: &App) -> Task<Message> {
    // A screen or dialog over the window keeps the keys itself.
    if app.server_screen.is_open()
        || app.channel_screen.is_open()
        || crate::server_dialogs::is_open(app.client.state())
        || crate::link_dialogs::is_open(app.client.state())
    {
        return Task::none();
    }
    // The picture viewer and a video full screen keep them from the
    // fields under them.
    if app.client.state().viewer.is_some() || app.playback.clip_full {
        return operation::focus(crate::app::NOBODY);
    }
    // A popover's field, while it's open.
    if app.client.state().thread_list().is_some() {
        return operation::focus(crate::header_popovers::THREADS_SEARCH);
    }
    if crate::header_popovers::popover_open(app) {
        return operation::focus(crate::header_popovers::PEOPLE_FIELD);
    }
    operation::focus(if app.composer.editing.is_some() {
        EDIT
    } else if writing_aside(app) {
        crate::thread::COMPOSER
    } else if app.client.state().new_post().is_some() {
        // A forum has no composer: its New Post's body has the keys.
        crate::forum::BODY
    } else {
        COMPOSER
    })
}

/// A key nothing took: one that writes goes on the end of the composer
/// written in last (the thread beside's or the channel's), which takes the
/// keys, as Discord's window sends typing anywhere to its composer; Tab
/// only gives it the keys. Not under a card or a popup, nor push to talk's
/// own key in a call; stepping through the messages, only letters and
/// digits leave them.
pub fn type_anywhere(
    app: &mut App,
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Task<Message> {
    let typed = field::Typed::of(key, modifiers);
    let tab = *key == keyboard::Key::Named(keyboard::key::Named::Tab)
        && !modifiers.command()
        && !modifiers.alt();
    if (typed.is_none() && !tab) || composer_covered(app) {
        return Task::none();
    }
    if app.keyed.is_some() {
        let letters = matches!(&typed, Some(field::Typed::Text(text))
            if text.chars().all(char::is_alphanumeric));
        if !letters {
            return Task::none();
        }
        crate::timeline::keys::leave_messages(app);
    }
    // Holding the key to talk shouldn't also write it: only that key, by
    // where it is, while it sends.
    let voice = &app.client.state().voice;
    if app.talk_pressed && voice.active() && !voice.muted {
        return Task::none();
    }
    let (which, field) = if writing_aside(app) {
        let Some(aside) = &app.aside else {
            return Task::none();
        };
        if !app.client.state().can_write_in(&aside.room) {
            return Task::none();
        }
        (Writing::Aside, crate::thread::COMPOSER)
    } else {
        if !composer_look(app.ctx()).enabled || app.composer.editing.is_some() {
            return Task::none();
        }
        (Writing::Draft, COMPOSER)
    };
    if let Some(typed) = typed {
        let draft = writing(app.ctx(), which);
        draft.type_at_end(&typed);
        let (text, caret) = (draft.text(), draft.caret());
        edited(app, which, text, caret);
    }
    operation::focus(field)
}

/// Something over the window that keys typed with nothing focused
/// aren't for: a card, a popup, the picker or a full view.
fn composer_covered(app: &App) -> bool {
    let state = app.client.state();
    app.profiles.card.is_some()
        || app.profiles.shown.is_some()
        || app.cards.server.is_some()
        || app.cards.expression.is_some()
        || state.role.is_some()
        || app.win.picker.is_open()
        || app.call.popup.is_some()
        || app.playback.clip_full
        || crate::call::stage::full_shown(app)
        || app.inbox.is_some()
        || state.select_menu.is_some()
        || state.forwarding.is_some()
        || state.viewer.is_some()
}

/// The thread beside the channel's composer was written in last, and
/// shows: keys typed anywhere go to it, and Escape cancels its reply.
pub fn writing_aside(app: &App) -> bool {
    app.composer.writing_last == Writing::Aside
        && app.aside.is_some()
        && crate::thread::aside_shown(app)
}

fn field_event(app: &mut App, which: Writing, event: field::Event) -> Task<Message> {
    // The keys come back to the composer written in last: the thread's
    // or the channel's.
    if matches!(which, Writing::Draft | Writing::Aside) {
        app.composer.writing_last = which;
    }
    match event {
        field::Event::Edited(text, caret) => {
            app.interface_sound(recore_core::sounds::InterfaceSound::Type);
            edited(app, which, text, caret);
            Task::none()
        }
        field::Event::Composing => {
            crate::spelling::composing(app, which);
            Task::none()
        }
        field::Event::Moved(caret) => {
            let text = writing(app.ctx(), which).text();
            crate::suggest::caret_moved(app, which, &text, caret);
            Task::none()
        }
        field::Event::Send => {
            app.interface_sound(recore_core::sounds::InterfaceSound::Send);
            match which {
                Writing::Draft => send(app),
                Writing::Edit => crate::edit::save_edit(app),
                Writing::Aside => crate::thread::update(app, crate::thread::Event::Send),
                Writing::NewThread => crate::thread::update(app, crate::thread::Event::Create),
                Writing::NewPost => crate::forum::update(app, crate::forum::Event::Create),
            }
        }
        field::Event::Step(step) => {
            let count = crate::suggest::suggestion_count(app);
            app.composer.suggest.step(step, count);
            if which == Writing::Draft && crate::commands::command_list(app.ctx()).is_some() {
                let lit = app.composer.suggest.lit(count);
                return crate::commands::keep_lit_in_view(app, lit, count);
            }
            Task::none()
        }
        field::Event::Choose => {
            if which == Writing::Draft
                && crate::suggest::suggestion_count(app) == 0
                && crate::spelling::apply(app)
            {
                return Task::none();
            }
            crate::suggest::choose_suggestion(app, which)
        }
        field::Event::Escape => {
            if crate::suggest::dismiss_suggestions(app) {
                return Task::none();
            }
            // With no list over it, Escape stops writing a command and
            // empties the composer.
            if which == Writing::Draft && crate::commands::close_command(app, false) {
                return app.sync();
            }
            // Under a server's or a channel's settings, Escape is
            // theirs: it comes to them as a key.
            if app.server_screen.is_open() || app.channel_screen.is_open() {
                return Task::none();
            }
            // Escape in the thread's composer cancels its reply first.
            if which == Writing::Aside && app.client.state().aside_compose().is_some() {
                return crate::thread::update(app, crate::thread::Event::CancelReply);
            }
            if matches!(
                which,
                Writing::Aside | Writing::NewThread | Writing::NewPost
            ) {
                return app.escape();
            }
            // Escape in the edit's field ends the edit.
            if which == Writing::Edit {
                app.client.dispatch(Action::CancelEdit);
                return Task::batch([app.sync(), operation::focus(COMPOSER)]);
            }
            app.escape()
        }
        field::Event::Emoji => match which {
            Writing::Draft => crate::picker::open_picker(app, Target::Composer, Tab::Emoji),
            Writing::Edit => crate::picker::open_picker(app, Target::Edit, Tab::Emoji),
            _ => Task::none(),
        },
        // Ctrl+G opens the GIFs, as Discord's; again, it closes them.
        field::Event::Gifs if which == Writing::Draft && app.client.state().can_pick_gifs() => {
            crate::picker::open_picker(app, Target::Composer, Tab::Gifs)
        }
        field::Event::Gifs => Task::none(),
        field::Event::Menu(at) => {
            let row = MenuRow::new;
            let mut rows = Vec::new();
            if writing(app.ctx(), which).selection().is_some() {
                rows.push(row(
                    MenuAct::Field(which, MenuEdit::Cut),
                    "Cut",
                    Icon::Scissors,
                ));
                rows.push(row(
                    MenuAct::Field(which, MenuEdit::Copy),
                    "Copy",
                    Icon::Copy,
                ));
            }
            rows.push(row(
                MenuAct::Field(which, MenuEdit::Paste),
                "Paste",
                Icon::Paste,
            ));
            rows.push(row(
                MenuAct::Field(which, MenuEdit::SelectAll),
                "Select All",
                Icon::TextSelect,
            ));
            app.win.menu = Some(Menu::new(at, rows).width(crate::menus::FIELD_WIDTH));
            operation::focus(NOBODY)
        }
        // Up in an empty composer edits the last message of one's own: the
        // core's rule, which leaves Up to a command being written.
        field::Event::UpFromEmpty => {
            if which != Writing::Draft {
                return Task::none();
            }
            app.client.dispatch(Action::EditLast);
            crate::edit::take_edit(app)
        }
        field::Event::UpToMessages => crate::timeline::keys::step_into_messages(app),
        field::Event::Paste => paste(app, which),
        field::Event::Focus(focused) => {
            if which == Writing::Draft {
                app.composer.focused = focused;
            }
            if focused {
                crate::timeline::keys::leave_messages(app);
            }
            Task::none()
        }
        field::Event::Tab(back) => crate::commands::command_tab(app, back),
        field::Event::BackFromEmpty => {
            if crate::commands::close_command(app, true) {
                return app.sync();
            }
            Task::none()
        }
    }
}

/// Ctrl+V and the menu's Paste: in the composer, files copied in a file
/// manager or a picture go with the message, and otherwise the text is
/// pasted; the edit's field takes text alone. The clipboard is read for
/// files even with the message full, so the core says why they can't go;
/// where files never go (the preview, a channel not to write in), it's
/// the text that is pasted.
fn paste(app: &App, which: Writing) -> Task<Message> {
    use crate::uploads::Event as Uploads;
    let state = app.client.state();
    let full = |staged: &[recore_core::chat::Staged]| staged.len() >= recore_core::chat::MAX_FILES;
    let files_go = match which {
        Writing::Draft => state.can_attach() || full(state.staged()),
        Writing::Aside => state.aside_can_attach() || full(state.aside_staged()),
        _ => false,
    };
    if files_go && let Some(read) = crate::desktop::desktop().paste_reader() {
        return Task::perform(
            async move { tokio::task::spawn_blocking(read).await.ok().flatten() },
            move |files| {
                Message::Write(Event::Uploads(if which == Writing::Aside {
                    Uploads::ClipboardAside(files)
                } else {
                    Uploads::Clipboard(files)
                }))
            },
        );
    }
    iced::clipboard::read().map(move |text| Message::Write(Event::Pasted(which, text)))
}

/// The + opens Discord's menu where threads or polls can start, and the
/// file dialog elsewhere.
fn plus(app: &mut App) -> Task<Message> {
    let look = composer_look(app.ctx());
    if !look.plus {
        return crate::uploads::attach(app);
    }
    let mut rows = vec![MenuRow {
        disabled: !look.attach,
        subtitle: None,
        check: crate::controls::Check::None,
        ..MenuRow::new(MenuAct::Upload, "Upload a File", Icon::File)
    }];
    if app.client.app.can_start_threads() {
        rows.push(MenuRow {
            disabled: false,
            subtitle: None,
            check: crate::controls::Check::None,
            ..MenuRow::new(MenuAct::NewThread, "Create Thread", Icon::Thread)
        });
    }
    if app.client.app.can_create_polls() {
        rows.push(MenuRow {
            disabled: false,
            subtitle: None,
            check: crate::controls::Check::None,
            ..MenuRow::new(MenuAct::NewPoll, "Create Poll", Icon::ChartBar)
        });
    }
    let at = iced::Point::new(
        crate::conversation::chat_left(app) + GUTTER + 10.0,
        composer_top(app) - 8.0,
    );
    app.win.menu = Some(Menu::new(at, rows).anchor(crate::widgets::Anchor::Bottom));
    operation::focus(NOBODY)
}

/// The draft with Discord's `@silent` at its start taken off, or put on.
pub(crate) fn toggle_silent(text: &str) -> String {
    match text.strip_prefix("@silent") {
        Some(rest) if rest.is_empty() || rest.starts_with(char::is_whitespace) => {
            rest.trim_start().to_owned()
        }
        _ => format!("@silent {text}"),
    }
}

/// Enter: the draft sent. The core empties the draft once the message is
/// sent; `sync` follows it. With Translate messages before sending on, the
/// draft goes to Google first and [`send_text`] sends what comes back.
fn send(app: &mut App) -> Task<Message> {
    if app.client.state().selected.is_none() {
        return Task::none();
    }
    if let Some(task) = crate::commands::command_enter(app) {
        return task;
    }
    let commanding = app.client.state().command_draft().is_some();
    let draft = app.composer.draft.text();
    if !commanding && app.layout.prefs.translate_send.unwrap_or(false) && !draft.trim().is_empty() {
        return crate::translate::send_out(app, draft);
    }
    send_draft(app, draft, commanding)
}

/// Sends `text` as the room's draft, as Enter does once a translation has
/// come back.
pub(crate) fn send_text(app: &mut App, text: String) -> Task<Message> {
    send_draft(app, text, false)
}

fn send_draft(app: &mut App, text: String, commanding: bool) -> Task<Message> {
    if app.client.state().selected.is_none() {
        return Task::none();
    }
    if !commanding {
        app.client.dispatch(Action::SetDraft(text));
    }
    // Sending goes to the newest message, as Discord does, so the sent
    // one shows; from a window of older messages, once the newest have
    // replaced it.
    let mut to_end = Task::none();
    if app.client.state().can_send() {
        if app.client.state().detached {
            app.timeline.latest_due = true;
        } else {
            to_end = crate::timeline::scroll::go_to_end(app);
        }
    }
    app.client.dispatch(Action::Send);
    app.composer.suggest.clear(&mut app.client);
    Task::batch([to_end, app.sync()])
}

/// A field's menu chose Cut, Copy, Paste or Select All.
pub fn field_menu(app: &mut App, which: Writing, edit: MenuEdit) -> Task<Message> {
    app.win.menu = None;
    let focus = operation::focus(crate::thread::field_id(which));
    if edit == MenuEdit::Paste {
        return Task::batch([focus, paste(app, which)]);
    }
    let draft = writing(app.ctx(), which);
    let copied = draft.act(edit, None);
    if edit == MenuEdit::Cut {
        let (text, caret) = (draft.text(), draft.caret());
        edited(app, which, text, caret);
    }
    match copied {
        Some(copied) => Task::batch([focus, iced::clipboard::write(copied)]),
        None => focus,
    }
}

/// A field's text changed: the core's draft or edit follows, and so do the
/// suggestions. A draft changed tells the room this account is typing (the
/// core's).
pub fn edited(app: &mut App, which: Writing, text: String, caret: usize) {
    keep_text(app, which, text.clone());
    // The core may have kept something else: the field shows that.
    let kept = writing(app.ctx(), which).text();
    let caret = if kept == text {
        caret
    } else {
        writing(app.ctx(), which).caret()
    };
    crate::suggest::caret_moved(app, which, &kept, caret);
    app.composer.spelling = (which == Writing::Draft
        && app.layout.prefs.word_suggestions.unwrap_or(true))
    .then(|| crate::autocorrect::for_draft(app, &kept, caret))
    .flatten();
}

/// The core's draft or edit takes a field's text.
fn keep_text(app: &mut App, which: Writing, text: String) {
    match which {
        Writing::Edit => app.client.dispatch(Action::SetEditText(text)),
        Writing::Draft if app.client.state().selected.is_some() => {
            app.client.dispatch(Action::SetDraft(text.clone()));
            app.composer.draft_known = app.client.state().draft().to_owned();
            // The core took the text as something else, as a command's
            // name and a space picks it: the field shows what it kept.
            if app.composer.draft_known != text {
                app.composer.draft.reset(&app.composer.draft_known);
            }
        }
        Writing::Draft => {}
        Writing::Aside => {
            app.client.dispatch(Action::SetAsideDraft(text));
            app.drafts.aside_known = app.client.state().aside_draft().to_owned();
        }
        Writing::NewThread => {
            app.client.dispatch(Action::WriteNewThread {
                name: None,
                body: Some(text.clone()),
            });
            app.drafts.first_known = text;
        }
        Writing::NewPost => crate::forum::forum_body(app, text),
    }
}

/// Puts `text` in a field with the caret at `caret`, and in the core's
/// draft or edit with it.
pub fn replace_text(app: &mut App, which: Writing, text: String, caret: usize) {
    writing(app.ctx(), which).replace(&text, caret);
    keep_text(app, which, text);
}

/// An emoji's text goes in at the field's caret.
pub fn insert(app: &mut App, which: Writing, inserted: &str) {
    let draft = writing(app.ctx(), which);
    let text = draft.text();
    let at = draft.caret().min(text.len());
    let text = format!("{}{inserted}{}", &text[..at], &text[at..]);
    let caret = at + inserted.len();
    replace_text(app, which, text.clone(), caret);
    crate::suggest::caret_moved(app, which, &text, caret);
}

/// A composer button, kept [`BUTTON_INSET`] off the box's foot, so it stays
/// on the last line as the field grows.
fn lift<'a>(button: Element<'a, Message>) -> Element<'a, Message> {
    container(button)
        .padding(Padding::ZERO.bottom(BUTTON_INSET))
        .into()
}

/// What the composer shows for the open room.
pub fn composer_look(ctx: crate::ctx::Ctx<'_>) -> Look {
    let state = ctx.state;
    let room = state.selected_room();
    let discord = room.is_some_and(|room| room.network == Network::Discord);
    let enabled = state.room_ready
        && !state.busy
        && state.shows_text()
        && room.is_some_and(|room| {
            if discord {
                state.writable(room)
            } else {
                state.preview || room.encrypted
            }
        });
    let timed_out = room
        .and_then(|room| room.timed_out_until)
        .filter(|_| discord)
        .map(|until| {
            format!(
                "You're timed out in this server until {}.",
                crate::place_menus::until(until / 1000, ctx.layout.prefs.twelve_hour_clock)
            )
        });
    let request = room.filter(|room| room.request).map(|room| {
        if room.category == recore_core::discord::SPAM {
            format!("Discord thinks {} may be spam", room.name)
        } else {
            format!("{} wants to message you", room.name)
        }
    });
    let locked = timed_out.is_none()
        && !state.preview
        && discord
        && room.is_some_and(|room| !room.request && (room.read_only || room.locked));
    // Without Matrix everything is Discord, so the lines on encryption
    // say nothing new; what stops sending still shows.
    let discord_only = !cfg!(feature = "matrix");
    let hint = if room.is_none() {
        "Choose a DM or server channel to write a message."
    } else if state.preview {
        ""
    } else if request.is_some() {
        if discord_only {
            ""
        } else {
            "Discord · not end-to-end encrypted."
        }
    } else if let Some(timed_out) = &timed_out {
        timed_out
    } else if locked {
        ""
    } else if discord && !state.discord_online {
        "Discord is disconnected. ReCore is reconnecting."
    } else if discord_only {
        ""
    } else if discord {
        "Discord · not end-to-end encrypted. Enter to send, Shift+Enter for a new line."
    } else if !state.trusted {
        "Sending is locked until this device is verified."
    } else if room.is_some_and(|room| room.encrypted) {
        "End-to-end encrypted. Enter to send, Shift+Enter for a new line."
    } else {
        "Unencrypted room. Reading only in this build."
    }
    .to_owned();
    Look {
        enabled,
        locked,
        encrypted: room.is_some_and(|room| room.encrypted),
        hint,
        request,
        attach: state.can_attach(),
        plus: ctx.app.can_start_threads() || ctx.app.can_create_polls(),
        discord,
        silent: ctx.composer.draft.starts_with("@silent")
            && ctx
                .composer
                .draft
                .char_at(7)
                .is_none_or(char::is_whitespace),
    }
}

/// An icon button in `tint`, greyed without a message.
pub fn composer_button(
    ctx: crate::ctx::Ctx<'_>,
    icon: Icon,
    label: &'static str,
    glyph: u32,
    tint: iced::Color,
    message: Option<Message>,
) -> Element<'static, Message> {
    let tint = if message.is_some() {
        tint
    } else {
        faded(tint, 0.45)
    };
    let enabled = message.is_some();
    let face = ctx.look.glyph(icon, glyph, tint).boxed(32);
    let face = button(face)
        .padding(0)
        .width(32)
        .height(32)
        .on_press_maybe(message)
        .style(lit_style(None, ctx.look.theme.wash(0.1), 6.0));
    let face = tooltip(
        face,
        controls::tip(ctx.look.theme, label),
        tooltip::Position::Top,
    )
    .gap(6);
    // ReCord's composer tools tick under the pointer.
    if enabled {
        iced::widget::mouse_area(face)
            .on_enter(Message::Interface(
                recore_core::sounds::InterfaceSound::Hover,
            ))
            .into()
    } else {
        face.into()
    }
}

pub fn view<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let look = composer_look(ctx);
    // The bars over the composer keep their places whether they show or
    // not: iced matches a column's children by position, so a bar pushed
    // only while it shows would move the field to another slot, and the
    // field's state (the focus, and so Tab, Enter and the edits with it)
    // would be made anew on the frame the bar appears. An empty slot holds
    // the place and takes no room.
    let mut compose = column![];
    if let Some(title) = &look.request {
        compose = compose.push(request_bar(ctx, title));
    } else {
        compose = compose.push(match reply_label(ctx.state.compose()) {
            Some((label, ping)) => reply_bar(ctx, label, ping),
            None => Space::new().into(),
        });
        let offered = ctx.layout.prefs.word_suggestions.unwrap_or(true);
        compose = compose.push(match crate::spelling::current(ctx.composer) {
            Some(suggestion) if offered => spelling_bar(ctx, suggestion),
            _ => Space::new().into(),
        });
        compose = compose.push(if look.locked {
            locked_bar(ctx)
        } else {
            composer_box(ctx, &look)
        });
    }
    let foot: Element<'a, Message> = if look.hint.is_empty() {
        Space::new().height(GUTTER).into()
    } else {
        let muted = ctx.look.theme.muted;
        let mut line = row![].spacing(6).align_y(Alignment::Center);
        if look.encrypted {
            line = line.push(ctx.look.glyph(Icon::Lock, 12, muted));
        }
        line = line.push(text(look.hint).size(13).color(muted));
        container(line)
            .height(HINT)
            .padding(Padding::ZERO.left(4))
            .align_y(Alignment::Center)
            .into()
    };
    container(compose.push(foot))
        .padding(Padding::ZERO.left(GUTTER).right(GUTTER))
        .into()
}

/// The word under the composer's caret, offered with its accents: Tab takes
/// it, and so does a press on it.
fn spelling_bar<'a>(
    ctx: crate::ctx::Ctx<'a>,
    suggestion: &'a crate::spelling::Suggestion,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let radius = if theme.square { 0.0 } else { 4.0 };
    let line = row![
        text("Did you mean").size(12).color(theme.muted),
        button(
            text(suggestion.fixed())
                .size(13)
                .font(BOLD())
                .color(theme.accent)
        )
        .padding([1, 5])
        .on_press(Message::Write(Event::Spelling))
        .style(lit_style(None, theme.wash(0.08), radius)),
        text("Tab").size(11).color(theme.faint),
        button(ctx.look.glyph(Icon::X, 12, theme.faint))
            .padding(2)
            .on_press(Message::Write(Event::DismissSpelling))
            .style(lit_style(None, theme.wash(0.08), radius)),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    container(line)
        .height(28)
        .padding(Padding::ZERO.left(8).right(4))
        .width(Length::Fill)
        .align_y(Alignment::Center)
        .style(move |_| crate::style::fill(theme.sunken))
        .into()
}
/// The box with the field and its buttons, and the suggestions over it.
fn composer_box<'a>(ctx: crate::ctx::Ctx<'a>, look: &Look) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let header = ctx.header;
    let replying = ctx.state.compose().is_some();
    // Discord's words: `Message @sam` in a DM, `Message #general` in a
    // channel, and a group's or thread's own name as it is.
    let name = &header.name;
    let placeholder = if replying {
        "Write a reply".to_owned()
    } else {
        match (header.direct, header.one_to_one, header.thread) {
            (true, true, _) => format!("Message @{name}"),
            (false, _, false) => format!("Message #{name}"),
            _ => format!("Message {name}"),
        }
    };
    let commanding = ctx.state.command_draft().is_some();
    let placeholder = crate::commands::command_placeholder(ctx).unwrap_or(placeholder);
    let field = Field::new(
        &ctx.composer.draft,
        Id::new(COMPOSER),
        placeholder,
        |event| Message::Write(Event::Field(Writing::Draft, event)),
        field::Colors {
            ground: iced::Color::TRANSPARENT,
            text: theme.text,
            placeholder: if ctx.themes.square_avatars() {
                theme.faint
            } else {
                theme.placeholder
            },
            selection: theme.selection,
        },
    )
    .size(crate::settings::message_size(ctx.layout.prefs.text_size))
    .padding(Padding {
        // Under the pills, 10 px from them.
        top: if commanding { 10.0 } else { PAD },
        right: 6.0,
        bottom: PAD,
        left: 6.0,
    })
    .enabled(look.enabled)
    .pastes()
    .spelling(
        ctx.layout.prefs.word_suggestions.unwrap_or(true)
            && crate::spelling::current(ctx.composer).is_some(),
    )
    .tabs(commanding)
    .suggesting(crate::suggest::suggesting(ctx, Writing::Draft));
    // Always in a column after the same kind of widget, so the field
    // keeps its state (the keys) as the pills come and go.
    let pills = crate::commands::command_pills(ctx);
    let field = column![pills, field];
    // A tab's button is lit while its tab shows, the emoji's also while
    // the emoji picker alone is open over the composer.
    let shown = ctx
        .picker
        .filter(|picker| picker.target == Target::Composer)
        .map(|picker| picker.tab);
    let tint = |tab: Tab| {
        if shown == Some(tab) {
            theme.text
        } else {
            theme.secondary
        }
    };
    let plus = composer_button(
        ctx,
        Icon::Add,
        if look.plus {
            "More options"
        } else {
            "Attach files"
        },
        27,
        theme.secondary,
        (look.attach || look.plus).then_some(Message::Write(Event::Plus)),
    );
    let opens = |tab: Tab| {
        look.enabled
            .then_some(Message::OpenPicker(Target::Composer, tab))
    };
    // As Discord's: the GIFs, the stickers, then the emoji, in one
    // picker, in a Discord room.
    let mut buttons = row![].spacing(6).align_y(Alignment::End);
    let state = ctx.state;
    if state.can_pick_gifs() {
        buttons = buttons.push(lift(composer_button(
            ctx,
            Icon::Gif,
            "Open GIF picker",
            22,
            tint(Tab::Gifs),
            opens(Tab::Gifs),
        )));
    }
    if state.can_pick_stickers() {
        buttons = buttons.push(lift(composer_button(
            ctx,
            Icon::Sticker,
            "Open sticker picker",
            22,
            tint(Tab::Stickers),
            opens(Tab::Stickers),
        )));
    }
    let count = ctx.composer.draft.chars();
    if look.discord && count > 0 {
        let ink = if count > 2000 {
            theme.danger
        } else {
            theme.muted
        };
        buttons = buttons.push(lift(
            text(format!("{count}/2000")).size(11).color(ink).into(),
        ));
    }
    if look.discord && look.enabled {
        buttons = buttons.push(lift(composer_button(
            ctx,
            if look.silent {
                Icon::BellOff
            } else {
                Icon::Bell
            },
            if look.silent {
                "Send with notifications"
            } else {
                "Send silently"
            },
            22,
            if look.silent {
                theme.accent
            } else {
                theme.secondary
            },
            Some(Message::Write(Event::Silent)),
        )));
    }
    let translate = ctx.layout.prefs.translate_send.unwrap_or(false);
    let emoji = buttons
        .push(lift(composer_button(
            ctx,
            Icon::Smile,
            "Add an emoji",
            22,
            tint(Tab::Emoji),
            opens(Tab::Emoji),
        )))
        // ReCord's translation, at hand: lit while messages are sent
        // translated, and a press turns it on or off.
        .push(lift(composer_button(
            ctx,
            Icon::Translate,
            if translate {
                "Stop translating messages"
            } else {
                "Translate messages before sending"
            },
            22,
            if translate {
                theme.accent
            } else {
                theme.secondary
            },
            (look.enabled && ctx.state.selected.is_some())
                .then_some(Message::Write(Event::ToggleTranslate)),
        )));
    let line = row![lift(plus), field, emoji]
        .spacing(6)
        .align_y(Alignment::End)
        .padding(Padding::ZERO.left(10).right(8));
    let square = ctx.themes.square_avatars();
    let ground = if square {
        theme.field
    } else if look.enabled {
        theme.composer
    } else {
        theme.field
    };
    // Discord's outline: faint, and the signal under the keys in ReCord.
    let edge = if square {
        if ctx.composer.focused {
            theme.accent
        } else {
            theme.line
        }
    } else if ctx.composer.focused {
        faded(theme.muted, 0.55)
    } else {
        theme.outline_field
    };
    let radius = if square {
        0.0.into()
    } else if replying {
        corners([0.0, 0.0, 10.0, 10.0])
    } else {
        10.0.into()
    };
    let mut inside = column![];
    if let Some(tray) = crate::uploads::tray(ctx) {
        inside = inside.push(tray);
    }
    let lit = square && ctx.composer.focused;
    let frame = container(inside.push(line))
        .width(Length::Fill)
        .style(move |_| {
            let mut style = crate::style::outlined(ground, edge, radius);
            if lit {
                // ReCord's focus: a signal ring and a soft outer glow.
                style.shadow = iced::Shadow {
                    color: Color {
                        a: 0.07,
                        ..theme.accent
                    },
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: 24.0,
                };
            }
            style
        });
    // ReCord's dithered strip along the composer's top edge: a canvas, so
    // its checkerboard is made once into one mesh and drawn as it is.
    let frame: Element<'a, Message> = if square {
        let strip = container(
            iced::widget::Canvas::new(Dither {
                color: Color {
                    a: 0.28,
                    ..theme.accent
                },
            })
            .width(Length::Fill)
            .height(Length::Fixed(2.0)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Start);
        stack![frame, strip].into()
    } else {
        frame.into()
    };
    Popover::new(
        frame,
        crate::suggest::suggestions_view(ctx, Writing::Draft),
        Side::Above,
    )
    .into()
}

/// ReCord's dithered strip: a two-pixel checkerboard of the signal along
/// the composer's top edge, what the theme's `::before` paints. Its mesh
/// is made once for the strip's width and kept in the canvas' cache: the
/// frame draws one triangle mesh rather than one quad for each square.
struct Dither {
    color: Color,
}

#[derive(Default)]
struct DitherState {
    cache: iced::widget::canvas::Cache,
    color: std::cell::Cell<Option<Color>>,
}

impl<M> iced::widget::canvas::Program<M> for Dither {
    type State = DitherState;

    fn draw(
        &self,
        state: &DitherState,
        renderer: &Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        if state.color.get() != Some(self.color) {
            state.cache.clear();
            state.color.set(Some(self.color));
        }
        let color = self.color;
        let geometry = state.cache.draw(
            renderer,
            bounds.size(),
            |frame: &mut iced::widget::canvas::Frame| {
                let mut row = 0;
                let mut y = 0.0;
                while y < bounds.height {
                    let mut column = 0;
                    let mut x = 0.0;
                    while x < bounds.width {
                        // Filled top-right and bottom-left, as the conic dither.
                        if (row + column) % 2 == 1 {
                            frame.fill_rectangle(
                                iced::Point::new(x, y),
                                Size::new(2.0, 2.0),
                                color,
                            );
                        }
                        x += 2.0;
                        column += 1;
                    }
                    y += 2.0;
                    row += 1;
                }
            },
        );
        vec![geometry]
    }
}

/// The bar over the field while replying: who to, the mention's toggle,
/// and the cancel.
fn reply_bar<'a>(ctx: crate::ctx::Ctx<'a>, label: String, ping: bool) -> Element<'a, Message> {
    reply_bar_for(
        ctx,
        label,
        ping,
        Message::Write(Event::TogglePing),
        Message::Write(Event::CancelReply),
    )
}

/// A reply's bar over a composer: who to, the mention's toggle
/// (`toggle`) and the cancel (`cancel`).
pub fn reply_bar_for<'a>(
    ctx: crate::ctx::Ctx<'a>,
    label: String,
    ping: bool,
    toggle: Message,
    cancel: Message,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let ground = theme.sidebar;
    let ink = if ping { theme.accent } else { theme.muted };
    let toggle = button(
        text(if ping { "@ On" } else { "@ Off" })
            .size(13)
            .font(BOLD())
            .color(ink),
    )
    .padding([0, 8])
    .height(28)
    .on_press(toggle)
    .style(lit_style(None, theme.hover, 6.0));
    let toggle = tooltip(
        container(toggle).height(28).align_y(Alignment::Center),
        controls::tip(theme, "Mention the person you reply to"),
        tooltip::Position::Top,
    )
    .gap(6);
    container(
        row![
            ctx.look.glyph(Icon::Reply, 16, theme.muted),
            text(label)
                .size(14)
                .color(theme.secondary)
                .wrapping(text::Wrapping::None)
                .width(Length::Fill),
            toggle,
            ctx.look
                .icon_button(Icon::X, "Cancel reply", cancel)
                .size(28.0, 16),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(Padding::ZERO.left(14).right(4))
    .height(BAR)
    .clip(true)
    .align_y(Alignment::Center)
    .style(move |_| rounded(ground, corners([10.0, 10.0, 0.0, 0.0])))
    .into()
}

/// Discord's bar in place of the composer where this account may not
/// write.
fn locked_bar<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let (ground, edge) = (theme.field, theme.outline_field);
    container(
        text("You do not have permission to send messages in this channel.")
            .size(16)
            .color(theme.muted)
            .wrapping(text::Wrapping::None),
    )
    .padding(Padding::ZERO.left(16).right(16))
    .width(Length::Fill)
    .height(HEIGHT)
    .clip(true)
    .align_y(Alignment::Center)
    .style(move |_| crate::style::outlined(ground, edge, 10.0))
    .into()
}

/// A message request's answer.
fn request_bar<'a>(ctx: crate::ctx::Ctx<'a>, title: &str) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let ground = theme.field;
    let words = column![
        text(title.to_owned())
            .size(15)
            .font(BOLD())
            .color(theme.text)
            .wrapping(text::Wrapping::None),
        text("Accept to reply. Ignoring removes it without telling them.")
            .size(14)
            .color(theme.secondary)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(2)
    .width(Length::Fill)
    .clip(true);
    container(
        row![
            words,
            controls::action(
                theme,
                "Ignore",
                controls::Kind::Plain,
                Some(Message::Write(Event::Answer(false)))
            ),
            controls::action(
                theme,
                "Accept",
                controls::Kind::Primary,
                Some(Message::Write(Event::Answer(true)))
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(Padding {
        top: 12.0,
        right: 12.0,
        bottom: 12.0,
        left: 16.0,
    })
    .width(Length::Fill)
    .style(move |_| rounded(ground, 10))
    .into()
}

/// The draft's or the edit's text field.
pub fn writing<'a>(ctx: crate::ctx::Ctx<'a>, which: Writing) -> &'a field::Draft {
    match which {
        Writing::Draft => &ctx.composer.draft,
        Writing::Edit => &ctx.composer.edit,
        Writing::Aside => &ctx.drafts.aside,
        Writing::NewThread => &ctx.drafts.first,
        Writing::NewPost => &ctx.forum_post.body,
    }
}

#[cfg(test)]
mod tests {
    use super::toggle_silent;

    #[test]
    fn the_silent_toggle_takes_the_prefix_on_and_off() {
        assert_eq!(toggle_silent("hello"), "@silent hello");
        assert_eq!(toggle_silent("@silent hello"), "hello");
        assert_eq!(toggle_silent("@silent"), "");
        assert_eq!(toggle_silent("@silentish"), "@silent @silentish");
    }
}
