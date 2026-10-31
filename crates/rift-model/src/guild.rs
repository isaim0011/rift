//! Guild models and components.

use crate::channel::Channel;
use crate::presence::PresenceUpdate;
use crate::serde_util::{
    de_opt_permissions, de_opt_snowflake, de_opt_vec_snowflake, de_permissions, de_snowflake,
    de_vec_snowflake, ser_opt_permissions, ser_opt_snowflake, ser_opt_vec_snowflake,
    ser_permissions, ser_snowflake, ser_vec_snowflake,
};
use crate::user::User;
use crate::voice::VoiceState;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Serialize};

/// An unavailable guild representation.
///
/// Sent in `READY` and `GUILD_DELETE`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnavailableGuild {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Whether the guild is unavailable due to an outage.
    #[serde(default)]
    pub unavailable: Option<bool>,
}

/// Tags on a role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTags {
    /// The id of the bot this role belongs to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub bot_id: Option<Snowflake>,

    /// The id of the integration this role belongs to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub integration_id: Option<Snowflake>,

    /// Whether this is the guild's Booster role.
    #[serde(default)]
    pub premium_subscriber: Option<bool>,

    /// The id of this role's subscription sku and listing.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub subscription_listing_id: Option<Snowflake>,

    /// Whether this role is available for purchase.
    #[serde(default)]
    pub available_for_purchase: Option<bool>,

    /// Whether this role is a guild's linked role.
    #[serde(default)]
    pub guild_connections: Option<bool>,
}

/// A Discord role object.
///
/// See <https://discord.com/developers/docs/topics/permissions#role-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Role id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Role name.
    pub name: String,

    /// Integer color representation of hexadecimal color code.
    #[serde(default)]
    pub color: u32,

    /// If this role is pinned in the user listing.
    #[serde(default)]
    pub hoist: bool,

    /// Role icon hash.
    #[serde(default)]
    pub icon: Option<String>,

    /// Role unicode emoji.
    #[serde(default)]
    pub unicode_emoji: Option<String>,

    /// Position of this role.
    #[serde(default)]
    pub position: i32,

    /// Permission bit set.
    #[serde(deserialize_with = "de_permissions", serialize_with = "ser_permissions")]
    pub permissions: Permissions,

    /// Whether this role is managed by an integration.
    #[serde(default)]
    pub managed: bool,

    /// Whether this role is mentionable.
    #[serde(default)]
    pub mentionable: bool,

    /// The tags this role has.
    #[serde(default)]
    pub tags: Option<RoleTags>,

    /// Role flags combined as a bitfield.
    #[serde(default)]
    pub flags: u64,
}

/// A member of a Discord guild.
///
/// See <https://discord.com/developers/docs/resources/guild#guild-member-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildMember {
    /// The user this guild member represents.
    #[serde(default)]
    pub user: Option<User>,

    /// This user's guild nickname.
    #[serde(default)]
    pub nick: Option<String>,

    /// The member's guild avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,

    /// Array of role object IDs.
    #[serde(
        default,
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub roles: Vec<Snowflake>,

    /// When the user joined the guild.
    #[serde(default)]
    pub joined_at: String,

    /// When the user started boosting the guild.
    #[serde(default)]
    pub premium_since: Option<String>,

    /// Whether the user is deafened in voice channels.
    #[serde(default)]
    pub deaf: Option<bool>,

    /// Whether the user is muted in voice channels.
    #[serde(default)]
    pub mute: Option<bool>,

    /// Guild member flags.
    #[serde(default)]
    pub flags: Option<u64>,

    /// Whether the user has not yet passed the guild's Membership Screening requirements.
    #[serde(default)]
    pub pending: Option<bool>,

    /// Total permissions of the member in the channel, including overwrites.
    #[serde(
        default,
        deserialize_with = "de_opt_permissions",
        serialize_with = "ser_opt_permissions"
    )]
    pub permissions: Option<Permissions>,

    /// When the user's timeout will expire and the user will be able to communicate in the guild again.
    #[serde(default)]
    pub communication_disabled_until: Option<String>,

    /// Data for the member's guild avatar decoration.
    #[serde(default)]
    pub avatar_decoration_data: Option<serde_json::Value>,
}

