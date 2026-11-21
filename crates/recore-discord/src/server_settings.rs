//! A Discord server's settings, as its Server Settings pages show them,
//! read from Discord's answers. Each page's are fetched when it opens and
//! dropped when it closes; see "Server settings" in docs/discord/server-settings.md.

use base64::Engine;
use serde_json::Value;

use crate::{
    ready,
    settings::{Field, fields, message, put_bytes, put_field, put_fixed64},
};

/// A server's profile, as Server Profile shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerProfile {
    pub name: String,
    pub description: String,
    /// The address of its icon, when it has one.
    pub icon: Option<String>,
}

/// A server's profile from `GET /guilds/{id}/profile`.
pub fn parse_profile(guild: &str, profile: &Value) -> ServerProfile {
    ServerProfile {
        name: profile["name"].as_str().unwrap_or_default().to_owned(),
        description: profile["description"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        icon: profile["icon_hash"]
            .as_str()
            .map(|hash| ready::server_icon(guild, hash)),
    }
}

/// What a server's Engagement and Boost Perks pages show, from `GET
/// /guilds/{id}` or a GUILD_UPDATE.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuildSettings {
    /// The channel system messages go to, by id.
    pub system_channel: Option<String>,
    /// Which system messages Discord leaves out: Discord's
    /// `system_channel_flags`, each bit suppressing one.
    pub system_flags: u64,
    /// Whether members who set nothing are notified of every message: 0,
    /// or only of mentions: 1.
    pub default_notifications: u8,
    /// The voice channel idle members move to, by id, and after how many
    /// seconds.
    pub afk_channel: Option<String>,
    pub afk_timeout: u32,
    /// The addresses of its banner and invite background, when set.
    pub banner: Option<String>,
    pub splash: Option<String>,
    pub progress_bar: bool,
    /// Its boost level, 0 to 3, and Discord's features, which the level
    /// unlocks, such as `BANNER`.
    pub premium_tier: u8,
    pub features: Vec<String>,
    /// What Safety Setup sets: what members must have done before they
    /// may talk, 0 none to 4 a verified phone; whose images Discord
    /// filters, 0 no one, 1 those without roles, 2 everyone; and whether
    /// moderators need two-factor authentication, 1, or not, 0.
    pub verification_level: u8,
    pub explicit_content_filter: u8,
    pub mfa_level: u8,
    /// When its paused invites and DMs resume, in Unix seconds, from its
    /// `incidents_data`.
    pub invites_paused_until: Option<u64>,
    pub dms_paused_until: Option<u64>,
    /// The emoji slots its boosts add to each kind's 50, from its
    /// `premium_features`, when Discord sends them.
    pub extra_emoji_slots: Option<u16>,
}

impl GuildSettings {
    pub fn has(&self, feature: &str) -> bool {
        self.features.iter().any(|known| known == feature)
    }
}

/// A server's settings from its guild object, whose fields come at its top
/// or, as READY has them, under `properties`.
pub fn parse_guild(guild: &Value) -> GuildSettings {
    let id = guild["id"].as_str().unwrap_or_default();
    let field = |key: &str| match &guild[key] {
        Value::Null => &guild["properties"][key],
        found => found,
    };
    let text = |key: &str| field(key).as_str().map(str::to_owned);
    GuildSettings {
        system_channel: text("system_channel_id"),
        system_flags: field("system_channel_flags").as_u64().unwrap_or_default(),
        default_notifications: field("default_message_notifications")
            .as_u64()
            .unwrap_or_default() as u8,
        afk_channel: text("afk_channel_id"),
        afk_timeout: field("afk_timeout").as_u64().unwrap_or(300) as u32,
        banner: field("banner").as_str().map(|hash| ready::banner(id, hash)),
        splash: field("splash").as_str().map(|hash| ready::splash(id, hash)),
        progress_bar: field("premium_progress_bar_enabled")
            .as_bool()
            .unwrap_or_default(),
        premium_tier: field("premium_tier").as_u64().unwrap_or_default() as u8,
        features: field("features")
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|feature| Some(feature.as_str()?.to_owned()))
            .collect(),
        verification_level: field("verification_level").as_u64().unwrap_or_default() as u8,
        explicit_content_filter: field("explicit_content_filter")
            .as_u64()
            .unwrap_or_default() as u8,
        mfa_level: field("mfa_level").as_u64().unwrap_or_default() as u8,
        invites_paused_until: parse_incidents(field("incidents_data")).0,
        dms_paused_until: parse_incidents(field("incidents_data")).1,
        extra_emoji_slots: field("premium_features")["additional_emoji_slots"]
            .as_u64()
            .map(|slots| u16::try_from(slots).unwrap_or(u16::MAX)),
    }
}

