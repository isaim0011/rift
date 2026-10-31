//! Channel and thread models.

use crate::flags::ChannelFlags;
use crate::serde_util::{
    de_opt_permissions, de_opt_snowflake, de_opt_vec_snowflake, de_permissions, de_snowflake,
    ser_opt_permissions, ser_opt_snowflake, ser_opt_vec_snowflake, ser_permissions, ser_snowflake,
};
use crate::user::User;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Serialize};

/// Type of a channel.
///
/// See <https://discord.com/developers/docs/resources/channel#channel-object-channel-types>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "u8", into = "u8")]
pub enum ChannelType {
    /// A text channel within a server.
    GuildText,
    /// A direct message between users.
    Dm,
    /// A voice channel within a server.
    GuildVoice,
    /// A direct message between multiple users.
    GroupDm,
    /// An organizational category that contains up to 50 channels.
    GuildCategory,
    /// A channel that users can follow and crosspost into their own server.
    GuildAnnouncement,
    /// A temporary sub-channel within a `GUILD_ANNOUNCEMENT` channel.
    AnnouncementThread,
    /// A temporary sub-channel within a `GUILD_TEXT` or `GUILD_FORUM` channel.
    PublicThread,
    /// A temporary sub-channel within a `GUILD_TEXT` channel that is only viewable by those invited and those with the `MANAGE_THREADS` permission.
    PrivateThread,
    /// A voice channel for hosting events with an audience.
    GuildStageVoice,
    /// The channel in a hub containing the listed servers.
    GuildDirectory,
    /// Channel that can only contain threads.
    GuildForum,
    /// Channel that can only contain threads, similar to `GUILD_FORUM` channels.
    GuildMedia,
    /// An unknown or future channel type.
    Unknown(u8),
}

impl From<u8> for ChannelType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::GuildText,
            1 => Self::Dm,
            2 => Self::GuildVoice,
            3 => Self::GroupDm,
            4 => Self::GuildCategory,
            5 => Self::GuildAnnouncement,
            10 => Self::AnnouncementThread,
            11 => Self::PublicThread,
            12 => Self::PrivateThread,
            13 => Self::GuildStageVoice,
            14 => Self::GuildDirectory,
            15 => Self::GuildForum,
            16 => Self::GuildMedia,
            other => Self::Unknown(other),
        }
    }
}

impl From<ChannelType> for u8 {
    fn from(t: ChannelType) -> Self {
        match t {
            ChannelType::GuildText => 0,
            ChannelType::Dm => 1,
            ChannelType::GuildVoice => 2,
            ChannelType::GroupDm => 3,
            ChannelType::GuildCategory => 4,
            ChannelType::GuildAnnouncement => 5,
            ChannelType::AnnouncementThread => 10,
            ChannelType::PublicThread => 11,
            ChannelType::PrivateThread => 12,
            ChannelType::GuildStageVoice => 13,
            ChannelType::GuildDirectory => 14,
            ChannelType::GuildForum => 15,
            ChannelType::GuildMedia => 16,
            ChannelType::Unknown(other) => other,
        }
    }
}

/// An explicit permission overwrite for a role or member in a channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelOverwrite {
    /// Role or user id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Either `0` (role) or `1` (member).
    #[serde(rename = "type")]
    pub kind: u8,

    /// Permission bit set explicitly allowed.
    #[serde(deserialize_with = "de_permissions", serialize_with = "ser_permissions")]
    pub allow: Permissions,

    /// Permission bit set explicitly denied.
    #[serde(deserialize_with = "de_permissions", serialize_with = "ser_permissions")]
    pub deny: Permissions,
}

/// Metadata about a thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMetadata {
    /// Whether the thread is archived.
    #[serde(default)]
    pub archived: bool,

    /// The thread will stop showing in the channel list after `auto_archive_duration` minutes of inactivity.
    #[serde(default)]
    pub auto_archive_duration: u32,

    /// Timestamp when the thread's archive status was last changed, used for calculating recent activity.
    #[serde(default)]
    pub archive_timestamp: String,

    /// Whether the thread is locked; when a thread is locked, only users with `MANAGE_THREADS` can unarchive it.
    #[serde(default)]
    pub locked: bool,

    /// Whether non-moderators can add other non-moderators to a thread; only available on private threads.
    #[serde(default)]
    pub invitable: Option<bool>,

    /// Timestamp when the thread was created; only reported on threads created after 2022-01-09.
    #[serde(default)]
    pub create_timestamp: Option<String>,
}

/// A member of a thread.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadMember {
    /// ID of the thread.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub id: Option<Snowflake>,

    /// ID of the user.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub user_id: Option<Snowflake>,

    /// Time the user joined the thread.
    #[serde(default)]
    pub join_timestamp: String,

    /// Any user-thread settings, currently only used for notifications.
    #[serde(default)]
    pub flags: u64,

    /// Additional information about the user.
    #[serde(default)]
    pub member: Option<crate::guild::GuildMember>,
}

/// A tag that is able to be applied to a thread in a `GUILD_FORUM` or `GUILD_MEDIA` channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForumTag {
    /// The id of the tag.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The name of the tag (0-20 characters).
    pub name: String,

    /// Whether this tag can only be added to or removed from threads by a member with the `MANAGE_THREADS` permission.
    #[serde(default)]
    pub moderated: bool,

    /// The id of a guild's custom emoji.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub emoji_id: Option<Snowflake>,

    /// The unicode character of the emoji.
    #[serde(default)]
    pub emoji_name: Option<String>,
}

/// The emoji to show in the add reaction button on a thread in a forum/media channel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefaultReaction {
    /// The id of a guild's custom emoji.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub emoji_id: Option<Snowflake>,

    /// The unicode character of the emoji.
    #[serde(default)]
    pub emoji_name: Option<String>,
}

