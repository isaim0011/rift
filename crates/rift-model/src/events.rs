//! Payload structures for specific Gateway dispatch events.

use crate::channel::{Channel, ThreadMember};
use crate::guild::{
    AuditLogEntry, Emoji, Guild, GuildMember, Integration, Role, SoundboardSound,
    StageInstance, Sticker, UnavailableGuild,
};
use crate::interaction::{ApplicationCommandPermission, Interaction};
use crate::message::{Message, ReactionEmoji};
use crate::monetization::{Entitlement, Subscription};
use crate::presence::PresenceUpdate;
use crate::serde_util::{
    de_opt_snowflake, de_opt_vec_snowflake, de_snowflake, de_vec_snowflake, ser_opt_snowflake,
    ser_opt_vec_snowflake, ser_snowflake, ser_vec_snowflake,
};
use crate::user::User;
use crate::voice::{VoiceChannelEffect, VoiceState};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// Partial application object sent in `READY`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadyApplication {
    /// ID of the application.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Application flags.
    #[serde(default)]
    pub flags: Option<u64>,
}

/// Dispatched when a client has completed the initial handshake with the Gateway.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#ready>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ready {
    /// Gateway version.
    pub v: u8,

    /// Information about the current user.
    pub user: User,

    /// Guilds the current user is in (unavailable initially).
    pub guilds: Vec<UnavailableGuild>,

    /// Used for resuming connections.
    pub session_id: String,

    /// Gateway URL for resuming connections.
    pub resume_gateway_url: String,

    /// Shard information associated with this session (`[shard_id, num_shards]`).
    #[serde(default)]
    pub shard: Option<[u32; 2]>,

    /// Partial application object.
    pub application: ReadyApplication,
}

/// Dispatched when a client has sent a resume payload and completed replaying missed events.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#resumed>
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resumed {
    /// Debug trace information Discord may attach.
    #[serde(default)]
    pub _trace: Option<Vec<String>>,
}

/// Dispatched when a message is pinned or unpinned in a text channel.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#channel-pins-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPinsUpdate {
    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// Time at which the most recent pinned message was pinned.
    #[serde(default)]
    pub last_pin_timestamp: Option<String>,
}

/// Dispatched to sync active threads for a guild.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#thread-list-sync>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadListSync {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Parent channel IDs whose threads are being synced (omitted = entire guild).
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub channel_ids: Option<Vec<Snowflake>>,

    /// Active thread channel objects.
    pub threads: Vec<Channel>,

    /// All thread member objects from the synced threads for the current user.
    pub members: Vec<ThreadMember>,
}

/// Dispatched when any user is added to or removed from a thread.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#thread-members-update>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadMembersUpdate {
    /// ID of the thread.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Approximate number of members in the thread, capped at 50.
    pub member_count: u32,

    /// Users who were added to the thread.
    #[serde(default)]
    pub added_members: Option<Vec<ThreadMember>>,

    /// IDs of the users who were removed from the thread.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub removed_member_ids: Option<Vec<Snowflake>>,
}

/// Dispatched when a thread is deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#thread-delete>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadDelete {
    /// ID of the thread.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// ID of the parent channel.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub parent_id: Option<Snowflake>,

    /// Type of channel.
    #[serde(rename = "type")]
    pub kind: u8,
}

/// Dispatched when the current user's thread member is updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#thread-member-update>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ThreadMemberUpdate {
    /// The thread member data.
    #[serde(flatten)]
    pub member: ThreadMember,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,
}

/// Dispatched when a member joins a guild.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-member-add>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildMemberAdd {
    /// The member who joined.
    #[serde(flatten)]
    pub member: GuildMember,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
}