/// When a server's paused invites and DMs resume, in Unix seconds, from
/// its `incidents_data`.
pub fn parse_incidents(incidents: &Value) -> (Option<u64>, Option<u64>) {
    let until = |key: &str| {
        incidents[key]
            .as_str()
            .and_then(crate::message::iso_ms)
            .map(|ms| ms / 1000)
    };
    (until("invites_disabled_until"), until("dms_disabled_until"))
}

/// An invite to a server, as Invites in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Invite {
    pub code: String,
    pub inviter: Option<BannedUser>,
    /// The channel it leads to, by id and name.
    pub channel: Option<(String, String)>,
    pub uses: u64,
    /// Its limit, 0 for none.
    pub max_uses: u64,
    /// When it expires, in Unix seconds; none for never.
    pub expires: Option<u64>,
    /// The roles it gives, by id.
    pub roles: Vec<String>,
    /// It skips join applications: Discord's `IS_APPLICATION_BYPASS` flag.
    pub bypass: bool,
}

/// An invite from `GET /guilds/{id}/invites`.
pub fn parse_invite(invite: &Value) -> Option<Invite> {
    let created = invite["created_at"]
        .as_str()
        .and_then(crate::message::iso_ms)
        .unwrap_or_default()
        / 1000;
    let max_age = invite["max_age"].as_u64().unwrap_or_default();
    Some(Invite {
        code: invite["code"].as_str()?.to_owned(),
        inviter: parse_banned_user(&invite["inviter"]),
        channel: invite["channel"]["id"].as_str().map(|id| {
            let name = invite["channel"]["name"].as_str().unwrap_or_default();
            (id.to_owned(), name.to_owned())
        }),
        uses: invite["uses"].as_u64().unwrap_or_default(),
        max_uses: invite["max_uses"].as_u64().unwrap_or_default(),
        expires: (max_age > 0).then_some(created + max_age),
        // Ids, or role objects.
        roles: invite["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|role| role.as_str().or_else(|| role["id"].as_str()))
            .map(str::to_owned)
            .collect(),
        bypass: invite["flags"].as_u64().unwrap_or_default() & INVITE_BYPASS != 0,
    })
}

/// Discord's invite flag for an invite that skips join applications.
const INVITE_BYPASS: u64 = 1 << 3;

/// A member as Members' search finds them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FoundMember {
    pub user: BannedUser,
    pub nick: Option<String>,
    pub roles: Vec<String>,
    /// When they joined the server, in Unix milliseconds.
    pub joined_at: u64,
    /// Until when they're timed out, and when Discord last saw them send
    /// unusual DMs, in Unix milliseconds; 0 for neither.
    pub timed_out_until: u64,
    pub unusual_dms_until: u64,
    /// Discord's member `flags`, which mark a quarantined name.
    pub flags: u64,
    /// Discord flagged the account for spam.
    pub spammer: bool,
    /// A bot's account, which can't be given a server.
    pub bot: bool,
    /// How they joined: Discord's `join_source_type`, the invite's code,
    /// and an integration's type.
    pub join_source: u8,
    pub invite_code: Option<String>,
    pub integration: Option<String>,
}

/// Discord's user flag for an account it suspects of spam.
const SPAMMER: u64 = 1 << 20;

