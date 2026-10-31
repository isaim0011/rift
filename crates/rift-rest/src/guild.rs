//! Guild resource endpoints.
//!
//! Complete coverage for Discord Guild endpoints from the official documentation
//! and OpenAPI specification:
//! - Guild get, modify, preview, vanity URL, voice regions
//! - Channels: list, create, modify positions, active threads
//! - Members: list, get, search, add, modify, delete (kick), modify current member (@me),
//!   add/remove role, member permissions
//! - Roles: list, create, modify positions, get, modify, delete, member counts, role permissions
//! - Bans: list, get, create (ban), delete (unban), bulk ban
//! - Prune: preview prune, begin prune
//! - Emojis: list, create, get, modify, delete
//! - Stickers: list, create, get, modify, delete
//! - Audit log: list guild audit log entries
//! - Integrations: list, delete
//! - Webhooks: list guild webhooks
//! - Invites: list guild invites
//! - Widget: get settings, modify settings, get widget JSON, get widget PNG
//! - Welcome Screen: get, modify
//! - Onboarding: get, put
//! - Incident Actions: update incident actions
//! - Voice States: get/update self (@me), get/update user voice state
//! - Templates: get by code, list, create, sync, modify, delete
//! - Auto Moderation: list, create, get, modify, delete rules
//! - Scheduled Events: list, create, get, modify, delete, list users, count users,
//!   create/modify/delete exceptions, list exception users
//! - Soundboard: list, create, get, modify, delete sounds
//! - Join Requests: list requests, action join request
//! - Messages Search: search guild messages

use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake, ser_opt_snowflake, ser_snowflake};
use crate::transport::Transport;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;

// ───────────────────────── Serde helpers ─────────────────────────

#[allow(dead_code)]
fn ser_permissions_str<S: Serializer>(v: &Permissions, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

fn ser_opt_perm<S: Serializer>(v: &Option<Permissions>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(p) => s.collect_str(p),
        None => s.serialize_none(),
    }
}

fn ser_opt_perm_field<S: Serializer>(v: &Option<Option<Permissions>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(Some(p)) => s.collect_str(p),
        Some(None) => s.serialize_none(),
        None => s.serialize_none(),
    }
}

fn de_perm_str<'de, D: Deserializer<'de>>(d: D) -> Result<Permissions, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Permissions>().map_err(serde::de::Error::custom)
}

fn de_opt_perm<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Permissions>, D::Error> {
    let s = Option::<String>::deserialize(d)?;
    s.map(|v| v.parse::<Permissions>().map_err(serde::de::Error::custom)).transpose()
}

fn ser_opt_snowflake_field<S: Serializer>(v: &Option<Option<Snowflake>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(Some(id)) => s.collect_str(id),
        Some(None) => s.serialize_none(),
        None => s.serialize_none(),
    }
}

fn de_snowflakes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let strings = Option::<Vec<String>>::deserialize(d)?;
    match strings {
        Some(vec) => vec.into_iter().map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom)).collect(),
        None => Ok(Vec::new()),
    }
}

fn ser_snowflakes<S: Serializer>(v: &[Snowflake], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(v.iter().map(|x| x.to_string()))
}

fn ser_opt_snowflakes<S: Serializer>(v: &Option<Vec<Snowflake>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(vec) => ser_snowflakes(vec, s),
        None => s.serialize_none(),
    }
}

fn ser_opt_opt_snowflakes<S: Serializer>(v: &Option<Option<Vec<Snowflake>>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(Some(vec)) => ser_snowflakes(vec, s),
        Some(None) => s.serialize_none(),
        None => s.serialize_none(),
    }
}

// ───────────────────────── Validation Errors ─────────────────────────

/// Validation errors for guild endpoint parameters.
#[derive(Clone, Debug, PartialEq)]
pub enum GuildError {
    /// Guild name is not 2-100 characters.
    BadGuildName {
        /// The invalid character count.
        len: usize,
    },
    /// Channel name is not 1-100 characters.
    BadChannelName {
        /// The invalid character count.
        len: usize,
    },
    /// Role name is not 1-100 characters.
    BadRoleName {
        /// The invalid character count.
        len: usize,
    },
    /// Nickname exceeds 32 characters.
    BadNick {
        /// The invalid character count.
        len: usize,
    },
    /// Member list/query limit is out of 1-1000.
    BadMemberLimit {
        /// The invalid limit value.
        value: u16,
    },
    /// Search members query string is empty or exceeds 100 characters.
    BadSearchQuery {
        /// The invalid query character count.
        len: usize,
    },
    /// Delete message seconds is out of 0-604800 (7 days).
    BadDeleteMessageSeconds {
        /// The invalid seconds value.
        value: u32,
    },
    /// Bulk ban user_ids count is out of 1-200.
    BadBulkBanCount {
        /// The invalid user count.
        count: usize,
    },
    /// Prune days is out of 1-30.
    BadPruneDays {
        /// The invalid days value.
        value: u32,
    },
    /// Emoji name is not 2-32 characters.
    BadEmojiName {
        /// The invalid character count.
        len: usize,
    },
    /// Sticker name is not 2-30 characters.
    BadStickerName {
        /// The invalid character count.
        len: usize,
    },
    /// Sticker description exceeds 100 characters.
    BadStickerDescription {
        /// The invalid character count.
        len: usize,
    },
    /// Sticker tags exceeds 200 characters.
    BadStickerTags {
        /// The invalid character count.
        len: usize,
    },
    /// Audit log limit is out of 1-100.
    BadAuditLogLimit {
        /// The invalid limit value.
        value: u8,
    },
    /// Scheduled event name is not 1-100 characters.
    BadEventName {
        /// The invalid character count.
        len: usize,
    },
    /// Soundboard sound name is not 2-32 characters.
    BadSoundName {
        /// The invalid character count.
        len: usize,
    },
    /// Soundboard sound volume is not between 0.0 and 1.0.
    BadSoundVolume {
        /// The invalid volume value.
        volume: f64,
    },
}

impl fmt::Display for GuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadGuildName { len } => write!(f, "guild name is {len} chars (must be 2-100)"),
            Self::BadChannelName { len } => write!(f, "channel name is {len} chars (must be 1-100)"),
            Self::BadRoleName { len } => write!(f, "role name is {len} chars (must be 1-100)"),
            Self::BadNick { len } => write!(f, "member nickname is {len} chars (must be 1-32)"),
            Self::BadMemberLimit { value } => write!(f, "member limit {value} must be 1-1000"),
            Self::BadSearchQuery { len } => write!(f, "search query length {len} must be 1-100"),
            Self::BadDeleteMessageSeconds { value } => write!(f, "delete_message_seconds {value} is out of 0-604800"),
            Self::BadBulkBanCount { count } => write!(f, "bulk ban count {count} must be 1-200"),
            Self::BadPruneDays { value } => write!(f, "prune days {value} must be 1-30"),
            Self::BadEmojiName { len } => write!(f, "emoji name is {len} chars (must be 2-32)"),
            Self::BadStickerName { len } => write!(f, "sticker name is {len} chars (must be 2-30)"),
            Self::BadStickerDescription { len } => write!(f, "sticker description is {len} chars (must be <= 100)"),
            Self::BadStickerTags { len } => write!(f, "sticker tags is {len} chars (must be <= 200)"),
            Self::BadAuditLogLimit { value } => write!(f, "audit log limit {value} must be 1-100"),
            Self::BadEventName { len } => write!(f, "scheduled event name is {len} chars (must be 1-100)"),
            Self::BadSoundName { len } => write!(f, "soundboard sound name is {len} chars (must be 2-32)"),
            Self::BadSoundVolume { volume } => write!(f, "soundboard volume {volume} must be 0.0-1.0"),
        }
    }
}

impl std::error::Error for GuildError {}

impl From<GuildError> for Error {
    fn from(e: GuildError) -> Self {
        Error::Transport(format!("guild validation: {e}"))
    }
}

fn check_guild_name(name: &str) -> Result<(), GuildError> {
    let len = name.trim().chars().count();
    if !(2..=100).contains(&len) { Err(GuildError::BadGuildName { len }) } else { Ok(()) }
}

fn check_channel_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(1..=100).contains(&len) { Err(GuildError::BadChannelName { len }) } else { Ok(()) }
}

fn check_role_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(1..=100).contains(&len) { Err(GuildError::BadRoleName { len }) } else { Ok(()) }
}

fn check_nick(nick: &str) -> Result<(), GuildError> {
    let len = nick.chars().count();
    if len > 32 { Err(GuildError::BadNick { len }) } else { Ok(()) }
}

fn check_member_limit(v: u16) -> Result<(), GuildError> {
    if !(1..=1000).contains(&v) { Err(GuildError::BadMemberLimit { value: v }) } else { Ok(()) }
}

fn check_search_query(q: &str) -> Result<(), GuildError> {
    let len = q.chars().count();
    if !(1..=100).contains(&len) { Err(GuildError::BadSearchQuery { len }) } else { Ok(()) }
}

fn check_delete_message_seconds(v: u32) -> Result<(), GuildError> {
    if v > 604_800 { Err(GuildError::BadDeleteMessageSeconds { value: v }) } else { Ok(()) }
}

fn check_bulk_ban_count(count: usize) -> Result<(), GuildError> {
    if !(1..=200).contains(&count) { Err(GuildError::BadBulkBanCount { count }) } else { Ok(()) }
}

fn check_prune_days(days: u32) -> Result<(), GuildError> {
    if !(1..=30).contains(&days) { Err(GuildError::BadPruneDays { value: days }) } else { Ok(()) }
}

fn check_emoji_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(2..=32).contains(&len) { Err(GuildError::BadEmojiName { len }) } else { Ok(()) }
}

fn check_sticker_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(2..=30).contains(&len) { Err(GuildError::BadStickerName { len }) } else { Ok(()) }
}

fn check_sticker_desc(desc: &str) -> Result<(), GuildError> {
    let len = desc.chars().count();
    if len > 100 { Err(GuildError::BadStickerDescription { len }) } else { Ok(()) }
}

fn check_sticker_tags(tags: &str) -> Result<(), GuildError> {
    let len = tags.chars().count();
    if len > 200 { Err(GuildError::BadStickerTags { len }) } else { Ok(()) }
}

fn check_audit_log_limit(limit: u8) -> Result<(), GuildError> {
    if !(1..=100).contains(&limit) { Err(GuildError::BadAuditLogLimit { value: limit }) } else { Ok(()) }
}

fn check_event_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(1..=100).contains(&len) { Err(GuildError::BadEventName { len }) } else { Ok(()) }
}

fn check_sound_name(name: &str) -> Result<(), GuildError> {
    let len = name.chars().count();
    if !(2..=32).contains(&len) { Err(GuildError::BadSoundName { len }) } else { Ok(()) }
}

fn check_sound_volume(vol: f64) -> Result<(), GuildError> {
    if !(0.0..=1.0).contains(&vol) { Err(GuildError::BadSoundVolume { volume: vol }) } else { Ok(()) }
}

// ───────────────────────── Guild Types ─────────────────────────

/// A Discord guild (server).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Guild {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Guild name (2-100 characters).
    pub name: String,
    /// Icon hash.
    pub icon: Option<String>,
    /// Guild description.
    pub description: Option<String>,
    /// Home header hash.
    pub home_header: Option<String>,
    /// Splash hash.
    pub splash: Option<String>,
    /// Discovery splash hash.
    pub discovery_splash: Option<String>,
    /// Enabled guild features.
    #[serde(default)]
    pub features: Vec<String>,
    /// Banner hash.
    pub banner: Option<String>,
    /// Owner user id.
    #[serde(deserialize_with = "de_snowflake")]
    pub owner_id: Snowflake,
    /// Application id of the guild creator if it is bot-created.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub application_id: Option<Snowflake>,
    /// Voice region id for the guild.
    #[serde(default)]
    pub region: Option<String>,
    /// Id of afk channel.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub afk_channel_id: Option<Snowflake>,
    /// AFK timeout in seconds.
    #[serde(default)]
    pub afk_timeout: u32,
    /// Id of system channel.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub system_channel_id: Option<Snowflake>,
    /// System channel flags.
    #[serde(default)]
    pub system_channel_flags: u32,
    /// Id of widget channel.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub widget_channel_id: Option<Snowflake>,
    /// Whether widget is enabled.
    #[serde(default)]
    pub widget_enabled: Option<bool>,
    /// Verification level required for the guild.
    #[serde(default)]
    pub verification_level: u8,
    /// Default message notifications level.
    #[serde(default)]
    pub default_message_notifications: u8,
    /// Explicit content filter level.
    #[serde(default)]
    pub explicit_content_filter: u8,
    /// Roles in the guild.
    #[serde(default)]
    pub roles: Vec<Role>,
    /// Custom guild emojis.
    #[serde(default)]
    pub emojis: Vec<Emoji>,
    /// Required MFA level for the guild.
    #[serde(default)]
    pub mfa_level: u8,
    /// Guild NSFW level.
    #[serde(default)]
    pub nsfw_level: u8,
    /// Premium tier (Server Boost level).
    #[serde(default)]
    pub premium_tier: u8,
    /// Number of boosts this guild currently has.
    #[serde(default)]
    pub premium_subscription_count: Option<u32>,
    /// Preferred locale of a Community guild.
    #[serde(default)]
    pub preferred_locale: String,
    /// Id of the channel where guild rules/guidelines are displayed.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub rules_channel_id: Option<Snowflake>,
    /// Id of the channel where safety alerts are sent.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub safety_alerts_channel_id: Option<Snowflake>,
    /// Id of the channel where community updates are displayed.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub public_updates_channel_id: Option<Snowflake>,
    /// Maximum number of presences for the guild (null for large guilds).
    #[serde(default)]
    pub max_presences: Option<u32>,
    /// Maximum number of members for the guild.
    #[serde(default)]
    pub max_members: Option<u32>,
    /// Vanity url code for the guild.
    pub vanity_url_code: Option<String>,
    /// Whether the guild has the boost progress bar enabled.
    #[serde(default)]
    pub premium_progress_bar_enabled: bool,
    /// Approximate member count (if requested with `with_counts=true`).
    #[serde(default)]
    pub approximate_member_count: Option<u32>,
    /// Approximate presence count (if requested with `with_counts=true`).
    #[serde(default)]
    pub approximate_presence_count: Option<u32>,
    /// Total permissions for the user in the guild (excluding channel overwrites).
    #[serde(default, deserialize_with = "de_opt_perm")]
    pub permissions: Option<Permissions>,
}

/// Modify guild request body.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyGuild {
    /// New guild name (2-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New guild description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Voice region id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<Option<String>>,
    /// Verification level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<u8>,
    /// Default message notification level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_message_notifications: Option<u8>,
    /// Explicit content filter level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<u8>,
    /// AFK voice channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub afk_channel_id: Option<Option<Snowflake>>,
    /// AFK timeout in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u32>,
    /// Base64 128x128 jpeg image for the guild icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
    /// User id to transfer guild ownership to (must be guild owner).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub owner_id: Option<Snowflake>,
    /// Base64 16:9 png/jpeg image for the guild splash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<Option<String>>,
    /// Base64 16:9 png/jpeg image for the guild discovery splash.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<Option<String>>,
    /// Base64 16:9 png/jpeg image for the guild banner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<String>>,
    /// System channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub system_channel_id: Option<Option<Snowflake>>,
    /// System channel flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<u32>,
    /// Rules channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub rules_channel_id: Option<Option<Snowflake>>,
    /// Public updates channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub public_updates_channel_id: Option<Option<Snowflake>>,
    /// Preferred locale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_locale: Option<String>,
    /// Enabled guild features.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
    /// Whether the boost progress bar should be enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar_enabled: Option<bool>,
    /// Safety alerts channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub safety_alerts_channel_id: Option<Option<Snowflake>>,
    /// Base64 16:9 png/jpeg image for the guild home header.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_header: Option<Option<String>>,
}

/// A guild preview object.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuildPreview {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Guild name.
    pub name: String,
    /// Icon hash.
    pub icon: Option<String>,
    /// Home header hash.
    #[serde(default)]
    pub home_header: Option<String>,
    /// Splash hash.
    pub splash: Option<String>,
    /// Discovery splash hash.
    pub discovery_splash: Option<String>,
    /// Custom guild emojis.
    #[serde(default)]
    pub emojis: Vec<Emoji>,
    /// Enabled guild features.
    #[serde(default)]
    pub features: Vec<String>,
    /// Approximate member count.
    pub approximate_member_count: u32,
    /// Approximate presence count.
    pub approximate_presence_count: u32,
    /// Description for the guild.
    pub description: Option<String>,
    /// Custom guild stickers.
    #[serde(default)]
    pub stickers: Vec<GuildSticker>,
}

/// A voice region for a guild.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VoiceRegion {
    /// Unique ID for the region.
    pub id: String,
    /// Name of the region.
    pub name: String,
    /// True if this is a single-server that is closest to the current user's client.
    pub optimal: bool,
    /// Whether this is a deprecated voice region.
    pub deprecated: bool,
    /// Whether this is a custom voice region (used for events/etc).
    pub custom: bool,
}

/// Vanity URL response.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct VanityUrl {
    /// Vanity URL code.
    pub code: Option<String>,
    /// Uses count.
    pub uses: u32,
}

// ───────────────────────── Channel Types ─────────────────────────

/// Request payload to create a channel in a guild.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateGuildChannel {
    /// Channel name (1-100 characters).
    pub name: String,
    /// Channel type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    /// Channel topic (0-1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// Bitrate (in bits) for voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    /// User limit for voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,
    /// Rate limit per user (slowmode) in seconds (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,
    /// Sorting position of the channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    /// Explicit permission overwrites.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Vec<crate::channel::PermissionOverwrite>>,
    /// Parent category ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub parent_id: Option<Snowflake>,
    /// Whether the channel is NSFW.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    /// Voice region ID for voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<String>,
    /// Camera video quality mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<u8>,
    /// Default duration that the clients use for newly created threads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,
}

/// Request item to modify the position of a channel in a guild.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ModifyGuildChannelPosition {
    /// Channel id.
    #[serde(serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Sorting position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
    /// Syncs the permission overwrites with the parent category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_permissions: Option<bool>,
    /// Parent category id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub parent_id: Option<Option<Snowflake>>,
}

/// Active threads response.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ActiveThreads {
    /// List of active threads.
    pub threads: Vec<crate::channel::Channel>,
    /// List of thread members for the client in these threads.
    pub members: Vec<ThreadMember>,
}

/// A thread member object.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ThreadMember {
    /// Thread ID.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub id: Option<Snowflake>,
    /// User ID.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub user_id: Option<Snowflake>,
    /// ISO8601 timestamp of when the user joined the thread.
    pub join_timestamp: String,
    /// Any user-thread settings.
    pub flags: u32,
}

// ───────────────────────── Member Types ─────────────────────────

/// A full member of a Discord guild.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildMember {
    /// The user this guild member represents.
    pub user: Option<crate::interaction::User>,
    /// Server nickname.
    pub nick: Option<String>,
    /// Member guild avatar hash.
    pub avatar: Option<String>,
    /// Member guild banner hash.
    pub banner: Option<String>,
    /// Array of role IDs assigned to the member.
    #[serde(default, deserialize_with = "de_snowflakes")]
    pub roles: Vec<Snowflake>,
    /// ISO8601 timestamp when the user joined the guild.
    pub joined_at: String,
    /// ISO8601 timestamp when the user started boosting the guild.
    pub premium_since: Option<String>,
    /// Whether the user is deafened in voice channels.
    #[serde(default)]
    pub deaf: bool,
    /// Whether the user is muted in voice channels.
    #[serde(default)]
    pub mute: bool,
    /// Guild member flags bitfield.
    #[serde(default)]
    pub flags: u32,
    /// Whether the user has not yet passed Membership Screening.
    pub pending: Option<bool>,
    /// ISO8601 timestamp when the timeout will expire.
    pub communication_disabled_until: Option<String>,
    /// Total permissions in an interaction context.
    #[serde(default, deserialize_with = "de_opt_perm")]
    pub permissions: Option<Permissions>,
}

/// Query parameters for listing guild members.
#[derive(Clone, Debug, Default, Serialize)]
pub struct ListGuildMembers {
    /// Max number of members to return (1-1000). Default 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    /// The highest user id in the previous page.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub after: Option<Snowflake>,
}