/// An emoji object.
///
/// See <https://discord.com/developers/docs/resources/emoji#emoji-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emoji {
    /// Emoji id.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub id: Option<Snowflake>,

    /// Emoji name.
    #[serde(default)]
    pub name: Option<String>,

    /// Roles allowed to use this emoji.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub roles: Option<Vec<Snowflake>>,

    /// User that created this emoji.
    #[serde(default)]
    pub user: Option<User>,

    /// Whether this emoji must be wrapped in colons.
    #[serde(default)]
    pub require_colons: Option<bool>,

    /// Whether this emoji is managed.
    #[serde(default)]
    pub managed: Option<bool>,

    /// Whether this emoji is animated.
    #[serde(default)]
    pub animated: Option<bool>,

    /// Whether this emoji can be used, may be false due to loss of Server Boosts.
    #[serde(default)]
    pub available: Option<bool>,
}

/// A sticker object.
///
/// See <https://discord.com/developers/docs/resources/sticker#sticker-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sticker {
    /// ID of the sticker.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// For standard stickers, id of the pack the sticker is from.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub pack_id: Option<Snowflake>,

    /// Name of the sticker.
    pub name: String,

    /// Description of the sticker.
    #[serde(default)]
    pub description: Option<String>,

    /// Autocomplete/suggestion tags for the sticker (comma separated).
    #[serde(default)]
    pub tags: String,

    /// Type of sticker.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,

    /// Type of sticker format.
    #[serde(default)]
    pub format_type: u8,

    /// Whether this guild sticker can be used.
    #[serde(default)]
    pub available: Option<bool>,

    /// Id of the guild that owns this sticker.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// User that uploaded the guild sticker.
    #[serde(default)]
    pub user: Option<User>,

    /// The standard sticker's sort order within its pack.
    #[serde(default)]
    pub sort_value: Option<u32>,
}

/// A scheduled event in a guild.
///
/// See <https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildScheduledEvent {
    /// The id of the scheduled event.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The guild id which the scheduled event belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The channel id in which the scheduled event will be hosted, or null.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// The id of the user that created the scheduled event.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub creator_id: Option<Snowflake>,

    /// The name of the scheduled event.
    pub name: String,

    /// The description of the scheduled event.
    #[serde(default)]
    pub description: Option<String>,

    /// The time the scheduled event will start.
    pub scheduled_start_time: String,

    /// The time the scheduled event will end, or null.
    #[serde(default)]
    pub scheduled_end_time: Option<String>,

    /// The privacy level of the scheduled event.
    #[serde(default)]
    pub privacy_level: u8,

    /// The status of the scheduled event.
    #[serde(default)]
    pub status: u8,

    /// The type of the scheduled event.
    #[serde(default)]
    pub entity_type: u8,

    /// The id of an entity associated with a guild scheduled event.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub entity_id: Option<Snowflake>,

    /// Additional metadata for the scheduled event.
    #[serde(default)]
    pub entity_metadata: Option<serde_json::Value>,

    /// The user that created the scheduled event.
    #[serde(default)]
    pub creator: Option<User>,

    /// The number of users subscribed to the scheduled event.
    #[serde(default)]
    pub user_count: Option<u32>,

    /// The cover image hash of the scheduled event.
    #[serde(default)]
    pub image: Option<String>,
}

/// A stage instance object.
///
/// See <https://discord.com/developers/docs/resources/stage-instance#stage-instance-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageInstance {
    /// The id of this Stage instance.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The guild id of the associated Stage channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The id of the associated Stage channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// The topic of the Stage instance.
    pub topic: String,

    /// The privacy level of the Stage instance.
    #[serde(default)]
    pub privacy_level: u8,

    /// Whether or not Stage Discovery is disabled.
    #[serde(default)]
    pub discoverable_disabled: Option<bool>,

    /// The id of the scheduled event for this Stage instance.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_scheduled_event_id: Option<Snowflake>,
}

/// A soundboard sound object.
///
/// See <https://discord.com/developers/docs/resources/soundboard#soundboard-sound-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoundboardSound {
    /// The id of this sound.
    #[serde(
        rename = "sound_id",
        deserialize_with = "de_snowflake",
        serialize_with = "ser_snowflake"
    )]
    pub sound_id: Snowflake,

    /// The name of this sound.
    pub name: String,

    /// The volume of this sound, from 0 to 1.
    #[serde(default)]
    pub volume: f64,

    /// The id of this sound's custom emoji.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub emoji_id: Option<Snowflake>,

    /// The unicode character of this sound's custom emoji.
    #[serde(default)]
    pub emoji_name: Option<String>,

    /// The id of the guild this sound belongs to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Whether this sound can be used, may be false due to loss of Server Boosts.
    #[serde(default)]
    pub available: bool,

    /// The user that created this sound.
    #[serde(default)]
    pub user: Option<User>,
}