/// The members of a `members-search` answer, and how many match in all.
pub fn parse_members_search(found: &Value) -> (Vec<FoundMember>, Option<u64>) {
    let time = |value: &Value| {
        value
            .as_str()
            .and_then(crate::message::iso_ms)
            .unwrap_or_default()
    };
    let members = found["members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|found| {
            let member = &found["member"];
            let user = &member["user"];
            let user_flags = user["public_flags"]
                .as_u64()
                .or_else(|| user["flags"].as_u64())
                .unwrap_or_default();
            Some(FoundMember {
                user: parse_banned_user(user)?,
                nick: member["nick"]
                    .as_str()
                    .filter(|nick| !nick.is_empty())
                    .map(str::to_owned),
                roles: member["roles"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|role| Some(role.as_str()?.to_owned()))
                    .collect(),
                joined_at: time(&member["joined_at"]),
                timed_out_until: time(&member["communication_disabled_until"]),
                unusual_dms_until: time(&member["unusual_dm_activity_until"]),
                flags: member["flags"].as_u64().unwrap_or_default(),
                spammer: user_flags & SPAMMER != 0,
                bot: user["bot"].as_bool().unwrap_or_default(),
                join_source: found["join_source_type"].as_u64().unwrap_or_default() as u8,
                invite_code: found["source_invite_code"].as_str().map(str::to_owned),
                integration: found["integration_type"].as_str().map(str::to_owned),
            })
        })
        .collect();
    (members, found["total_result_count"].as_u64())
}

/// Someone banned from a server, as Bans lists them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ban {
    pub user: BannedUser,
    /// Why, as the moderator gave it; none from a GUILD_BAN_ADD.
    pub reason: Option<String>,
}

/// A banned user, as the ban gives them: banned users may share no server
/// with this account, so their names aren't learned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BannedUser {
    pub id: String,
    pub username: String,
    pub global_name: Option<String>,
    /// The address of their avatar, or Discord's default one.
    pub avatar: String,
}

/// A ban from `GET /guilds/{id}/bans`, its search, or a GUILD_BAN_ADD,
/// whose `user` Discord's client needs; one without is skipped.
pub fn parse_ban(ban: &Value) -> Option<Ban> {
    Some(Ban {
        user: parse_banned_user(&ban["user"])?,
        reason: ban["reason"]
            .as_str()
            .filter(|reason| !reason.is_empty())
            .map(str::to_owned),
    })
}

/// A user object, as a ban carries it.
pub fn parse_banned_user(user: &Value) -> Option<BannedUser> {
    let id = user["id"].as_str()?;
    Some(BannedUser {
        id: id.to_owned(),
        username: user["username"].as_str().unwrap_or_default().to_owned(),
        global_name: user["global_name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        avatar: ready::avatar(id, user["avatar"].as_str()),
    })
}

/// A server's custom emoji, as Emoji in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuildEmoji {
    pub id: String,
    pub name: String,
    pub animated: bool,
    /// A server that lost boosts keeps emoji it no longer has room for,
    /// unavailable.
    pub available: bool,
    /// Who uploaded it, which `GET /guilds/{id}/emojis` says and the
    /// gateway doesn't.
    pub user: Option<BannedUser>,
}

/// An emoji from `GET /guilds/{id}/emojis` or its `POST`.
pub fn parse_emoji(emoji: &Value) -> Option<GuildEmoji> {
    Some(GuildEmoji {
        id: emoji["id"].as_str()?.to_owned(),
        name: emoji["name"].as_str()?.to_owned(),
        animated: emoji["animated"].as_bool().unwrap_or_default(),
        available: emoji["available"].as_bool().unwrap_or(true),
        user: parse_banned_user(&emoji["user"]),
    })
}

/// A server's sticker, as Stickers in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuildSticker {
    pub id: String,
    pub name: String,
    /// Its related emoji: a Unicode emoji, or a custom emoji's id.
    pub tags: String,
    pub description: String,
    /// Discord's `format_type`: 1 PNG, 2 APNG, 3 Lottie, 4 GIF.
    pub format: u8,
    pub available: bool,
    /// Who uploaded it: a user, or, as the gateway has it, only an id.
    pub user: Option<BannedUser>,
    pub user_id: Option<String>,
}

