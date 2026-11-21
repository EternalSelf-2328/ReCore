//! The Discord account: linking it, its status, and the game it shares.

use super::*;

#[cfg(feature = "discord")]
fn saved(id: &str, name: &str) -> crate::discord_accounts::SavedAccount {
    crate::discord_accounts::SavedAccount {
        id: id.into(),
        name: name.into(),
        username: name.to_lowercase(),
    }
}

#[test]
#[cfg(feature = "discord")]
fn links_discord_by_qr_code_and_unlinks_it() {
    use crate::discord_link::Progress;
    use recore_discord::service::{Snapshot, Status};
    // Signed in to Matrix, ReCore links Discord from settings and connects it.
    let mut app = connected();
    assert!(matches!(
        app.dispatch(Action::LinkDiscord).as_slice(),
        [Effect::LinkDiscord]
    ));
    assert_eq!(app.state.discord_link, Some(DiscordLink::Starting));
    app.linking(Progress::Code("https://discord.com/ra/one".into()));
    assert_eq!(
        app.state.discord_link,
        Some(DiscordLink::Code("https://discord.com/ra/one".into()))
    );
    // An expired code makes way for a new one.
    assert!(matches!(
        app.linking(Progress::Expired).as_slice(),
        [Effect::LinkDiscord]
    ));
    assert_eq!(app.state.discord_link, Some(DiscordLink::Starting));
    // Without a code first, it's a failure, not a loop.
    assert!(app.linking(Progress::Expired).is_empty());
    assert!(matches!(
        app.state.discord_link,
        Some(DiscordLink::Failed(_))
    ));
    app.dispatch(Action::LinkDiscord);
    app.linking(Progress::Scanned("alice".into()));
    assert_eq!(
        app.state.discord_link,
        Some(DiscordLink::Scanned("alice".into()))
    );
    // Discord wants a captcha: the browser opens its page, and the dialog
    // can open it again.
    let page = "http://127.0.0.1:9/captcha/1".to_owned();
    assert!(matches!(
        app.linking(Progress::Captcha(page.clone())).as_slice(),
        [Effect::OpenPage(opened)] if *opened == page
    ));
    assert_eq!(
        app.state.discord_link,
        Some(DiscordLink::Captcha(page.clone()))
    );
    assert!(matches!(
        app.dispatch(Action::OpenDiscordCaptcha).as_slice(),
        [Effect::OpenPage(opened)] if *opened == page
    ));
    assert!(matches!(
        app.linking(Progress::Linked(saved("1", "alice"))).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::SwitchAccount(id)))] if id == "1"
    ));
    assert_eq!(app.state.discord_link, None);
    assert!(app.state.discord_linked);
    assert_eq!(
        app.state
            .discord_accounts
            .active()
            .map(|account| account.id.as_str()),
        Some("1")
    );
    // Another account can be linked while this one is in use.
    assert!(matches!(
        app.dispatch(Action::LinkDiscord).as_slice(),
        [Effect::LinkDiscord]
    ));
    app.dispatch(Action::CancelDiscordLink);

    // Unlinking forgets everything Discord showed, and only this account.
    app.apply(discord_event(Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        friends: vec!["2".into()].into(),
        ..Default::default()
    }));
    assert!(matches!(
        app.dispatch(Action::UnlinkDiscord).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::RemoveAccount { id, fallback }))]
            if id == "1" && fallback.is_none()
    ));
    app.apply(Event::DiscordUnlinked);
    assert!(!app.state.discord_linked);
    assert_eq!(app.state.discord_user, None);
    assert!(app.state.discord_friends.is_empty());

    // Stopping linking ignores what the stopped linking says next.
    app.dispatch(Action::LinkDiscord);
    assert!(matches!(
        app.dispatch(Action::CancelDiscordLink).as_slice(),
        [Effect::StopDiscordLink]
    ));
    assert!(
        app.linking(Progress::Linked(saved("1", "alice")))
            .is_empty()
    );
    assert!(!app.state.discord_linked);

    // Without a session, as in a build without Matrix, linking signs in.
    let mut app = Application::default();
    app.dispatch(Action::LinkDiscord);
    assert!(matches!(
        app.linking(Progress::Linked(saved("1", "alice"))).as_slice(),
        [Effect::Start(chat::Start::Restore { discord_account: Some(id) })] if id == "1"
    ));
    assert!(app.state.busy);
}