/// Query parameters for searching guild members.
#[derive(Clone, Debug, Serialize)]
pub struct SearchGuildMembers {
    /// Query string to match username(s) and nickname(s) against (1-100 characters).
    pub query: String,
    /// Max number of members to return (1-1000). Default 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
}

/// Request body to add a member to a guild using an OAuth2 access token.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct AddGuildMember {
    /// An oauth2 access token granted with the `guilds.join` scope.
    pub access_token: String,
    /// Value to set users nickname to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    /// Array of role IDs the member is assigned.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub roles: Option<Vec<Snowflake>>,
    /// Whether the user is muted in voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    /// Whether the user is deafened in voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
    /// Guild member flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

/// Request body to modify attributes of a guild member.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildMember {
    /// Value to set users nickname to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<String>>,
    /// Array of role IDs the member is assigned.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub roles: Option<Vec<Snowflake>>,
    /// Whether the user is muted in voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    /// Whether the user is deafened in voice channels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
    /// ID of channel to move user to (if connected to voice).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub channel_id: Option<Option<Snowflake>>,
    /// ISO8601 timestamp when timeout will expire (up to 28 days in future).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Option<String>>,
    /// Guild member flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

/// Request body to modify the current user's nickname in a guild.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyCurrentMember {
    /// Value to set current user's nickname to (or null to reset).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<String>>,
    /// Value to set user's avatar to (base64 image or null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
    /// Value to set user's bio to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<Option<String>>,
    /// Value to set user's banner to (base64 image or null).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<String>>,
}

// ───────────────────────── Role Types ─────────────────────────

/// A role in a Discord guild.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Role {
    /// Role id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Role name.
    pub name: String,
    /// Integer representation of hexadecimal color code.
    pub color: u32,
    /// If this role is pinned in the user listing.
    pub hoist: bool,
    /// Role icon hash.
    pub icon: Option<String>,
    /// Role unicode emoji.
    pub unicode_emoji: Option<String>,
    /// Position of this role.
    pub position: i32,
    /// Permission bit set.
    #[serde(deserialize_with = "de_perm_str", serialize_with = "ser_permissions_str")]
    pub permissions: Permissions,
    /// Whether this role is managed by an integration.
    pub managed: bool,
    /// Whether this role is mentionable.
    pub mentionable: bool,
    /// Role flags combined bitfield.
    #[serde(default)]
    pub flags: u32,
}

/// Request body to create a new role in a guild.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CreateGuildRole {
    /// Name of the role (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Bitwise permissions for the role.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_perm")]
    pub permissions: Option<Permissions>,
    /// RGB color value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// Whether the role should be displayed separately in the sidebar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    /// Role icon image (data URI scheme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The role's unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<String>,
    /// Whether the role should be mentionable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
}

/// Request body to modify an existing role in a guild.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildRole {
    /// Name of the role (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Option<String>>,
    /// Bitwise permissions for the role.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_perm_field")]
    pub permissions: Option<Option<Permissions>>,
    /// RGB color value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Option<u32>>,
    /// Whether the role should be displayed separately in the sidebar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<Option<bool>>,
    /// Role icon image (data URI scheme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<String>>,
    /// The role's unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<Option<String>>,
    /// Whether the role should be mentionable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<Option<bool>>,
}

/// Request item to modify the position of a role in a guild.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ModifyGuildRolePosition {
    /// Role id.
    #[serde(serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Sorting position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

// ───────────────────────── Ban Types ─────────────────────────

/// A ban object.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Ban {
    /// Reason for the ban.
    pub reason: Option<String>,
    /// Banned user.
    pub user: crate::interaction::User,
}

/// Request body to create a ban for a user.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CreateGuildBan {
    /// Number of seconds to delete messages for (0-604800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
    /// Number of days to delete messages for (0-7, deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_days: Option<u32>,
}

/// Request body to bulk ban users from a guild.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BulkBanUsers {
    /// List of user IDs to ban (1-200).
    #[serde(serialize_with = "ser_snowflakes")]
    pub user_ids: Vec<Snowflake>,
    /// Number of seconds to delete messages for (0-604800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u32>,
}

/// Response returned from bulk banning users.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct BulkBanResponse {
    /// List of user IDs that were successfully banned.
    #[serde(default, deserialize_with = "de_snowflakes")]
    pub banned_users: Vec<Snowflake>,
    /// List of user IDs that could not be banned.
    #[serde(default, deserialize_with = "de_snowflakes")]
    pub failed_users: Vec<Snowflake>,
}

// ───────────────────────── Prune Types ─────────────────────────

/// Result of a preview prune or executed prune.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PruneCount {
    /// Number of members that would be / were removed.
    pub pruned: Option<u32>,
}

/// Request body to begin a guild prune.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct BeginPrune {
    /// Number of days to prune (1-30). Default 7.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    /// Whether pruned is returned, discouraged for large guilds. Default true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_prune_count: Option<bool>,
    /// Role IDs to include in the prune.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub include_roles: Option<Vec<Snowflake>>,
}

// ───────────────────────── Emoji Types ─────────────────────────

/// A custom guild emoji.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Emoji {
    /// Emoji id.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub id: Option<Snowflake>,
    /// Emoji name.
    pub name: Option<String>,
    /// Roles allowed to use this emoji.
    #[serde(default, deserialize_with = "de_snowflakes", serialize_with = "ser_snowflakes")]
    pub roles: Vec<Snowflake>,
    /// User that created this emoji.
    pub user: Option<crate::interaction::User>,
    /// Whether this emoji must be wrapped in colons.
    #[serde(default)]
    pub require_colons: Option<bool>,
    /// Whether this emoji is managed by an integration.
    #[serde(default)]
    pub managed: Option<bool>,
    /// Whether this emoji is animated.
    #[serde(default)]
    pub animated: Option<bool>,
    /// Whether this emoji can be used.
    #[serde(default)]
    pub available: Option<bool>,
}

/// Request body to create a guild emoji.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateGuildEmoji {
    /// Name of the emoji (2-32 characters).
    pub name: String,
    /// 128x128 image data URI scheme string.
    pub image: String,
    /// Roles allowed to use this emoji.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub roles: Option<Vec<Snowflake>>,
}

/// Request body to modify a guild emoji.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildEmoji {
    /// Name of the emoji (2-32 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Roles allowed to use this emoji.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_opt_snowflakes")]
    pub roles: Option<Option<Vec<Snowflake>>>,
}

// ───────────────────────── Sticker Types ─────────────────────────

/// A custom sticker in a guild.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuildSticker {
    /// Sticker id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// For standard stickers, id of the pack the sticker is from.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub pack_id: Option<Snowflake>,
    /// Name of the sticker (2-30 characters).
    pub name: String,
    /// Description of the sticker (max 100 characters).
    pub description: Option<String>,
    /// Autocomplete/suggestion tags for the sticker (max 200 characters).
    pub tags: String,
    /// Type of sticker.
    #[serde(rename = "type")]
    pub kind: u8,
    /// Type of sticker format (1 = PNG, 2 = APNG, 3 = LOTTIE, 4 = GIF).
    pub format_type: u8,
    /// Whether this sticker can be used.
    #[serde(default)]
    pub available: Option<bool>,
    /// Guild id that owns the sticker.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// User that uploaded the sticker.
    pub user: Option<crate::interaction::User>,
    /// Standard sticker sort value.
    #[serde(default)]
    pub sort_value: Option<i32>,
}

/// Request body to create a guild sticker.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateGuildSticker {
    /// Name of the sticker (2-30 characters).
    pub name: String,
    /// Description of the sticker (max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Autocomplete/suggestion tags (max 200 characters).
    pub tags: String,
    /// Base64 data or media content for the sticker file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
}

/// Request body to modify a guild sticker.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildSticker {
    /// Name of the sticker (2-30 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description of the sticker (max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Autocomplete/suggestion tags (max 200 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

// ───────────────────────── Audit Log Types ─────────────────────────

/// A guild's audit log.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AuditLog {
    /// List of audit log entries.
    pub audit_log_entries: Vec<AuditLogEntry>,
    /// List of users referenced in the audit log.
    #[serde(default)]
    pub users: Vec<crate::interaction::User>,
    /// List of partial integration objects.
    #[serde(default)]
    pub integrations: Vec<Integration>,
    /// List of webhooks referenced.
    #[serde(default)]
    pub webhooks: Vec<crate::channel::FollowedChannel>,
    /// List of guild scheduled events.
    #[serde(default)]
    pub guild_scheduled_events: Vec<GuildScheduledEvent>,
    /// List of threads referenced.
    #[serde(default)]
    pub threads: Vec<crate::channel::Channel>,
    /// List of auto moderation rules.
    #[serde(default)]
    pub auto_moderation_rules: Vec<AutoModerationRule>,
}

/// An entry in a guild audit log.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AuditLogEntry {
    /// ID of the entry.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// ID of the affected entity.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub target_id: Option<Snowflake>,
    /// Changes made to the target_id.
    pub changes: Option<Vec<AuditLogChange>>,
    /// User or app that made the changes.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub user_id: Option<Snowflake>,
    /// Type of action that occurred.
    pub action_type: u32,
    /// Additional info for certain action types.
    pub options: Option<serde_json::Value>,
    /// Reason for the change (1-512 characters).
    pub reason: Option<String>,
}

/// A change record in an audit log entry.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AuditLogChange {
    /// New value of the key.
    pub new_value: Option<serde_json::Value>,
    /// Old value of the key.
    pub old_value: Option<serde_json::Value>,
    /// Name of audit log change key.
    pub key: String,
}

/// Query parameters for fetching guild audit logs.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GetAuditLog {
    /// Entries from a specific user.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub user_id: Option<Snowflake>,
    /// Filter entries targeting a specific resource id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub target_id: Option<Snowflake>,
    /// Filter by action type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<u32>,
    /// Filter entries before a specific entry id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub before: Option<Snowflake>,
    /// Filter entries after a specific entry id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub after: Option<Snowflake>,
    /// Number of entries to return (1-100). Default 50.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
}

// ───────────────────────── Integration Types ─────────────────────────

/// A guild integration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Integration {
    /// Integration id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Integration name.
    pub name: String,
    /// Integration type (twitch, youtube, discord, etc.).
    #[serde(rename = "type")]
    pub kind: String,
    /// Is this integration enabled.
    pub enabled: bool,
    /// Is this integration syncing.
    #[serde(default)]
    pub syncing: Option<bool>,
    /// Id that this integration uses for "subscribed" users.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub role_id: Option<Snowflake>,
    /// Whether emoticons should be synced for this integration.
    #[serde(default)]
    pub enable_emoticons: Option<bool>,
    /// The behavior of expiring subscribers.
    #[serde(default)]
    pub expire_behavior: Option<u8>,
    /// The grace period (in days) before expiring subscribers.
    #[serde(default)]
    pub expire_grace_period: Option<u32>,
    /// User for this integration.
    pub user: Option<crate::interaction::User>,
    /// Integration account information.
    pub account: IntegrationAccount,
}

/// Integration account information.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct IntegrationAccount {
    /// Id of the account.
    pub id: String,
    /// Name of the account.
    pub name: String,
}

// ───────────────────────── Widget Types ─────────────────────────

/// Settings for a guild's widget.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetSettings {
    /// Whether the widget is enabled.
    pub enabled: bool,
    /// Channel id the widget will create invites to.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
}

/// Request body to modify a guild's widget settings.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyWidgetSettings {
    /// Whether the widget is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Channel id for the widget.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub channel_id: Option<Option<Snowflake>>,
}

/// Guild widget object.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Widget {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Guild name.
    pub name: String,
    /// Instant invite code.
    pub instant_invite: Option<String>,
    /// List of voice channels.
    #[serde(default)]
    pub channels: Vec<WidgetChannel>,
    /// List of online members.
    #[serde(default)]
    pub members: Vec<WidgetMember>,
    /// Number of online members.
    pub presence_count: u32,
}

/// Channel in a guild widget.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WidgetChannel {
    /// Channel id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Channel name.
    pub name: String,
    /// Position.
    pub position: i32,
}

/// Member in a guild widget.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WidgetMember {
    /// User id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Username.
    pub username: String,
    /// Discriminator (or "0000").
    pub discriminator: Option<String>,
    /// Avatar hash.
    pub avatar: Option<String>,
    /// Presence status.
    pub status: String,
    /// Avatar URL.
    pub avatar_url: Option<String>,
}

// ───────────────────────── Welcome Screen & Onboarding ─────────────────────────

/// A guild's welcome screen.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WelcomeScreen {
    /// Server description shown in welcome screen.
    pub description: Option<String>,
    /// Channels shown in the welcome screen.
    #[serde(default)]
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

/// A channel highlighted on a welcome screen.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WelcomeScreenChannel {
    /// Channel id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,
    /// Description shown for the channel.
    pub description: String,
    /// Emoji id if emoji is custom.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub emoji_id: Option<Snowflake>,
    /// Emoji name if standard unicode emoji.
    pub emoji_name: Option<String>,
}

/// Request body to modify a guild's welcome screen.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyWelcomeScreen {
    /// Whether the welcome screen is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Option<bool>>,
    /// Channels shown in the welcome screen (up to 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub welcome_channels: Option<Option<Vec<WelcomeScreenChannel>>>,
    /// Description shown in the welcome screen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
}

/// Guild onboarding configuration.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GuildOnboarding {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
    /// Prompts shown to new members.
    #[serde(default)]
    pub prompts: Vec<serde_json::Value>,
    /// Default channel IDs that users are joined to.
    #[serde(default, deserialize_with = "de_snowflakes", serialize_with = "ser_snowflakes")]
    pub default_channel_ids: Vec<Snowflake>,
    /// Whether onboarding is enabled.
    pub enabled: bool,
    /// Current mode of onboarding.
    pub mode: u8,
}

/// Guild incident actions (security lockouts).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuildIncidentsData {
    /// When invites are disabled until (ISO8601 string or null).
    pub invites_disabled_until: Option<String>,
    /// When DMs are disabled until (ISO8601 string or null).
    pub dms_disabled_until: Option<String>,
}

// ───────────────────────── Voice State Types ─────────────────────────

/// A user's voice state in a guild.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct VoiceState {
    /// Guild id this voice state is for.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Channel id this user is connected to.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// User id.
    #[serde(deserialize_with = "de_snowflake")]
    pub user_id: Snowflake,
    /// Guild member who this voice state is for.
    pub member: Option<GuildMember>,
    /// Session id.
    pub session_id: String,
    /// Whether this user is deafened by the server.
    pub deaf: bool,
    /// Whether this user is muted by the server.
    pub mute: bool,
    /// Whether this user is locally deafened.
    pub self_deaf: bool,
    /// Whether this user is locally muted.
    pub self_mute: bool,
    /// Whether this user is streaming using "Go Live".
    pub self_stream: Option<bool>,
    /// Whether this user's camera is enabled.
    pub self_video: bool,
    /// Whether this user is muted by the current user.
    pub suppress: bool,
    /// Time at which the user requested to speak.
    pub request_to_speak_timestamp: Option<String>,
}

/// Request body to update current user's voice state.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyCurrentVoiceState {
    /// Channel id to connect to / stage instance.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub channel_id: Option<Option<Snowflake>>,
    /// Toggles the user's suppress state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
    /// ISO8601 timestamp to set or clear request to speak.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_to_speak_timestamp: Option<Option<String>>,
}

/// Request body to update another user's voice state.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ModifyVoiceState {
    /// Channel id the user is currently in.
    #[serde(serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,
    /// Toggles the user's suppress state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress: Option<bool>,
}

// ───────────────────────── Template Types ─────────────────────────

/// A guild template object.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildTemplate {
    /// The template code (unique ID).
    pub code: String,
    /// Template name.
    pub name: String,
    /// Description for the template.
    pub description: Option<String>,
    /// Number of times this template has been used.
    pub usage_count: u32,
    /// ID of the user who created the template.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub creator_id: Snowflake,
    /// User who created the template.
    pub creator: Option<crate::interaction::User>,
    /// ISO8601 timestamp of when the template was created.
    pub created_at: String,
    /// ISO8601 timestamp of when the template was last updated.
    pub updated_at: String,
    /// ID of the guild the template is based on.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub source_guild_id: Snowflake,
    /// Snapshot of the guild.
    pub serialized_source_guild: serde_json::Value,
    /// Whether the template has unsynced changes.
    pub is_dirty: Option<bool>,
}

/// Request body to create a guild template.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateGuildTemplate {
    /// Name of the template (1-100 characters).
    pub name: String,
    /// Description for the template (0-120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request body to modify a guild template.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildTemplate {
    /// Name of the template (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Description for the template (0-120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
}

// ───────────────────────── Auto Moderation Types ─────────────────────────

/// An auto moderation rule.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AutoModerationRule {
    /// Rule id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Guild id this rule belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
    /// Rule name.
    pub name: String,
    /// User which first created this rule.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub creator_id: Snowflake,
    /// The rule event type.
    pub event_type: u8,
    /// The rule trigger type.
    pub trigger_type: u8,
    /// Rule trigger metadata.
    pub trigger_metadata: serde_json::Value,
    /// Actions which will execute when the rule is triggered.
    pub actions: Vec<serde_json::Value>,
    /// Whether the rule is enabled.
    pub enabled: bool,
    /// Role IDs that should not be affected by the rule (max 20).
    #[serde(default, deserialize_with = "de_snowflakes", serialize_with = "ser_snowflakes")]
    pub exempt_roles: Vec<Snowflake>,
    /// Channel IDs that should not be affected by the rule (max 50).
    #[serde(default, deserialize_with = "de_snowflakes", serialize_with = "ser_snowflakes")]
    pub exempt_channels: Vec<Snowflake>,
}

/// Request body to create an auto moderation rule.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateAutoModerationRule {
    /// Rule name.
    pub name: String,
    /// Event type.
    pub event_type: u8,
    /// Trigger type.
    pub trigger_type: u8,
    /// Trigger metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<serde_json::Value>,
    /// Actions which will execute when the rule is triggered.
    pub actions: Vec<serde_json::Value>,
    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Role IDs that should not be affected by the rule (max 20).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub exempt_roles: Option<Vec<Snowflake>>,
    /// Channel IDs that should not be affected by the rule (max 50).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub exempt_channels: Option<Vec<Snowflake>>,
}

/// Request body to modify an auto moderation rule.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyAutoModerationRule {
    /// Rule name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Event type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<u8>,
    /// Trigger metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<serde_json::Value>,
    /// Actions which will execute when the rule is triggered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<serde_json::Value>>,
    /// Whether the rule is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Role IDs that should not be affected by the rule (max 20).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub exempt_roles: Option<Vec<Snowflake>>,
    /// Channel IDs that should not be affected by the rule (max 50).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub exempt_channels: Option<Vec<Snowflake>>,
}

// ───────────────────────── Scheduled Event Types ─────────────────────────

/// A scheduled event in a guild.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildScheduledEvent {
    /// Scheduled event id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Guild id which the event belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
    /// Channel id in which the event will be hosted (if entity_type is Stage or Voice).
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// User id of the creator of the event.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub creator_id: Option<Snowflake>,
    /// Name of the event (1-100 characters).
    pub name: String,
    /// Description of the event (1-1000 characters).
    pub description: Option<String>,
    /// ISO8601 timestamp when the event is scheduled to start.
    pub scheduled_start_time: String,
    /// ISO8601 timestamp when the event is scheduled to end.
    pub scheduled_end_time: Option<String>,
    /// Privacy level of the event (2 = GuildOnly).
    pub privacy_level: u8,
    /// Status of the event (1 = Scheduled, 2 = Active, 3 = Completed, 4 = Canceled).
    pub status: u8,
    /// Entity type (1 = StageInstance, 2 = Voice, 3 = External).
    pub entity_type: u8,
    /// Entity id of the associated entity.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub entity_id: Option<Snowflake>,
    /// Additional metadata for the entity (e.g. location for external events).
    pub entity_metadata: Option<serde_json::Value>,
    /// Creator user object.
    pub creator: Option<crate::interaction::User>,
    /// Number of users subscribed to the event.
    pub user_count: Option<u32>,
    /// Cover image hash.
    pub image: Option<String>,
}