/// Dispatched when a guild member is updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-member-update>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildMemberUpdate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// User role IDs.
    #[serde(
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub roles: Vec<Snowflake>,

    /// The user.
    pub user: User,

    /// Nickname of the user in the guild.
    #[serde(default)]
    pub nick: Option<String>,

    /// Member's guild avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,

    /// When the user joined the guild.
    #[serde(default)]
    pub joined_at: Option<String>,

    /// When the user started boosting the guild.
    #[serde(default)]
    pub premium_since: Option<String>,

    /// Whether the user is deafened in voice channels.
    #[serde(default)]
    pub deaf: Option<bool>,

    /// Whether the user is muted in voice channels.
    #[serde(default)]
    pub mute: Option<bool>,

    /// Whether the user has not yet passed membership screening.
    #[serde(default)]
    pub pending: Option<bool>,

    /// When the user's timeout will expire.
    #[serde(default)]
    pub communication_disabled_until: Option<String>,

    /// Guild member flags.
    #[serde(default)]
    pub flags: Option<u64>,

    /// Avatar decoration data.
    #[serde(default)]
    pub avatar_decoration_data: Option<serde_json::Value>,
}

/// Dispatched when a user is removed from a guild (leave/kick/ban).
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-member-remove>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildMemberRemove {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The user who was removed.
    pub user: User,
}

/// Dispatched in response to Request Guild Members command.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-members-chunk>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildMembersChunk {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Set of guild members.
    pub members: Vec<GuildMember>,

    /// Chunk index in the stream (0 <= chunk_index < chunk_count).
    pub chunk_index: u32,

    /// Total number of expected chunks.
    pub chunk_count: u32,

    /// If passing an invalid ID to `RequestGuildMembers`, it will be returned here.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub not_found: Option<Vec<Snowflake>>,

    /// If passing `true` to `RequestGuildMembers`, presences of the returned members will be here.
    #[serde(default)]
    pub presences: Option<Vec<PresenceUpdate>>,

    /// Nonce used in the Request Guild Members command.
    #[serde(default)]
    pub nonce: Option<String>,
}

/// Dispatched when a guild role is created.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-role-create>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildRoleCreate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The role created.
    pub role: Role,
}

/// Dispatched when a guild role is updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-role-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildRoleUpdate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The role updated.
    pub role: Role,
}

/// Dispatched when a guild role is deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-role-delete>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildRoleDelete {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// ID of the role.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub role_id: Snowflake,
}

/// Dispatched when a message is deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-delete>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelete {
    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,
}

/// Dispatched when multiple messages are deleted at once.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-delete-bulk>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDeleteBulk {
    /// IDs of the messages.
    #[serde(
        deserialize_with = "de_vec_snowflake",
        serialize_with = "ser_vec_snowflake"
    )]
    pub ids: Vec<Snowflake>,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,
}

/// Dispatched when a user adds a reaction to a message.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-reaction-add>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageReactionAdd {
    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Member who reacted if this happened in a guild.
    #[serde(default)]
    pub member: Option<GuildMember>,

    /// The emoji used.
    pub emoji: ReactionEmoji,

    /// ID of the author of the message that was reacted to.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub message_author_id: Option<Snowflake>,

    /// True if this was a super-reaction.
    #[serde(default)]
    pub burst: Option<bool>,

    /// Colors used for super-reaction animation.
    #[serde(default)]
    pub burst_colors: Option<Vec<String>>,

    /// The type of the reaction.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
}

/// Dispatched when a user removes a reaction from a message.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-reaction-remove>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReactionRemove {
    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// The emoji removed.
    pub emoji: ReactionEmoji,

    /// True if this was a super-reaction.
    #[serde(default)]
    pub burst: Option<bool>,

    /// The type of the reaction.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
}

/// Dispatched when a user explicitly removes all reactions from a message.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-reaction-remove-all>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReactionRemoveAll {
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,
}

/// Dispatched when a bot removes all instances of a given emoji from the reactions of a message.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-reaction-remove-emoji>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReactionRemoveEmoji {
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// The emoji that was removed.
    pub emoji: ReactionEmoji,
}

/// Dispatched when a user starts typing in a channel.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#typing-start>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TypingStart {
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// Unix time (in seconds) of when the user started typing.
    pub timestamp: u64,

    /// Member who started typing if this happened in a guild.
    #[serde(default)]
    pub member: Option<GuildMember>,
}