/// A guild audit log entry.
///
/// See <https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// ID of the entry.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the affected entity (webhook, user, role, etc.).
    #[serde(default)]
    pub target_id: Option<String>,

    /// Changes made to the `target_id`.
    #[serde(default)]
    pub changes: Option<Vec<serde_json::Value>>,

    /// User or app that made the changes.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub user_id: Option<Snowflake>,

    /// Type of action that occurred.
    pub action_type: u32,

    /// Additional info for certain action types.
    #[serde(default)]
    pub options: Option<serde_json::Value>,

    /// Reason for the change (1-512 characters).
    #[serde(default)]
    pub reason: Option<String>,
}

/// Account information for a guild integration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationAccount {
    /// Id of the account.
    pub id: String,

    /// Name of the account.
    pub name: String,
}

/// Application information for a guild integration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationApplication {
    /// The id of the app.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The name of the app.
    pub name: String,

    /// The icon hash of the app.
    #[serde(default)]
    pub icon: Option<String>,

    /// The description of the app.
    #[serde(default)]
    pub description: String,

    /// The bot associated with this application.
    #[serde(default)]
    pub bot: Option<User>,
}

/// A guild integration object.
///
/// See <https://discord.com/developers/docs/resources/guild#integration-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Integration {
    /// Integration id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Integration name.
    pub name: String,

    /// Integration type (twitch, youtube, discord, or guild_subscription).
    #[serde(rename = "type")]
    pub kind: String,

    /// Is this integration enabled.
    #[serde(default)]
    pub enabled: Option<bool>,

    /// Is this integration syncing.
    #[serde(default)]
    pub syncing: Option<bool>,

    /// Id that this integration uses for "subscribers".
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
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
    #[serde(default)]
    pub user: Option<User>,

    /// Integration account information.
    pub account: IntegrationAccount,

    /// When this integration was last synced.
    #[serde(default)]
    pub synced_at: Option<String>,

    /// How many subscribers this integration has.
    #[serde(default)]
    pub subscriber_count: Option<u32>,

    /// Has this integration been revoked.
    #[serde(default)]
    pub revoked: Option<bool>,

    /// The bot/OAuth2 application for discord integrations.
    #[serde(default)]
    pub application: Option<IntegrationApplication>,

    /// Guild id (attached in certain Gateway payloads).
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,
}