/// A sticker from `GET /guilds/{id}/stickers`, its `POST` or `PATCH`, or
/// GUILD_STICKERS_UPDATE.
pub fn parse_sticker(sticker: &Value) -> Option<GuildSticker> {
    let user = parse_banned_user(&sticker["user"]);
    Some(GuildSticker {
        id: sticker["id"].as_str()?.to_owned(),
        name: sticker["name"].as_str()?.to_owned(),
        tags: sticker["tags"].as_str().unwrap_or_default().to_owned(),
        description: sticker["description"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        format: sticker["format_type"].as_u64().unwrap_or(1) as u8,
        available: sticker["available"].as_bool().unwrap_or(true),
        user_id: user
            .as_ref()
            .map(|user| user.id.clone())
            .or_else(|| sticker["user_id"].as_str().map(str::to_owned)),
        user,
    })
}

/// A server's soundboard sound, as Soundboard in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub volume: f32,
    /// A Unicode emoji, or the name of a custom one.
    pub emoji: Option<String>,
    pub available: bool,
    pub user_id: Option<String>,
    /// Who added it, as a user, when Discord says.
    pub uploader: Option<String>,
}

/// A sound from `GET /guilds/{id}/soundboard-sounds` or its `POST`.
pub fn parse_sound(sound: &Value) -> Option<Sound> {
    let user = &sound["user"];
    let uploader = user["global_name"]
        .as_str()
        .filter(|name| !name.is_empty())
        .or_else(|| user["username"].as_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned);
    Some(Sound {
        id: sound["sound_id"].as_str()?.to_owned(),
        name: sound["name"].as_str()?.to_owned(),
        volume: sound["volume"].as_f64().unwrap_or(1.0).clamp(0.0, 1.0) as f32,
        emoji: sound["emoji_name"]
            .as_str()
            .filter(|emoji| !emoji.is_empty())
            .or_else(|| sound["emoji_id"].as_str())
            .map(str::to_owned),
        available: sound["available"].as_bool().unwrap_or(true),
        user_id: sound["user_id"]
            .as_str()
            .or_else(|| user["id"].as_str())
            .map(str::to_owned),
        uploader,
    })
}

/// A webhook in a server, as Webhooks in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Webhook {
    pub id: String,
    pub name: String,
    pub kind: u8,
    pub channel_id: Option<String>,
    pub token: Option<String>,
    pub url: Option<String>,
    pub creator: Option<String>,
    pub application: Option<String>,
}

/// A webhook from `GET /guilds/{id}/webhooks` or `POST /channels/{id}/webhooks`.
pub fn parse_webhook(webhook: &Value) -> Option<Webhook> {
    let text = |key: &str| {
        webhook[key]
            .as_str()
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Some(Webhook {
        id: webhook["id"].as_str()?.to_owned(),
        name: text("name").unwrap_or_default(),
        kind: webhook["type"].as_u64().unwrap_or(1) as u8,
        channel_id: text("channel_id"),
        token: text("token"),
        url: text("url"),
        creator: webhook["user"]["global_name"]
            .as_str()
            .or_else(|| webhook["user"]["username"].as_str())
            .map(str::to_owned),
        application: webhook["application_id"].as_str().map(str::to_owned),
    })
}

/// An integration in a server, as Integrations in its settings lists it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Integration {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub account: String,
    pub subscriber_count: Option<u64>,
    /// When it last synced, in Unix seconds.
    pub synced: Option<u64>,
}

/// An integration from `GET /guilds/{id}/integrations`.
pub fn parse_integration(integration: &Value) -> Option<Integration> {
    Some(Integration {
        id: integration["id"].as_str()?.to_owned(),
        name: integration["name"].as_str().unwrap_or_default().to_owned(),
        kind: integration["type"].as_str().unwrap_or("discord").to_owned(),
        enabled: integration["enabled"].as_bool().unwrap_or(true),
        account: integration["account"]["name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .or_else(|| integration["application"]["name"].as_str())
            .unwrap_or_default()
            .to_owned(),
        subscriber_count: integration["subscriber_count"].as_u64(),
        synced: integration["synced_at"]
            .as_str()
            .and_then(crate::message::iso_ms)
            .map(|ms| ms / 1000),
    })
}

/// A rule of a server's AutoMod, as its settings list it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModerationRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger: u8,
    pub event: u8,
    pub creator: Option<String>,
    pub keywords: Vec<String>,
    pub regexes: Vec<String>,
    pub presets: Vec<u8>,
    pub actions: Vec<u8>,
    pub alert_channel: Option<String>,
    pub timeout: Option<u32>,
}