#[test]
#[cfg(feature = "discord")]
fn links_discord_by_token() {
    use crate::discord_link::Progress;
    let mut app = Application::default();
    assert!(
        app.dispatch(Action::LinkDiscordToken("  ".into()))
            .is_empty(),
        "nothing pasted"
    );
    assert!(matches!(
        app.dispatch(Action::LinkDiscordToken("abc.def".into())).as_slice(),
        [Effect::LinkDiscordToken(token)] if token == "abc.def"
    ));
    assert_eq!(app.state.discord_link, Some(DiscordLink::Checking));
    app.linking(Progress::Failed("Discord didn't accept that token.".into()));
    assert!(matches!(
        app.state.discord_link,
        Some(DiscordLink::Failed(_))
    ));
    // Pasted again, it signs in as linking by QR code does.
    app.dispatch(Action::LinkDiscordToken("abc.def".into()));
    assert!(matches!(
        app.linking(Progress::Linked(saved("1", "alice"))).as_slice(),
        [Effect::Start(chat::Start::Restore { discord_account: Some(id) })] if id == "1"
    ));
    assert!(app.state.discord_linked);
}

#[test]
#[cfg(feature = "discord")]
fn email_login_asks_for_the_code_discord_wants() {
    use crate::discord_link::Progress;
    let mut app = Application::default();
    // Neither an email without a password nor a password without one goes.
    assert!(
        app.dispatch(Action::LinkDiscordEmail {
            email: "alice@example.org".into(),
            password: String::new(),
        })
        .is_empty()
    );
    assert!(matches!(
        app.dispatch(Action::LinkDiscordEmail {
            email: " alice@example.org ".into(),
            password: "secret".into(),
        })
        .as_slice(),
        [Effect::LinkDiscordEmail { email, password }]
            if email == "alice@example.org" && password == "secret"
    ));
    assert_eq!(app.state.discord_link, Some(DiscordLink::EmailChecking));
    // Discord wants a captcha on the way: the browser opens its page.
    let page = "http://127.0.0.1:9/captcha/2".to_owned();
    assert!(matches!(
        app.linking(Progress::Captcha(page.clone())).as_slice(),
        [Effect::OpenPage(opened)] if *opened == page
    ));
    // Then an authenticator code.
    app.linking(Progress::Mfa);
    assert_eq!(app.state.discord_link, Some(DiscordLink::Mfa));
    assert!(app.dispatch(Action::DiscordMfa("  ".into())).is_empty());
    assert!(matches!(
        app.dispatch(Action::DiscordMfa("123456".into())).as_slice(),
        [Effect::DiscordMfaCode(code)] if code == "123456"
    ));
    assert_eq!(app.state.discord_link, Some(DiscordLink::MfaChecking));
    // A wrong one leaves the prompt up for another.
    app.linking(Progress::MfaError(
        "Discord didn't accept that code.".into(),
    ));
    assert_eq!(
        app.state.discord_link,
        Some(DiscordLink::MfaError(
            "Discord didn't accept that code.".into()
        ))
    );
    assert!(matches!(
        app.dispatch(Action::DiscordMfa("654321".into())).as_slice(),
        [Effect::DiscordMfaCode(code)] if code == "654321"
    ));
    // The right one signs in as any link does.
    assert!(matches!(
        app.linking(Progress::Linked(saved("1", "alice"))).as_slice(),
        [Effect::Start(chat::Start::Restore { discord_account: Some(id) })] if id == "1"
    ));
    assert_eq!(app.state.discord_link, None);
}

#[test]
#[cfg(feature = "discord")]
fn another_linked_account_switches_and_the_removed_one_falls_back() {
    use crate::discord_accounts::DiscordAccounts;
    let mut app = connected();
    app.dispatch(Action::RecallDiscordAccounts(DiscordAccounts {
        saved: vec![saved("1", "alice"), saved("2", "bob")],
        active: Some("1".into()),
    }));
    assert_eq!(
        app.state
            .discord_accounts
            .active()
            .map(|each| each.id.as_str()),
        Some("1")
    );
    // Switching keeps both accounts, and tells Discord to start again on the
    // other one.
    assert!(matches!(
        app.dispatch(Action::SwitchDiscordAccount("2".into())).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::SwitchAccount(id)))] if id == "2"
    ));
    assert_eq!(app.state.discord_accounts.saved.len(), 2);
    assert_eq!(
        app.state
            .discord_accounts
            .active()
            .map(|each| each.id.as_str()),
        Some("2")
    );
    // Removing the account in use falls back to the one left.
    assert!(matches!(
        app.dispatch(Action::RemoveDiscordAccount("2".into())).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::RemoveAccount { id, fallback: Some(next) }))]
            if id == "2" && next == "1"
    ));
    assert_eq!(app.state.discord_accounts.saved.len(), 1);
    assert_eq!(
        app.state
            .discord_accounts
            .active()
            .map(|each| each.id.as_str()),
        Some("1")
    );
    // The link a session names is listed, as one linked before accounts were
    // kept apart.
    assert!(matches!(
        app.dispatch(Action::RemoveDiscordAccount("1".into())).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::RemoveAccount { id, fallback: None }))]
            if id == "1"
    ));
    assert!(app.state.discord_accounts.is_empty());
}