/// Request body to create a guild scheduled event.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateGuildScheduledEvent {
    /// Channel id of the stage / voice channel (required if not external).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// Entity metadata (e.g. location for external event).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<serde_json::Value>,
    /// Name of the event (1-100 characters).
    pub name: String,
    /// Privacy level of the event (2 = GuildOnly).
    pub privacy_level: u8,
    /// Scheduled start time in ISO8601 format.
    pub scheduled_start_time: String,
    /// Scheduled end time in ISO8601 format (required for external events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<String>,
    /// Description of the event (max 1000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Entity type (1 = StageInstance, 2 = Voice, 3 = External).
    pub entity_type: u8,
    /// Cover image data URI scheme string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Request body to modify a guild scheduled event.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyGuildScheduledEvent {
    /// Channel id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub channel_id: Option<Option<Snowflake>>,
    /// Entity metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<Option<serde_json::Value>>,
    /// Name of the event (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Privacy level of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<u8>,
    /// Scheduled start time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<String>,
    /// Scheduled end time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<Option<String>>,
    /// Description of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Entity type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<u8>,
    /// Status of the event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u8>,
    /// Cover image data URI scheme string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Option<String>>,
}

/// A subscriber user to a scheduled event.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildScheduledEventUser {
    /// Event id.
    #[serde(deserialize_with = "de_snowflake")]
    pub guild_scheduled_event_id: Snowflake,
    /// User.
    pub user: crate::interaction::User,
    /// Member object if requested.
    pub member: Option<GuildMember>,
}

/// User count response for a scheduled event.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuildScheduledEventUserCount {
    /// Event id.
    #[serde(deserialize_with = "de_snowflake")]
    pub guild_scheduled_event_id: Snowflake,
    /// Number of users subscribed.
    pub user_count: u32,
}

/// An exception to a recurrence rule in a scheduled event.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuildScheduledEventException {
    /// ID of the event.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub event_id: Snowflake,
    /// ID of the exception.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub event_exception_id: Snowflake,
    /// ISO8601 timestamp of start time.
    pub scheduled_start_time: Option<String>,
    /// ISO8601 timestamp of end time.
    pub scheduled_end_time: Option<String>,
    /// Whether this occurrence is canceled.
    pub is_canceled: bool,
}

/// Request body to create a scheduled event recurrence exception.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateGuildScheduledEventException {
    /// The original start time of the occurrence to create an exception for (ISO8601).
    pub original_scheduled_start_time: String,
    /// Scheduled start time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<String>,
    /// Scheduled end time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<String>,
    /// Whether this occurrence is canceled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_canceled: Option<bool>,
    /// Recurrence exception ID (legacy compatibility field).
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub event_exception_id: Option<Snowflake>,
}

impl CreateGuildScheduledEventException {
    /// Create a new exception for a scheduled event recurrence.
    pub fn new(original_scheduled_start_time: impl Into<String>) -> Self {
        Self {
            original_scheduled_start_time: original_scheduled_start_time.into(),
            scheduled_start_time: None,
            scheduled_end_time: None,
            is_canceled: None,
            event_exception_id: None,
        }
    }
}

/// Request body to modify a scheduled event recurrence exception.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyGuildScheduledEventException {
    /// Scheduled start time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<Option<String>>,
    /// Scheduled end time in ISO8601 format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<Option<String>>,
    /// Whether this occurrence is canceled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_canceled: Option<bool>,
}

// ───────────────────────── Soundboard Types ─────────────────────────

/// A soundboard sound.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundboardSound {
    /// Name of the sound (2-32 characters).
    pub name: String,
    /// Sound id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub sound_id: Snowflake,
    /// Volume of the soundboard sound (0.0 - 1.0).
    pub volume: f64,
    /// ID of custom emoji.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub emoji_id: Option<Snowflake>,
    /// Name of standard emoji.
    pub emoji_name: Option<String>,
    /// Guild id this sound belongs to.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "ser_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Whether this sound can be used.
    pub available: bool,
    /// User who created this sound.
    pub user: Option<crate::interaction::User>,
}

/// Page of soundboard sounds.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SoundboardSoundsPage {
    /// List of soundboard sounds.
    pub items: Vec<SoundboardSound>,
}

/// Request body to create a soundboard sound.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateSoundboardSound {
    /// Name of the sound (2-32 characters).
    pub name: String,
    /// Audio data URI scheme string (e.g. mp3).
    pub sound: String,
    /// Volume of the soundboard sound (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    /// ID of custom emoji.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub emoji_id: Option<Snowflake>,
    /// Name of standard unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

/// Request body to modify a soundboard sound.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifySoundboardSound {
    /// Name of the sound (2-32 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Volume of the soundboard sound (0.0 - 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<Option<f64>>,
    /// ID of custom emoji.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub emoji_id: Option<Option<Snowflake>>,
    /// Name of standard unicode emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<Option<String>>,
}

// ───────────────────────── Join Request Types ─────────────────────────

/// A membership join request.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildJoinRequest {
    /// Join request id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Created timestamp.
    pub created_at: String,
    /// Reviewed timestamp.
    pub reviewed_at: Option<String>,
    /// Status ("STARTED", "SUBMITTED", "APPROVED", "REJECTED").
    pub application_status: String,
    /// Rejection reason if rejected.
    pub rejection_reason: Option<String>,
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake")]
    pub guild_id: Snowflake,
    /// User id of the applicant.
    #[serde(deserialize_with = "de_snowflake")]
    pub user_id: Snowflake,
    /// Applicant user object.
    pub user: Option<crate::interaction::User>,
}

/// Page of membership join requests.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GuildJoinRequestsPage {
    /// Total count.
    pub total: Option<u32>,
    /// Requests list.
    pub guild_join_requests: Vec<GuildJoinRequest>,
}

/// Request body to approve or reject a join request.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionGuildJoinRequest {
    /// Action to take ("APPROVED" or "REJECTED").
    pub action: String,
    /// Optional reason when rejecting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