/// An AutoMod rule from `GET /guilds/{id}/auto-moderation/rules` or its
/// `POST`.
pub fn parse_rule(rule: &Value) -> Option<ModerationRule> {
    let texts = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|text| Some(text.as_str()?.to_owned()))
            .collect()
    };
    let actions = rule["actions"].as_array().into_iter().flatten();
    let mut kinds = Vec::new();
    let mut alert_channel = None;
    let mut timeout = None;
    for action in actions {
        let kind = action["type"].as_u64().unwrap_or_default() as u8;
        kinds.push(kind);
        match kind {
            2 => alert_channel = action["metadata"]["channel_id"].as_str().map(str::to_owned),
            3 => {
                timeout = action["metadata"]["duration_seconds"]
                    .as_u64()
                    .map(|seconds| u32::try_from(seconds).unwrap_or(u32::MAX));
            }
            _ => {}
        }
    }
    Some(ModerationRule {
        id: rule["id"].as_str()?.to_owned(),
        name: rule["name"].as_str().unwrap_or_default().to_owned(),
        enabled: rule["enabled"].as_bool().unwrap_or_default(),
        trigger: rule["trigger_type"].as_u64().unwrap_or_default() as u8,
        event: rule["event_type"].as_u64().unwrap_or(1) as u8,
        creator: rule["creator_id"].as_str().map(str::to_owned),
        keywords: texts(&rule["trigger_metadata"]["keyword_filter"]),
        regexes: texts(&rule["trigger_metadata"]["regex_patterns"]),
        presets: rule["trigger_metadata"]["presets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|preset| preset.as_u64())
            .map(|preset| preset as u8)
            .collect(),
        actions: kinds,
        alert_channel,
        timeout,
    })
}

/// What a server's privacy page shows: whether this account takes DMs from
/// its members, filters them into message requests, and shows its activity
/// there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Privacy {
    pub dms: bool,
    pub message_requests: bool,
    pub activity: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            dms: true,
            message_requests: true,
            activity: true,
        }
    }
}

/// `PreloadedUserSettings.privacy`, and its lists. A guild in each is
/// restricted: no DMs, no message requests from it, no activity in it.
const PRIVACY: u64 = 8;
const RESTRICTED_GUILDS: u64 = 3;
const ACTIVITY_RESTRICTED_GUILDS: u64 = 11;
const MESSAGE_REQUEST_RESTRICTED_GUILDS: u64 = 14;

/// What this account's settings say of `guild` in `encoded`, base64
/// `PreloadedUserSettings`.
pub fn parse_privacy(encoded: &str, guild: &str) -> Privacy {
    let Ok(data) = base64::engine::general_purpose::STANDARD.decode(encoded) else {
        return Privacy::default();
    };
    let Some(privacy) = message(&data, PRIVACY) else {
        return Privacy::default();
    };
    let listed = |field| guild_ids_has(privacy, field, guild);
    Privacy {
        dms: !listed(RESTRICTED_GUILDS),
        message_requests: !listed(MESSAGE_REQUEST_RESTRICTED_GUILDS),
        activity: !listed(ACTIVITY_RESTRICTED_GUILDS),
    }
}

/// The settings with `guild` added to, or left out of, each of the privacy
/// lists so they show `privacy`: base64 `PreloadedUserSettings` to send.
/// The lists the page doesn't show stay as they are.
pub fn privacy_settings(encoded: &str, guild: &str, privacy: Privacy) -> Option<String> {
    let settings = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let existing = message(&settings, PRIVACY).unwrap_or_default();
    let mut written = Vec::with_capacity(existing.len() + 8);
    for (field, value) in fields(existing) {
        if !matches!(
            field,
            RESTRICTED_GUILDS | ACTIVITY_RESTRICTED_GUILDS | MESSAGE_REQUEST_RESTRICTED_GUILDS
        ) {
            put_field(&mut written, field, &value);
        }
    }
    for (field, on) in [
        (RESTRICTED_GUILDS, privacy.dms),
        (ACTIVITY_RESTRICTED_GUILDS, privacy.activity),
        (MESSAGE_REQUEST_RESTRICTED_GUILDS, privacy.message_requests),
    ] {
        if let Some(id) = guild.parse::<u64>().ok().filter(|_| !on) {
            put_fixed64(&mut written, field, id);
        }
    }
    let mut out = Vec::with_capacity(settings.len() + written.len() + 8);
    for (field, value) in fields(&settings) {
        if field != PRIVACY {
            put_field(&mut out, field, &value);
        }
    }
    put_bytes(&mut out, PRIVACY, &written);
    Some(base64::engine::general_purpose::STANDARD.encode(out))
}