/// Dispatched when an application command's permissions are updated in a guild.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#application-command-permissions-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCommandPermissionsUpdate {
    /// ID of the command or the application ID.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the application the command belongs to.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub application_id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The permissions for the command in the guild.
    pub permissions: Vec<ApplicationCommandPermission>,
}

/// Dispatched when a user is banned from a guild.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-ban-add>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildBanAdd {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// User who was banned.
    pub user: User,
}

/// Dispatched when a user is unbanned from a guild.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-ban-remove>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildBanRemove {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// User who was unbanned.
    pub user: User,
}

/// Dispatched when a guild's emojis have been updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-emojis-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildEmojisUpdate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Array of emojis.
    pub emojis: Vec<Emoji>,
}

/// Dispatched when a guild's stickers have been updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-stickers-update>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildStickersUpdate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// Array of stickers.
    pub stickers: Vec<Sticker>,
}

/// Dispatched when a guild's integrations have been updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-integrations-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildIntegrationsUpdate {
    /// ID of the guild whose integrations were updated.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
}

/// Dispatched when a user subscribes to a scheduled event.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-scheduled-event-user-add>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventUserAdd {
    /// ID of the scheduled event.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_scheduled_event_id: Snowflake,

    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
}

/// Dispatched when a user unsubscribes from a scheduled event.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-scheduled-event-user-remove>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuildScheduledEventUserRemove {
    /// ID of the scheduled event.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_scheduled_event_id: Snowflake,

    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
}

/// Dispatched when guild soundboard sounds are updated.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#guild-soundboard-sounds-update>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuildSoundboardSoundsUpdate {
    /// Array of soundboard sound objects.
    pub soundboard_sounds: Vec<SoundboardSound>,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
}

/// Dispatched when an integration is deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#integration-delete>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationDelete {
    /// Integration id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// ID of the bot/OAuth2 application for this discord integration.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub application_id: Option<Snowflake>,
}

/// Dispatched when an invite to a channel is created.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#invite-create>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InviteCreate {
    /// The channel the invite is for.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// The unique invite code.
    pub code: String,

    /// The time at which the invite was created.
    pub created_at: String,

    /// The guild of the invite.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// The user that created the invite.
    #[serde(default)]
    pub inviter: Option<User>,

    /// How long the invite is valid for (in seconds).
    pub max_age: u32,

    /// The maximum number of times the invite can be used.
    pub max_uses: u32,

    /// Whether or not the invite is temporary.
    pub temporary: bool,

    /// How many times the invite has been used.
    pub uses: u32,
}

/// Dispatched when an invite is deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#invite-delete>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InviteDelete {
    /// The channel of the invite.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// The guild of the invite.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// The unique invite code.
    pub code: String,
}

/// Dispatched when a user votes on a poll.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-poll-vote-add>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagePollVoteAdd {
    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// ID of the answer.
    pub answer_id: u32,
}

/// Dispatched when a user removes their vote on a poll.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#message-poll-vote-remove>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagePollVoteRemove {
    /// ID of the user.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub user_id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// ID of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// ID of the answer.
    pub answer_id: u32,
}

/// Dispatched when a guild channel's webhook is created, updated, or deleted.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#webhooks-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhooksUpdate {
    /// ID of the guild.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,
}

/// Type alias for `GUILD_CREATE` event payload.
pub type GuildCreate = Guild;
/// Type alias for `GUILD_UPDATE` event payload.
pub type GuildUpdate = Guild;
/// Type alias for `GUILD_DELETE` event payload.
pub type GuildDelete = UnavailableGuild;

/// Type alias for `CHANNEL_CREATE` event payload.
pub type ChannelCreate = Channel;
/// Type alias for `CHANNEL_UPDATE` event payload.
pub type ChannelUpdate = Channel;
/// Type alias for `CHANNEL_DELETE` event payload.
pub type ChannelDelete = Channel;