#[test]
#[cfg(feature = "discord")]
fn the_add_friend_page_goes_when_the_friends_screen_does() {
    use recore_discord::service::{Snapshot, Status};
    let mut app = Application::default();
    app.apply(discord_event(Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        ..Default::default()
    }));
    app.dispatch(Action::Navigate(Navigation::DirectMessages));
    app.dispatch(Action::OpenDmScreen(DmScreen::Friends));
    app.dispatch(Action::OpenAddFriend);
    assert!(app.state.add_friend.is_some());
    // The friends screen again leaves it, friend requests are another screen.
    app.dispatch(Action::OpenDmScreen(DmScreen::Friends));
    assert!(app.state.add_friend.is_some());
    app.dispatch(Action::OpenDmScreen(DmScreen::Requests));
    assert!(app.state.add_friend.is_none());
}

#[test]
#[cfg(feature = "discord")]
fn adds_friends_by_username_and_sets_the_custom_status() {
    use recore_discord::service::{Snapshot, Status};
    let mut app = Application::default();
    app.apply(discord_event(Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        ..Default::default()
    }));

    app.dispatch(Action::OpenAddFriend);
    assert!(matches!(
        app.dispatch(Action::AddFriend(" @Erin ".into())).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::AddFriend { username }))] if username == "erin"
    ));
    assert!(
        app.dispatch(Action::AddFriend("erin".into())).is_empty(),
        "one request at a time"
    );
    app.apply(Event::FriendAdded(Err(
        "Discord has nobody by that username.".into(),
    )));
    let dialog = app.state.add_friend.as_ref().expect("still open");
    assert!(!dialog.sending);
    assert!(matches!(&dialog.outcome, Some(Err(_))));

    app.dispatch(Action::OpenStatusEditor);
    assert_eq!(
        app.dispatch(Action::SetCustomStatus {
            emoji: Some(":pog:".into()),
            text: "Hi".into(),
            clear_after: ClearAfter::Never,
        })
        .len(),
        0
    );
    assert_eq!(
        app.state
            .status_editor
            .as_ref()
            .and_then(|editor| editor.error.as_deref()),
        Some("A custom emoji in a status needs Discord Nitro.")
    );
    let effects = app.dispatch(Action::SetCustomStatus {
        emoji: Some("🔧".into()),
        text: " Fixing the bridge ".into(),
        clear_after: ClearAfter::Minutes(30),
    });
    let [Effect::Backend(Command::Discord(DiscordCommand::SetCustomStatus(Some(status))))] =
        effects.as_slice()
    else {
        panic!("the status goes to Discord");
    };
    assert_eq!(status.text, "Fixing the bridge");
    assert_eq!(status.emoji.as_deref(), Some("🔧"));
    let left = status.expires_at_ms.unwrap() as i64 - chrono::Local::now().timestamp_millis();
    assert!((29 * 60_000..=30 * 60_000).contains(&left));
    app.apply(Event::CustomStatusSet(Ok(())));
    assert!(app.state.status_editor.is_none());

    // Nothing to say clears it.
    app.dispatch(Action::OpenStatusEditor);
    assert!(matches!(
        app.dispatch(Action::SetCustomStatus {
            emoji: None,
            text: " ".into(),
            clear_after: ClearAfter::Today,
        })
        .as_slice(),
        [Effect::Backend(Command::Discord(
            DiscordCommand::SetCustomStatus(None)
        ))]
    ));
}

#[test]
#[cfg(feature = "discord")]
fn discord_dialogs_stop_waiting_when_the_backend_cant_answer() {
    use recore_discord::service::{Snapshot, Status};
    let mut app = Application::default();
    app.apply(discord_event(Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        ..Default::default()
    }));
    app.dispatch(Action::OpenAddFriend);
    let effects = app.dispatch(Action::AddFriend("erin".into()));
    let [Effect::Backend(command)] = effects.as_slice() else {
        panic!("the request goes to Discord");
    };
    app.backend_failed(command);
    let dialog = app.state.add_friend.as_ref().expect("still open");
    assert!(!dialog.sending);
    assert!(matches!(&dialog.outcome, Some(Err(_))));

    // The backend stopping ends the status editor's and a profile's waits.
    app.dispatch(Action::OpenStatusEditor);
    app.dispatch(Action::SetCustomStatus {
        emoji: None,
        text: "Hi".into(),
        clear_after: ClearAfter::Never,
    });
    assert!(
        app.state
            .status_editor
            .as_ref()
            .is_some_and(|editor| editor.saving)
    );
    app.state.profile = Some(Arc::new(ProfileView {
        profile: Profile {
            id: "2".into(),
            ..Default::default()
        },
        large: false,
        loading: true,
        kept: false,
        note: None,
        ..Default::default()
    }));
    app.stopped();
    let editor = app.state.status_editor.as_ref().expect("still open");
    assert!(!editor.saving && editor.error.is_some());
    let profile = app.state.profile.as_ref().expect("still open");
    assert!(!profile.loading && profile.note.is_some());
}