/// A Discord guild (server) object.
///
/// See <https://discord.com/developers/docs/resources/guild#guild-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guild {
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Guild name (2-100 characters, excluding trailing and leading whitespace).
    pub name: String,

    /// Icon hash.
    #[serde(default)]
    pub icon: Option<String>,

    /// Icon hash, returned when in the template object.
    #[serde(default)]
    pub icon_hash: Option<String>,

    /// Splash hash.
    #[serde(default)]
    pub splash: Option<String>,

    /// Discovery splash hash.
    #[serde(default)]
    pub discovery_splash: Option<String>,

    /// True if the user is the owner of the guild.
    #[serde(default)]
    pub owner: Option<bool>,

    /// Id of owner.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub owner_id: Option<Snowflake>,

    /// Total permissions for the user in the guild (excludes channel overwrites).
    #[serde(
        default,
        deserialize_with = "de_opt_permissions",
        serialize_with = "ser_opt_permissions"
    )]
    pub permissions: Option<Permissions>,

    /// Voice region id for the guild.
    #[serde(default)]
    pub region: Option<String>,

    /// Id of afk channel.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub afk_channel_id: Option<Snowflake>,

    /// Afk timeout in seconds.
    #[serde(default)]
    pub afk_timeout: Option<u32>,

    /// True if the server widget is enabled.
    #[serde(default)]
    pub widget_enabled: Option<bool>,

    /// The channel id that the widget will generate an invite to, or null.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub widget_channel_id: Option<Snowflake>,

    /// Verification level required for the guild.
    #[serde(default)]
    pub verification_level: Option<u8>,

    /// Default message notifications level.
    #[serde(default)]
    pub default_message_notifications: Option<u8>,

    /// Explicit content filter level.
    #[serde(default)]
    pub explicit_content_filter: Option<u8>,

    /// Roles in the guild.
    #[serde(default)]
    pub roles: Option<Vec<Role>>,

    /// Custom guild emojis.
    #[serde(default)]
    pub emojis: Option<Vec<Emoji>>,

    /// Enabled guild features.
    #[serde(default)]
    pub features: Option<Vec<String>>,

    /// Required MFA level for the guild.
    #[serde(default)]
    pub mfa_level: Option<u8>,

    /// Application id of the guild creator if it is bot-created.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub application_id: Option<Snowflake>,

    /// The id of the channel where guild notices such as welcome messages and boost events are posted.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub system_channel_id: Option<Snowflake>,

    /// System channel flags.
    #[serde(default)]
    pub system_channel_flags: Option<u64>,

    /// The id of the channel where Community guilds can display rules and/or guidelines.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub rules_channel_id: Option<Snowflake>,

    /// The maximum number of presences for the guild (null is always returned, apart from the largest of guilds).
    #[serde(default)]
    pub max_presences: Option<u64>,

    /// The maximum number of members for the guild.
    #[serde(default)]
    pub max_members: Option<u64>,

    /// The vanity url code for the guild.
    #[serde(default)]
    pub vanity_url_code: Option<String>,

    /// The description of a guild.
    #[serde(default)]
    pub description: Option<String>,

    /// Banner hash.
    #[serde(default)]
    pub banner: Option<String>,

    /// Premium tier (Server Boost level).
    #[serde(default)]
    pub premium_tier: Option<u8>,

    /// The number of boosts this guild currently has.
    #[serde(default)]
    pub premium_subscription_count: Option<u64>,

    /// The preferred locale of a Community guild.
    #[serde(default)]
    pub preferred_locale: Option<String>,

    /// The id of the channel where admins and moderators of Community guilds receive notices from Discord.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub public_updates_channel_id: Option<Snowflake>,

    /// The maximum amount of users in a video channel.
    #[serde(default)]
    pub max_video_channel_users: Option<u32>,

    /// The maximum amount of users in a stage video channel.
    #[serde(default)]
    pub max_stage_video_channel_users: Option<u32>,

    /// Approximate number of members in this guild.
    #[serde(default)]
    pub approximate_member_count: Option<u64>,

    /// Approximate number of non-offline members in this guild.
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,

    /// Custom guild stickers.
    #[serde(default)]
    pub stickers: Option<Vec<Sticker>>,

    /// Whether the guild has the boost progress bar enabled.
    #[serde(default)]
    pub premium_progress_bar_enabled: Option<bool>,

    /// The id of the channel where admins and moderators of Community guilds receive safety alerts.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub safety_alerts_channel_id: Option<Snowflake>,

    // Additional fields dispatched over Gateway during GUILD_CREATE:
    /// When this guild was joined at.
    #[serde(default)]
    pub joined_at: Option<String>,

    /// True if this is considered a large guild.
    #[serde(default)]
    pub large: Option<bool>,

    /// True if this guild is unavailable due to an outage.
    #[serde(default)]
    pub unavailable: Option<bool>,

    /// Total number of members in this guild.
    #[serde(default)]
    pub member_count: Option<u64>,

    /// States of members currently in voice channels; lacks the `guild_id` key.
    #[serde(default)]
    pub voice_states: Option<Vec<VoiceState>>,

    /// Users in the guild.
    #[serde(default)]
    pub members: Option<Vec<GuildMember>>,

    /// Channels in the guild.
    #[serde(default)]
    pub channels: Option<Vec<Channel>>,

    /// All active threads in the guild that current user has permission to view.
    #[serde(default)]
    pub threads: Option<Vec<Channel>>,

    /// Presences of the members in the guild.
    #[serde(default)]
    pub presences: Option<Vec<PresenceUpdate>>,

    /// Stage instances in the guild.
    #[serde(default)]
    pub stage_instances: Option<Vec<StageInstance>>,

    /// Scheduled events in the guild.
    #[serde(default)]
    pub guild_scheduled_events: Option<Vec<GuildScheduledEvent>>,
}