fn query_string<T: Serialize>(v: &T) -> String {
    let j = serde_json::to_value(v).unwrap_or_default();
    let obj = match j.as_object() {
        Some(o) => o,
        None => return String::new(),
    };
    let parts: Vec<String> = obj.iter()
        .filter_map(|(k, v)| {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => return None,
            };
            Some(format!("{k}={s}"))
        })
        .collect();
    if parts.is_empty() { String::new() } else { format!("?{}", parts.join("&")) }
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    // ──────── Guild Core ────────

    /// `GET /guilds/templates/{code}` – fetch a guild template by code.
    pub async fn get_guild_template(&self, code: &str) -> Result<GuildTemplate, Error> {
        self.request("GET", &format!("/guilds/templates/{code}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}` – get guild by id.
    pub async fn get_guild(&self, guild_id: Snowflake, with_counts: Option<bool>) -> Result<Guild, Error> {
        let qs = with_counts.map(|b| format!("?with_counts={b}")).unwrap_or_default();
        self.request("GET", &format!("/guilds/{guild_id}{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}` – modify guild attributes.
    pub async fn update_guild(
        &self, guild_id: Snowflake, body: &ModifyGuild, reason: Option<&str>,
    ) -> Result<Guild, Error> {
        if let Some(name) = &body.name { check_guild_name(name)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/preview` – get public preview for a discoverable guild.
    pub async fn get_guild_preview(&self, guild_id: Snowflake) -> Result<GuildPreview, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/preview"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/vanity-url` – get vanity url code and uses.
    pub async fn get_guild_vanity_url(&self, guild_id: Snowflake) -> Result<VanityUrl, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/vanity-url"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/regions` – list voice regions available for the guild.
    pub async fn list_guild_voice_regions(&self, guild_id: Snowflake) -> Result<Vec<VoiceRegion>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/regions"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Channels ────────

    /// `GET /guilds/{guild_id}/channels` – list channels in a guild.
    pub async fn list_guild_channels(&self, guild_id: Snowflake) -> Result<Vec<crate::channel::Channel>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/channels"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/channels` – create a new guild channel.
    pub async fn create_guild_channel(
        &self, guild_id: Snowflake, body: &CreateGuildChannel, reason: Option<&str>,
    ) -> Result<crate::channel::Channel, Error> {
        check_channel_name(&body.name)?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/channels"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/channels` – modify channel positions in bulk.
    pub async fn bulk_update_guild_channels(
        &self, guild_id: Snowflake, positions: &[ModifyGuildChannelPosition], reason: Option<&str>,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(positions).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/channels"), Some(b), headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/threads/active` – get all active threads in the guild.
    pub async fn get_active_guild_threads(&self, guild_id: Snowflake) -> Result<ActiveThreads, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/threads/active"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Members ────────

    /// `GET /guilds/{guild_id}/members` – list members in a guild.
    pub async fn list_guild_members(
        &self, guild_id: Snowflake, query: &ListGuildMembers,
    ) -> Result<Vec<GuildMember>, Error> {
        if let Some(lim) = query.limit { check_member_limit(lim)?; }
        let qs = query_string(query);
        self.request("GET", &format!("/guilds/{guild_id}/members{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/members/search` – search guild members whose username/nickname starts with query.
    pub async fn search_guild_members(
        &self, guild_id: Snowflake, query: &SearchGuildMembers,
    ) -> Result<Vec<GuildMember>, Error> {
        check_search_query(&query.query)?;
        if let Some(lim) = query.limit { check_member_limit(lim)?; }
        let qs = query_string(query);
        self.request("GET", &format!("/guilds/{guild_id}/members/search{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/members/{user_id}` – get a specific member in a guild.
    pub async fn get_guild_member(&self, guild_id: Snowflake, user_id: Snowflake) -> Result<GuildMember, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/members/{user_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/members/{user_id}` – add a user to the guild via OAuth2 access token.
    /// Returns `Ok(Some(member))` when created (201), or `Ok(None)` if user was already in guild (204).
    pub async fn add_guild_member(
        &self, guild_id: Snowflake, user_id: Snowflake, body: &AddGuildMember,
    ) -> Result<Option<GuildMember>, Error> {
        if let Some(n) = &body.nick { check_nick(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let r = self.request("PUT", &format!("/guilds/{guild_id}/members/{user_id}"), Some(b)).await?.expect_success()?;
        if r.status == 204 {
            Ok(None)
        } else {
            r.json().map(Some)
        }
    }

    /// `PATCH /guilds/{guild_id}/members/{user_id}` – modify attributes of a guild member.
    pub async fn update_guild_member(
        &self, guild_id: Snowflake, user_id: Snowflake, body: &ModifyGuildMember, reason: Option<&str>,
    ) -> Result<GuildMember, Error> {
        if let Some(Some(n)) = &body.nick { check_nick(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/members/{user_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/members/{user_id}` – remove (kick) a member from the guild.
    pub async fn delete_guild_member(
        &self, guild_id: Snowflake, user_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/members/{user_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `PATCH /guilds/{guild_id}/members/@me` – modify current member's nickname.
    pub async fn update_my_guild_member(
        &self, guild_id: Snowflake, body: &ModifyCurrentMember, reason: Option<&str>,
    ) -> Result<GuildMember, Error> {
        if let Some(Some(n)) = &body.nick { check_nick(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/members/@me"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/members/{user_id}/roles/{role_id}` – add a role to a member.
    pub async fn add_guild_member_role(
        &self, guild_id: Snowflake, user_id: Snowflake, role_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "PUT",
            &format!("/guilds/{guild_id}/members/{user_id}/roles/{role_id}"),
            None,
            headers,
        ).await?.expect_success()?;
        Ok(())
    }

    /// `DELETE /guilds/{guild_id}/members/{user_id}/roles/{role_id}` – remove a role from a member.
    pub async fn delete_guild_member_role(
        &self, guild_id: Snowflake, user_id: Snowflake, role_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "DELETE",
            &format!("/guilds/{guild_id}/members/{user_id}/roles/{role_id}"),
            None,
            headers,
        ).await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/permissions/member/{user_id}` – get computed permissions for a member.
    pub async fn get_guild_member_permissions(
        &self, guild_id: Snowflake, user_id: Snowflake,
    ) -> Result<Permissions, Error> {
        #[derive(Deserialize)]
        struct R {
            #[serde(deserialize_with = "de_perm_str")]
            permissions: Permissions,
        }
        let r: R = self.request("GET", &format!("/guilds/{guild_id}/permissions/member/{user_id}"), None)
            .await?.expect_success()?.json()?;
        Ok(r.permissions)
    }

    // ──────── Roles ────────

    /// `GET /guilds/{guild_id}/roles` – list roles in a guild.
    pub async fn list_guild_roles(&self, guild_id: Snowflake) -> Result<Vec<Role>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/roles"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/roles` – create a new role in a guild.
    pub async fn create_guild_role(
        &self, guild_id: Snowflake, body: &CreateGuildRole, reason: Option<&str>,
    ) -> Result<Role, Error> {
        if let Some(n) = &body.name { check_role_name(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/roles"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/roles` – modify role positions in bulk.
    pub async fn bulk_update_guild_roles(
        &self, guild_id: Snowflake, positions: &[ModifyGuildRolePosition], reason: Option<&str>,
    ) -> Result<Vec<Role>, Error> {
        let b = serde_json::to_vec(positions).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/roles"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/roles/member-counts` – get member counts for each role in a guild.
    pub async fn guild_role_member_counts(
        &self, guild_id: Snowflake,
    ) -> Result<HashMap<Snowflake, u32>, Error> {
        let raw: HashMap<String, u32> = self.request("GET", &format!("/guilds/{guild_id}/roles/member-counts"), None)
            .await?.expect_success()?.json()?;
        raw.into_iter()
            .map(|(k, v)| {
                let sf = k.parse::<Snowflake>().map_err(|e| Error::Decode(format!("invalid snowflake key {k}: {e}")))?;
                Ok((sf, v))
            })
            .collect()
    }

    /// `GET /guilds/{guild_id}/roles/{role_id}` – get a specific role in a guild.
    pub async fn get_guild_role(&self, guild_id: Snowflake, role_id: Snowflake) -> Result<Role, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/roles/{role_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/roles/{role_id}` – modify an existing role.
    pub async fn update_guild_role(
        &self, guild_id: Snowflake, role_id: Snowflake, body: &ModifyGuildRole, reason: Option<&str>,
    ) -> Result<Role, Error> {
        if let Some(Some(n)) = &body.name { check_role_name(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/roles/{role_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/roles/{role_id}` – delete a role from a guild.
    pub async fn delete_guild_role(
        &self, guild_id: Snowflake, role_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/roles/{role_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/permissions/role/{role_id}` – get computed permissions for a role.
    pub async fn get_guild_role_permissions(
        &self, guild_id: Snowflake, role_id: Snowflake,
    ) -> Result<Permissions, Error> {
        #[derive(Deserialize)]
        struct R {
            #[serde(deserialize_with = "de_perm_str")]
            permissions: Permissions,
        }
        let r: R = self.request("GET", &format!("/guilds/{guild_id}/permissions/role/{role_id}"), None)
            .await?.expect_success()?.json()?;
        Ok(r.permissions)
    }

    // ──────── Bans ────────

    /// `GET /guilds/{guild_id}/bans` – list bans in a guild.
    pub async fn list_guild_bans(
        &self, guild_id: Snowflake, limit: Option<u16>, before: Option<Snowflake>, after: Option<Snowflake>,
    ) -> Result<Vec<Ban>, Error> {
        if let Some(lim) = limit { check_member_limit(lim)?; }
        let mut qs = Vec::new();
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(a) = after { qs.push(format!("after={a}")); }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/bans{q_str}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/bans/{user_id}` – get ban object for a user.
    pub async fn get_guild_ban(&self, guild_id: Snowflake, user_id: Snowflake) -> Result<Ban, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/bans/{user_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/bans/{user_id}` – ban a user from the guild.
    pub async fn ban_user_from_guild(
        &self, guild_id: Snowflake, user_id: Snowflake, body: &CreateGuildBan, reason: Option<&str>,
    ) -> Result<(), Error> {
        if let Some(sec) = body.delete_message_seconds { check_delete_message_seconds(sec)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PUT", &format!("/guilds/{guild_id}/bans/{user_id}"), Some(b), headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `DELETE /guilds/{guild_id}/bans/{user_id}` – unban a user from the guild.
    pub async fn unban_user_from_guild(
        &self, guild_id: Snowflake, user_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/bans/{user_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `POST /guilds/{guild_id}/bulk-ban` – ban up to 200 users from a guild at once.
    pub async fn bulk_ban_users_from_guild(
        &self, guild_id: Snowflake, body: &BulkBanUsers, reason: Option<&str>,
    ) -> Result<BulkBanResponse, Error> {
        check_bulk_ban_count(body.user_ids.len())?;
        if let Some(sec) = body.delete_message_seconds { check_delete_message_seconds(sec)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/bulk-ban"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    // ──────── Prune ────────

    /// `GET /guilds/{guild_id}/prune` – preview how many members would be removed in a prune.
    pub async fn preview_prune_guild(
        &self, guild_id: Snowflake, days: Option<u32>, include_roles: Option<&[Snowflake]>,
    ) -> Result<PruneCount, Error> {
        let mut qs = Vec::new();
        if let Some(d) = days {
            check_prune_days(d)?;
            qs.push(format!("days={d}"));
        }
        if let Some(roles) = include_roles {
            let r_str = roles.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
            qs.push(format!("include_roles={r_str}"));
        }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/prune{q_str}"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/prune` – begin a prune operation.
    pub async fn prune_guild(
        &self, guild_id: Snowflake, body: &BeginPrune, reason: Option<&str>,
    ) -> Result<PruneCount, Error> {
        if let Some(d) = body.days { check_prune_days(d)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/prune"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    // ──────── Emojis ────────

    /// `GET /guilds/{guild_id}/emojis` – list emojis in a guild.
    pub async fn list_guild_emojis(&self, guild_id: Snowflake) -> Result<Vec<Emoji>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/emojis"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/emojis` – create a new emoji in a guild.
    pub async fn create_guild_emoji(
        &self, guild_id: Snowflake, body: &CreateGuildEmoji, reason: Option<&str>,
    ) -> Result<Emoji, Error> {
        check_emoji_name(&body.name)?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/emojis"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/emojis/{emoji_id}` – get a specific emoji in a guild.
    pub async fn get_guild_emoji(&self, guild_id: Snowflake, emoji_id: Snowflake) -> Result<Emoji, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/emojis/{emoji_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/emojis/{emoji_id}` – modify an existing emoji in a guild.
    pub async fn update_guild_emoji(
        &self, guild_id: Snowflake, emoji_id: Snowflake, body: &ModifyGuildEmoji, reason: Option<&str>,
    ) -> Result<Emoji, Error> {
        if let Some(n) = &body.name { check_emoji_name(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/emojis/{emoji_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/emojis/{emoji_id}` – delete an emoji from a guild.
    pub async fn delete_guild_emoji(
        &self, guild_id: Snowflake, emoji_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/emojis/{emoji_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    // ──────── Stickers ────────

    /// `GET /guilds/{guild_id}/stickers` – list stickers in a guild.
    pub async fn list_guild_stickers(&self, guild_id: Snowflake) -> Result<Vec<GuildSticker>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/stickers"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/stickers` – create a new sticker in a guild.
    pub async fn create_guild_sticker(
        &self, guild_id: Snowflake, body: &CreateGuildSticker, reason: Option<&str>,
    ) -> Result<GuildSticker, Error> {
        check_sticker_name(&body.name)?;
        if let Some(d) = &body.description { check_sticker_desc(d)?; }
        check_sticker_tags(&body.tags)?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/stickers"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/stickers/{sticker_id}` – get a specific sticker in a guild.
    pub async fn get_guild_sticker(&self, guild_id: Snowflake, sticker_id: Snowflake) -> Result<GuildSticker, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/stickers/{sticker_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/stickers/{sticker_id}` – modify an existing sticker in a guild.
    pub async fn update_guild_sticker(
        &self, guild_id: Snowflake, sticker_id: Snowflake, body: &ModifyGuildSticker, reason: Option<&str>,
    ) -> Result<GuildSticker, Error> {
        if let Some(n) = &body.name { check_sticker_name(n)?; }
        if let Some(Some(d)) = &body.description { check_sticker_desc(d)?; }
        if let Some(t) = &body.tags { check_sticker_tags(t)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/stickers/{sticker_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/stickers/{sticker_id}` – delete a sticker from a guild.
    pub async fn delete_guild_sticker(
        &self, guild_id: Snowflake, sticker_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/stickers/{sticker_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    // ──────── Audit Log ────────

    /// `GET /guilds/{guild_id}/audit-logs` – list audit log entries for a guild.
    pub async fn list_guild_audit_log_entries(
        &self, guild_id: Snowflake, query: &GetAuditLog,
    ) -> Result<AuditLog, Error> {
        if let Some(lim) = query.limit { check_audit_log_limit(lim)?; }
        let qs = query_string(query);
        self.request("GET", &format!("/guilds/{guild_id}/audit-logs{qs}"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Integrations, Webhooks, Invites ────────

    /// `GET /guilds/{guild_id}/integrations` – list integrations for a guild.
    pub async fn list_guild_integrations(&self, guild_id: Snowflake) -> Result<Vec<Integration>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/integrations"), None)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/integrations/{integration_id}` – delete an integration from a guild.
    pub async fn delete_guild_integration(
        &self, guild_id: Snowflake, integration_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/integrations/{integration_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/invites` – list active invites for a guild.
    pub async fn list_guild_invites(&self, guild_id: Snowflake) -> Result<Vec<crate::channel::Invite>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/invites"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/webhooks` – list webhooks for a guild.
    pub async fn get_guild_webhooks(&self, guild_id: Snowflake) -> Result<Vec<serde_json::Value>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/webhooks"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Widget & Welcome Screen & Onboarding ────────

    /// `GET /guilds/{guild_id}/widget` – get widget settings for a guild.
    pub async fn get_guild_widget_settings(&self, guild_id: Snowflake) -> Result<WidgetSettings, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/widget"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/widget` – update widget settings for a guild.
    pub async fn update_guild_widget_settings(
        &self, guild_id: Snowflake, body: &ModifyWidgetSettings, reason: Option<&str>,
    ) -> Result<WidgetSettings, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/widget"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/widget.json` – get widget JSON data for a guild.
    pub async fn get_guild_widget(&self, guild_id: Snowflake) -> Result<Widget, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/widget.json"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/widget.png` – get widget PNG banner/shield image.
    pub async fn get_guild_widget_png(&self, guild_id: Snowflake, style: Option<&str>) -> Result<Vec<u8>, Error> {
        let qs = style.map(|s| format!("?style={s}")).unwrap_or_default();
        let r = self.request("GET", &format!("/guilds/{guild_id}/widget.png{qs}"), None)
            .await?.expect_success()?;
        Ok(r.body)
    }

    /// `GET /guilds/{guild_id}/welcome-screen` – get welcome screen for a Community guild.
    pub async fn get_guild_welcome_screen(&self, guild_id: Snowflake) -> Result<WelcomeScreen, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/welcome-screen"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/welcome-screen` – update welcome screen for a Community guild.
    pub async fn update_guild_welcome_screen(
        &self, guild_id: Snowflake, body: &ModifyWelcomeScreen, reason: Option<&str>,
    ) -> Result<WelcomeScreen, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/welcome-screen"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/onboarding` – get onboarding configuration for a guild.
    pub async fn get_guilds_onboarding(&self, guild_id: Snowflake) -> Result<GuildOnboarding, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/onboarding"), None)
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/onboarding` – update onboarding configuration for a guild.
    pub async fn put_guilds_onboarding(
        &self, guild_id: Snowflake, body: &GuildOnboarding, reason: Option<&str>,
    ) -> Result<GuildOnboarding, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PUT", &format!("/guilds/{guild_id}/onboarding"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/incident-actions` – update incident action settings for a guild.
    pub async fn update_guild_incident_actions(
        &self, guild_id: Snowflake, body: &GuildIncidentsData,
    ) -> Result<GuildIncidentsData, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PUT", &format!("/guilds/{guild_id}/incident-actions"), Some(b))
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/new-member-welcome` – get new member welcome screen data.
    pub async fn get_guild_new_member_welcome(&self, guild_id: Snowflake) -> Result<serde_json::Value, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/new-member-welcome"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Voice States ────────

    /// `GET /guilds/{guild_id}/voice-states/@me` – get current user's voice state in a guild.
    pub async fn get_self_voice_state(&self, guild_id: Snowflake) -> Result<VoiceState, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/voice-states/@me"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/voice-states/@me` – update current user's voice state in a guild.
    pub async fn update_self_voice_state(
        &self, guild_id: Snowflake, body: &ModifyCurrentVoiceState,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", &format!("/guilds/{guild_id}/voice-states/@me"), Some(b))
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/voice-states/{user_id}` – get a user's voice state in a guild.
    pub async fn get_voice_state(&self, guild_id: Snowflake, user_id: Snowflake) -> Result<VoiceState, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/voice-states/{user_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/voice-states/{user_id}` – update another user's voice state in a guild.
    pub async fn update_voice_state(
        &self, guild_id: Snowflake, user_id: Snowflake, body: &ModifyVoiceState, reason: Option<&str>,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/voice-states/{user_id}"), Some(b), headers)
            .await?.expect_success()?;
        Ok(())
    }

    // ──────── Templates ────────

    /// `GET /guilds/{guild_id}/templates` – list templates for a guild.
    pub async fn list_guild_templates(&self, guild_id: Snowflake) -> Result<Vec<GuildTemplate>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/templates"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/templates` – create a guild template.
    pub async fn create_guild_template(
        &self, guild_id: Snowflake, body: &CreateGuildTemplate,
    ) -> Result<GuildTemplate, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", &format!("/guilds/{guild_id}/templates"), Some(b))
            .await?.expect_success()?.json()
    }

    /// `PUT /guilds/{guild_id}/templates/{code}` – sync a guild template to current guild state.
    pub async fn sync_guild_template(&self, guild_id: Snowflake, code: &str) -> Result<GuildTemplate, Error> {
        self.request("PUT", &format!("/guilds/{guild_id}/templates/{code}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/templates/{code}` – modify a guild template.
    pub async fn update_guild_template(
        &self, guild_id: Snowflake, code: &str, body: &ModifyGuildTemplate,
    ) -> Result<GuildTemplate, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", &format!("/guilds/{guild_id}/templates/{code}"), Some(b))
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/templates/{code}` – delete a guild template.
    pub async fn delete_guild_template(&self, guild_id: Snowflake, code: &str) -> Result<GuildTemplate, Error> {
        self.request("DELETE", &format!("/guilds/{guild_id}/templates/{code}"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Auto Moderation ────────

    /// `GET /guilds/{guild_id}/auto-moderation/rules` – list auto moderation rules for a guild.
    pub async fn list_auto_moderation_rules(&self, guild_id: Snowflake) -> Result<Vec<AutoModerationRule>, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/auto-moderation/rules"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/auto-moderation/rules` – create an auto moderation rule.
    pub async fn create_auto_moderation_rule(
        &self, guild_id: Snowflake, body: &CreateAutoModerationRule, reason: Option<&str>,
    ) -> Result<AutoModerationRule, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/auto-moderation/rules"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/auto-moderation/rules/{rule_id}` – get a single auto moderation rule.
    pub async fn get_auto_moderation_rule(
        &self, guild_id: Snowflake, rule_id: Snowflake,
    ) -> Result<AutoModerationRule, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/auto-moderation/rules/{rule_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/auto-moderation/rules/{rule_id}` – modify an auto moderation rule.
    pub async fn update_auto_moderation_rule(
        &self, guild_id: Snowflake, rule_id: Snowflake, body: &ModifyAutoModerationRule, reason: Option<&str>,
    ) -> Result<AutoModerationRule, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "PATCH",
            &format!("/guilds/{guild_id}/auto-moderation/rules/{rule_id}"),
            Some(b),
            headers,
        ).await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/auto-moderation/rules/{rule_id}` – delete an auto moderation rule.
    pub async fn delete_auto_moderation_rule(
        &self, guild_id: Snowflake, rule_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "DELETE",
            &format!("/guilds/{guild_id}/auto-moderation/rules/{rule_id}"),
            None,
            headers,
        ).await?.expect_success()?;
        Ok(())
    }

    // ──────── Scheduled Events ────────

    /// `GET /guilds/{guild_id}/scheduled-events` – list scheduled events for a guild.
    pub async fn list_guild_scheduled_events(
        &self, guild_id: Snowflake, with_user_count: Option<bool>,
    ) -> Result<Vec<GuildScheduledEvent>, Error> {
        let qs = with_user_count.map(|b| format!("?with_user_count={b}")).unwrap_or_default();
        self.request("GET", &format!("/guilds/{guild_id}/scheduled-events{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/scheduled-events` – create a scheduled event.
    pub async fn create_guild_scheduled_event(
        &self, guild_id: Snowflake, body: &CreateGuildScheduledEvent, reason: Option<&str>,
    ) -> Result<GuildScheduledEvent, Error> {
        check_event_name(&body.name)?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/scheduled-events"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}` – get a scheduled event by id.
    pub async fn get_guild_scheduled_event(
        &self, guild_id: Snowflake, event_id: Snowflake, with_user_count: Option<bool>,
    ) -> Result<GuildScheduledEvent, Error> {
        let qs = with_user_count.map(|b| format!("?with_user_count={b}")).unwrap_or_default();
        self.request("GET", &format!("/guilds/{guild_id}/scheduled-events/{event_id}{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}` – modify a scheduled event.
    pub async fn update_guild_scheduled_event(
        &self, guild_id: Snowflake, event_id: Snowflake, body: &ModifyGuildScheduledEvent, reason: Option<&str>,
    ) -> Result<GuildScheduledEvent, Error> {
        if let Some(n) = &body.name { check_event_name(n)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "PATCH",
            &format!("/guilds/{guild_id}/scheduled-events/{event_id}"),
            Some(b),
            headers,
        ).await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}` – delete a scheduled event.
    pub async fn delete_guild_scheduled_event(
        &self, guild_id: Snowflake, event_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/scheduled-events/{event_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/users` – list users subscribed to an event.
    pub async fn list_guild_scheduled_event_users(
        &self, guild_id: Snowflake, event_id: Snowflake, limit: Option<u16>, with_member: Option<bool>,
        before: Option<Snowflake>, after: Option<Snowflake>,
    ) -> Result<Vec<GuildScheduledEventUser>, Error> {
        if let Some(lim) = limit { check_member_limit(lim)?; }
        let mut qs = Vec::new();
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        if let Some(m) = with_member { qs.push(format!("with_member={m}")); }
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(a) = after { qs.push(format!("after={a}")); }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/scheduled-events/{event_id}/users{q_str}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/users/counts` – count users subscribed to an event.
    pub async fn count_guild_scheduled_event_users(
        &self, guild_id: Snowflake, event_id: Snowflake, exception_ids: Option<&[Snowflake]>,
    ) -> Result<Vec<GuildScheduledEventUserCount>, Error> {
        let mut qs = Vec::new();
        if let Some(ids) = exception_ids {
            let id_str = ids.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
            qs.push(format!("guild_scheduled_event_exception_ids={id_str}"));
        }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/scheduled-events/{event_id}/users/counts{q_str}"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/exceptions` – create event exception.
    pub async fn create_guild_scheduled_event_exception(
        &self, guild_id: Snowflake, event_id: Snowflake, body: &CreateGuildScheduledEventException, reason: Option<&str>,
    ) -> Result<GuildScheduledEventException, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "POST",
            &format!("/guilds/{guild_id}/scheduled-events/{event_id}/exceptions"),
            Some(b),
            headers,
        ).await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/exceptions/{exception_id}` – modify event exception.
    pub async fn update_guild_scheduled_event_exception(
        &self, guild_id: Snowflake, event_id: Snowflake, exception_id: Snowflake,
        body: &ModifyGuildScheduledEventException, reason: Option<&str>,
    ) -> Result<GuildScheduledEventException, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "PATCH",
            &format!("/guilds/{guild_id}/scheduled-events/{event_id}/exceptions/{exception_id}"),
            Some(b),
            headers,
        ).await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/exceptions/{exception_id}` – delete event exception.
    pub async fn delete_guild_scheduled_event_exception(
        &self, guild_id: Snowflake, event_id: Snowflake, exception_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers(
            "DELETE",
            &format!("/guilds/{guild_id}/scheduled-events/{event_id}/exceptions/{exception_id}"),
            None,
            headers,
        ).await?.expect_success()?;
        Ok(())
    }

    /// `GET /guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/{guild_scheduled_event_exception_id}/users` – list users subscribed to an exception.
    #[allow(clippy::too_many_arguments)]
    pub async fn list_guild_scheduled_event_exception_users(
        &self, guild_id: Snowflake, event_id: Snowflake, exception_id: Snowflake, limit: Option<u16>,
        with_member: Option<bool>, before: Option<Snowflake>, after: Option<Snowflake>,
    ) -> Result<Vec<GuildScheduledEventUser>, Error> {
        if let Some(lim) = limit { check_member_limit(lim)?; }
        let mut qs = Vec::new();
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        if let Some(m) = with_member { qs.push(format!("with_member={m}")); }
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(a) = after { qs.push(format!("after={a}")); }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/scheduled-events/{event_id}/{exception_id}/users{q_str}"), None)
            .await?.expect_success()?.json()
    }

    // ──────── Soundboard ────────

    /// `GET /guilds/{guild_id}/soundboard-sounds` – list sounds in a guild's soundboard.
    pub async fn list_guild_soundboard_sounds(&self, guild_id: Snowflake) -> Result<SoundboardSoundsPage, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/soundboard-sounds"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /guilds/{guild_id}/soundboard-sounds` – create a new soundboard sound.
    pub async fn create_guild_soundboard_sound(
        &self, guild_id: Snowflake, body: &CreateSoundboardSound, reason: Option<&str>,
    ) -> Result<SoundboardSound, Error> {
        check_sound_name(&body.name)?;
        if let Some(v) = body.volume { check_sound_volume(v)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/guilds/{guild_id}/soundboard-sounds"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/soundboard-sounds/{sound_id}` – get a specific soundboard sound.
    pub async fn get_guild_soundboard_sound(
        &self, guild_id: Snowflake, sound_id: Snowflake,
    ) -> Result<SoundboardSound, Error> {
        self.request("GET", &format!("/guilds/{guild_id}/soundboard-sounds/{sound_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/soundboard-sounds/{sound_id}` – modify an existing soundboard sound.
    pub async fn update_guild_soundboard_sound(
        &self, guild_id: Snowflake, sound_id: Snowflake, body: &ModifySoundboardSound, reason: Option<&str>,
    ) -> Result<SoundboardSound, Error> {
        if let Some(n) = &body.name { check_sound_name(n)?; }
        if let Some(Some(v)) = body.volume { check_sound_volume(v)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/guilds/{guild_id}/soundboard-sounds/{sound_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `DELETE /guilds/{guild_id}/soundboard-sounds/{sound_id}` – delete a soundboard sound.
    pub async fn delete_guild_soundboard_sound(
        &self, guild_id: Snowflake, sound_id: Snowflake, reason: Option<&str>,
    ) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/guilds/{guild_id}/soundboard-sounds/{sound_id}"), None, headers)
            .await?.expect_success()?;
        Ok(())
    }

    // ──────── Join Requests & Message Search ────────

    /// `GET /guilds/{guild_id}/requests` – get membership join requests.
    pub async fn get_guild_join_requests(
        &self, guild_id: Snowflake, status: Option<&str>, limit: Option<u32>,
        before: Option<Snowflake>, after: Option<Snowflake>,
    ) -> Result<GuildJoinRequestsPage, Error> {
        let mut qs = Vec::new();
        if let Some(s) = status { qs.push(format!("status={s}")); }
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(a) = after { qs.push(format!("after={a}")); }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        self.request("GET", &format!("/guilds/{guild_id}/requests{q_str}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /guilds/{guild_id}/requests/{request_id}` – approve or reject a join request.
    pub async fn action_guild_join_request(
        &self, guild_id: Snowflake, request_id: Snowflake, body: &ActionGuildJoinRequest,
    ) -> Result<GuildJoinRequest, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", &format!("/guilds/{guild_id}/requests/{request_id}"), Some(b))
            .await?.expect_success()?.json()
    }

    /// `GET /guilds/{guild_id}/messages/search` – search messages in a guild.
    pub async fn guild_search(&self, guild_id: Snowflake, query_params: &str) -> Result<serde_json::Value, Error> {
        let q = if query_params.is_empty() || query_params.starts_with('?') {
            query_params.to_owned()
        } else {
            format!("?{query_params}")
        };
        self.request("GET", &format!("/guilds/{guild_id}/messages/search{q}"), None)
            .await?.expect_success()?.json()
    }
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};
    use serde_json::json;

    fn sf(n: u64) -> Snowflake { Snowflake::new(n) }

    fn minimal_guild_json(id: u64, name: &str) -> String {
        json!({
            "id": id.to_string(),
            "name": name,
            "icon": null,
            "description": null,
            "home_header": null,
            "splash": null,
            "discovery_splash": null,
            "features": [],
            "banner": null,
            "owner_id": "100",
            "region": "us-west",
            "afk_timeout": 300,
            "system_channel_flags": 0,
            "verification_level": 0,
            "roles": [],
            "default_message_notifications": 0,
            "mfa_level": 0,
            "explicit_content_filter": 0,
            "premium_tier": 0,
            "preferred_locale": "en-US",
            "premium_progress_bar_enabled": false,
            "nsfw": false,
            "nsfw_level": 0,
            "emojis": [],
            "stickers": []
        }).to_string()
    }
    fn minimal_member_val(id: u64) -> serde_json::Value {
        json!({
            "user": {
                "id": id.to_string(),
                "username": "testuser",
                "discriminator": "0001",
                "avatar": null
            },
            "nick": null,
            "avatar": null,
            "banner": null,
            "roles": [],
            "joined_at": "2026-01-01T00:00:00.000Z",
            "deaf": false,
            "mute": false,
            "flags": 0,
            "pending": false,
            "communication_disabled_until": null,
            "premium_since": null
        })
    }

    fn minimal_member_json(id: u64) -> String {
        minimal_member_val(id).to_string()
    }

    fn minimal_role_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "name": "Role",
            "color": 0,
            "hoist": false,
            "position": 1,
            "permissions": "0",
            "managed": false,
            "mentionable": false,
            "flags": 0
        })
    }

    fn minimal_role_json(id: u64) -> String {
        minimal_role_val(id).to_string()
    }

    fn minimal_emoji_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "name": "emoji",
            "roles": [],
            "require_colons": true,
            "managed": false,
            "animated": false,
            "available": true
        })
    }

    fn minimal_emoji_json(id: u64) -> String {
        minimal_emoji_val(id).to_string()
    }

    fn minimal_sticker_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "name": "sticker",
            "description": null,
            "tags": "tag",
            "type": 1,
            "format_type": 1
        })
    }

    fn minimal_sticker_json(id: u64) -> String {
        minimal_sticker_val(id).to_string()
    }

    fn minimal_template_val(code: &str) -> serde_json::Value {
        json!({
            "code": code,
            "name": "Template",
            "description": null,
            "usage_count": 0,
            "creator_id": "100",
            "created_at": "2026-01-01T00:00:00.000Z",
            "updated_at": "2026-01-01T00:00:00.000Z",
            "source_guild_id": "1",
            "serialized_source_guild": serde_json::from_str::<serde_json::Value>(&minimal_guild_json(1, "Source")).unwrap(),
            "is_dirty": false
        })
    }

    fn minimal_template_json(code: &str) -> String {
        minimal_template_val(code).to_string()
    }

    fn minimal_automod_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "guild_id": "1",
            "name": "rule",
            "creator_id": "100",
            "event_type": 1,
            "trigger_type": 1,
            "trigger_metadata": {},
            "actions": [],
            "enabled": true,
            "exempt_roles": [],
            "exempt_channels": []
        })
    }

    fn minimal_automod_json(id: u64) -> String {
        minimal_automod_val(id).to_string()
    }

    fn minimal_event_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "guild_id": "1",
            "channel_id": "2",
            "name": "Event",
            "privacy_level": 2,
            "status": 1,
            "entity_type": 2,
            "scheduled_start_time": "2026-11-01T00:00:00Z"
        })
    }

    fn minimal_event_json(id: u64) -> String {
        minimal_event_val(id).to_string()
    }

    fn minimal_event_exception_val(event_id: u64, exception_id: u64) -> serde_json::Value {
        json!({
            "event_id": event_id.to_string(),
            "event_exception_id": exception_id.to_string(),
            "scheduled_start_time": "2026-11-01T00:00:00Z",
            "scheduled_end_time": null,
            "is_canceled": false
        })
    }

    fn minimal_event_exception_json(event_id: u64, exception_id: u64) -> String {
        minimal_event_exception_val(event_id, exception_id).to_string()
    }

    fn minimal_sound_val(id: u64) -> serde_json::Value {
        json!({
            "sound_id": id.to_string(),
            "name": "sound",
            "volume": 0.5,
            "available": true
        })
    }

    fn minimal_sound_json(id: u64) -> String {
        minimal_sound_val(id).to_string()
    }

    fn minimal_join_request_val(id: u64) -> serde_json::Value {
        json!({
            "id": id.to_string(),
            "guild_id": "1",
            "user_id": "2",
            "created_at": "2026-01-01T00:00:00Z",
            "application_status": "SUBMITTED"
        })
    }

    fn minimal_join_request_json(id: u64) -> String {
        minimal_join_request_val(id).to_string()
    }

    fn minimal_voice_state_json(guild_id: u64, user_id: u64) -> String {
        json!({
            "guild_id": guild_id.to_string(),
            "channel_id": "2",
            "user_id": user_id.to_string(),
            "session_id": "sess_123",
            "deaf": false,
            "mute": false,
            "self_deaf": false,
            "self_mute": false,
            "self_video": false,
            "suppress": false
        }).to_string()
    }

    #[test]
    fn validation_rules_match_documented_limits() {
        assert!(matches!(check_guild_name("a"), Err(GuildError::BadGuildName { len: 1 })));
        assert!(matches!(check_guild_name(&"x".repeat(101)), Err(GuildError::BadGuildName { len: 101 })));
        assert!(check_guild_name("Valid Guild Name").is_ok());

        assert!(matches!(check_channel_name(""), Err(GuildError::BadChannelName { len: 0 })));
        assert!(matches!(check_channel_name(&"c".repeat(101)), Err(GuildError::BadChannelName { len: 101 })));
        assert!(check_channel_name("general").is_ok());

        assert!(matches!(check_role_name(""), Err(GuildError::BadRoleName { len: 0 })));
        assert!(matches!(check_role_name(&"r".repeat(101)), Err(GuildError::BadRoleName { len: 101 })));
        assert!(check_role_name("Moderator").is_ok());

        assert!(matches!(check_nick(&"n".repeat(33)), Err(GuildError::BadNick { len: 33 })));
        assert!(check_nick("CoolNick").is_ok());

        assert!(matches!(check_member_limit(0), Err(GuildError::BadMemberLimit { value: 0 })));
        assert!(matches!(check_member_limit(1001), Err(GuildError::BadMemberLimit { value: 1001 })));
        assert!(check_member_limit(100).is_ok());

        assert!(matches!(check_search_query(""), Err(GuildError::BadSearchQuery { len: 0 })));
        assert!(matches!(check_search_query(&"q".repeat(101)), Err(GuildError::BadSearchQuery { len: 101 })));
        assert!(check_search_query("alice").is_ok());

        assert!(matches!(check_delete_message_seconds(604801), Err(GuildError::BadDeleteMessageSeconds { value: 604801 })));
        assert!(check_delete_message_seconds(86400).is_ok());

        assert!(matches!(check_bulk_ban_count(0), Err(GuildError::BadBulkBanCount { count: 0 })));
        assert!(matches!(check_bulk_ban_count(201), Err(GuildError::BadBulkBanCount { count: 201 })));
        assert!(check_bulk_ban_count(5).is_ok());

        assert!(matches!(check_prune_days(0), Err(GuildError::BadPruneDays { value: 0 })));
        assert!(matches!(check_prune_days(31), Err(GuildError::BadPruneDays { value: 31 })));
        assert!(check_prune_days(7).is_ok());

        assert!(matches!(check_emoji_name("a"), Err(GuildError::BadEmojiName { len: 1 })));
        assert!(matches!(check_emoji_name(&"e".repeat(33)), Err(GuildError::BadEmojiName { len: 33 })));
        assert!(check_emoji_name("pepe_happy").is_ok());

        assert!(matches!(check_sticker_name("a"), Err(GuildError::BadStickerName { len: 1 })));
        assert!(matches!(check_sticker_name(&"s".repeat(31)), Err(GuildError::BadStickerName { len: 31 })));
        assert!(check_sticker_name("hype").is_ok());

        assert!(matches!(check_sticker_desc(&"d".repeat(101)), Err(GuildError::BadStickerDescription { len: 101 })));
        assert!(check_sticker_desc("A cool sticker").is_ok());

        assert!(matches!(check_sticker_tags(&"t".repeat(201)), Err(GuildError::BadStickerTags { len: 201 })));
        assert!(check_sticker_tags("happy, hype, fun").is_ok());

        assert!(matches!(check_audit_log_limit(0), Err(GuildError::BadAuditLogLimit { value: 0 })));
        assert!(matches!(check_audit_log_limit(101), Err(GuildError::BadAuditLogLimit { value: 101 })));
        assert!(check_audit_log_limit(50).is_ok());

        assert!(matches!(check_event_name(""), Err(GuildError::BadEventName { len: 0 })));
        assert!(matches!(check_event_name(&"e".repeat(101)), Err(GuildError::BadEventName { len: 101 })));
        assert!(check_event_name("Game Night").is_ok());

        assert!(matches!(check_sound_name("a"), Err(GuildError::BadSoundName { len: 1 })));
        assert!(matches!(check_sound_name(&"s".repeat(33)), Err(GuildError::BadSoundName { len: 33 })));
        assert!(check_sound_name("applause").is_ok());

        assert!(matches!(check_sound_volume(-0.1), Err(GuildError::BadSoundVolume { .. })));
        assert!(matches!(check_sound_volume(1.1), Err(GuildError::BadSoundVolume { .. })));
        assert!(check_sound_volume(0.75).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn core_guild_routes_pinning() {
        let g_raw = minimal_guild_json(1, "Test Guild");
        let preview_raw = json!({
            "id": "1", "name": "Preview Guild", "icon": null, "splash": null,
            "discovery_splash": null, "emojis": [], "features": [],
            "approximate_member_count": 42, "approximate_presence_count": 10,
            "description": null, "stickers": []
        }).to_string();
        let vanity_raw = json!({ "code": "my-server", "uses": 12 }).to_string();
        let regions_raw = json!([{
            "id": "us-west", "name": "US West", "optimal": true, "deprecated": false, "custom": false
        }]).to_string();

        let c = client(vec![
            resp(200, &g_raw),
            resp(200, &g_raw),
            resp(200, &preview_raw),
            resp(200, &vanity_raw),
            resp(200, &regions_raw),
        ]);

        let g1 = c.get_guild(sf(1), Some(true)).await.unwrap();
        assert_eq!(g1.name, "Test Guild");

        let mod_guild = ModifyGuild {
            name: Some("Updated Guild".into()),
            ..Default::default()
        };
        c.update_guild(sf(1), &mod_guild, Some("audit test")).await.unwrap();

        let prev = c.get_guild_preview(sf(1)).await.unwrap();
        assert_eq!(prev.approximate_member_count, 42);

        let van = c.get_guild_vanity_url(sf(1)).await.unwrap();
        assert_eq!(van.code.as_deref(), Some("my-server"));

        let regs = c.list_guild_voice_regions(sf(1)).await.unwrap();
        assert_eq!(regs[0].id, "us-west");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1?with_counts=true"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1"));
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit test"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/preview"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/vanity-url"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/regions"));
    }

    #[tokio::test(start_paused = true)]
    async fn channel_guild_routes_pinning() {
        let ch_raw = json!([{ "id": "2", "type": 0, "name": "general" }]).to_string();
        let ch_single = json!({ "id": "2", "type": 0, "name": "general" }).to_string();
        let threads_raw = json!({ "threads": [], "members": [] }).to_string();

        let c = client(vec![
            resp(200, &ch_raw),
            resp(201, &ch_single),
            resp(204, ""),
            resp(200, &threads_raw),
        ]);

        c.list_guild_channels(sf(1)).await.unwrap();
        c.create_guild_channel(sf(1), &CreateGuildChannel {
            name: "announcements".into(),
            kind: Some(5),
            topic: None,
            bitrate: None,
            user_limit: None,
            rate_limit_per_user: None,
            position: None,
            permission_overwrites: None,
            parent_id: None,
            nsfw: None,
            rtc_region: None,
            video_quality_mode: None,
            default_auto_archive_duration: None,
        }, Some("new channel")).await.unwrap();

        c.bulk_update_guild_channels(sf(1), &[ModifyGuildChannelPosition {
            id: sf(2),
            position: Some(1),
            lock_permissions: None,
            parent_id: None,
        }], None).await.unwrap();

        c.get_active_guild_threads(sf(1)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/channels"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/channels"));
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "new channel"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/channels"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/threads/active"));
    }

    #[tokio::test(start_paused = true)]
    async fn member_routes_pinning() {
        let mem_raw = json!({
            "user": { "id": "2", "username": "bob" },
            "nick": "Bobby",
            "roles": ["3", "4"],
            "joined_at": "2026-01-01T00:00:00Z",
            "deaf": false,
            "mute": false,
            "flags": 0
        }).to_string();
        let list_mem_raw = json!([serde_json::from_str::<serde_json::Value>(&mem_raw).unwrap()]).to_string();
        let perm_raw = json!({ "permissions": "1024" }).to_string();

        let c = client(vec![
            resp(200, &list_mem_raw),
            resp(200, &list_mem_raw),
            resp(200, &mem_raw),
            resp(201, &mem_raw),
            resp(204, ""),
            resp(200, &mem_raw),
            resp(204, ""),
            resp(200, &mem_raw),
            resp(204, ""),
            resp(204, ""),
            resp(200, &perm_raw),
        ]);

        c.list_guild_members(sf(1), &ListGuildMembers { limit: Some(50), after: Some(sf(100)) }).await.unwrap();
        c.search_guild_members(sf(1), &SearchGuildMembers { query: "bob".into(), limit: Some(10) }).await.unwrap();
        c.get_guild_member(sf(1), sf(2)).await.unwrap();

        let added = c.add_guild_member(sf(1), sf(2), &AddGuildMember {
            access_token: "tok123".into(),
            nick: Some("Bobby".into()),
            roles: None,
            mute: None,
            deaf: None,
            flags: None,
        }).await.unwrap();
        assert!(added.is_some());

        let added_existing = c.add_guild_member(sf(1), sf(2), &AddGuildMember {
            access_token: "tok123".into(),
            nick: None,
            roles: None,
            mute: None,
            deaf: None,
            flags: None,
        }).await.unwrap();
        assert!(added_existing.is_none());

        c.update_guild_member(sf(1), sf(2), &ModifyGuildMember {
            nick: Some(Some("Rob".into())),
            ..Default::default()
        }, Some("nick change")).await.unwrap();

        c.delete_guild_member(sf(1), sf(2), Some("kick")).await.unwrap();
        c.update_my_guild_member(sf(1), &ModifyCurrentMember { nick: Some(Some("SelfNick".into())), ..Default::default() }, None).await.unwrap();
        c.add_guild_member_role(sf(1), sf(2), sf(3), Some("role add")).await.unwrap();
        c.delete_guild_member_role(sf(1), sf(2), sf(3), Some("role del")).await.unwrap();
        let perm = c.get_guild_member_permissions(sf(1), sf(2)).await.unwrap();
        assert_eq!(perm, Permissions::from_bits_retain(1024));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/members?after=100&limit=50"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/members/search?limit=10&query=bob"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/members/2"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/members/2"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/members/2"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/members/2"));
        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/members/2"));
        assert_eq!((seen[7].method.as_str(), seen[7].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/members/@me"));
        assert_eq!((seen[8].method.as_str(), seen[8].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/members/2/roles/3"));
        assert_eq!((seen[9].method.as_str(), seen[9].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/members/2/roles/3"));
        assert_eq!((seen[10].method.as_str(), seen[10].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/permissions/member/2"));
    }

    #[tokio::test(start_paused = true)]
    async fn role_routes_pinning() {
        let role_raw = json!({
            "id": "3",
            "name": "Admin",
            "color": 0xFF0000,
            "hoist": true,
            "icon": null,
            "unicode_emoji": null,
            "position": 1,
            "permissions": "8",
            "managed": false,
            "mentionable": true,
            "flags": 0
        }).to_string();
        let list_roles = json!([serde_json::from_str::<serde_json::Value>(&role_raw).unwrap()]).to_string();
        let member_counts = json!({ "3": 5 }).to_string();
        let perm_raw = json!({ "permissions": "8" }).to_string();

        let c = client(vec![
            resp(200, &list_roles),
            resp(200, &role_raw),
            resp(200, &list_roles),
            resp(200, &member_counts),
            resp(200, &role_raw),
            resp(200, &role_raw),
            resp(204, ""),
            resp(200, &perm_raw),
        ]);

        c.list_guild_roles(sf(1)).await.unwrap();
        c.create_guild_role(sf(1), &CreateGuildRole {
            name: Some("Mod".into()),
            permissions: Some(Permissions::MANAGE_ROLES),
            ..Default::default()
        }, Some("role create")).await.unwrap();

        c.bulk_update_guild_roles(sf(1), &[ModifyGuildRolePosition { id: sf(3), position: Some(2) }], None).await.unwrap();
        let counts = c.guild_role_member_counts(sf(1)).await.unwrap();
        assert_eq!(counts.get(&sf(3)), Some(&5));

        c.get_guild_role(sf(1), sf(3)).await.unwrap();
        c.update_guild_role(sf(1), sf(3), &ModifyGuildRole {
            name: Some(Some("Super Admin".into())),
            ..Default::default()
        }, Some("role rename")).await.unwrap();

        c.delete_guild_role(sf(1), sf(3), Some("delete role")).await.unwrap();
        c.get_guild_role_permissions(sf(1), sf(3)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/roles"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/roles"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/roles"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/roles/member-counts"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/roles/3"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/roles/3"));
        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/roles/3"));
        assert_eq!((seen[7].method.as_str(), seen[7].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/permissions/role/3"));
    }

    #[tokio::test(start_paused = true)]
    async fn ban_and_prune_routes_pinning() {
        let ban_raw = json!({
            "reason": "bad behavior",
            "user": { "id": "2", "username": "baduser" }
        }).to_string();
        let list_bans = json!([serde_json::from_str::<serde_json::Value>(&ban_raw).unwrap()]).to_string();
        let bulk_resp = json!({ "banned_users": ["2", "3"], "failed_users": [] }).to_string();
        let prune_resp = json!({ "pruned": 15 }).to_string();

        let c = client(vec![
            resp(200, &list_bans),
            resp(200, &ban_raw),
            resp(204, ""),
            resp(204, ""),
            resp(200, &bulk_resp),
            resp(200, &prune_resp),
            resp(200, &prune_resp),
        ]);

        c.list_guild_bans(sf(1), Some(50), None, None).await.unwrap();
        c.get_guild_ban(sf(1), sf(2)).await.unwrap();
        c.ban_user_from_guild(sf(1), sf(2), &CreateGuildBan { delete_message_seconds: Some(86400), ..Default::default() }, Some("bye")).await.unwrap();
        c.unban_user_from_guild(sf(1), sf(2), Some("welcome back")).await.unwrap();
        c.bulk_ban_users_from_guild(sf(1), &BulkBanUsers { user_ids: vec![sf(2), sf(3)], delete_message_seconds: None }, None).await.unwrap();

        let p1 = c.preview_prune_guild(sf(1), Some(7), Some(&[sf(4)])).await.unwrap();
        assert_eq!(p1.pruned, Some(15));
        let p2 = c.prune_guild(sf(1), &BeginPrune { days: Some(7), compute_prune_count: Some(true), include_roles: None }, None).await.unwrap();
        assert_eq!(p2.pruned, Some(15));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/bans?limit=50"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/bans/2"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/bans/2"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/bans/2"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/bulk-ban"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/prune?days=7&include_roles=4"));
        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/prune"));
    }

    #[tokio::test(start_paused = true)]
    async fn emoji_and_sticker_routes_pinning() {
        let emoji_raw = json!({
            "id": "5", "name": "happy", "roles": [], "user": null,
            "require_colons": true, "managed": false, "animated": false, "available": true
        }).to_string();
        let list_emojis = json!([serde_json::from_str::<serde_json::Value>(&emoji_raw).unwrap()]).to_string();

        let sticker_raw = json!({
            "id": "6", "name": "hype", "description": "very hyped", "tags": "hype",
            "type": 2, "format_type": 1, "available": true, "guild_id": "1"
        }).to_string();
        let list_stickers = json!([serde_json::from_str::<serde_json::Value>(&sticker_raw).unwrap()]).to_string();

        let c = client(vec![
            resp(200, &list_emojis),
            resp(201, &emoji_raw),
            resp(200, &emoji_raw),
            resp(200, &emoji_raw),
            resp(204, ""),
            resp(200, &list_stickers),
            resp(201, &sticker_raw),
            resp(200, &sticker_raw),
            resp(200, &sticker_raw),
            resp(204, ""),
        ]);

        c.list_guild_emojis(sf(1)).await.unwrap();
        c.create_guild_emoji(sf(1), &CreateGuildEmoji {
            name: "happy".into(),
            image: "data:image/png;base64,abc".into(),
            roles: None,
        }, Some("new emoji")).await.unwrap();
        c.get_guild_emoji(sf(1), sf(5)).await.unwrap();
        c.update_guild_emoji(sf(1), sf(5), &ModifyGuildEmoji { name: Some("super_happy".into()), roles: None }, None).await.unwrap();
        c.delete_guild_emoji(sf(1), sf(5), Some("remove emoji")).await.unwrap();

        c.list_guild_stickers(sf(1)).await.unwrap();
        c.create_guild_sticker(sf(1), &CreateGuildSticker {
            name: "hype".into(),
            description: Some("very hyped".into()),
            tags: "hype".into(),
            file: None,
        }, Some("new sticker")).await.unwrap();
        c.get_guild_sticker(sf(1), sf(6)).await.unwrap();
        c.update_guild_sticker(sf(1), sf(6), &ModifyGuildSticker { name: Some("mega_hype".into()), description: None, tags: None }, None).await.unwrap();
        c.delete_guild_sticker(sf(1), sf(6), Some("remove sticker")).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/emojis"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/emojis"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/emojis/5"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/emojis/5"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/emojis/5"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/stickers"));
        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/stickers"));
        assert_eq!((seen[7].method.as_str(), seen[7].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/stickers/6"));
        assert_eq!((seen[8].method.as_str(), seen[8].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/stickers/6"));
        assert_eq!((seen[9].method.as_str(), seen[9].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/stickers/6"));
    }

    #[tokio::test(start_paused = true)]
    async fn audit_log_and_integrations_routes_pinning() {
        let audit_raw = json!({
            "audit_log_entries": [{
                "id": "10",
                "target_id": "2",
                "changes": [],
                "user_id": "1",
                "action_type": 1,
                "options": null,
                "reason": "test"
            }],
            "users": [],
            "integrations": [],
            "webhooks": [],
            "guild_scheduled_events": [],
            "threads": [],
            "auto_moderation_rules": []
        }).to_string();
        let integ_raw = json!([{
            "id": "11", "name": "YouTube", "type": "youtube", "enabled": true,
            "account": { "id": "acc1", "name": "Channel 1" }
        }]).to_string();

        let c = client(vec![
            resp(200, &audit_raw),
            resp(200, &integ_raw),
            resp(204, ""),
            resp(200, "[]"),
            resp(200, "[]"),
        ]);

        c.list_guild_audit_log_entries(sf(1), &GetAuditLog { limit: Some(50), ..Default::default() }).await.unwrap();
        c.list_guild_integrations(sf(1)).await.unwrap();
        c.delete_guild_integration(sf(1), sf(11), Some("remove integration")).await.unwrap();
        c.list_guild_invites(sf(1)).await.unwrap();
        c.get_guild_webhooks(sf(1)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/audit-logs?limit=50"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/integrations"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/integrations/11"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/invites"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/webhooks"));
    }

    #[tokio::test(start_paused = true)]
    async fn widget_welcome_onboarding_voice_routes_pinning() {
        let widget_settings_raw = json!({ "enabled": true, "channel_id": "2" }).to_string();
        let widget_raw = json!({ "id": "1", "name": "Guild", "instant_invite": null, "channels": [], "members": [], "presence_count": 5 }).to_string();
        let welcome_raw = json!({ "description": "Welcome!", "welcome_channels": [] }).to_string();
        let onboarding_raw = json!({ "guild_id": "1", "prompts": [], "default_channel_ids": [], "enabled": true, "mode": 0 }).to_string();
        let incidents_raw = json!({ "invites_disabled_until": null, "dms_disabled_until": null }).to_string();
        let voice_state_raw = json!({
            "guild_id": "1", "channel_id": "2", "user_id": "3", "session_id": "sess1",
            "deaf": false, "mute": false, "self_deaf": false, "self_mute": false, "self_video": false, "suppress": false
        }).to_string();

        let c = client(vec![
            resp(200, &widget_settings_raw),
            resp(200, &widget_settings_raw),
            resp(200, &widget_raw),
            resp(200, "PNG_BYTES"),
            resp(200, &welcome_raw),
            resp(200, &welcome_raw),
            resp(200, &onboarding_raw),
            resp(200, &onboarding_raw),
            resp(200, &incidents_raw),
            resp(200, "{}"),
            resp(200, &voice_state_raw),
            resp(204, ""),
            resp(200, &voice_state_raw),
            resp(204, ""),
        ]);

        c.get_guild_widget_settings(sf(1)).await.unwrap();
        c.update_guild_widget_settings(sf(1), &ModifyWidgetSettings { enabled: Some(true), channel_id: None }, None).await.unwrap();
        c.get_guild_widget(sf(1)).await.unwrap();
        let png = c.get_guild_widget_png(sf(1), Some("shield")).await.unwrap();
        assert_eq!(png, b"PNG_BYTES");

        c.get_guild_welcome_screen(sf(1)).await.unwrap();
        c.update_guild_welcome_screen(sf(1), &ModifyWelcomeScreen { description: Some(Some("Hey".into())), ..Default::default() }, None).await.unwrap();

        c.get_guilds_onboarding(sf(1)).await.unwrap();
        c.put_guilds_onboarding(sf(1), &GuildOnboarding { guild_id: sf(1), prompts: vec![], default_channel_ids: vec![], enabled: true, mode: 0 }, None).await.unwrap();
        c.update_guild_incident_actions(sf(1), &GuildIncidentsData { invites_disabled_until: None, dms_disabled_until: None }).await.unwrap();
        c.get_guild_new_member_welcome(sf(1)).await.unwrap();

        c.get_self_voice_state(sf(1)).await.unwrap();
        c.update_self_voice_state(sf(1), &ModifyCurrentVoiceState { suppress: Some(false), ..Default::default() }).await.unwrap();
        c.get_voice_state(sf(1), sf(3)).await.unwrap();
        c.update_voice_state(sf(1), sf(3), &ModifyVoiceState { channel_id: sf(2), suppress: Some(true) }, None).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/widget"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/widget"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/widget.json"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/widget.png?style=shield"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/welcome-screen"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/welcome-screen"));
        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/onboarding"));
        assert_eq!((seen[7].method.as_str(), seen[7].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/onboarding"));
        assert_eq!((seen[8].method.as_str(), seen[8].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/incident-actions"));
        assert_eq!((seen[9].method.as_str(), seen[9].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/new-member-welcome"));
        assert_eq!((seen[10].method.as_str(), seen[10].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/voice-states/@me"));
        assert_eq!((seen[11].method.as_str(), seen[11].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/voice-states/@me"));
        assert_eq!((seen[12].method.as_str(), seen[12].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/voice-states/3"));
        assert_eq!((seen[13].method.as_str(), seen[13].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/voice-states/3"));
    }

    #[tokio::test(start_paused = true)]
    async fn templates_automod_events_soundboard_join_requests_routes_pinning() {
        let tmpl_raw = json!({
            "code": "tmpl1", "name": "Community Template", "description": null, "usage_count": 0,
            "creator_id": "100", "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
            "source_guild_id": "1", "serialized_source_guild": {}
        }).to_string();
        let automod_raw = json!({
            "id": "7", "guild_id": "1", "name": "Block Links", "creator_id": "100",
            "event_type": 1, "trigger_type": 1, "trigger_metadata": {}, "actions": [],
            "enabled": true, "exempt_roles": [], "exempt_channels": []
        }).to_string();
        let event_raw = json!({
            "id": "8", "guild_id": "1", "channel_id": "2", "name": "Weekly Sync",
            "scheduled_start_time": "2026-11-01T00:00:00Z", "privacy_level": 2, "status": 1,
            "entity_type": 2
        }).to_string();
        let exception_raw = json!({
            "event_id": "8", "event_exception_id": "9", "is_canceled": true
        }).to_string();
        let sound_raw = json!({
            "name": "applause", "sound_id": "10", "volume": 0.8, "available": true
        }).to_string();
        let sounds_page = json!({ "items": [serde_json::from_str::<serde_json::Value>(&sound_raw).unwrap()] }).to_string();
        let req_raw = json!({
            "id": "12", "created_at": "2026-01-01T00:00:00Z", "application_status": "SUBMITTED",
            "guild_id": "1", "user_id": "2"
        }).to_string();
        let reqs_page = json!({ "guild_join_requests": [serde_json::from_str::<serde_json::Value>(&req_raw).unwrap()] }).to_string();

        let c = client(vec![
            resp(200, &tmpl_raw),
            resp(200, &json!([serde_json::from_str::<serde_json::Value>(&tmpl_raw).unwrap()]).to_string()),
            resp(200, &tmpl_raw),
            resp(200, &tmpl_raw),
            resp(200, &tmpl_raw),
            resp(200, &tmpl_raw),
            // automod
            resp(200, &json!([serde_json::from_str::<serde_json::Value>(&automod_raw).unwrap()]).to_string()),
            resp(200, &automod_raw),
            resp(200, &automod_raw),
            resp(200, &automod_raw),
            resp(204, ""),
            // events
            resp(200, &json!([serde_json::from_str::<serde_json::Value>(&event_raw).unwrap()]).to_string()),
            resp(200, &event_raw),
            resp(200, &event_raw),
            resp(200, &event_raw),
            resp(204, ""),
            resp(200, "[]"),
            resp(200, "[]"),
            resp(200, &exception_raw),
            resp(200, &exception_raw),
            resp(204, ""),
            resp(200, "[]"),
            // soundboard
            resp(200, &sounds_page),
            resp(201, &sound_raw),
            resp(200, &sound_raw),
            resp(200, &sound_raw),
            resp(204, ""),
            // join requests & search
            resp(200, &reqs_page),
            resp(200, &req_raw),
            resp(200, "{}"),
        ]);

        c.get_guild_template("tmpl1").await.unwrap();
        c.list_guild_templates(sf(1)).await.unwrap();
        c.create_guild_template(sf(1), &CreateGuildTemplate { name: "Community".into(), description: None }).await.unwrap();
        c.sync_guild_template(sf(1), "tmpl1").await.unwrap();
        c.update_guild_template(sf(1), "tmpl1", &ModifyGuildTemplate { name: Some("Renamed".into()), description: None }).await.unwrap();
        c.delete_guild_template(sf(1), "tmpl1").await.unwrap();

        c.list_auto_moderation_rules(sf(1)).await.unwrap();
        c.create_auto_moderation_rule(sf(1), &CreateAutoModerationRule {
            name: "Rule 1".into(),
            event_type: 1,
            trigger_type: 1,
            trigger_metadata: None,
            actions: vec![],
            enabled: Some(true),
            exempt_roles: None,
            exempt_channels: None,
        }, None).await.unwrap();
        c.get_auto_moderation_rule(sf(1), sf(7)).await.unwrap();
        c.update_auto_moderation_rule(sf(1), sf(7), &ModifyAutoModerationRule { name: Some("Rule 1 mod".into()), ..Default::default() }, None).await.unwrap();
        c.delete_auto_moderation_rule(sf(1), sf(7), None).await.unwrap();

        c.list_guild_scheduled_events(sf(1), Some(true)).await.unwrap();
        c.create_guild_scheduled_event(sf(1), &CreateGuildScheduledEvent {
            channel_id: Some(sf(2)),
            entity_metadata: None,
            name: "Weekly Sync".into(),
            privacy_level: 2,
            scheduled_start_time: "2026-11-01T00:00:00Z".into(),
            scheduled_end_time: None,
            description: None,
            entity_type: 2,
            image: None,
        }, None).await.unwrap();
        c.get_guild_scheduled_event(sf(1), sf(8), None).await.unwrap();
        c.update_guild_scheduled_event(sf(1), sf(8), &ModifyGuildScheduledEvent { name: Some("Rescheduled".into()), ..Default::default() }, None).await.unwrap();
        c.delete_guild_scheduled_event(sf(1), sf(8), None).await.unwrap();
        c.list_guild_scheduled_event_users(sf(1), sf(8), Some(50), None, None, None).await.unwrap();
        c.count_guild_scheduled_event_users(sf(1), sf(8), None).await.unwrap();
        c.create_guild_scheduled_event_exception(sf(1), sf(8), &CreateGuildScheduledEventException::new("2026-11-01T00:00:00Z"), None).await.unwrap();
        c.update_guild_scheduled_event_exception(sf(1), sf(8), sf(9), &ModifyGuildScheduledEventException { is_canceled: Some(true), ..Default::default() }, None).await.unwrap();
        c.delete_guild_scheduled_event_exception(sf(1), sf(8), sf(9), None).await.unwrap();
        c.list_guild_scheduled_event_exception_users(sf(1), sf(8), sf(9), None, None, None, None).await.unwrap();

        c.list_guild_soundboard_sounds(sf(1)).await.unwrap();
        c.create_guild_soundboard_sound(sf(1), &CreateSoundboardSound {
            name: "applause".into(),
            sound: "data:audio/mp3;base64,abc".into(),
            volume: Some(0.8),
            emoji_id: None,
            emoji_name: None,
        }, None).await.unwrap();
        c.get_guild_soundboard_sound(sf(1), sf(10)).await.unwrap();
        c.update_guild_soundboard_sound(sf(1), sf(10), &ModifySoundboardSound { name: Some("cheer".into()), ..Default::default() }, None).await.unwrap();
        c.delete_guild_soundboard_sound(sf(1), sf(10), None).await.unwrap();

        c.get_guild_join_requests(sf(1), Some("SUBMITTED"), Some(10), None, None).await.unwrap();
        c.action_guild_join_request(sf(1), sf(12), &ActionGuildJoinRequest { action: "APPROVED".into(), rejection_reason: None }).await.unwrap();
        c.guild_search(sf(1), "content=hello").await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/templates/tmpl1"));
        assert_eq!((seen[1].method.as_str(), seen[1].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/templates"));
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/templates"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("PUT", "https://discord.com/api/v10/guilds/1/templates/tmpl1"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/templates/tmpl1"));
        assert_eq!((seen[5].method.as_str(), seen[5].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/templates/tmpl1"));

        assert_eq!((seen[6].method.as_str(), seen[6].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/auto-moderation/rules"));
        assert_eq!((seen[7].method.as_str(), seen[7].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/auto-moderation/rules"));
        assert_eq!((seen[8].method.as_str(), seen[8].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/auto-moderation/rules/7"));
        assert_eq!((seen[9].method.as_str(), seen[9].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/auto-moderation/rules/7"));
        assert_eq!((seen[10].method.as_str(), seen[10].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/auto-moderation/rules/7"));

        assert_eq!((seen[11].method.as_str(), seen[11].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/scheduled-events?with_user_count=true"));
        assert_eq!((seen[12].method.as_str(), seen[12].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/scheduled-events"));
        assert_eq!((seen[13].method.as_str(), seen[13].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/scheduled-events/8"));
        assert_eq!((seen[14].method.as_str(), seen[14].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/scheduled-events/8"));
        assert_eq!((seen[15].method.as_str(), seen[15].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/scheduled-events/8"));
        assert_eq!((seen[16].method.as_str(), seen[16].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/scheduled-events/8/users?limit=50"));
        assert_eq!((seen[17].method.as_str(), seen[17].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/scheduled-events/8/users/counts"));
        assert_eq!((seen[18].method.as_str(), seen[18].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/scheduled-events/8/exceptions"));
        assert_eq!((seen[19].method.as_str(), seen[19].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/scheduled-events/8/exceptions/9"));
        assert_eq!((seen[20].method.as_str(), seen[20].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/scheduled-events/8/exceptions/9"));
        assert_eq!((seen[21].method.as_str(), seen[21].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/scheduled-events/8/9/users"));

        assert_eq!((seen[22].method.as_str(), seen[22].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/soundboard-sounds"));
        assert_eq!((seen[23].method.as_str(), seen[23].url.as_str()), ("POST", "https://discord.com/api/v10/guilds/1/soundboard-sounds"));
        assert_eq!((seen[24].method.as_str(), seen[24].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/soundboard-sounds/10"));
        assert_eq!((seen[25].method.as_str(), seen[25].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/soundboard-sounds/10"));
        assert_eq!((seen[26].method.as_str(), seen[26].url.as_str()), ("DELETE", "https://discord.com/api/v10/guilds/1/soundboard-sounds/10"));

        assert_eq!((seen[27].method.as_str(), seen[27].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/requests?status=SUBMITTED&limit=10"));
        assert_eq!((seen[28].method.as_str(), seen[28].url.as_str()), ("PATCH", "https://discord.com/api/v10/guilds/1/requests/12"));
        assert_eq!((seen[29].method.as_str(), seen[29].url.as_str()), ("GET", "https://discord.com/api/v10/guilds/1/messages/search?content=hello"));
    }

    
    // =========================================================================
    // Individual Route-Pinning Tests (93 public endpoints)
    // =========================================================================

    #[tokio::test(start_paused = true)]
    async fn pin_01_get_guild_template() {
        let c = client(vec![resp(200, &minimal_template_json("tmpl123"))]);
        let t = c.get_guild_template("tmpl123").await.unwrap();
        assert_eq!(t.code, "tmpl123");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/templates/tmpl123");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_02_get_guild() {
        let c = client(vec![resp(200, &minimal_guild_json(1, "Test Guild"))]);
        let g = c.get_guild(sf(1), Some(true)).await.unwrap();
        assert_eq!(g.name, "Test Guild");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1?with_counts=true");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_03_update_guild() {
        let c = client(vec![resp(200, &minimal_guild_json(1, "Renamed Guild"))]);
        let body = ModifyGuild {
            name: Some("Renamed Guild".into()),
            home_header: Some(Some("data:image/png;base64,header".into())),
            ..Default::default()
        };
        let g = c.update_guild(sf(1), &body, Some("audit_guild")).await.unwrap();
        assert_eq!(g.name, "Renamed Guild");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "Renamed Guild");
        assert_eq!(val["home_header"], "data:image/png;base64,header");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_guild"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_04_get_guild_preview() {
        let raw = json!({
            "id": "1", "name": "Preview Guild", "icon": null, "description": null,
            "home_header": null, "splash": null, "discovery_splash": null,
            "features": [], "approximate_member_count": 100, "approximate_presence_count": 20,
            "emojis": [], "stickers": []
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let p = c.get_guild_preview(sf(1)).await.unwrap();
        assert_eq!(p.approximate_member_count, 100);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/preview");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_05_get_guild_vanity_url() {
        let raw = json!({ "code": "rift-community", "uses": 42 }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let v = c.get_guild_vanity_url(sf(1)).await.unwrap();
        assert_eq!(v.code.as_deref(), Some("rift-community"));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/vanity-url");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_06_list_guild_voice_regions() {
        let raw = json!([{
            "id": "us-east", "name": "US East", "optimal": true, "deprecated": false, "custom": false
        }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let r = c.list_guild_voice_regions(sf(1)).await.unwrap();
        assert_eq!(r[0].id, "us-east");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/regions");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_07_list_guild_channels() {
        let raw = json!([{ "id": "10", "type": 0 }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let chs = c.list_guild_channels(sf(1)).await.unwrap();
        assert_eq!(chs.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/channels");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_08_create_guild_channel() {
        let raw = json!({ "id": "11", "type": 0, "name": "announcements" }).to_string();
        let c = client(vec![resp(201, &raw)]);
        let body = CreateGuildChannel {
            name: "announcements".into(),
            kind: Some(0),
            topic: Some("Server News".into()),
            bitrate: None,
            user_limit: None,
            rate_limit_per_user: None,
            position: None,
            permission_overwrites: None,
            parent_id: None,
            nsfw: None,
            rtc_region: None,
            video_quality_mode: None,
            default_auto_archive_duration: None,
        };
        let ch = c.create_guild_channel(sf(1), &body, Some("audit_create_chan")).await.unwrap();
        assert_eq!(ch.id, sf(11));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/channels");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "announcements");
        assert_eq!(val["type"], 0);
        assert_eq!(val["topic"], "Server News");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_create_chan"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_09_bulk_update_guild_channels() {
        let c = client(vec![resp(204, "")]);
        let pos = [ModifyGuildChannelPosition {
            id: sf(11),
            position: Some(2),
            lock_permissions: None,
            parent_id: None,
        }];
        c.bulk_update_guild_channels(sf(1), &pos, Some("audit_bulk_chan")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/channels");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val, json!([{"id": "11", "position": 2}]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_bulk_chan"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_10_get_active_guild_threads() {
        let raw = json!({ "threads": [], "members": [] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let act = c.get_active_guild_threads(sf(1)).await.unwrap();
        assert!(act.threads.is_empty());
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/threads/active");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_11_list_guild_members() {
        let c = client(vec![resp(200, &json!([minimal_member_val(2)]).to_string())]);
        let query = ListGuildMembers { limit: Some(50), after: Some(sf(100)) };
        let mems = c.list_guild_members(sf(1), &query).await.unwrap();
        assert_eq!(mems.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members?after=100&limit=50");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_12_search_guild_members() {
        let c = client(vec![resp(200, &json!([minimal_member_val(2)]).to_string())]);
        let query = SearchGuildMembers { query: "alice".into(), limit: Some(10) };
        let mems = c.search_guild_members(sf(1), &query).await.unwrap();
        assert_eq!(mems.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/search?limit=10&query=alice");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_13_get_guild_member() {
        let c = client(vec![resp(200, &minimal_member_json(2))]);
        let m = c.get_guild_member(sf(1), sf(2)).await.unwrap();
        assert_eq!(m.user.as_ref().unwrap().id, sf(2));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_14_add_guild_member() {
        let c = client(vec![resp(201, &minimal_member_json(2))]);
        let body = AddGuildMember {
            access_token: "tok123".into(),
            nick: Some("Alice".into()),
            roles: Some(vec![sf(3)]),
            mute: Some(false),
            deaf: Some(false),
            flags: Some(1),
        };
        let m = c.add_guild_member(sf(1), sf(2), &body).await.unwrap();
        assert!(m.is_some());
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["access_token"], "tok123");
        assert_eq!(val["nick"], "Alice");
        assert_eq!(val["roles"], json!(["3"]));
        assert_eq!(val["flags"], 1);
    }

    #[tokio::test(start_paused = true)]
    async fn pin_15_update_guild_member() {
        let c = client(vec![resp(200, &minimal_member_json(2))]);
        let body = ModifyGuildMember {
            nick: Some(Some("Alicia".into())),
            mute: Some(true),
            ..Default::default()
        };
        c.update_guild_member(sf(1), sf(2), &body, Some("audit_mem")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["nick"], "Alicia");
        assert_eq!(val["mute"], true);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_mem"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_16_delete_guild_member() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_member(sf(1), sf(2), Some("audit_kick")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_kick"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_17_update_my_guild_member() {
        let c = client(vec![resp(200, &minimal_member_json(10))]);
        let body = ModifyCurrentMember {
            nick: Some(Some("MySelf".into())),
            bio: Some(Some("My bio".into())),
            ..Default::default()
        };
        c.update_my_guild_member(sf(1), &body, Some("audit_me")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/@me");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["nick"], "MySelf");
        assert_eq!(val["bio"], "My bio");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_me"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_18_add_guild_member_role() {
        let c = client(vec![resp(204, "")]);
        c.add_guild_member_role(sf(1), sf(2), sf(3), Some("audit_add_role")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2/roles/3");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_add_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_19_delete_guild_member_role() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_member_role(sf(1), sf(2), sf(3), Some("audit_del_role")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/members/2/roles/3");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_20_get_guild_member_permissions() {
        let raw = json!({ "permissions": "1024" }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let p = c.get_guild_member_permissions(sf(1), sf(2)).await.unwrap();
        assert_eq!(p, Permissions::from_bits_retain(1024));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/permissions/member/2");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_21_list_guild_roles() {
        let c = client(vec![resp(200, &json!([minimal_role_val(3)]).to_string())]);
        let roles = c.list_guild_roles(sf(1)).await.unwrap();
        assert_eq!(roles[0].id, sf(3));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_22_create_guild_role() {
        let c = client(vec![resp(200, &minimal_role_json(3))]);
        let body = CreateGuildRole {
            name: Some("Mod".into()),
            color: Some(0x00ff00),
            hoist: Some(true),
            ..Default::default()
        };
        let r = c.create_guild_role(sf(1), &body, Some("audit_create_role")).await.unwrap();
        assert_eq!(r.id, sf(3));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "Mod");
        assert_eq!(val["color"], 65280);
        assert_eq!(val["hoist"], true);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_create_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_23_bulk_update_guild_roles() {
        let c = client(vec![resp(200, &json!([minimal_role_val(3)]).to_string())]);
        let pos = [ModifyGuildRolePosition { id: sf(3), position: Some(2) }];
        c.bulk_update_guild_roles(sf(1), &pos, Some("audit_bulk_role")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val, json!([{"id": "3", "position": 2}]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_bulk_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_24_guild_role_member_counts() {
        let raw = json!({ "3": 12 }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let counts = c.guild_role_member_counts(sf(1)).await.unwrap();
        assert_eq!(counts.get(&sf(3)), Some(&12));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles/member-counts");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_25_get_guild_role() {
        let c = client(vec![resp(200, &minimal_role_json(3))]);
        let r = c.get_guild_role(sf(1), sf(3)).await.unwrap();
        assert_eq!(r.id, sf(3));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles/3");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_26_update_guild_role() {
        let c = client(vec![resp(200, &minimal_role_json(3))]);
        let body = ModifyGuildRole {
            name: Some(Some("Admin".into())),
            mentionable: Some(Some(true)),
            ..Default::default()
        };
        c.update_guild_role(sf(1), sf(3), &body, Some("audit_up_role")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles/3");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "Admin");
        assert_eq!(val["mentionable"], true);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_27_delete_guild_role() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_role(sf(1), sf(3), Some("audit_del_role")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/roles/3");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_role"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_28_get_guild_role_permissions() {
        let raw = json!({ "permissions": "2048" }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let p = c.get_guild_role_permissions(sf(1), sf(3)).await.unwrap();
        assert_eq!(p, Permissions::from_bits_retain(2048));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/permissions/role/3");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_29_list_guild_bans() {
        let raw = json!([{
            "reason": null,
            "user": { "id": "2", "username": "baduser", "discriminator": "0001", "avatar": null }
        }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let bans = c.list_guild_bans(sf(1), Some(50), Some(sf(100)), Some(sf(50))).await.unwrap();
        assert_eq!(bans.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/bans?limit=50&before=100&after=50");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_30_get_guild_ban() {
        let raw = json!({
            "reason": "rules violation",
            "user": { "id": "2", "username": "baduser", "discriminator": "0001", "avatar": null }
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let b = c.get_guild_ban(sf(1), sf(2)).await.unwrap();
        assert_eq!(b.reason.as_deref(), Some("rules violation"));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/bans/2");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_31_ban_user_from_guild() {
        let c = client(vec![resp(204, "")]);
        let body = CreateGuildBan { delete_message_seconds: Some(3600), ..Default::default() };
        c.ban_user_from_guild(sf(1), sf(2), &body, Some("audit_ban")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/bans/2");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["delete_message_seconds"], 3600);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_ban"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_32_unban_user_from_guild() {
        let c = client(vec![resp(204, "")]);
        c.unban_user_from_guild(sf(1), sf(2), Some("audit_unban")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/bans/2");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_unban"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_33_bulk_ban_users_from_guild() {
        let raw = json!({ "banned_users": ["2", "3"], "failed_users": [] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = BulkBanUsers { user_ids: vec![sf(2), sf(3)], delete_message_seconds: Some(60) };
        let res = c.bulk_ban_users_from_guild(sf(1), &body, Some("audit_bulk_ban")).await.unwrap();
        assert_eq!(res.banned_users.len(), 2);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/bulk-ban");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["user_ids"], json!(["2", "3"]));
        assert_eq!(val["delete_message_seconds"], 60);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_bulk_ban"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_34_preview_prune_guild() {
        let raw = json!({ "pruned": 15 }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let res = c.preview_prune_guild(sf(1), Some(7), Some(&[sf(3), sf(4)])).await.unwrap();
        assert_eq!(res.pruned, Some(15));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/prune?days=7&include_roles=3,4");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_35_prune_guild() {
        let raw = json!({ "pruned": 15 }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = BeginPrune { days: Some(7), compute_prune_count: Some(true), include_roles: Some(vec![sf(3)]) };
        let res = c.prune_guild(sf(1), &body, Some("audit_prune")).await.unwrap();
        assert_eq!(res.pruned, Some(15));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/prune");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["days"], 7);
        assert_eq!(val["compute_prune_count"], true);
        assert_eq!(val["include_roles"], json!(["3"]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_prune"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_36_list_guild_emojis() {
        let c = client(vec![resp(200, &json!([minimal_emoji_val(5)]).to_string())]);
        let emojis = c.list_guild_emojis(sf(1)).await.unwrap();
        assert_eq!(emojis[0].id, Some(sf(5)));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/emojis");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_37_create_guild_emoji() {
        let c = client(vec![resp(201, &minimal_emoji_json(5))]);
        let body = CreateGuildEmoji {
            name: "smile_cat".into(),
            image: "data:image/png;base64,data".into(),
            roles: Some(vec![sf(3)]),
        };
        let em = c.create_guild_emoji(sf(1), &body, Some("audit_emoji")).await.unwrap();
        assert_eq!(em.id, Some(sf(5)));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/emojis");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "smile_cat");
        assert_eq!(val["image"], "data:image/png;base64,data");
        assert_eq!(val["roles"], json!(["3"]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_emoji"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_38_get_guild_emoji() {
        let c = client(vec![resp(200, &minimal_emoji_json(5))]);
        let em = c.get_guild_emoji(sf(1), sf(5)).await.unwrap();
        assert_eq!(em.id, Some(sf(5)));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/emojis/5");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_39_update_guild_emoji() {
        let c = client(vec![resp(200, &minimal_emoji_json(5))]);
        let body = ModifyGuildEmoji {
            name: Some("smile_cat2".into()),
            roles: Some(Some(vec![sf(3)])),
        };
        c.update_guild_emoji(sf(1), sf(5), &body, Some("audit_up_emoji")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/emojis/5");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "smile_cat2");
        assert_eq!(val["roles"], json!(["3"]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_emoji"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_40_delete_guild_emoji() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_emoji(sf(1), sf(5), Some("audit_del_emoji")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/emojis/5");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_emoji"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_41_list_guild_stickers() {
        let c = client(vec![resp(200, &json!([minimal_sticker_val(6)]).to_string())]);
        let stickers = c.list_guild_stickers(sf(1)).await.unwrap();
        assert_eq!(stickers[0].id, sf(6));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/stickers");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_42_create_guild_sticker() {
        let c = client(vec![resp(200, &minimal_sticker_json(6))]);
        let body = CreateGuildSticker {
            name: "hype_dog".into(),
            description: Some("a hyped dog".into()),
            tags: "hype".into(),
            file: Some("data:image/png;base64,data".into()),
        };
        let st = c.create_guild_sticker(sf(1), &body, Some("audit_sticker")).await.unwrap();
        assert_eq!(st.id, sf(6));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/stickers");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "hype_dog");
        assert_eq!(val["description"], "a hyped dog");
        assert_eq!(val["tags"], "hype");
        assert_eq!(val["file"], "data:image/png;base64,data");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_sticker"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_43_get_guild_sticker() {
        let c = client(vec![resp(200, &minimal_sticker_json(6))]);
        let st = c.get_guild_sticker(sf(1), sf(6)).await.unwrap();
        assert_eq!(st.id, sf(6));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/stickers/6");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_44_update_guild_sticker() {
        let c = client(vec![resp(200, &minimal_sticker_json(6))]);
        let body = ModifyGuildSticker {
            name: Some("hype_dog2".into()),
            description: Some(Some("updated desc".into())),
            tags: Some("fun".into()),
        };
        c.update_guild_sticker(sf(1), sf(6), &body, Some("audit_up_sticker")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/stickers/6");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "hype_dog2");
        assert_eq!(val["description"], "updated desc");
        assert_eq!(val["tags"], "fun");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_sticker"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_45_delete_guild_sticker() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_sticker(sf(1), sf(6), Some("audit_del_sticker")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/stickers/6");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_sticker"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_46_list_guild_audit_log_entries() {
        let raw = json!({
            "audit_log_entries": [], "application_commands": [], "auto_moderation_rules": [],
            "guild_scheduled_events": [], "integrations": [], "threads": [], "users": [], "webhooks": []
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let query = GetAuditLog {
            user_id: Some(sf(2)),
            target_id: Some(sf(5)),
            action_type: Some(1),
            before: None,
            after: None,
            limit: Some(50),
        };
        let log = c.list_guild_audit_log_entries(sf(1), &query).await.unwrap();
        assert!(log.audit_log_entries.is_empty());
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/audit-logs?action_type=1&limit=50&target_id=5&user_id=2");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_47_list_guild_integrations() {
        let raw = json!([{
            "id": "7", "name": "Twitch Integration", "type": "twitch", "enabled": true,
            "account": { "id": "123", "name": "twitch_user" }
        }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let ints = c.list_guild_integrations(sf(1)).await.unwrap();
        assert_eq!(ints[0].id, sf(7));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/integrations");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_48_delete_guild_integration() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_integration(sf(1), sf(7), Some("audit_del_int")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/integrations/7");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_int"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_49_list_guild_invites() {
        let c = client(vec![resp(200, "[]")]);
        let invs = c.list_guild_invites(sf(1)).await.unwrap();
        assert!(invs.is_empty());
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/invites");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_50_get_guild_webhooks() {
        let c = client(vec![resp(200, "[]")]);
        let hooks = c.get_guild_webhooks(sf(1)).await.unwrap();
        assert!(hooks.is_empty());
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/webhooks");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_51_get_guild_widget_settings() {
        let raw = json!({ "enabled": true, "channel_id": "2" }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let s = c.get_guild_widget_settings(sf(1)).await.unwrap();
        assert!(s.enabled);
        assert_eq!(s.channel_id, Some(sf(2)));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/widget");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_52_update_guild_widget_settings() {
        let raw = json!({ "enabled": false, "channel_id": null }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = ModifyWidgetSettings { enabled: Some(false), channel_id: Some(None) };
        let s = c.update_guild_widget_settings(sf(1), &body, Some("audit_widget")).await.unwrap();
        assert!(!s.enabled);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/widget");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["enabled"], false);
        assert!(val["channel_id"].is_null());
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_widget"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_53_get_guild_widget() {
        let raw = json!({
            "id": "1", "name": "Rift Server", "instant_invite": null,
            "channels": [], "members": [], "presence_count": 5
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let w = c.get_guild_widget(sf(1)).await.unwrap();
        assert_eq!(w.presence_count, 5);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/widget.json");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_54_get_guild_widget_png() {
        let c = client(vec![resp(200, "fake-png-bytes")]);
        let bytes = c.get_guild_widget_png(sf(1), Some("shield")).await.unwrap();
        assert_eq!(bytes, b"fake-png-bytes");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/widget.png?style=shield");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_55_get_guild_welcome_screen() {
        let raw = json!({ "description": "Welcome friend", "welcome_channels": [] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let ws = c.get_guild_welcome_screen(sf(1)).await.unwrap();
        assert_eq!(ws.description.as_deref(), Some("Welcome friend"));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/welcome-screen");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_56_update_guild_welcome_screen() {
        let raw = json!({ "description": "Welcome updated", "welcome_channels": [] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = ModifyWelcomeScreen {
            enabled: Some(Some(true)),
            welcome_channels: None,
            description: Some(Some("Welcome updated".into())),
        };
        let ws = c.update_guild_welcome_screen(sf(1), &body, Some("audit_welcome")).await.unwrap();
        assert_eq!(ws.description.as_deref(), Some("Welcome updated"));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/welcome-screen");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["enabled"], true);
        assert_eq!(val["description"], "Welcome updated");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_welcome"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_57_get_guilds_onboarding() {
        let raw = json!({
            "guild_id": "1", "prompts": [], "default_channel_ids": [], "enabled": true, "mode": 0
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let ob = c.get_guilds_onboarding(sf(1)).await.unwrap();
        assert!(ob.enabled);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/onboarding");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_58_put_guilds_onboarding() {
        let raw = json!({
            "guild_id": "1", "prompts": [], "default_channel_ids": ["2"], "enabled": true, "mode": 1
        }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = GuildOnboarding {
            guild_id: sf(1),
            prompts: vec![],
            default_channel_ids: vec![sf(2)],
            enabled: true,
            mode: 1,
        };
        let ob = c.put_guilds_onboarding(sf(1), &body, Some("audit_onboarding")).await.unwrap();
        assert_eq!(ob.mode, 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/onboarding");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["guild_id"], "1");
        assert_eq!(val["default_channel_ids"], json!(["2"]));
        assert_eq!(val["enabled"], true);
        assert_eq!(val["mode"], 1);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_onboarding"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_59_update_guild_incident_actions() {
        let raw = json!({ "invites_disabled_until": "2026-12-01T00:00:00Z", "dms_disabled_until": null }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let body = GuildIncidentsData {
            invites_disabled_until: Some("2026-12-01T00:00:00Z".into()),
            dms_disabled_until: None,
        };
        let inc = c.update_guild_incident_actions(sf(1), &body).await.unwrap();
        assert_eq!(inc.invites_disabled_until.as_deref(), Some("2026-12-01T00:00:00Z"));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/incident-actions");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["invites_disabled_until"], "2026-12-01T00:00:00Z");
        assert!(val["dms_disabled_until"].is_null());
    }

    #[tokio::test(start_paused = true)]
    async fn pin_60_get_guild_new_member_welcome() {
        let raw = json!({ "enabled": true }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let welcome = c.get_guild_new_member_welcome(sf(1)).await.unwrap();
        assert_eq!(welcome["enabled"], true);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/new-member-welcome");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_61_get_self_voice_state() {
        let c = client(vec![resp(200, &minimal_voice_state_json(1, 10))]);
        let vs = c.get_self_voice_state(sf(1)).await.unwrap();
        assert_eq!(vs.guild_id, Some(sf(1)));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/voice-states/@me");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_62_update_self_voice_state() {
        let c = client(vec![resp(204, "")]);
        let body = ModifyCurrentVoiceState {
            channel_id: Some(Some(sf(2))),
            suppress: Some(false),
            request_to_speak_timestamp: Some(Some("2026-11-01T00:00:00Z".into())),
        };
        c.update_self_voice_state(sf(1), &body).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/voice-states/@me");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["channel_id"], "2");
        assert_eq!(val["suppress"], false);
        assert_eq!(val["request_to_speak_timestamp"], "2026-11-01T00:00:00Z");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_63_get_voice_state() {
        let c = client(vec![resp(200, &minimal_voice_state_json(1, 2))]);
        let vs = c.get_voice_state(sf(1), sf(2)).await.unwrap();
        assert_eq!(vs.user_id, sf(2));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/voice-states/2");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_64_update_voice_state() {
        let c = client(vec![resp(204, "")]);
        let body = ModifyVoiceState { channel_id: sf(2), suppress: Some(true) };
        c.update_voice_state(sf(1), sf(2), &body, Some("audit_voice")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/voice-states/2");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["channel_id"], "2");
        assert_eq!(val["suppress"], true);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_voice"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_65_list_guild_templates() {
        let c = client(vec![resp(200, &json!([minimal_template_val("tmpl_123")]).to_string())]);
        let tmpls = c.list_guild_templates(sf(1)).await.unwrap();
        assert_eq!(tmpls[0].code, "tmpl_123");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/templates");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_66_create_guild_template() {
        let c = client(vec![resp(200, &minimal_template_json("tmpl_new"))]);
        let body = CreateGuildTemplate { name: "MyTemplate".into(), description: Some("desc".into()) };
        let t = c.create_guild_template(sf(1), &body).await.unwrap();
        assert_eq!(t.code, "tmpl_new");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/templates");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "MyTemplate");
        assert_eq!(val["description"], "desc");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_67_sync_guild_template() {
        let c = client(vec![resp(200, &minimal_template_json("tmpl_123"))]);
        let t = c.sync_guild_template(sf(1), "tmpl_123").await.unwrap();
        assert_eq!(t.code, "tmpl_123");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/templates/tmpl_123");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_68_update_guild_template() {
        let c = client(vec![resp(200, &minimal_template_json("tmpl_123"))]);
        let body = ModifyGuildTemplate { name: Some("TmplUpdated".into()), description: Some(Some("new desc".into())) };
        let t = c.update_guild_template(sf(1), "tmpl_123", &body).await.unwrap();
        assert_eq!(t.code, "tmpl_123");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/templates/tmpl_123");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "TmplUpdated");
        assert_eq!(val["description"], "new desc");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_69_delete_guild_template() {
        let c = client(vec![resp(200, &minimal_template_json("tmpl_123"))]);
        let t = c.delete_guild_template(sf(1), "tmpl_123").await.unwrap();
        assert_eq!(t.code, "tmpl_123");
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/templates/tmpl_123");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_70_list_auto_moderation_rules() {
        let c = client(vec![resp(200, &json!([minimal_automod_val(8)]).to_string())]);
        let rules = c.list_auto_moderation_rules(sf(1)).await.unwrap();
        assert_eq!(rules[0].id, sf(8));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/auto-moderation/rules");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_71_create_auto_moderation_rule() {
        let c = client(vec![resp(200, &minimal_automod_json(8))]);
        let body = CreateAutoModerationRule {
            name: "KeywordFilter".into(),
            event_type: 1,
            trigger_type: 1,
            trigger_metadata: Some(json!({"keyword_filter": ["spam"]})),
            actions: vec![json!({"type": 1})],
            enabled: Some(true),
            exempt_roles: Some(vec![sf(3)]),
            exempt_channels: None,
        };
        let r = c.create_auto_moderation_rule(sf(1), &body, Some("audit_automod")).await.unwrap();
        assert_eq!(r.id, sf(8));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/auto-moderation/rules");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "KeywordFilter");
        assert_eq!(val["event_type"], 1);
        assert_eq!(val["trigger_type"], 1);
        assert_eq!(val["exempt_roles"], json!(["3"]));
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_automod"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_72_get_auto_moderation_rule() {
        let c = client(vec![resp(200, &minimal_automod_json(8))]);
        let r = c.get_auto_moderation_rule(sf(1), sf(8)).await.unwrap();
        assert_eq!(r.id, sf(8));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/auto-moderation/rules/8");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_73_update_auto_moderation_rule() {
        let c = client(vec![resp(200, &minimal_automod_json(8))]);
        let body = ModifyAutoModerationRule {
            name: Some("FilterUpdated".into()),
            enabled: Some(false),
            ..Default::default()
        };
        c.update_auto_moderation_rule(sf(1), sf(8), &body, Some("audit_up_automod")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/auto-moderation/rules/8");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "FilterUpdated");
        assert_eq!(val["enabled"], false);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_automod"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_74_delete_auto_moderation_rule() {
        let c = client(vec![resp(204, "")]);
        c.delete_auto_moderation_rule(sf(1), sf(8), Some("audit_del_automod")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/auto-moderation/rules/8");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_automod"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_75_list_guild_scheduled_events() {
        let c = client(vec![resp(200, &json!([minimal_event_val(9)]).to_string())]);
        let events = c.list_guild_scheduled_events(sf(1), Some(true)).await.unwrap();
        assert_eq!(events[0].id, sf(9));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events?with_user_count=true");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_76_create_guild_scheduled_event() {
        let c = client(vec![resp(200, &minimal_event_json(9))]);
        let body = CreateGuildScheduledEvent {
            channel_id: Some(sf(2)),
            entity_metadata: None,
            name: "Community Call".into(),
            privacy_level: 2,
            scheduled_start_time: "2026-11-01T20:00:00Z".into(),
            scheduled_end_time: None,
            description: Some("Monthly call".into()),
            entity_type: 2,
            image: None,
        };
        let ev = c.create_guild_scheduled_event(sf(1), &body, Some("audit_event")).await.unwrap();
        assert_eq!(ev.id, sf(9));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "Community Call");
        assert_eq!(val["channel_id"], "2");
        assert_eq!(val["privacy_level"], 2);
        assert_eq!(val["scheduled_start_time"], "2026-11-01T20:00:00Z");
        assert_eq!(val["entity_type"], 2);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_event"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_77_get_guild_scheduled_event() {
        let c = client(vec![resp(200, &minimal_event_json(9))]);
        let ev = c.get_guild_scheduled_event(sf(1), sf(9), Some(true)).await.unwrap();
        assert_eq!(ev.id, sf(9));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9?with_user_count=true");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_78_update_guild_scheduled_event() {
        let c = client(vec![resp(200, &minimal_event_json(9))]);
        let body = ModifyGuildScheduledEvent {
            name: Some("Community Call Rescheduled".into()),
            ..Default::default()
        };
        c.update_guild_scheduled_event(sf(1), sf(9), &body, Some("audit_up_event")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "Community Call Rescheduled");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_event"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_79_delete_guild_scheduled_event() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_scheduled_event(sf(1), sf(9), Some("audit_del_event")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_event"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_80_list_guild_scheduled_event_users() {
        let raw = json!([{
            "guild_scheduled_event_id": "9",
            "user": { "id": "2", "username": "fan", "discriminator": "0001", "avatar": null },
            "member": null
        }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let us = c.list_guild_scheduled_event_users(sf(1), sf(9), Some(50), Some(true), Some(sf(100)), Some(sf(50))).await.unwrap();
        assert_eq!(us.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/users?limit=50&with_member=true&before=100&after=50");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_81_count_guild_scheduled_event_users() {
        let raw = json!([{"guild_scheduled_event_id": "9", "event_exception_id": "11", "user_count": 42}]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let counts = c.count_guild_scheduled_event_users(sf(1), sf(9), Some(&[sf(11), sf(12)])).await.unwrap();
        assert_eq!(counts[0].user_count, 42);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/users/counts?guild_scheduled_event_exception_ids=11,12");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_82_create_guild_scheduled_event_exception() {
        let c = client(vec![resp(200, &minimal_event_exception_json(9, 11))]);
        let body = CreateGuildScheduledEventException::new("2026-11-01T20:00:00Z");
        let ex = c.create_guild_scheduled_event_exception(sf(1), sf(9), &body, Some("audit_ex_create")).await.unwrap();
        assert_eq!(ex.event_exception_id, sf(11));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/exceptions");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["original_scheduled_start_time"], "2026-11-01T20:00:00Z");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_ex_create"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_83_update_guild_scheduled_event_exception() {
        let c = client(vec![resp(200, &minimal_event_exception_json(9, 11))]);
        let body = ModifyGuildScheduledEventException { is_canceled: Some(true), ..Default::default() };
        let ex = c.update_guild_scheduled_event_exception(sf(1), sf(9), sf(11), &body, Some("audit_ex_up")).await.unwrap();
        assert_eq!(ex.event_exception_id, sf(11));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/exceptions/11");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["is_canceled"], true);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_ex_up"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_84_delete_guild_scheduled_event_exception() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_scheduled_event_exception(sf(1), sf(9), sf(11), Some("audit_ex_del")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/exceptions/11");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_ex_del"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_85_list_guild_scheduled_event_exception_users() {
        let raw = json!([{
            "guild_scheduled_event_id": "9",
            "user": { "id": "2", "username": "fan", "discriminator": "0001", "avatar": null },
            "member": null
        }]).to_string();
        let c = client(vec![resp(200, &raw)]);
        let us = c.list_guild_scheduled_event_exception_users(sf(1), sf(9), sf(11), Some(30), Some(false), None, None).await.unwrap();
        assert_eq!(us.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/scheduled-events/9/11/users?limit=30&with_member=false");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_86_list_guild_soundboard_sounds() {
        let raw = json!({ "items": [minimal_sound_val(12)] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let page = c.list_guild_soundboard_sounds(sf(1)).await.unwrap();
        assert_eq!(page.items.len(), 1);
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/soundboard-sounds");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_87_create_guild_soundboard_sound() {
        let c = client(vec![resp(201, &minimal_sound_json(12))]);
        let body = CreateSoundboardSound {
            name: "horn".into(),
            sound: "data:audio/mp3;base64,data".into(),
            volume: Some(0.8),
            emoji_id: None,
            emoji_name: None,
        };
        let s = c.create_guild_soundboard_sound(sf(1), &body, Some("audit_sound")).await.unwrap();
        assert_eq!(s.sound_id, sf(12));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/soundboard-sounds");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "horn");
        assert_eq!(val["sound"], "data:audio/mp3;base64,data");
        assert_eq!(val["volume"], 0.8);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_sound"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_88_get_guild_soundboard_sound() {
        let c = client(vec![resp(200, &minimal_sound_json(12))]);
        let s = c.get_guild_soundboard_sound(sf(1), sf(12)).await.unwrap();
        assert_eq!(s.sound_id, sf(12));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/soundboard-sounds/12");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_89_update_guild_soundboard_sound() {
        let c = client(vec![resp(200, &minimal_sound_json(12))]);
        let body = ModifySoundboardSound {
            name: Some("horn_louder".into()),
            volume: Some(Some(1.0)),
            emoji_id: None,
            emoji_name: None,
        };
        c.update_guild_soundboard_sound(sf(1), sf(12), &body, Some("audit_up_sound")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/soundboard-sounds/12");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["name"], "horn_louder");
        assert_eq!(val["volume"], 1.0);
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_up_sound"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_90_delete_guild_soundboard_sound() {
        let c = client(vec![resp(204, "")]);
        c.delete_guild_soundboard_sound(sf(1), sf(12), Some("audit_del_sound")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/soundboard-sounds/12");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit_del_sound"));
    }

    #[tokio::test(start_paused = true)]
    async fn pin_91_get_guild_join_requests() {
        let raw = json!({ "total": 1, "guild_join_requests": [minimal_join_request_val(13)] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let reqs = c.get_guild_join_requests(sf(1), Some("SUBMITTED"), Some(10), Some(sf(100)), Some(sf(50))).await.unwrap();
        assert_eq!(reqs.total, Some(1));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/requests?status=SUBMITTED&limit=10&before=100&after=50");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_92_action_guild_join_request() {
        let c = client(vec![resp(200, &minimal_join_request_json(13))]);
        let body = ActionGuildJoinRequest { action: "APPROVED".into(), rejection_reason: None };
        let jr = c.action_guild_join_request(sf(1), sf(13), &body).await.unwrap();
        assert_eq!(jr.id, sf(13));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/requests/13");
        let val: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(val["action"], "APPROVED");
    }

    #[tokio::test(start_paused = true)]
    async fn pin_93_guild_search() {
        let raw = json!({ "messages": [] }).to_string();
        let c = client(vec![resp(200, &raw)]);
        let s = c.guild_search(sf(1), "content=hello&has=link").await.unwrap();
        assert_eq!(s["messages"], json!([]));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/guilds/1/messages/search?content=hello&has=link");
    }

    // =========================================================================
    // Comprehensive Validation Tests for Every Local Limit (17 limit functions)
    // =========================================================================

    #[test]
    fn validation_limits_guild_name() {
        assert_eq!(check_guild_name("a"), Err(GuildError::BadGuildName { len: 1 }));
        assert_eq!(check_guild_name(&"x".repeat(101)), Err(GuildError::BadGuildName { len: 101 }));
        assert!(check_guild_name("ab").is_ok());
        assert!(check_guild_name(&"x".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_channel_name() {
        assert_eq!(check_channel_name(""), Err(GuildError::BadChannelName { len: 0 }));
        assert_eq!(check_channel_name(&"c".repeat(101)), Err(GuildError::BadChannelName { len: 101 }));
        assert!(check_channel_name("c").is_ok());
        assert!(check_channel_name(&"c".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_role_name() {
        assert_eq!(check_role_name(""), Err(GuildError::BadRoleName { len: 0 }));
        assert_eq!(check_role_name(&"r".repeat(101)), Err(GuildError::BadRoleName { len: 101 }));
        assert!(check_role_name("r").is_ok());
        assert!(check_role_name(&"r".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_nick() {
        assert_eq!(check_nick(&"n".repeat(33)), Err(GuildError::BadNick { len: 33 }));
        assert!(check_nick("").is_ok());
        assert!(check_nick(&"n".repeat(32)).is_ok());
    }

    #[test]
    fn validation_limits_member_limit() {
        assert_eq!(check_member_limit(0), Err(GuildError::BadMemberLimit { value: 0 }));
        assert_eq!(check_member_limit(1001), Err(GuildError::BadMemberLimit { value: 1001 }));
        assert!(check_member_limit(1).is_ok());
        assert!(check_member_limit(1000).is_ok());
    }

    #[test]
    fn validation_limits_search_query() {
        assert_eq!(check_search_query(""), Err(GuildError::BadSearchQuery { len: 0 }));
        assert_eq!(check_search_query(&"q".repeat(101)), Err(GuildError::BadSearchQuery { len: 101 }));
        assert!(check_search_query("q").is_ok());
        assert!(check_search_query(&"q".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_delete_message_seconds() {
        assert_eq!(check_delete_message_seconds(604_801), Err(GuildError::BadDeleteMessageSeconds { value: 604_801 }));
        assert!(check_delete_message_seconds(0).is_ok());
        assert!(check_delete_message_seconds(604_800).is_ok());
    }

    #[test]
    fn validation_limits_bulk_ban_count() {
        assert_eq!(check_bulk_ban_count(0), Err(GuildError::BadBulkBanCount { count: 0 }));
        assert_eq!(check_bulk_ban_count(201), Err(GuildError::BadBulkBanCount { count: 201 }));
        assert!(check_bulk_ban_count(1).is_ok());
        assert!(check_bulk_ban_count(200).is_ok());
    }

    #[test]
    fn validation_limits_prune_days() {
        assert_eq!(check_prune_days(0), Err(GuildError::BadPruneDays { value: 0 }));
        assert_eq!(check_prune_days(31), Err(GuildError::BadPruneDays { value: 31 }));
        assert!(check_prune_days(1).is_ok());
        assert!(check_prune_days(30).is_ok());
    }

    #[test]
    fn validation_limits_emoji_name() {
        assert_eq!(check_emoji_name("e"), Err(GuildError::BadEmojiName { len: 1 }));
        assert_eq!(check_emoji_name(&"e".repeat(33)), Err(GuildError::BadEmojiName { len: 33 }));
        assert!(check_emoji_name("em").is_ok());
        assert!(check_emoji_name(&"e".repeat(32)).is_ok());
    }

    #[test]
    fn validation_limits_sticker_name() {
        assert_eq!(check_sticker_name("s"), Err(GuildError::BadStickerName { len: 1 }));
        assert_eq!(check_sticker_name(&"s".repeat(31)), Err(GuildError::BadStickerName { len: 31 }));
        assert!(check_sticker_name("st").is_ok());
        assert!(check_sticker_name(&"s".repeat(30)).is_ok());
    }

    #[test]
    fn validation_limits_sticker_desc() {
        assert_eq!(check_sticker_desc(&"d".repeat(101)), Err(GuildError::BadStickerDescription { len: 101 }));
        assert!(check_sticker_desc("").is_ok());
        assert!(check_sticker_desc(&"d".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_sticker_tags() {
        assert_eq!(check_sticker_tags(&"t".repeat(201)), Err(GuildError::BadStickerTags { len: 201 }));
        assert!(check_sticker_tags("").is_ok());
        assert!(check_sticker_tags(&"t".repeat(200)).is_ok());
    }

    #[test]
    fn validation_limits_audit_log_limit() {
        assert_eq!(check_audit_log_limit(0), Err(GuildError::BadAuditLogLimit { value: 0 }));
        assert_eq!(check_audit_log_limit(101), Err(GuildError::BadAuditLogLimit { value: 101 }));
        assert!(check_audit_log_limit(1).is_ok());
        assert!(check_audit_log_limit(100).is_ok());
    }

    #[test]
    fn validation_limits_event_name() {
        assert_eq!(check_event_name(""), Err(GuildError::BadEventName { len: 0 }));
        assert_eq!(check_event_name(&"e".repeat(101)), Err(GuildError::BadEventName { len: 101 }));
        assert!(check_event_name("e").is_ok());
        assert!(check_event_name(&"e".repeat(100)).is_ok());
    }

    #[test]
    fn validation_limits_sound_name() {
        assert_eq!(check_sound_name("s"), Err(GuildError::BadSoundName { len: 1 }));
        assert_eq!(check_sound_name(&"s".repeat(33)), Err(GuildError::BadSoundName { len: 33 }));
        assert!(check_sound_name("so").is_ok());
        assert!(check_sound_name(&"s".repeat(32)).is_ok());
    }

    #[test]
    fn validation_limits_sound_volume() {
        assert!(matches!(check_sound_volume(-0.01), Err(GuildError::BadSoundVolume { .. })));
        assert!(matches!(check_sound_volume(1.01), Err(GuildError::BadSoundVolume { .. })));
        assert!(check_sound_volume(0.0).is_ok());
        assert!(check_sound_volume(1.0).is_ok());
        assert!(check_sound_volume(0.5).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn validation_client_rejects_invalid_inputs_before_network() {
        let c = client(vec![]);

        // 1. update_guild with bad name
        let bad_g = ModifyGuild { name: Some("a".into()), ..Default::default() };
        assert!(c.update_guild(sf(1), &bad_g, None).await.is_err());

        // 2. create_guild_channel with bad name
        let bad_ch = CreateGuildChannel {
            name: "".into(), kind: None, topic: None, bitrate: None, user_limit: None,
            rate_limit_per_user: None, position: None, permission_overwrites: None,
            parent_id: None, nsfw: None, rtc_region: None, video_quality_mode: None,
            default_auto_archive_duration: None,
        };
        assert!(c.create_guild_channel(sf(1), &bad_ch, None).await.is_err());

        // 3. create_guild_role with bad name
        let bad_role = CreateGuildRole { name: Some("".into()), ..Default::default() };
        assert!(c.create_guild_role(sf(1), &bad_role, None).await.is_err());

        // 4. update_guild_role with bad name
        let bad_up_role = ModifyGuildRole { name: Some(Some("".into())), ..Default::default() };
        assert!(c.update_guild_role(sf(1), sf(2), &bad_up_role, None).await.is_err());

        // 5. add_guild_member with bad nickname
        let bad_member = AddGuildMember {
            access_token: "tok".into(), nick: Some("x".repeat(33)),
            roles: None, mute: None, deaf: None, flags: None,
        };
        assert!(c.add_guild_member(sf(1), sf(2), &bad_member).await.is_err());

        // 6. update_guild_member with bad nickname
        let bad_up_member = ModifyGuildMember { nick: Some(Some("x".repeat(33))), ..Default::default() };
        assert!(c.update_guild_member(sf(1), sf(2), &bad_up_member, None).await.is_err());

        // 7. update_my_guild_member with bad nickname
        let bad_me = ModifyCurrentMember { nick: Some(Some("x".repeat(33))), ..Default::default() };
        assert!(c.update_my_guild_member(sf(1), &bad_me, None).await.is_err());

        // 8. list_guild_members with bad limit
        let bad_m_query = ListGuildMembers { limit: Some(0), after: None };
        assert!(c.list_guild_members(sf(1), &bad_m_query).await.is_err());

        // 9. search_guild_members with bad query
        let bad_s_query = SearchGuildMembers { query: "".into(), limit: None };
        assert!(c.search_guild_members(sf(1), &bad_s_query).await.is_err());

        // 10. list_guild_bans with bad limit
        assert!(c.list_guild_bans(sf(1), Some(1001), None, None).await.is_err());

        // 11. ban_user_from_guild with bad seconds
        let bad_ban = CreateGuildBan { delete_message_seconds: Some(604_801), ..Default::default() };
        assert!(c.ban_user_from_guild(sf(1), sf(2), &bad_ban, None).await.is_err());

        // 12. bulk_ban_users_from_guild with bad user count
        let bad_bulk = BulkBanUsers { user_ids: vec![], delete_message_seconds: None };
        assert!(c.bulk_ban_users_from_guild(sf(1), &bad_bulk, None).await.is_err());

        // 13. preview_prune_guild with bad days
        assert!(c.preview_prune_guild(sf(1), Some(0), None).await.is_err());

        // 14. prune_guild with bad days
        let bad_prune = BeginPrune { days: Some(31), compute_prune_count: None, include_roles: None };
        assert!(c.prune_guild(sf(1), &bad_prune, None).await.is_err());

        // 15. create_guild_emoji with bad name
        let bad_emoji = CreateGuildEmoji { name: "x".into(), image: "data".into(), roles: None };
        assert!(c.create_guild_emoji(sf(1), &bad_emoji, None).await.is_err());

        // 16. update_guild_emoji with bad name
        let bad_up_emoji = ModifyGuildEmoji { name: Some("x".into()), roles: None };
        assert!(c.update_guild_emoji(sf(1), sf(2), &bad_up_emoji, None).await.is_err());

        // 17. create_guild_sticker with bad name
        let bad_sticker = CreateGuildSticker {
            name: "x".into(), description: None, tags: "tag".into(), file: Some("data".into()),
        };
        assert!(c.create_guild_sticker(sf(1), &bad_sticker, None).await.is_err());

        // 18. update_guild_sticker with bad description
        let bad_up_sticker = ModifyGuildSticker {
            name: None, description: Some(Some("d".repeat(101))), tags: None,
        };
        assert!(c.update_guild_sticker(sf(1), sf(2), &bad_up_sticker, None).await.is_err());

        // 19. list_guild_audit_log_entries with bad limit
        let bad_audit_query = GetAuditLog {
            user_id: None, target_id: None, action_type: None, before: None, after: None, limit: Some(101),
        };
        assert!(c.list_guild_audit_log_entries(sf(1), &bad_audit_query).await.is_err());

        // 20. create_guild_scheduled_event with bad name
        let bad_event = CreateGuildScheduledEvent {
            channel_id: Some(sf(2)), entity_metadata: None, name: "".into(), privacy_level: 2,
            scheduled_start_time: "2026-11-01T00:00:00Z".into(), scheduled_end_time: None,
            description: None, entity_type: 2, image: None,
        };
        assert!(c.create_guild_scheduled_event(sf(1), &bad_event, None).await.is_err());

        // 21. update_guild_scheduled_event with bad name
        let bad_up_event = ModifyGuildScheduledEvent { name: Some("".into()), ..Default::default() };
        assert!(c.update_guild_scheduled_event(sf(1), sf(2), &bad_up_event, None).await.is_err());

        // 22. list_guild_scheduled_event_users with bad limit
        assert!(c.list_guild_scheduled_event_users(sf(1), sf(2), Some(0), None, None, None).await.is_err());

        // 23. list_guild_scheduled_event_exception_users with bad limit
        assert!(c.list_guild_scheduled_event_exception_users(sf(1), sf(2), sf(3), Some(1001), None, None, None).await.is_err());

        // 24. create_guild_soundboard_sound with bad name
        let bad_sound = CreateSoundboardSound {
            name: "x".into(), sound: "data".into(), volume: None, emoji_id: None, emoji_name: None,
        };
        assert!(c.create_guild_soundboard_sound(sf(1), &bad_sound, None).await.is_err());

        // 25. update_guild_soundboard_sound with bad volume
        let bad_up_sound = ModifySoundboardSound {
            name: None, volume: Some(Some(1.5)), emoji_id: None, emoji_name: None,
        };
        assert!(c.update_guild_soundboard_sound(sf(1), sf(2), &bad_up_sound, None).await.is_err());

        // Ensure fake client never received any requests
        assert_eq!(c.transport().seen.lock().unwrap().len(), 0);
    }
}