/// Whether a repeated `fixed64` field in `data` holds `guild`.
fn guild_ids_has(data: &[u8], number: u64, guild: &str) -> bool {
    let Ok(wanted) = guild.parse::<u64>() else {
        return false;
    };
    fields(data).any(|(field, value)| {
        field == number
            && match value {
                Field::Fixed64(id) => id == wanted,
                Field::Bytes(packed) => packed
                    .chunks_exact(8)
                    .any(|bytes| u64::from_le_bytes(bytes.try_into().unwrap()) == wanted),
                _ => false,
            }
    })
}

/// A server's Discovery, from `GET /guilds/{id}/discovery-requirements`
/// and `GET /guilds/{id}/discovery-metadata`, and its `DISCOVERABLE`
/// feature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    pub enabled: bool,
    pub sufficient: bool,
    pub sufficient_without_grace: bool,
    pub safe_environment: bool,
    pub healthy: bool,
    pub health_score_pending: bool,
    pub size: bool,
    pub protected: bool,
    pub valid_rules_channel: bool,
    pub retention_healthy: bool,
    pub engagement_healthy: bool,
    pub age: bool,
    pub minimum_size: Option<u64>,
    pub minimum_age: Option<u64>,
    /// When a grace period ends, in Unix seconds.
    pub grace_period_end: Option<u64>,
    pub keywords: Vec<String>,
    pub about: String,
    pub published: bool,
    pub disabled: bool,
}

/// A server's Discovery from its quest requirements and metadata, and
/// whether its features list `DISCOVERABLE`.
pub fn parse_discovery(requirements: &Value, metadata: &Value, features: &[String]) -> Discovery {
    let truth = |key: &str| requirements[key].as_bool().unwrap_or_default();
    let time = |key: &str| {
        requirements[key]
            .as_str()
            .and_then(crate::message::iso_ms)
            .map(|ms| ms / 1000)
    };
    Discovery {
        enabled: features.iter().any(|feature| feature == "DISCOVERABLE"),
        sufficient: truth("sufficient"),
        sufficient_without_grace: truth("sufficient_without_grace_period"),
        safe_environment: truth("safe_environment"),
        healthy: truth("healthy"),
        health_score_pending: truth("health_score_pending"),
        size: truth("size"),
        protected: truth("protected"),
        valid_rules_channel: truth("valid_rules_channel"),
        retention_healthy: truth("retention_healthy"),
        engagement_healthy: truth("engagement_healthy"),
        age: truth("age"),
        minimum_size: requirements["minimum_size"].as_u64(),
        minimum_age: requirements["minimum_age"].as_u64(),
        grace_period_end: time("grace_period_end_date"),
        keywords: metadata["keywords"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|word| Some(word.as_str()?.to_owned()))
            .collect(),
        about: metadata["about"].as_str().unwrap_or_default().to_owned(),
        published: metadata["is_published"].as_bool().unwrap_or_default(),
        disabled: features
            .iter()
            .any(|feature| feature == "DISCOVERABLE_DISABLED"),
    }
}

/// A role in full, as Roles in a server's settings shows and edits it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub permissions: u64,
    /// Higher roles list first; @everyone is 0.
    pub position: i64,
    /// Its colour, 0 for none, and a gradient's second and third, which
    /// ReCore keeps as they are.
    pub color: u32,
    pub secondary_color: Option<u32>,
    pub tertiary_color: Option<u32>,
    /// Listed apart from online members.
    pub hoist: bool,
    pub mentionable: bool,
    /// What gives the role, when not people: a bot, an integration or
    /// boosting. Nobody grants it by hand.
    pub managed: Option<Managed>,
    /// Its icon's address, or a Unicode emoji in its place.
    pub icon: Option<String>,
    pub unicode_emoji: Option<String>,
}