#[test]
fn clears_a_status_today_at_midnight() {
    use chrono::{Datelike, Local, TimeZone, Timelike};
    let evening = Local.with_ymd_and_hms(2026, 10, 7, 21, 30, 0).unwrap();
    let at = ClearAfter::Today.at(evening).unwrap();
    let midnight = Local.timestamp_millis_opt(at as i64).unwrap();
    assert_eq!(
        (midnight.day(), midnight.hour(), midnight.minute()),
        (8, 0, 0)
    );
    assert_eq!(ClearAfter::Never.at(evening), None);
}

#[test]
#[cfg(feature = "discord")]
fn shares_the_game_as_the_account_settings_say() {
    use recore_discord::{
        own_activity::Playing,
        service::{Snapshot, Status},
    };
    let mut app = Application::default();
    let snapshot = |show_game: bool, playing: Option<&str>| Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        show_game,
        playing: playing.map(|name| {
            Arc::new(Playing {
                name: name.into(),
                started_ms: Some(5),
                kind: recore_discord::activity::Kind::Playing,
            })
        }),
        ..Default::default()
    };
    app.apply(discord_event(snapshot(true, Some("Game"))));
    assert!(app.state.discord_show_game);
    assert_eq!(
        app.state.discord_playing,
        Some(chat::Playing {
            name: "Game".into(),
            started_ms: Some(5),
            kind: recore_discord::activity::Kind::Playing,
        })
    );
    assert!(
        app.dispatch(Action::ShowGame(true)).is_empty(),
        "it's shared already"
    );
    assert!(matches!(
        app.dispatch(Action::ShowGame(false)).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::ShowGame(
            false
        )))]
    ));
    assert!(!app.state.discord_show_game, "the switch moves at once");
    app.apply(Event::ShowGameSet(Err("Discord said no.".into())));
    assert!(app.state.discord_show_game, "and back when Discord refuses");
    assert!(app.state.notice.is_some());
    app.apply(discord_event(snapshot(false, None)));
    assert!(!app.state.discord_show_game);
    assert_eq!(app.state.discord_playing, None);
}

#[test]
#[cfg(feature = "discord")]
fn sets_the_status_and_holds_notifications_while_busy() {
    use recore_discord::service::{Snapshot, Status};
    let mut app = Application::default();
    let snapshot = |presence: &str| Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        presence: presence.into(),
        ..Default::default()
    };
    app.apply(discord_event(snapshot("online")));
    assert!(
        app.dispatch(Action::SetStatus("online".into())).is_empty(),
        "it's already online"
    );
    assert!(matches!(
        app.dispatch(Action::SetStatus("dnd".into())).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::SetStatus(status)))] if status == "dnd"
    ));
    app.apply(Event::StatusSet(Ok(())));
    app.apply(discord_event(snapshot("dnd")));
    assert_eq!(app.state.discord_presence.as_deref(), Some("dnd"));
    let notify = |app: &mut Application| {
        app.apply(Event::Notify(chat::Notification {
            room: "dm".into(),
            author: "Sam".into(),
            body: "Around?".into(),
            avatar: None,
            mention: false,
        }))
    };
    assert!(
        notify(&mut app).is_empty(),
        "Do Not Disturb holds them back"
    );
    app.apply(discord_event(snapshot("idle")));
    assert_eq!(notify(&mut app).len(), 1);
    app.apply(Event::StatusSet(Err("Discord said no.".into())));
    assert_eq!(
        app.state.notice.as_deref(),
        Some("Couldn't change your status: Discord said no.")
    );
}

#[test]
fn going_idle_while_signed_out_tells_nobody() {
    let inactive = || Action::Inactive {
        since: Some(1),
        show_idle: true,
    };
    let mut app = Application::default();
    assert!(app.dispatch(inactive()).is_empty());
    let mut app = connected();
    assert!(matches!(
        app.dispatch(inactive()).as_slice(),
        [Effect::Backend(Command::Discord(
            DiscordCommand::Inactive { .. }
        ))]
    ));
}