/// Type alias for `THREAD_CREATE` event payload.
pub type ThreadCreate = Channel;
/// Type alias for `THREAD_UPDATE` event payload.
pub type ThreadUpdate = Channel;

/// Type alias for `MESSAGE_CREATE` event payload.
pub type MessageCreate = Message;
/// Type alias for `MESSAGE_UPDATE` event payload.
pub type MessageUpdate = Message;

/// Type alias for `INTERACTION_CREATE` event payload.
pub type InteractionCreate = Interaction;

/// Type alias for `VOICE_STATE_UPDATE` event payload.
pub type VoiceStateUpdate = VoiceState;

/// Type alias for `AUTO_MODERATION_RULE_CREATE` event payload.
pub type AutoModerationRuleCreate = crate::auto_moderation::AutoModerationRule;
/// Type alias for `AUTO_MODERATION_RULE_UPDATE` event payload.
pub type AutoModerationRuleUpdate = crate::auto_moderation::AutoModerationRule;
/// Type alias for `AUTO_MODERATION_RULE_DELETE` event payload.
pub type AutoModerationRuleDelete = crate::auto_moderation::AutoModerationRule;

/// Type alias for `ENTITLEMENT_CREATE` event payload.
pub type EntitlementCreate = Entitlement;
/// Type alias for `ENTITLEMENT_UPDATE` event payload.
pub type EntitlementUpdate = Entitlement;
/// Type alias for `ENTITLEMENT_DELETE` event payload.
pub type EntitlementDelete = Entitlement;

/// Type alias for `SUBSCRIPTION_CREATE` event payload.
pub type SubscriptionCreate = Subscription;
/// Type alias for `SUBSCRIPTION_UPDATE` event payload.
pub type SubscriptionUpdate = Subscription;
/// Type alias for `SUBSCRIPTION_DELETE` event payload.
pub type SubscriptionDelete = Subscription;

/// Type alias for `STAGE_INSTANCE_CREATE` event payload.
pub type StageInstanceCreate = StageInstance;
/// Type alias for `STAGE_INSTANCE_UPDATE` event payload.
pub type StageInstanceUpdate = StageInstance;
/// Type alias for `STAGE_INSTANCE_DELETE` event payload.
pub type StageInstanceDelete = StageInstance;

/// Type alias for `GUILD_SCHEDULED_EVENT_CREATE` event payload.
pub type GuildScheduledEventCreate = crate::guild::GuildScheduledEvent;
/// Type alias for `GUILD_SCHEDULED_EVENT_UPDATE` event payload.
pub type GuildScheduledEventUpdate = crate::guild::GuildScheduledEvent;
/// Type alias for `GUILD_SCHEDULED_EVENT_DELETE` event payload.
pub type GuildScheduledEventDelete = crate::guild::GuildScheduledEvent;

/// Type alias for `GUILD_SOUNDBOARD_SOUND_CREATE` event payload.
pub type GuildSoundboardSoundCreate = SoundboardSound;
/// Type alias for `GUILD_SOUNDBOARD_SOUND_UPDATE` event payload.
pub type GuildSoundboardSoundUpdate = SoundboardSound;
/// Type alias for `GUILD_SOUNDBOARD_SOUND_DELETE` event payload.
pub type GuildSoundboardSoundDelete = SoundboardSound;

/// Type alias for `INTEGRATION_CREATE` event payload.
pub type IntegrationCreate = Integration;
/// Type alias for `INTEGRATION_UPDATE` event payload.
pub type IntegrationUpdate = Integration;

/// Type alias for `GUILD_AUDIT_LOG_ENTRY_CREATE` event payload.
pub type GuildAuditLogEntryCreate = AuditLogEntry;

/// Type alias for `USER_UPDATE` event payload.
pub type UserUpdate = User;

/// Type alias for `VOICE_CHANNEL_EFFECT_SEND` event payload.
pub type VoiceChannelEffectSend = VoiceChannelEffect;