/// What manages a role, from its `tags`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Managed {
    Bot,
    Booster,
    Integration,
}

/// A role object from `GET /guilds/{id}/roles` or a GUILD_ROLE_* event.
pub fn parse_role(role: &Value) -> Option<Role> {
    let id = role["id"].as_str()?.to_owned();
    let tags = &role["tags"];
    let managed = role["managed"].as_bool().unwrap_or_default().then(|| {
        if tags.get("bot_id").is_some() {
            Managed::Bot
        } else if tags.get("premium_subscriber").is_some() {
            Managed::Booster
        } else {
            Managed::Integration
        }
    });
    let colors = &role["colors"];
    let color = |key: &str| colors[key].as_u64().map(|color| color as u32);
    Some(Role {
        name: role["name"].as_str().unwrap_or_default().to_owned(),
        permissions: role["permissions"]
            .as_str()
            .and_then(|bits| bits.parse().ok())
            .unwrap_or_default(),
        position: role["position"].as_i64().unwrap_or_default(),
        color: color("primary_color")
            .or_else(|| role["color"].as_u64().map(|color| color as u32))
            .unwrap_or_default(),
        secondary_color: color("secondary_color"),
        tertiary_color: color("tertiary_color"),
        hoist: role["hoist"].as_bool().unwrap_or_default(),
        mentionable: role["mentionable"].as_bool().unwrap_or_default(),
        managed,
        icon: role["icon"]
            .as_str()
            .map(|hash| ready::role_icon(&id, hash)),
        unicode_emoji: role["unicode_emoji"]
            .as_str()
            .filter(|emoji| !emoji.is_empty())
            .map(str::to_owned),
        id,
    })
}

/// What Roles shows: a server's roles, how many members hold each, and
/// the boost level and features that decide what a role may have.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roles {
    /// Highest first, @everyone last.
    pub roles: Vec<Role>,
    /// By role id; @everyone isn't counted.
    pub counts: std::collections::HashMap<String, u64>,
    pub premium_tier: u8,
    pub features: Vec<String>,
}

/// Roles from a server's guild object and Discord's member counts.
pub fn parse_roles_page(guild: &Value, counts: &Value) -> Roles {
    let settings = parse_guild(guild);
    Roles {
        roles: parse_roles(&guild["roles"]),
        counts: counts
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(role, count)| Some((role.clone(), count.as_u64()?)))
            .collect(),
        premium_tier: settings.premium_tier,
        features: settings.features,
    }
}

