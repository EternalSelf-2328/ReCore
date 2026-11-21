//! Dialogs over the dimmed window, drawn in the kit's frame
//! ([`crate::dialog`]): Delete's confirmation and the question before
//! following a masked link.

use iced::{
    Alignment, Element, Length, Task,
    keyboard::key,
    widget::{Space, button, column, container, row, text},
};
use recore_core::application::Action;

use crate::{
    app::{App, Message},
    controls::{Kind, action, lights},
    ctx::Ctx,
    dialog::{CONFIRM_WIDTH, dialog},
    fonts::{BOLD, MEDIUM},
    form,
    pictures::Icon,
    style::{rounded, scrollbar},
    widgets::scroll_view,
};

/// The dialogs' answers.
#[derive(Clone, Debug)]
pub enum Event {
    /// The delete confirmation's answers.
    ConfirmDelete,
    CancelDelete,
    /// The question before a masked link: visit it, or go back.
    Leave(bool),
    /// The report dialog: a reason chosen, sending it, or closing.
    ReportReason(i64),
    ReportSend,
    ReportClose,
    /// The bulk delete dialog: a message chosen or not, deleting those
    /// chosen, or closing.
    BulkToggle(String),
    BulkSend,
    BulkClose,
    /// The folder editor: its name, colour and servers, saving, deleting or
    /// closing.
    FolderName(String),
    FolderColor(Option<u32>),
    FolderServer(String),
    FolderSave,
    FolderDelete,
    FolderClose,
}