/// A Discord channel or thread object.
///
/// Obfuscated channels (with [`ChannelFlags::CHANNEL_OBFUSCATED`] set) strip metadata and
/// may contain placeholder names (`___hidden___`). All metadata fields are optional and
/// should never be assumed to be present.
///
/// See <https://discord.com/developers/docs/resources/channel#channel-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    /// The id of this channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The type of channel.
    #[serde(rename = "type")]
    pub kind: ChannelType,

    /// The id of the guild (may be missing for some channel objects received over gateway or DMs).
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Sorting position of the channel.
    #[serde(default)]
    pub position: Option<i32>,

    /// Explicit permission overwrites for members and roles.
    #[serde(default)]
    pub permission_overwrites: Option<Vec<ChannelOverwrite>>,

    /// The name of the channel (1-100 characters). May be placeholder (`___hidden___`) if obfuscated.
    #[serde(default)]
    pub name: Option<String>,

    /// The channel topic (0-4096 characters for `GUILD_FORUM` and `GUILD_MEDIA`, 0-1024 characters for others).
    #[serde(default)]
    pub topic: Option<String>,

    /// Whether the channel is nsfw.
    #[serde(default)]
    pub nsfw: Option<bool>,

    /// The id of the last message sent in this channel.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub last_message_id: Option<Snowflake>,

    /// The bitrate (in bits) of the voice channel.
    #[serde(default)]
    pub bitrate: Option<u32>,

    /// The user limit of the voice channel.
    #[serde(default)]
    pub user_limit: Option<u32>,

    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(default)]
    pub rate_limit_per_user: Option<u32>,

    /// The recipients of the DM.
    #[serde(default)]
    pub recipients: Option<Vec<User>>,

    /// Icon hash of the group DM.
    #[serde(default)]
    pub icon: Option<String>,

    /// Id of the creator of the group DM or thread.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub owner_id: Option<Snowflake>,

    /// Application id of the group DM creator if it is bot-created.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub application_id: Option<Snowflake>,

    /// For group DM channels: whether the channel is managed by an application.
    #[serde(default)]
    pub managed: Option<bool>,

    /// For guild channels: id of the parent category for a channel, or id of the text channel this thread was created during.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub parent_id: Option<Snowflake>,

    /// When the last pinned message was pinned.
    #[serde(default)]
    pub last_pin_timestamp: Option<String>,

    /// Voice region id for the voice channel, automatic when set to null.
    #[serde(default)]
    pub rtc_region: Option<String>,

    /// The camera video quality mode of the voice channel, 1 when not present.
    #[serde(default)]
    pub video_quality_mode: Option<u8>,

    /// Number of messages (not including the initial message or deleted messages) in a thread.
    #[serde(default)]
    pub message_count: Option<u32>,

    /// An approximate count of users in a thread, stops counting at 50.
    #[serde(default)]
    pub member_count: Option<u32>,

    /// Thread-specific fields not needed by other channels.
    #[serde(default)]
    pub thread_metadata: Option<ThreadMetadata>,

    /// Thread member object for the current user, if they have joined the thread.
    #[serde(default)]
    pub member: Option<ThreadMember>,

    /// Default duration, in minutes, that clients (not the API) use for newly created threads.
    #[serde(default)]
    pub default_auto_archive_duration: Option<u32>,

    /// Computed permissions for the invoking user in the channel, including overwrites.
    #[serde(
        default,
        deserialize_with = "de_opt_permissions",
        serialize_with = "ser_opt_permissions"
    )]
    pub permissions: Option<Permissions>,

    /// Channel flags combined as a bitfield.
    #[serde(default)]
    pub flags: Option<u32>,

    /// Number of messages ever sent in a thread, similar to `message_count`.
    #[serde(default)]
    pub total_message_sent: Option<u32>,

    /// The set of tags that can be used in a `GUILD_FORUM` or `GUILD_MEDIA` channel.
    #[serde(default)]
    pub available_tags: Option<Vec<ForumTag>>,

    /// The IDs of the set of tags that have been applied to a thread in a forum/media channel.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub applied_tags: Option<Vec<Snowflake>>,

    /// The emoji to show in the add reaction button on a thread in a `GUILD_FORUM` or `GUILD_MEDIA` channel.
    #[serde(default)]
    pub default_reaction_emoji: Option<DefaultReaction>,

    /// The initial `rate_limit_per_user` to set on newly created threads in a channel.
    #[serde(default)]
    pub default_thread_rate_limit_per_user: Option<u32>,

    /// The default sort order type used to order posts in `GUILD_FORUM` and `GUILD_MEDIA` channels.
    #[serde(default)]
    pub default_sort_order: Option<u8>,

    /// The default forum layout view used to display posts in `GUILD_FORUM` channels.
    #[serde(default)]
    pub default_forum_layout: Option<u8>,

    /// Whether this thread was newly created (sent only in `THREAD_CREATE`).
    #[serde(default)]
    pub newly_created: Option<bool>,
}

impl Channel {
    /// Returns true if this channel has the [`ChannelFlags::CHANNEL_OBFUSCATED`] flag set,
    /// indicating its metadata is hidden and must not be assumed to carry metadata.
    pub fn is_obfuscated(&self) -> bool {
        self.flags.is_some_and(|f| {
            ChannelFlags::from_bits_retain(f).contains(ChannelFlags::CHANNEL_OBFUSCATED)
        })
    }

    /// Parsed [`ChannelFlags`] for this channel.
    pub fn channel_flags(&self) -> ChannelFlags {
        self.flags
            .map(ChannelFlags::from_bits_retain)
            .unwrap_or(ChannelFlags::NONE)
    }
}