/// A server's roles, highest first, @everyone last, as Roles lists them.
pub fn parse_roles(roles: &Value) -> Vec<Role> {
    let mut roles: Vec<Role> = roles
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(parse_role)
        .collect();
    roles.sort_by(|a, b| b.position.cmp(&a.position).then(a.id.cmp(&b.id)));
    roles
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn roles_read_highest_first_with_what_manages_them() {
        let roles = parse_roles(&json!([
            {"id": "1", "name": "@everyone", "permissions": "1024", "position": 0, "color": 0},
            {"id": "3", "name": "Bot", "position": 2, "managed": true, "tags": {"bot_id": "9"},
             "colors": {"primary_color": 255, "secondary_color": 65280, "tertiary_color": null}},
            {"id": "2", "name": "Mods", "permissions": "8192", "position": 1, "color": 3447003,
             "hoist": true, "icon": "abc", "unicode_emoji": ""},
        ]));
        let ids: Vec<&str> = roles.iter().map(|role| role.id.as_str()).collect();
        assert_eq!(ids, ["3", "2", "1"]);
        assert_eq!(roles[0].managed, Some(Managed::Bot));
        assert_eq!(
            (
                roles[0].color,
                roles[0].secondary_color,
                roles[0].tertiary_color
            ),
            (255, Some(65280), None)
        );
        assert_eq!((roles[1].permissions, roles[1].color), (8192, 3447003));
        assert!(roles[1].hoist && roles[1].unicode_emoji.is_none());
        assert!(
            roles[1]
                .icon
                .as_deref()
                .unwrap()
                .contains("/role-icons/2/abc.png")
        );
    }

    #[test]
    fn a_profile_reads_as_server_profile_shows_it() {
        let profile = parse_profile(
            "1",
            &json!({"id": "1", "name": "Crew", "icon_hash": "abc", "description": null}),
        );
        assert_eq!(profile.name, "Crew");
        assert_eq!(profile.description, "");
        assert!(profile.icon.unwrap().ends_with("/icons/1/abc.png?size=128"));
    }

    #[test]
    fn a_guild_reads_from_its_top_or_its_properties() {
        let settings = parse_guild(&json!({"id": "1", "properties": {
            "system_channel_id": "11", "system_channel_flags": 5,
            "default_message_notifications": 1, "afk_channel_id": null,
            "afk_timeout": 900, "banner": "b", "splash": null,
            "premium_progress_bar_enabled": true, "premium_tier": 2,
            "features": ["BANNER", "INVITE_SPLASH"],
        }}));
        assert_eq!(settings.system_channel.as_deref(), Some("11"));
        assert_eq!(
            (settings.system_flags, settings.default_notifications),
            (5, 1)
        );
        assert_eq!(
            (settings.afk_channel.as_deref(), settings.afk_timeout),
            (None, 900)
        );
        assert!(
            settings
                .banner
                .as_deref()
                .unwrap()
                .ends_with("/banners/1/b.png?size=600")
        );
        assert!(settings.progress_bar && settings.has("BANNER"));
        let update = parse_guild(&json!({"id": "1", "premium_tier": 1, "afk_timeout": 60}));
        assert_eq!((update.premium_tier, update.afk_timeout), (1, 60));
    }

    #[test]
    fn privacy_reads_and_writes_the_three_lists_and_keeps_the_rest() {
        // A status, so a write can show it stays, and a privacy with the
        // guild in all three lists, one packed as proto3 writes them, and
        // a field ReCore doesn't know.
        let mut privacy = Vec::new();
        put_fixed64(&mut privacy, RESTRICTED_GUILDS, 10);
        let mut packed = Vec::new();
        packed.extend_from_slice(&10_u64.to_le_bytes());
        put_bytes(&mut privacy, MESSAGE_REQUEST_RESTRICTED_GUILDS, &packed);
        put_fixed64(&mut privacy, ACTIVITY_RESTRICTED_GUILDS, 10);
        put_fixed64(&mut privacy, 9, 7);
        let mut settings = Vec::new();
        put_bytes(&mut settings, 11, &[1, 2, 3]);
        put_bytes(&mut settings, PRIVACY, &privacy);
        let encoded = base64::engine::general_purpose::STANDARD.encode(&settings);

        assert_eq!(
            parse_privacy(&encoded, "10"),
            Privacy {
                dms: false,
                message_requests: false,
                activity: false
            }
        );
        assert_eq!(
            parse_privacy(&encoded, "11"),
            Privacy {
                dms: true,
                message_requests: true,
                activity: true
            }
        );

        let changed = privacy_settings(
            &encoded,
            "10",
            Privacy {
                dms: true,
                message_requests: false,
                activity: false,
            },
        )
        .unwrap();
        let written = base64::engine::general_purpose::STANDARD
            .decode(changed)
            .unwrap();
        assert!(message(&written, 11).is_some(), "the status stays");
        let kept = message(&written, PRIVACY).unwrap();
        assert_eq!(guild_ids_has(kept, RESTRICTED_GUILDS, "10"), false);
        assert!(guild_ids_has(kept, MESSAGE_REQUEST_RESTRICTED_GUILDS, "10"));
        assert!(guild_ids_has(kept, ACTIVITY_RESTRICTED_GUILDS, "10"));
        assert!(
            fields(kept).any(|(number, value)| number == 9 && matches!(value, Field::Fixed64(7))),
            "a privacy field ReCore doesn't show stays"
        );
    }
}