/// What each answer does.
pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::ConfirmDelete => {
            app.client.dispatch(Action::ConfirmDelete);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::CancelDelete => {
            app.client.dispatch(Action::CancelDelete);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::Leave(visit) => leave(app, visit),
        Event::ReportReason(reason) => {
            app.client.dispatch(Action::ChooseReportReason(reason));
            app.sync()
        }
        Event::ReportSend => {
            app.client.dispatch(Action::SendReport);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::ReportClose => {
            app.client.dispatch(Action::CloseReport);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::BulkToggle(item) => {
            app.client.dispatch(Action::ToggleBulkDelete(item));
            app.sync()
        }
        Event::BulkSend => {
            app.client.dispatch(Action::ConfirmBulkDelete);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::BulkClose => {
            app.client.dispatch(Action::CloseBulkDelete);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::FolderName(name) => {
            app.client.dispatch(Action::SetFolderName(name));
            Task::none()
        }
        Event::FolderColor(color) => {
            app.client.dispatch(Action::SetFolderColor(color));
            Task::none()
        }
        Event::FolderServer(server) => {
            app.client.dispatch(Action::ToggleFolderServer(server));
            Task::none()
        }
        Event::FolderSave => {
            app.client.dispatch(Action::SaveFolder);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::FolderDelete => {
            app.client.dispatch(Action::DeleteFolder);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
        Event::FolderClose => {
            app.client.dispatch(Action::CloseFolder);
            Task::batch([app.sync(), crate::composer::refocus(app)])
        }
    }
}

/// Enter in the folder editor saves it.
pub fn folder_enter(app: &mut App) -> Task<Message> {
    if app
        .client
        .state()
        .folder_editor
        .as_ref()
        .is_some_and(|editor| !editor.saving && editor.servers.iter().any(|server| server.chosen))
    {
        return app.update(Message::Dialog(Event::FolderSave));
    }
    Task::none()
}

/// Enter in the report dialog sends it when a reason is chosen.
pub fn report_enter(app: &mut App) -> Task<Message> {
    if app
        .client
        .state()
        .report
        .as_ref()
        .is_some_and(|report| report.selected.is_some() && !report.sending)
    {
        return app.update(Message::Dialog(Event::ReportSend));
    }
    Task::none()
}

/// Enter in the bulk delete dialog deletes those chosen.
pub fn bulk_enter(app: &mut App) -> Task<Message> {
    if app
        .client
        .state()
        .bulk_delete
        .as_ref()
        .is_some_and(|bulk| !bulk.chosen.is_empty() && !bulk.sending)
    {
        return app.update(Message::Dialog(Event::BulkSend));
    }
    Task::none()
}

/// The confirmation's keys: Escape cancels, Enter does what the focused
/// button does, Tab and the side arrows go round the two buttons.
pub fn confirm_key(app: &mut App, named: Option<key::Named>) -> Task<Message> {
    match named {
        Some(key::Named::Escape) => app.update(Message::Dialog(Event::CancelDelete)),
        Some(key::Named::Enter) => {
            if app.confirm_focus {
                app.update(Message::Dialog(Event::ConfirmDelete))
            } else {
                app.update(Message::Dialog(Event::CancelDelete))
            }
        }
        Some(key::Named::Tab | key::Named::ArrowLeft | key::Named::ArrowRight) => {
            app.confirm_focus = !app.confirm_focus;
            Task::none()
        }
        _ => Task::none(),
    }
}

/// Delete's confirmation over the dimmed window: the question, the
/// message as it shows, Cancel and Delete (focused when `focus`, as it is
/// first). A click on the dimmed window cancels; nothing under it takes a
/// key.
pub fn delete_view(focus: bool, cx: Ctx<'_>) -> Option<Element<'_, Message>> {
    let state = cx.state;
    let item = state.confirm_delete.as_ref()?;
    let message = state.message(item);
    let theme = cx.look.theme;
    let line = theme.line;
    let mut body = column![
        text("Delete Message")
            .size(20)
            .font(BOLD())
            .color(theme.text),
        text("Are you sure you want to delete this message?")
            .size(15)
            .color(theme.secondary),
    ]
    .spacing(12)
    .padding(16);
    if let Some(message) = message {
        body = body.push(
            container(
                column![
                    text(message.author.clone())
                        .font(MEDIUM())
                        .color(theme.text),
                    text(message.body.clone())
                        .color(theme.text)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(2),
            )
            .padding(12)
            .width(Length::Fill)
            .max_height(160)
            .clip(true)
            .style(move |_| container::Style {
                border: crate::style::edge(line, 6.0),
                ..Default::default()
            }),
        );
    }
    body = body.push(
        row![
            Space::new().width(Length::Fill),
            action(
                theme,
                "Cancel",
                Kind::Ghost,
                Some(Message::Dialog(Event::CancelDelete))
            )
            .ring(!focus),
            action(
                theme,
                "Delete",
                Kind::Danger,
                Some(Message::Dialog(Event::ConfirmDelete))
            )
            .ring(focus),
        ]
        .spacing(8),
    );
    Some(dialog(
        theme,
        body.into(),
        CONFIRM_WIDTH,
        Message::Dialog(Event::CancelDelete),
    ))
}

/// Reporting a message: the message, Discord's reasons, and Send.
pub fn report_view<'a>(cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let state = cx.state;
    let report = state.report.as_ref()?;
    let theme = cx.look.theme;
    let line = theme.line;
    let mut body = column![
        text("Report Message")
            .size(20)
            .font(BOLD())
            .color(theme.text),
        text("Reports are anonymous. Discord reviews what you send.")
            .size(14)
            .color(theme.secondary),
    ]
    .spacing(12)
    .padding(16);
    if let Some(message) = state.message(&report.item) {
        body = body.push(
            container(
                column![
                    text(message.author.clone())
                        .font(MEDIUM())
                        .color(theme.text),
                    text(message.body.clone())
                        .color(theme.text)
                        .wrapping(text::Wrapping::WordOrGlyph),
                ]
                .spacing(2),
            )
            .padding(12)
            .width(Length::Fill)
            .max_height(120)
            .clip(true)
            .style(move |_| container::Style {
                border: crate::style::edge(line, 6.0),
                ..Default::default()
            }),
        );
    }
    let selected = report.selected;
    let mut reasons = column![].spacing(4);
    match &report.reasons {
        None => {
            reasons = reasons.push(
                text("Getting Discord's reasons...")
                    .size(14)
                    .color(theme.muted),
            );
        }
        Some(found) if found.is_empty() && report.error.is_none() => {
            reasons = reasons.push(
                text("Discord offers no reasons for this message.")
                    .size(14)
                    .color(theme.muted),
            );
        }
        Some(found) => {
            for reason in found {
                let chosen = selected == Some(reason.reason);
                let (text_color, background) = if chosen {
                    (theme.on_accent, theme.accent)
                } else {
                    (theme.text, theme.sidebar)
                };
                let detail = reason.description.clone();
                let label = reason.label.clone();
                reasons = reasons.push(
                    button(
                        column![
                            text(label).size(15).font(MEDIUM()).color(text_color),
                            text(detail).size(13).color(if chosen {
                                text_color
                            } else {
                                theme.secondary
                            }),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                    )
                    .padding(10)
                    .width(Length::Fill)
                    .on_press(Message::Dialog(Event::ReportReason(reason.reason)))
                    .style(move |_, _| button::Style {
                        background: Some(background.into()),
                        border: crate::style::round(6),
                        ..Default::default()
                    }),
                );
            }
        }
    }
    let room = (cx.look.window.height - 48.0 - 40.0 - 40.0 - 120.0 - 60.0).max(80.0);
    body = body.push(scroll_view(reasons).height(room).style(scrollbar(theme)));
    if let Some(error) = &report.error {
        body = body.push(form::error(theme, error.clone()));
    }
    body = body.push(
        row![
            Space::new().width(Length::Fill),
            action(
                theme,
                "Cancel",
                Kind::Ghost,
                Some(Message::Dialog(Event::ReportClose))
            ),
            action(
                theme,
                if report.sending {
                    "Sending..."
                } else {
                    "Report"
                },
                Kind::Danger,
                (report.selected.is_some() && !report.sending)
                    .then_some(Message::Dialog(Event::ReportSend)),
            ),
        ]
        .spacing(8),
    );
    Some(dialog(
        theme,
        body.into(),
        CONFIRM_WIDTH,
        Message::Dialog(Event::ReportClose),
    ))
}

/// Deleting messages at once: the recent ones this account may delete, each
/// ticked or not, and Delete.
pub fn bulk_delete_view<'a>(cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let state = cx.state;
    let bulk = state.bulk_delete.as_ref()?;
    let theme = cx.look.theme;
    let mut body = column![
        text("Delete Messages")
            .size(20)
            .font(BOLD())
            .color(theme.text),
        text("Choose the messages to delete at once, up to 100 from the last two weeks.")
            .size(14)
            .color(theme.secondary),
    ]
    .spacing(12)
    .padding(16);
    let mut list = column![].spacing(2);
    if bulk.items.is_empty() {
        list = list.push(
            container(
                text("No recent messages here can be deleted.")
                    .size(14)
                    .color(theme.muted),
            )
            .padding(8),
        );
    }
    for item in &bulk.items {
        let Some(message) = state.message(item) else {
            continue;
        };
        let chosen = bulk.chosen.iter().any(|chosen| chosen == item);
        let (accent, faint, text_color) = (theme.accent, theme.faint, theme.text);
        let tick = container(if chosen {
            Element::from(cx.look.glyph(Icon::Check, 14, theme.on_accent))
        } else {
            Space::new().into()
        })
        .center(20)
        .style(move |_| container::Style {
            background: chosen.then(|| accent.into()),
            border: iced::Border {
                color: faint,
                width: if chosen { 0.0 } else { 2.0 },
                radius: 5.0.into(),
            },
            ..Default::default()
        });
        let row = row![
            tick,
            column![
                text(message.author.clone())
                    .size(13)
                    .font(MEDIUM())
                    .color(text_color),
                text(message.body.clone())
                    .size(13)
                    .color(theme.secondary)
                    .wrapping(text::Wrapping::None),
            ]
            .width(Length::Fill)
            .clip(true),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let hover = theme.hover;
        list = list.push(
            button(row)
                .padding([6, 8])
                .width(Length::Fill)
                .on_press(Message::Dialog(Event::BulkToggle(item.clone())))
                .style(move |_, status| button::Style {
                    background: lights(status).then(|| hover.into()),
                    border: crate::style::round(6),
                    ..Default::default()
                }),
        );
    }
    let room = (cx.look.window.height - 48.0 - 40.0 - 40.0 - 40.0).max(120.0);
    body = body.push(scroll_view(list).height(room).style(scrollbar(theme)));
    if let Some(error) = &bulk.error {
        body = body.push(form::error(theme, error.clone()));
    }
    let chosen = bulk.chosen.len();
    body = body.push(
        row![
            text(format!("{chosen} chosen"))
                .size(13)
                .color(theme.muted)
                .width(Length::Fill),
            action(
                theme,
                "Cancel",
                Kind::Ghost,
                Some(Message::Dialog(Event::BulkClose))
            ),
            action(
                theme,
                if bulk.sending {
                    "Deleting..."
                } else if chosen == 1 {
                    "Delete 1 Message"
                } else {
                    "Delete Messages"
                },
                Kind::Danger,
                (chosen > 0 && !bulk.sending).then_some(Message::Dialog(Event::BulkSend)),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    Some(dialog(
        theme,
        body.into(),
        CONFIRM_WIDTH + 80.0,
        Message::Dialog(Event::BulkClose),
    ))
}

/// The colours Discord's folder dialog offers, and the default.
const FOLDER_COLORS: [u32; 9] = [
    0x5865f2, 0x3ba55d, 0xfaa61a, 0xed4245, 0xeb459e, 0x9b59b6, 0x00b0f4, 0x747f8d, 0x2b2d31,
];
const FOLDER_NAME: &str = "folder-name";

/// A server folder being made or changed: its name, colour and servers.
pub fn folder_view<'a>(cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let editor = cx.state.folder_editor.as_ref()?;
    let theme = cx.look.theme;
    let title = if editor.id.is_some() {
        "Edit Folder"
    } else {
        "Create Folder"
    };
    let mut body = column![
        text(title).size(20).font(BOLD()).color(theme.text),
        text("Choose the servers this folder holds.")
            .size(14)
            .color(theme.secondary),
    ]
    .spacing(12)
    .padding(16);
    let name = form::field(theme, FOLDER_NAME, "Folder name", &editor.name)
        .padding(10)
        .on_input(|name| Message::Dialog(Event::FolderName(name)));
    body = body.push(name);
    let mut colors = row![].spacing(6).align_y(Alignment::Center);
    for color in FOLDER_COLORS {
        let chosen = editor.color == Some(color);
        let swatch = container(Space::new())
            .width(24)
            .height(24)
            .style(move |_| {
                let mut style = rounded(crate::style::hex(color), 12.0);
                if chosen {
                    style.border = iced::Border {
                        color: theme.text,
                        width: 2.0,
                        radius: 12.0.into(),
                    };
                }
                style
            });
        colors = colors.push(
            button(swatch)
                .padding(0)
                .on_press(Message::Dialog(Event::FolderColor(Some(color))))
                .style(|_, _| button::Style::default()),
        );
    }
    let default = button(text("Default").size(12).color(if editor.color.is_none() {
        theme.text
    } else {
        theme.muted
    }))
    .padding([4, 8])
    .on_press(Message::Dialog(Event::FolderColor(None)))
    .style(move |_, status| button::Style {
        background: lights(status).then(|| theme.hover.into()),
        border: crate::style::round(6),
        ..Default::default()
    });
    colors = colors.push(default);
    body = body.push(colors);
    let mut servers = column![].spacing(1);
    for server in &editor.servers {
        let mut face = row![
            cx.avatar(server.icon.as_deref(), &server.name, 24.0, 12.0, 1.0),
            text(server.name.as_str())
                .size(14)
                .color(theme.text)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if server.chosen {
            face = face.push(cx.look.glyph(Icon::Check, 16, theme.accent));
        }
        let hover = theme.hover;
        servers = servers.push(
            button(face)
                .padding([5, 8])
                .width(Length::Fill)
                .on_press(Message::Dialog(Event::FolderServer(server.id.clone())))
                .style(move |_, status| button::Style {
                    background: lights(status).then(|| hover.into()),
                    border: crate::style::round(5),
                    ..Default::default()
                }),
        );
    }
    let room = (cx.look.window.height - 48.0 - 40.0 - 40.0 - 150.0).max(100.0);
    body = body.push(scroll_view(servers).height(room).style(scrollbar(theme)));
    if let Some(error) = &editor.error {
        body = body.push(form::error(theme, error.clone()));
    }
    let mut foot = row![].spacing(8).align_y(Alignment::Center);
    if editor.id.is_some() {
        foot = foot.push(action(
            theme,
            "Delete Folder",
            Kind::Danger,
            (!editor.saving).then_some(Message::Dialog(Event::FolderDelete)),
        ));
    }
    foot = foot.push(Space::new().width(Length::Fill)).push(action(
        theme,
        "Cancel",
        Kind::Ghost,
        Some(Message::Dialog(Event::FolderClose)),
    ));
    foot = foot.push(action(
        theme,
        if editor.saving { "Saving..." } else { "Save" },
        Kind::Primary,
        (!editor.saving && editor.servers.iter().any(|server| server.chosen))
            .then_some(Message::Dialog(Event::FolderSave)),
    ));
    body = body.push(foot);
    Some(dialog(
        theme,
        body.into(),
        CONFIRM_WIDTH + 60.0,
        Message::Dialog(Event::FolderClose),
    ))
}

/// The question before following a masked link (one whose words aren't
/// where it goes): where it goes, Go back and Visit site. Escape goes back,
/// Enter visits.
pub fn leaving_view<'a>(link: Option<&'a str>, cx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let link = link?;
    let theme = cx.look.theme;
    let sunken = theme.sunken;
    let body = column![
        text("Leaving ReCore")
            .size(18)
            .font(BOLD())
            .color(theme.text),
        text("This link goes to").size(15).color(theme.secondary),
        text(recore_core::links::host(link))
            .size(17)
            .font(BOLD())
            .color(theme.text),
        container(
            text(link)
                .size(14)
                .color(theme.secondary)
                .wrapping(text::Wrapping::Glyph),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |_| crate::style::rounded(sunken, 6)),
        row![
            Space::new().width(Length::Fill),
            action(
                theme,
                "Go back",
                Kind::Ghost,
                Some(Message::Dialog(Event::Leave(false)))
            ),
            action(
                theme,
                "Visit site",
                Kind::Primary,
                Some(Message::Dialog(Event::Leave(true)))
            ),
        ]
        .spacing(8),
    ]
    .spacing(12)
    .padding(20);
    Some(dialog(
        theme,
        body.into(),
        CONFIRM_WIDTH,
        Message::Dialog(Event::Leave(false)),
    ))
}

/// The question's answer: the link opens, or not.
pub fn leave(app: &mut App, visit: bool) -> Task<Message> {
    if let Some(link) = app.leaving.take()
        && visit
    {
        crate::browser::open(&link);
    }
    Task::none()
}

/// An emoji to pick in a form: 40 px on a sunken ground, the emoji chosen
/// (`key`) or a smile with a plus, and with one, a small badge on its
/// corner that takes it off.
pub fn emoji_slot<'a>(
    ctx: crate::ctx::Ctx<'a>,
    key: Option<&'a str>,
    pick: Message,
    remove: Message,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let face: Element<'a, Message> = match key {
        Some(key) => crate::suggest::emoji_face(ctx, key, 22),
        None => ctx
            .look
            .glyph(crate::pictures::Icon::SmilePlus, 20, theme.secondary)
            .into(),
    };
    let pick = iced::widget::button(container(face).center(40))
        .padding(0)
        .width(40)
        .height(40)
        .on_press(pick)
        .style(crate::controls::lit_style(
            Some(theme.sunken),
            theme.hover,
            6.0,
        ));
    if key.is_none() {
        return pick.into();
    }
    let (faint, danger, raised) = (theme.faint, theme.danger, theme.raised);
    let remove = iced::widget::button(
        container(ctx.look.glyph(crate::pictures::Icon::X, 10, theme.text)).center(14),
    )
    .padding(0)
    .width(18)
    .height(18)
    .on_press(remove)
    .style(move |_, status| iced::widget::button::Style {
        background: Some(
            if crate::controls::lights(status) {
                danger
            } else {
                faint
            }
            .into(),
        ),
        border: iced::Border {
            color: raised,
            width: 2.0,
            radius: 9.0.into(),
        },
        ..Default::default()
    });
    iced::widget::stack![pick, crate::controls::placed(remove, 28.0, 0.0)].into()
}
