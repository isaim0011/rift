//! Channel resource endpoints.
//!
//! Coverage from the docs: Get Channel, Modify Channel (guild, Group DM,
//! thread), Delete/Close Channel, Edit/Delete Channel Permissions,
//! Get Channel Invites, Create Channel Invite, Follow Announcement Channel,
//! Trigger Typing, Get/List Messages (already in message.rs for Create),
//! Crosspost, all reaction endpoints, Pin/Unpin, Get Pinned Messages,
//! thread management (Start Thread from/without Message, Forum/Media thread,
//! Join/Leave/Add/Remove Thread Member, Get Thread Member, List Thread
//! Members, List Archived Threads), Get Channel Member/Role Permissions,
//! Set Voice Channel Status.

use crate::error::Error;
use crate::guild::ThreadMember;
use crate::message::{de_opt_snowflake, de_snowflake, ser_opt_snowflake, ser_snowflake, ser_snowflakes, CreateMessage, Message};
use crate::transport::Transport;
use crate::client::RestClient;
use crate::multipart::{build_multipart, validate_files, UploadFile};
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

fn ser_permissions_str<S: Serializer>(v: &Permissions, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}
fn de_perm_str<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Permissions, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Permissions>().map_err(serde::de::Error::custom)
}
fn de_opt_perm<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Permissions>, D::Error> {
    let s = Option::<String>::deserialize(d)?;
    s.map(|v| v.parse::<Permissions>().map_err(serde::de::Error::custom)).transpose()
}

// ───────────────────────── Channel types ─────────────────────────

/// The integer type of a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ChannelType {
    /// A text channel within a server.
    GuildText = 0,
    /// A direct message between users.
    Dm = 1,
    /// A voice channel within a server.
    GuildVoice = 2,
    /// A direct message between multiple users.
    GroupDm = 3,
    /// An organizational category that contains up to 50 channels.
    GuildCategory = 4,
    /// A channel that users can follow and crosspost into their own server.
    GuildAnnouncement = 5,
    /// A temporary sub-channel within a GUILD_ANNOUNCEMENT channel.
    AnnouncementThread = 10,
    /// A temporary sub-channel within a GUILD_TEXT or GUILD_FORUM channel.
    PublicThread = 11,
    /// A temporary sub-channel within a GUILD_TEXT channel that is only viewable by those invited.
    PrivateThread = 12,
    /// A voice channel for hosting events with an audience.
    GuildStageVoice = 13,
    /// The channel in a Student Hub containing the list of student hubs.
    GuildDirectory = 14,
    /// Channel that can only contain threads.
    GuildForum = 15,
    /// Channel that can only contain threads and is optimized for media.
    GuildMedia = 16,
}

impl ChannelType {
    /// Parse a raw value.
    pub const fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::GuildText, 1 => Self::Dm, 2 => Self::GuildVoice,
            3 => Self::GroupDm, 4 => Self::GuildCategory, 5 => Self::GuildAnnouncement,
            10 => Self::AnnouncementThread, 11 => Self::PublicThread, 12 => Self::PrivateThread,
            13 => Self::GuildStageVoice, 14 => Self::GuildDirectory,
            15 => Self::GuildForum, 16 => Self::GuildMedia,
            _ => return None,
        })
    }
    /// Raw integer.
    pub const fn as_u8(self) -> u8 { self as u8 }
    /// True for all thread types.
    pub const fn is_thread(self) -> bool {
        matches!(self, Self::AnnouncementThread | Self::PublicThread | Self::PrivateThread)
    }
    /// True for forum/media channels (can only contain threads).
    pub const fn is_forum_like(self) -> bool {
        matches!(self, Self::GuildForum | Self::GuildMedia)
    }
}

impl<'de> Deserialize<'de> for ChannelType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Ok(Self::from_u8(v).unwrap_or(ChannelType::GuildText))
    }
}

impl Serialize for ChannelType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

/// A channel flag bitfield.
pub mod flags {
    /// Pinned in parent forum/media channel.
    pub const PINNED: u32                   = 1 << 1;
    /// Require tag in forum/media channel.
    pub const REQUIRE_TAG: u32              = 1 << 4;
    /// Hide media download options.
    pub const HIDE_MEDIA_DOWNLOAD: u32      = 1 << 15;
    /// Set when channel metadata is obfuscated; gateway only. Check this
    /// before using any name/topic/overwrite data.
    pub const CHANNEL_OBFUSCATED: u32       = 1 << 17;
    /// Display spoiler banner over media channel.
    pub const IS_SPOILER_CHANNEL: u32       = 1 << 21;
}

/// A permission overwrite on a channel.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionOverwrite {
    /// Role or user id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// `0` = role, `1` = member.
    #[serde(rename = "type")]
    pub kind: u8,
    /// Allowed bits.
    #[serde(deserialize_with = "de_perm_str", serialize_with = "ser_permissions_str",
            default)]
    pub allow: Permissions,
    /// Denied bits.
    #[serde(deserialize_with = "de_perm_str", serialize_with = "ser_permissions_str",
            default)]
    pub deny: Permissions,
}

/// Thread-specific metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ThreadMetadata {
    /// Whether the thread is archived.
    pub archived: bool,
    /// Duration in minutes to automatically archive the thread after recent activity.
    pub auto_archive_duration: u32,
    /// Timestamp when the thread's archive status was last changed, ISO8601.
    pub archive_timestamp: String,
    /// Whether the thread is locked.
    pub locked: bool,
    /// Whether non-moderators can add other non-moderators to a thread (private threads only).
    #[serde(default)]
    pub invitable: Option<bool>,
    /// Timestamp when the thread was created, ISO8601.
    #[serde(default)]
    pub create_timestamp: Option<String>,
}

/// Forum/media channel tag.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ForumTag {
    /// The id of the tag.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// The name of the tag (0-20 characters).
    pub name: String,
    /// Whether this tag can only be added or removed by moderators.
    pub moderated: bool,
    /// The id of a guild's custom emoji.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub emoji_id: Option<Snowflake>,
    /// The unicode character of the emoji.
    #[serde(default)]
    pub emoji_name: Option<String>,
}

/// A channel object (partial; fields present vary by type).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Channel {
    /// The id of this channel.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The type of channel.
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// The id of the guild (may be missing for some channel objects received over gateway guild dispatches).
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Sorting position of the channel.
    #[serde(default)]
    pub position: Option<i32>,
    /// The name of the channel (1-100 characters).
    #[serde(default)]
    pub name: Option<String>,
    /// The channel topic (0-4096 characters for forum/media, 0-1024 for others).
    #[serde(default)]
    pub topic: Option<String>,
    /// Whether the channel is nsfw.
    #[serde(default)]
    pub nsfw: Option<bool>,
    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(default)]
    pub rate_limit_per_user: Option<u32>,
    /// For guild channels: id of the parent category. For threads: id of the parent channel.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub parent_id: Option<Snowflake>,
    /// Explicit permission overwrites for members and roles.
    #[serde(default)]
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    /// Channel flags combined as a bitfield.
    #[serde(default)]
    pub flags: Option<u32>,
    /// Thread-specific fields not needed by other channels.
    #[serde(default)]
    pub thread_metadata: Option<ThreadMetadata>,
    /// Computed permissions for the invoking user (from interactions only).
    #[serde(default, deserialize_with = "de_opt_perm")]
    pub permissions: Option<Permissions>,
}

impl Channel {
    /// True if this channel's metadata has been obfuscated. Check before
    /// using name, topic, overwrites or any optional field.
    pub fn is_obfuscated(&self) -> bool {
        self.flags.is_some_and(|f| f & flags::CHANNEL_OBFUSCATED != 0)
    }
    /// True for text, announcement, voice, forum and media (can have messages).
    pub fn can_have_messages(&self) -> bool {
        matches!(
            self.kind,
            ChannelType::GuildText | ChannelType::GuildVoice | ChannelType::GuildAnnouncement
                | ChannelType::GuildStageVoice | ChannelType::GuildForum | ChannelType::GuildMedia
                | ChannelType::AnnouncementThread | ChannelType::PublicThread | ChannelType::PrivateThread
                | ChannelType::Dm | ChannelType::GroupDm
        )
    }
}

// ─────────────────── Modify Channel bodies ───────────────────────

/// Modify body for a guild channel.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyGuildChannel {
    /// 1-100 character channel name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The type of channel; only conversion between text and announcement is supported.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    /// Channel position in the left-hand listing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,
    /// 0-1024 character channel topic (0-4096 characters for forum/media channels).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<Option<String>>,
    /// Whether the channel is 18+ restricted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<Option<bool>>,
    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
    /// The bitrate (in bits) of the voice or stage channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<Option<u32>>,
    /// The maximum number of users in a voice channel.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<Option<u32>>,
    /// Channel permission overwrites.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Option<Vec<PermissionOverwrite>>>,
    /// Id of the new parent category for a channel.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub parent_id: Option<Option<Snowflake>>,
}

fn ser_opt_snowflake_field<S: Serializer>(v: &Option<Option<Snowflake>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(Some(id)) => s.collect_str(id),
        Some(None) => s.serialize_none(),
        None => s.serialize_none(),
    }
}

/// Modify body for a thread channel.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyThread {
    /// 1-100 character thread name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the thread is archived.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
    /// Duration in minutes to automatically archive the thread after recent activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    /// Whether the thread is locked; when a thread is locked, only users with MANAGE_THREADS can unarchive it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,
    /// Whether non-moderators can add other non-moderators to a thread (private threads only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
    /// Channel flags combined as a bitfield.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// The tags to have applied to a forum or media thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<String>>,
}

// ─────────────────── Invite ─────────────────────────────────────

/// A partial invite object (channel endpoints; some fields may be absent).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Invite {
    /// The invite code.
    pub code: String,
    /// The guild this invite is for.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// The channel this invite is for.
    pub channel: Option<PartialChannel>,
    /// Duration (in seconds) after which the invite expires.
    pub max_age: Option<u32>,
    /// Maximum number of times this invite can be used.
    pub max_uses: Option<u32>,
    /// Number of times this invite has been used.
    pub uses: Option<u32>,
    /// Whether this invite only grants temporary membership.
    pub temporary: Option<bool>,
    /// When this invite was created, ISO8601.
    pub created_at: Option<String>,
}

/// Minimal channel for invite payloads.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PartialChannel {
    /// The id of the channel.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The name of the channel.
    pub name: Option<String>,
    /// The type of channel.
    #[serde(rename = "type")]
    pub kind: ChannelType,
}

/// Body for Create Channel Invite.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct CreateInvite {
    /// Duration of invite in seconds before expiry, or 0 for never.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u32>,
    /// Max number of uses or 0 for unlimited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,
    /// Whether this invite only grants temporary membership.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// If true, don't try to reuse a similar invite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,
    /// The type of target for this voice channel invite.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<u8>,
    /// The id of the user whose stream to display for this invite.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub target_user_id: Option<Option<Snowflake>>,
    /// The id of the embedded application to open for this invite.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub target_application_id: Option<Option<Snowflake>>,
}

/// The `followed_channel` object returned by Follow Announcement Channel.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FollowedChannel {
    /// Source channel id.
    #[serde(deserialize_with = "de_snowflake")]
    pub channel_id: Snowflake,
    /// Created target webhook id.
    #[serde(deserialize_with = "de_snowflake")]
    pub webhook_id: Snowflake,
}

// ─────────────────── Thread helpers ─────────────────────────────

/// Query params for Get Channel Messages.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GetMessages {
    /// Get messages around this message id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub around: Option<Option<Snowflake>>,
    /// Get messages before this message id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub before: Option<Option<Snowflake>>,
    /// Get messages after this message id.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake_field")]
    pub after: Option<Option<Snowflake>>,
    /// Max number of messages to return (1-100, default 50).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
}

/// Body for Start Thread from/without Message.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateThread {
    /// 1-100 character thread name.
    pub name: String,
    /// Duration in minutes to automatically archive the thread after recent activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    /// The type of thread to create.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    /// Whether non-moderators can add other non-moderators to a thread (private threads only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
}

/// A page of archived threads.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ArchivedThreadsPage {
    /// List of archived thread channels.
    pub threads: Vec<Channel>,
    /// Whether there are more archived threads to fetch.
    pub has_more: bool,
}

/// Request body for starting a thread in a forum or media channel.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateForumThread {
    /// 1-100 character thread name.
    pub name: String,
    /// Duration in minutes to auto-archive (60, 1440, 4320, 10080).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,
    /// Amount of seconds a user has to wait before sending another message (0-21600).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,
    /// IDs of tags applied to the thread (max 5).
    #[serde(skip_serializing_if = "Vec::is_empty", serialize_with = "ser_snowflakes")]
    pub applied_tags: Vec<Snowflake>,
    /// Contents of the initial message.
    pub message: CreateMessage,
}

impl CreateForumThread {
    /// Create a new forum or media thread request with a name and message.
    pub fn new(name: impl Into<String>, message: CreateMessage) -> Self {
        Self {
            name: name.into(),
            auto_archive_duration: None,
            rate_limit_per_user: None,
            applied_tags: Vec::new(),
            message,
        }
    }

    /// Set auto archive duration (60, 1440, 4320, 10080).
    pub fn auto_archive_duration(mut self, duration: u32) -> Self {
        self.auto_archive_duration = Some(duration);
        self
    }

    /// Set rate limit per user (slowmode) in seconds (0-21600).
    pub fn rate_limit_per_user(mut self, seconds: u32) -> Self {
        self.rate_limit_per_user = Some(Some(seconds));
        self
    }

    /// Set applied tags (max 5).
    pub fn applied_tags(mut self, tags: Vec<Snowflake>) -> Self {
        self.applied_tags = tags;
        self
    }

    /// Add an applied tag.
    pub fn tag(mut self, tag_id: Snowflake) -> Self {
        self.applied_tags.push(tag_id);
        self
    }

    /// Validate fields against Discord limits.
    pub fn validate(&self) -> Result<(), Error> {
        check_thread_name(&self.name)?;
        if let Some(d) = self.auto_archive_duration {
            check_archive_duration(d)?;
        }
        if let Some(Some(s)) = self.rate_limit_per_user {
            check_slowmode(s)?;
        }
        if self.applied_tags.len() > 5 {
            return Err(ChannelError::TooManyAppliedTags { count: self.applied_tags.len() }.into());
        }
        self.message.validate()?;
        Ok(())
    }
}

/// Query parameters for listing thread members.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ListThreadMembers {
    /// Whether to include guild member data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_member: Option<bool>,
    /// Get thread members after this user ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub after: Option<Snowflake>,
    /// Max number of thread members to return (1-100, default 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl ListThreadMembers {
    /// Create empty list parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Include guild member data.
    pub fn with_member(mut self, with: bool) -> Self {
        self.with_member = Some(with);
        self
    }

    /// Get thread members after this user ID.
    pub fn after(mut self, id: Snowflake) -> Self {
        self.after = Some(id);
        self
    }

    /// Max number of thread members to return (1-100).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// A response from listing pinned messages (`GET /channels/{id}/messages/pins`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PinnedMessagesResponse {
    /// The pinned messages.
    pub items: Vec<PinnedMessage>,
    /// Whether there are more pinned messages before this page.
    pub has_more: bool,
}

/// A pinned message entry with pin timestamp.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PinnedMessage {
    /// ISO8601 timestamp when the message was pinned.
    pub pinned_at: String,
    /// The message object.
    pub message: Message,
}

/// Response for polling answer voters (`GET /channels/{id}/polls/{message_id}/answers/{answer_id}`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct PollAnswerVoters {
    /// Users who voted for this answer.
    pub users: Vec<crate::interaction::User>,
}


/// Request body for adding a recipient to a Group DM (`PUT /channels/{id}/recipients/{user_id}`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AddGroupDmUser {
    /// Access token of a user that has granted your app the `gdm.join` scope.
    pub access_token: String,
    /// Nickname of the user being added.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
}

impl AddGroupDmUser {
    /// Create a new add-recipient request with the access token.
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            nick: None,
        }
    }

    /// Set nickname.
    pub fn nick(mut self, nick: impl Into<String>) -> Self {
        self.nick = Some(nick.into());
        self
    }
}

/// Request body for playing a soundboard sound in a voice channel (`POST /channels/{id}/send-soundboard-sound`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SendSoundboardSound {
    /// The id of the sound to play.
    #[serde(serialize_with = "ser_snowflake", deserialize_with = "de_snowflake")]
    pub sound_id: Snowflake,
    /// The id of the guild the sound is from (optional).
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake", deserialize_with = "de_opt_snowflake")]
    pub source_guild_id: Option<Snowflake>,
}

impl SendSoundboardSound {
    /// Create a new soundboard request.
    pub fn new(sound_id: Snowflake) -> Self {
        Self {
            sound_id,
            source_guild_id: None,
        }
    }

    /// Set source guild id.
    pub fn source_guild_id(mut self, id: Snowflake) -> Self {
        self.source_guild_id = Some(id);
        self
    }
}

/// Tag matching setting for thread search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadSearchTagSetting {
    /// The thread tags must contain all tags in the search query.
    MatchAll,
    /// The thread tags must contain at least one of tags in the search query.
    MatchSome,
}

impl ThreadSearchTagSetting {
    /// String representation as used in query param.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MatchAll => "match_all",
            Self::MatchSome => "match_some",
        }
    }
}

/// Sorting mode for thread search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreadSortingMode {
    /// Sort by relevance.
    Relevance,
    /// Sort by creation time.
    CreationTime,
    /// Sort by last message time.
    LastMessageTime,
    /// Sort by archive time.
    ArchiveTime,
}

impl ThreadSortingMode {
    /// String representation as used in query param.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::CreationTime => "creation_time",
            Self::LastMessageTime => "last_message_time",
            Self::ArchiveTime => "archive_time",
        }
    }
}

/// Sorting order for search results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortingOrder {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

impl SortingOrder {
    /// String representation as used in query param.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Query parameters for searching threads in a channel (`GET /channels/{id}/threads/search`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThreadSearchQuery {
    /// Query string to match against thread names (max 100 characters).
    pub name: Option<String>,
    /// Slop factor for fuzzy search (0-100).
    pub slop: Option<u32>,
    /// Minimum snowflake ID.
    pub min_id: Option<Snowflake>,
    /// Maximum snowflake ID.
    pub max_id: Option<Snowflake>,
    /// Tag IDs to filter by (max 20 tags).
    pub tags: Vec<Snowflake>,
    /// Tag setting (match all or match some).
    pub tag_setting: Option<ThreadSearchTagSetting>,
    /// Filter by archived status.
    pub archived: Option<bool>,
    /// Sorting mode.
    pub sort_by: Option<ThreadSortingMode>,
    /// Sorting order.
    pub sort_order: Option<SortingOrder>,
    /// Maximum number of threads to return (1-25).
    pub limit: Option<u32>,
    /// Number of threads to skip (0-9975).
    pub offset: Option<u32>,
}

impl ThreadSearchQuery {
    /// Create a new empty search query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set thread name query string (max 100 characters).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set slop factor (0-100).
    pub fn slop(mut self, slop: u32) -> Self {
        self.slop = Some(slop);
        self
    }

    /// Set minimum snowflake ID.
    pub fn min_id(mut self, id: Snowflake) -> Self {
        self.min_id = Some(id);
        self
    }

    /// Set maximum snowflake ID.
    pub fn max_id(mut self, id: Snowflake) -> Self {
        self.max_id = Some(id);
        self
    }

    /// Add a tag filter (max 20 tags).
    pub fn tag(mut self, tag: Snowflake) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set tag matching strategy.
    pub fn tag_setting(mut self, setting: ThreadSearchTagSetting) -> Self {
        self.tag_setting = Some(setting);
        self
    }

    /// Filter by archived status.
    pub fn archived(mut self, archived: bool) -> Self {
        self.archived = Some(archived);
        self
    }

    /// Set sorting mode.
    pub fn sort_by(mut self, mode: ThreadSortingMode) -> Self {
        self.sort_by = Some(mode);
        self
    }

    /// Set sort order.
    pub fn sort_order(mut self, order: SortingOrder) -> Self {
        self.sort_order = Some(order);
        self
    }

    /// Set max results limit (1-25).
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set result offset (0-9975).
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Validate query parameters against Discord limits.
    pub fn validate(&self) -> Result<(), ChannelError> {
        if let Some(name) = &self.name {
            let len = name.chars().count();
            if len > 100 {
                return Err(ChannelError::BadThreadSearchName { len });
            }
        }
        if let Some(slop) = self.slop {
            if slop > 100 {
                return Err(ChannelError::BadThreadSearchSlop { value: slop });
            }
        }
        if self.tags.len() > 20 {
            return Err(ChannelError::TooManyThreadSearchTags { count: self.tags.len() });
        }
        if let Some(limit) = self.limit {
            if limit == 0 || limit > 25 {
                return Err(ChannelError::BadThreadSearchLimit { value: limit });
            }
        }
        if let Some(offset) = self.offset {
            if offset > 9975 {
                return Err(ChannelError::BadThreadSearchOffset { value: offset });
            }
        }
        Ok(())
    }
}

/// Response from searching threads in a channel (`GET /channels/{id}/threads/search`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ThreadSearchResponse {
    /// Found thread channels.
    pub threads: Vec<Channel>,
    /// Thread members for the current user and threads returned.
    pub members: Vec<ThreadMember>,
    /// Whether there are more search results.
    pub has_more: bool,
    /// First message of each thread if requested/available.
    #[serde(default)]
    pub first_messages: Option<Vec<Message>>,
    /// Total count of matching results.
    pub total_results: u32,
}

/// Request body for creating a webhook in a channel (`POST /channels/{id}/webhooks`).
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreateWebhook {
    /// Name of the webhook (1-80 characters, cannot contain "clyde" or "discord").
    pub name: String,
    /// Image for the default webhook avatar (data URI string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

impl CreateWebhook {
    /// Create a new webhook request with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            avatar: None,
        }
    }

    /// Set the avatar image data URI.
    pub fn avatar(mut self, avatar: impl Into<String>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Validate name length and prohibited substrings ("clyde", "discord").
    pub fn validate(&self) -> Result<(), ChannelError> {
        check_webhook_name(&self.name)
    }
}

/// Validate webhook name: 1-80 characters, and must not contain "clyde" or "discord" (case-insensitive).
pub fn check_webhook_name(name: &str) -> Result<(), ChannelError> {
    let len = name.chars().count();
    if len == 0 || len > 80 {
        return Err(ChannelError::BadWebhookName { len });
    }
    let lower = name.to_ascii_lowercase();
    if lower.contains("clyde") || lower.contains("discord") {
        return Err(ChannelError::ProhibitedWebhookName { name: name.to_string() });
    }
    Ok(())
}

/// Validation errors for channel endpoint params.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelError {
    /// thread name is not 1-100 characters.
    BadThreadName {
        /// Actual length.
        len: usize,
    },
    /// rate_limit_per_user out of 0-21600.
    BadSlowmode {
        /// Actual value.
        value: u32,
    },
    /// auto_archive_duration is not one of 60, 1440, 4320, 10080.
    BadArchiveDuration {
        /// Actual value.
        value: u32,
    },
    /// GetMessages `limit` is not 1-100.
    BadMessageLimit {
        /// Actual value.
        value: u8,
    },
    /// Create invite max_age out of 0-604800.
    BadInviteMaxAge {
        /// Actual value.
        value: u32,
    },
    /// Create invite max_uses out of 0-100.
    BadInviteMaxUses {
        /// Actual value.
        value: u32,
    },
    /// Bulk delete messages count is not 2-100.
    BadBulkDeleteCount {
        /// Actual count.
        count: usize,
    },
    /// A message in bulk delete is older than 2 weeks.
    BulkDeleteMessageTooOld {
        /// The offending message ID.
        id: Snowflake,
    },
    /// Thread member limit is not 1-100.
    BadThreadMemberLimit {
        /// The invalid limit.
        value: u32,
    },
    /// More than 5 applied tags on a forum thread.
    TooManyAppliedTags {
        /// Actual count.
        count: usize,
    },
    /// Pin limit is not 1-50.
    BadPinLimit {
        /// Actual limit.
        value: u8,
    },
    /// Poll voter limit is not 1-100.
    BadPollVoterLimit {
        /// Actual limit.
        value: u8,
    },
    /// Thread search name exceeds 100 characters.
    BadThreadSearchName {
        /// Actual length.
        len: usize,
    },
    /// Thread search slop exceeds 100.
    BadThreadSearchSlop {
        /// Actual slop.
        value: u32,
    },
    /// More than 20 tags in thread search.
    TooManyThreadSearchTags {
        /// Actual count.
        count: usize,
    },
    /// Thread search limit is not 1-25.
    BadThreadSearchLimit {
        /// Actual limit.
        value: u32,
    },
    /// Thread search offset exceeds 9975.
    BadThreadSearchOffset {
        /// Actual offset.
        value: u32,
    },
    /// Webhook name is not 1-80 characters.
    BadWebhookName {
        /// Actual length.
        len: usize,
    },
    /// Webhook name contains prohibited substring "clyde" or "discord".
    ProhibitedWebhookName {
        /// Offending name.
        name: String,
    },
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadThreadName { len } => write!(f, "thread name is {len} chars (must be 1-100)"),
            Self::BadSlowmode { value } => write!(f, "rate_limit_per_user {value} is out of 0-21600"),
            Self::BadArchiveDuration { value } => write!(f, "auto_archive_duration {value} must be 60/1440/4320/10080"),
            Self::BadMessageLimit { value } => write!(f, "message limit {value} must be 1-100"),
            Self::BadInviteMaxAge { value } => write!(f, "invite max_age {value} is out of 0-604800"),
            Self::BadInviteMaxUses { value } => write!(f, "invite max_uses {value} is out of 0-100"),
            Self::BadBulkDeleteCount { count } => write!(f, "bulk delete message count {count} must be between 2 and 100"),
            Self::BulkDeleteMessageTooOld { id } => write!(f, "message {id} is older than 2 weeks (cannot be bulk deleted)"),
            Self::BadThreadMemberLimit { value } => write!(f, "thread member limit {value} must be 1-100"),
            Self::TooManyAppliedTags { count } => write!(f, "{count} applied tags (max 5)"),
            Self::BadPinLimit { value } => write!(f, "pin limit {value} must be 1-50"),
            Self::BadPollVoterLimit { value } => write!(f, "poll voter limit {value} must be 1-100"),
            Self::BadThreadSearchName { len } => write!(f, "thread search name is {len} characters (max 100)"),
            Self::BadThreadSearchSlop { value } => write!(f, "thread search slop {value} must be 0-100"),
            Self::TooManyThreadSearchTags { count } => write!(f, "{count} tags (max 20)"),
            Self::BadThreadSearchLimit { value } => write!(f, "thread search limit {value} must be 1-25"),
            Self::BadThreadSearchOffset { value } => write!(f, "thread search offset {value} exceeds 9975"),
            Self::BadWebhookName { len } => write!(f, "webhook name is {len} characters (must be 1-80)"),
            Self::ProhibitedWebhookName { name } => write!(f, "webhook name cannot contain 'clyde' or 'discord': {name:?}"),
        }
    }
}

impl std::error::Error for ChannelError {}

impl From<ChannelError> for Error {
    fn from(e: ChannelError) -> Self {
        Error::Transport(format!("channel validation: {e}"))
    }
}

fn check_thread_name(name: &str) -> Result<(), ChannelError> {
    let len = name.chars().count();
    if len == 0 || len > 100 { Err(ChannelError::BadThreadName { len }) } else { Ok(()) }
}
fn check_slowmode(v: u32) -> Result<(), ChannelError> {
    if v > 21600 { Err(ChannelError::BadSlowmode { value: v }) } else { Ok(()) }
}
fn check_archive_duration(v: u32) -> Result<(), ChannelError> {
    if ![60, 1440, 4320, 10080].contains(&v) { Err(ChannelError::BadArchiveDuration { value: v }) } else { Ok(()) }
}

fn query_string<T: Serialize>(v: &T) -> String {
    let j = serde_json::to_value(v).unwrap_or_default();
    let obj = j.as_object().unwrap();
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

// ─────────────────── REST methods ───────────────────────────────

impl<T: Transport> RestClient<T> {
    /// `GET /channels/{id}`.
    pub async fn get_channel(&self, channel_id: Snowflake) -> Result<Channel, Error> {
        self.request("GET", &format!("/channels/{channel_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `PATCH /channels/{id}` for a guild channel.
    pub async fn modify_guild_channel(
        &self, channel_id: Snowflake, body: &ModifyGuildChannel, reason: Option<&str>,
    ) -> Result<Channel, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PATCH", &format!("/channels/{channel_id}"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PATCH /channels/{id}` for a thread.
    pub async fn modify_thread(
        &self, channel_id: Snowflake, body: &ModifyThread,
    ) -> Result<Channel, Error> {
        if let Some(n) = &body.name { check_thread_name(n)?; }
        if let Some(d) = body.auto_archive_duration { check_archive_duration(d)?; }
        if let Some(Some(s)) = body.rate_limit_per_user { check_slowmode(s)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", &format!("/channels/{channel_id}"), Some(b))
            .await?.expect_success()?.json()
    }

    /// `DELETE /channels/{id}`.
    pub async fn delete_channel(&self, channel_id: Snowflake, reason: Option<&str>) -> Result<Channel, Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/channels/{channel_id}"), None, headers)
            .await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/messages` – list messages (up to 100).
    pub async fn get_messages(&self, channel_id: Snowflake, q: &GetMessages) -> Result<Vec<Message>, Error> {
        if let Some(lim) = q.limit {
            if lim == 0 || lim > 100 { return Err(ChannelError::BadMessageLimit { value: lim }.into()); }
        }
        let qs = query_string(q);
        self.request("GET", &format!("/channels/{channel_id}/messages{qs}"), None)
            .await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/messages/{id}`.
    pub async fn get_message(&self, channel_id: Snowflake, message_id: Snowflake) -> Result<Message, Error> {
        self.request("GET", &format!("/channels/{channel_id}/messages/{message_id}"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/messages/{id}/crosspost` – publish to announcement channel.
    pub async fn crosspost_message(&self, channel_id: Snowflake, message_id: Snowflake) -> Result<Message, Error> {
        self.request("POST", &format!("/channels/{channel_id}/messages/{message_id}/crosspost"), None)
            .await?.expect_success()?.json()
    }

    /// `DELETE /channels/{id}/messages/{id}/reactions` – clear all reactions.
    pub async fn clear_reactions(&self, channel_id: Snowflake, message_id: Snowflake) -> Result<(), Error> {
        self.request("DELETE", &format!("/channels/{channel_id}/messages/{message_id}/reactions"), None)
            .await?.expect_success()?; Ok(())
    }

    /// `DELETE /channels/{id}/messages/{id}/reactions/{emoji}` – clear one emoji's reactions.
    pub async fn clear_emoji_reactions(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{}", crate::message::percent_encode(emoji));
        self.request("DELETE", &path, None).await?.expect_success()?; Ok(())
    }

    /// `DELETE /channels/{id}/messages/{id}/reactions/{emoji}/{user_id}` – delete a user's reaction.
    pub async fn delete_user_reaction(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str, user_id: Snowflake) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{}/{user_id}", crate::message::percent_encode(emoji));
        self.request("DELETE", &path, None).await?.expect_success()?; Ok(())
    }

    /// `DELETE /channels/{id}/messages/{id}/reactions/{emoji}/@me` – delete own reaction.
    pub async fn delete_my_message_reaction(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{}/@me", crate::message::percent_encode(emoji));
        self.request("DELETE", &path, None).await?.expect_success()?; Ok(())
    }

    /// Alias for [`delete_my_message_reaction`](Self::delete_my_message_reaction).
    pub async fn delete_own_reaction(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str) -> Result<(), Error> {
        self.delete_my_message_reaction(channel_id, message_id, emoji).await
    }

    /// `GET /channels/{id}/messages/{id}/reactions/{emoji}` – list users who reacted.
    pub async fn get_reactions(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str, limit: Option<u8>) -> Result<Vec<crate::interaction::User>, Error> {
        let lim_qs = limit.map(|l| format!("?limit={l}")).unwrap_or_default();
        let path = format!("/channels/{channel_id}/messages/{message_id}/reactions/{}{lim_qs}", crate::message::percent_encode(emoji));
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/pins`.
    pub async fn get_pinned_messages(&self, channel_id: Snowflake) -> Result<Vec<Message>, Error> {
        self.request("GET", &format!("/channels/{channel_id}/pins"), None)
            .await?.expect_success()?.json()
    }

    /// `PUT /channels/{id}/pins/{id}`.
    pub async fn pin_message(&self, channel_id: Snowflake, message_id: Snowflake, reason: Option<&str>) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/pins/{message_id}");
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PUT", &path, None, headers).await?.expect_success()?; Ok(())
    }

    /// `DELETE /channels/{id}/pins/{id}`.
    pub async fn unpin_message(&self, channel_id: Snowflake, message_id: Snowflake, reason: Option<&str>) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/pins/{message_id}");
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &path, None, headers).await?.expect_success()?; Ok(())
    }

    /// `GET /channels/{id}/messages/pins` – list pinned messages with pagination.
    pub async fn list_pins(
        &self,
        channel_id: Snowflake,
        before: Option<&str>,
        limit: Option<u8>,
    ) -> Result<PinnedMessagesResponse, Error> {
        let mut qs = Vec::new();
        if let Some(b) = before {
            qs.push(format!("before={b}"));
        }
        if let Some(lim) = limit {
            if lim == 0 || lim > 50 {
                return Err(ChannelError::BadPinLimit { value: lim }.into());
            }
            qs.push(format!("limit={lim}"));
        }
        let q = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        let path = format!("/channels/{channel_id}/messages/pins{q}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PUT /channels/{id}/messages/pins/{id}` – pin a message via the messages/pins route.
    pub async fn create_pin(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/pins/{message_id}");
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PUT", &path, None, headers).await?.expect_success()?;
        Ok(())
    }

    /// `DELETE /channels/{id}/messages/pins/{id}` – delete a pin via the messages/pins route.
    pub async fn delete_pin(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/pins/{message_id}");
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &path, None, headers).await?.expect_success()?;
        Ok(())
    }

    /// `POST /channels/{id}/typing`.
    pub async fn trigger_typing(&self, channel_id: Snowflake) -> Result<(), Error> {
        self.request("POST", &format!("/channels/{channel_id}/typing"), None)
            .await?.expect_success()?; Ok(())
    }

    /// `PUT /channels/{id}/permissions/{overwrite_id}` – edit a permission overwrite.
    pub async fn edit_channel_permissions(&self, channel_id: Snowflake, overwrite_id: Snowflake, allow: Option<Permissions>, deny: Option<Permissions>, kind: u8, reason: Option<&str>) -> Result<(), Error> {
        let body = serde_json::json!({
            "allow": allow.unwrap_or(Permissions::NONE).bits().to_string(),
            "deny": deny.unwrap_or(Permissions::NONE).bits().to_string(),
            "type": kind
        });
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("PUT", &format!("/channels/{channel_id}/permissions/{overwrite_id}"), Some(serde_json::to_vec(&body).unwrap()), headers).await?.expect_success()?; Ok(())
    }

    /// `DELETE /channels/{id}/permissions/{overwrite_id}`.
    pub async fn delete_channel_permission(&self, channel_id: Snowflake, overwrite_id: Snowflake, reason: Option<&str>) -> Result<(), Error> {
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("DELETE", &format!("/channels/{channel_id}/permissions/{overwrite_id}"), None, headers).await?.expect_success()?; Ok(())
    }

    /// `GET /channels/{id}/invites`.
    pub async fn get_channel_invites(&self, channel_id: Snowflake) -> Result<Vec<Invite>, Error> {
        self.request("GET", &format!("/channels/{channel_id}/invites"), None)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/invites`.
    pub async fn create_invite(&self, channel_id: Snowflake, body: &CreateInvite, reason: Option<&str>) -> Result<Invite, Error> {
        if let Some(age) = body.max_age { if age > 604800 { return Err(ChannelError::BadInviteMaxAge { value: age }.into()); } }
        if let Some(uses) = body.max_uses { if uses > 100 { return Err(ChannelError::BadInviteMaxUses { value: uses }.into()); } }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/invites"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/followers` – follow an announcement channel.
    pub async fn follow_announcement_channel(&self, channel_id: Snowflake, webhook_channel_id: Snowflake, reason: Option<&str>) -> Result<FollowedChannel, Error> {
        let body = serde_json::json!({ "webhook_channel_id": webhook_channel_id.to_string() });
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/followers"), Some(serde_json::to_vec(&body).unwrap()), headers)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/messages/{id}/threads` – start a thread from a message.
    pub async fn start_thread_from_message(&self, channel_id: Snowflake, message_id: Snowflake, body: &CreateThread, reason: Option<&str>) -> Result<Channel, Error> {
        check_thread_name(&body.name)?;
        if let Some(d) = body.auto_archive_duration { check_archive_duration(d)?; }
        if let Some(Some(s)) = body.rate_limit_per_user { check_slowmode(s)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/messages/{message_id}/threads"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/threads` – start a thread without a message.
    pub async fn start_thread(&self, channel_id: Snowflake, body: &CreateThread, reason: Option<&str>) -> Result<Channel, Error> {
        check_thread_name(&body.name)?;
        if let Some(d) = body.auto_archive_duration { check_archive_duration(d)?; }
        if let Some(Some(s)) = body.rate_limit_per_user { check_slowmode(s)?; }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/threads"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `PUT /channels/{id}/thread-members/@me` – join a thread.
    pub async fn join_thread(&self, channel_id: Snowflake) -> Result<(), Error> {
        self.request("PUT", &format!("/channels/{channel_id}/thread-members/@me"), None)
            .await?.expect_success()?; Ok(())
    }
    /// `DELETE /channels/{id}/thread-members/@me` – leave a thread.
    pub async fn leave_thread(&self, channel_id: Snowflake) -> Result<(), Error> {
        self.request("DELETE", &format!("/channels/{channel_id}/thread-members/@me"), None)
            .await?.expect_success()?; Ok(())
    }
    /// `PUT /channels/{id}/thread-members/{user_id}`.
    pub async fn add_thread_member(&self, channel_id: Snowflake, user_id: Snowflake) -> Result<(), Error> {
        self.request("PUT", &format!("/channels/{channel_id}/thread-members/{user_id}"), None)
            .await?.expect_success()?; Ok(())
    }
    /// `DELETE /channels/{id}/thread-members/{user_id}`.
    pub async fn remove_thread_member(&self, channel_id: Snowflake, user_id: Snowflake) -> Result<(), Error> {
        self.request("DELETE", &format!("/channels/{channel_id}/thread-members/{user_id}"), None)
            .await?.expect_success()?; Ok(())
    }

    /// `GET /channels/{id}/threads/archived/public`.
    pub async fn list_public_archived_threads(&self, channel_id: Snowflake, before: Option<&str>, limit: Option<u32>) -> Result<ArchivedThreadsPage, Error> {
        let mut qs = Vec::new();
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        let path = format!("/channels/{channel_id}/threads/archived/public{}", if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) });
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/threads/archived/private`.
    pub async fn list_private_archived_threads(&self, channel_id: Snowflake, before: Option<&str>, limit: Option<u32>) -> Result<ArchivedThreadsPage, Error> {
        let mut qs = Vec::new();
        if let Some(b) = before { qs.push(format!("before={b}")); }
        if let Some(l) = limit { qs.push(format!("limit={l}")); }
        let path = format!("/channels/{channel_id}/threads/archived/private{}", if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) });
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/users/@me/threads/archived/private`.
    pub async fn list_joined_private_archived_threads(&self, channel_id: Snowflake) -> Result<ArchivedThreadsPage, Error> {
        self.request("GET", &format!("/channels/{channel_id}/users/@me/threads/archived/private"), None)
            .await?.expect_success()?.json()
    }

    /// `PUT /channels/{id}/voice-status` – set the voice channel status.
    pub async fn set_voice_channel_status(&self, channel_id: Snowflake, status: Option<&str>, reason: Option<&str>) -> Result<(), Error> {
        let body = serde_json::json!({ "status": status });
        let b = serde_json::to_vec(&body).unwrap();
        let mut r = self.request("PUT", &format!("/channels/{channel_id}/voice-status"), Some(b)).await?;
        if let Some(reason) = reason { r.headers.push(("X-Audit-Log-Reason".to_owned(), reason.to_owned())); }
        r.expect_success()?; Ok(())
    }

    /// `GET /channels/{id}/permissions/member/{user_id}` – computed permissions for a member.
    pub async fn get_channel_member_permissions(&self, channel_id: Snowflake, user_id: Snowflake) -> Result<Permissions, Error> {
        #[derive(Deserialize)]
        struct R { #[serde(deserialize_with = "de_perm_str")] permissions: Permissions }
        let r: R = self.request("GET", &format!("/channels/{channel_id}/permissions/member/{user_id}"), None)
            .await?.expect_success()?.json()?;
        Ok(r.permissions)
    }

    /// `GET /channels/{id}/permissions/role/{role_id}` – computed permissions for a role.
    pub async fn get_channel_role_permissions(&self, channel_id: Snowflake, role_id: Snowflake) -> Result<Permissions, Error> {
        #[derive(Deserialize)]
        struct R { #[serde(deserialize_with = "de_perm_str")] permissions: Permissions }
        let r: R = self.request("GET", &format!("/channels/{channel_id}/permissions/role/{role_id}"), None)
            .await?.expect_success()?.json()?;
        Ok(r.permissions)
    }

    /// `POST /channels/{id}/messages/bulk-delete`: delete 2-100 messages at once.
    ///
    /// Validates locally that between 2 and 100 messages are provided, and that none are
    /// older than 2 weeks (14 days), as enforced by Discord.
    pub async fn bulk_delete_messages(
        &self,
        channel_id: Snowflake,
        messages: &[Snowflake],
        reason: Option<&str>,
    ) -> Result<(), Error> {
        if messages.len() < 2 || messages.len() > 100 {
            return Err(ChannelError::BadBulkDeleteCount { count: messages.len() }.into());
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let two_weeks_ms = 14 * 24 * 60 * 60 * 1000;
        for &id in messages {
            if now_ms.saturating_sub(id.timestamp_ms()) > two_weeks_ms {
                return Err(ChannelError::BulkDeleteMessageTooOld { id }.into());
            }
        }
        let body = serde_json::json!({
            "messages": messages.iter().map(|id| id.to_string()).collect::<Vec<_>>()
        });
        let b = serde_json::to_vec(&body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/messages/bulk-delete"), Some(b), headers)
            .await?.expect_success()?;
        Ok(())
    }

    /// `GET /channels/{id}/thread-members/{user_id}`: get a thread member.
    pub async fn get_thread_member(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
        with_member: Option<bool>,
    ) -> Result<ThreadMember, Error> {
        let mut path = format!("/channels/{channel_id}/thread-members/{user_id}");
        if let Some(wm) = with_member {
            path.push_str(&format!("?with_member={wm}"));
        }
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/thread-members`: list thread members.
    pub async fn list_thread_members(
        &self,
        channel_id: Snowflake,
        query: Option<&ListThreadMembers>,
    ) -> Result<Vec<ThreadMember>, Error> {
        let mut qs = Vec::new();
        if let Some(q) = query {
            if let Some(wm) = q.with_member {
                qs.push(format!("with_member={wm}"));
            }
            if let Some(after) = q.after {
                qs.push(format!("after={after}"));
            }
            if let Some(limit) = q.limit {
                if limit == 0 || limit > 100 {
                    return Err(ChannelError::BadThreadMemberLimit { value: limit }.into());
                }
                qs.push(format!("limit={limit}"));
            }
        }
        let path = format!(
            "/channels/{channel_id}/thread-members{}",
            if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) }
        );
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/threads`: start a thread in a forum or media channel.
    pub async fn start_forum_thread(
        &self,
        channel_id: Snowflake,
        body: &CreateForumThread,
        reason: Option<&str>,
    ) -> Result<Channel, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_with_headers("POST", &format!("/channels/{channel_id}/threads"), Some(b), headers)
            .await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/threads`: start a thread in a forum or media channel with file uploads.
    pub async fn start_forum_thread_with_files(
        &self,
        channel_id: Snowflake,
        body: &CreateForumThread,
        files: Vec<UploadFile>,
        reason: Option<&str>,
    ) -> Result<Channel, Error> {
        validate_files(&files)?;
        check_thread_name(&body.name)?;
        if let Some(d) = body.auto_archive_duration {
            check_archive_duration(d)?;
        }
        if let Some(Some(s)) = body.rate_limit_per_user {
            check_slowmode(s)?;
        }
        if body.applied_tags.len() > 5 {
            return Err(ChannelError::TooManyAppliedTags { count: body.applied_tags.len() }.into());
        }

        let mut val = serde_json::to_value(body).map_err(|e| Error::Decode(e.to_string()))?;
        if let Some(msg) = val.get_mut("message").and_then(|m| m.as_object_mut()) {
            if !msg.contains_key("attachments") && !files.is_empty() {
                let atts: Vec<serde_json::Value> = files.iter().enumerate().map(|(i, f)| {
                    let mut obj = serde_json::json!({
                        "id": i,
                        "filename": f.filename,
                    });
                    if let Some(desc) = &f.description {
                        obj["description"] = serde_json::Value::String(desc.clone());
                    }
                    obj
                }).collect();
                msg.insert("attachments".to_string(), serde_json::Value::Array(atts));
            }
        }
        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        self.request_multipart_with_headers("POST", &format!("/channels/{channel_id}/threads"), multipart, headers)
            .await?.expect_success()?.json()
    }

    /// `GET /channels/{id}/polls/{message_id}/answers/{answer_id}` – get voters for a poll answer.
    pub async fn get_answer_voters(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        answer_id: u32,
        after: Option<Snowflake>,
        limit: Option<u8>,
    ) -> Result<PollAnswerVoters, Error> {
        let mut qs = Vec::new();
        if let Some(after) = after {
            qs.push(format!("after={after}"));
        }
        if let Some(lim) = limit {
            if lim == 0 || lim > 100 {
                return Err(ChannelError::BadPollVoterLimit { value: lim }.into());
            }
            qs.push(format!("limit={lim}"));
        }
        let q = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        let path = format!("/channels/{channel_id}/polls/{message_id}/answers/{answer_id}{q}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `POST /channels/{id}/polls/{message_id}/expire` – immediately expire a poll.
    pub async fn poll_expire(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Message, Error> {
        let path = format!("/channels/{channel_id}/polls/{message_id}/expire");
        self.request("POST", &path, None).await?.expect_success()?.json()
    }

    /// Alias for [`poll_expire`](Self::poll_expire).
    pub async fn expire_poll(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Message, Error> {
        self.poll_expire(channel_id, message_id).await
    }

    /// `DELETE /channels/{id}/recipients/{user_id}` – remove a recipient from a Group DM.
    pub async fn delete_group_dm_user(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/recipients/{user_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// Alias for [`delete_group_dm_user`](Self::delete_group_dm_user).
    pub async fn remove_group_dm_recipient(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), Error> {
        self.delete_group_dm_user(channel_id, user_id).await
    }

    /// `PUT /channels/{id}/recipients/{user_id}` – add a recipient to a Group DM.
    pub async fn add_group_dm_user(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
        body: &AddGroupDmUser,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/channels/{channel_id}/recipients/{user_id}");
        self.request("PUT", &path, Some(b)).await?.expect_success()?;
        Ok(())
    }

    /// `POST /channels/{id}/send-soundboard-sound` – play a soundboard sound in a voice channel.
    pub async fn send_soundboard_sound(
        &self,
        channel_id: Snowflake,
        body: &SendSoundboardSound,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/channels/{channel_id}/send-soundboard-sound");
        self.request("POST", &path, Some(b)).await?.expect_success()?;
        Ok(())
    }

    /// `GET /channels/{id}/threads/search` – search threads in a channel.
    pub async fn thread_search(
        &self,
        channel_id: Snowflake,
        query: Option<&ThreadSearchQuery>,
    ) -> Result<ThreadSearchResponse, Error> {
        let mut qs = Vec::new();
        if let Some(q) = query {
            q.validate()?;
            if let Some(name) = &q.name {
                qs.push(format!("name={}", crate::message::percent_encode(name)));
            }
            if let Some(slop) = q.slop {
                qs.push(format!("slop={slop}"));
            }
            if let Some(min_id) = q.min_id {
                qs.push(format!("min_id={min_id}"));
            }
            if let Some(max_id) = q.max_id {
                qs.push(format!("max_id={max_id}"));
            }
            if !q.tags.is_empty() {
                let tags_str = q.tags.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(",");
                qs.push(format!("tag={tags_str}"));
            }
            if let Some(ts) = q.tag_setting {
                qs.push(format!("tag_setting={}", ts.as_str()));
            }
            if let Some(archived) = q.archived {
                qs.push(format!("archived={archived}"));
            }
            if let Some(sort_by) = q.sort_by {
                qs.push(format!("sort_by={}", sort_by.as_str()));
            }
            if let Some(sort_order) = q.sort_order {
                qs.push(format!("sort_order={}", sort_order.as_str()));
            }
            if let Some(lim) = q.limit {
                qs.push(format!("limit={lim}"));
            }
            if let Some(off) = q.offset {
                qs.push(format!("offset={off}"));
            }
        }
        let q_str = if qs.is_empty() { String::new() } else { format!("?{}", qs.join("&")) };
        let path = format!("/channels/{channel_id}/threads/search{q_str}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// Alias for [`thread_search`](Self::thread_search).
    pub async fn search_threads(
        &self,
        channel_id: Snowflake,
        query: Option<&ThreadSearchQuery>,
    ) -> Result<ThreadSearchResponse, Error> {
        self.thread_search(channel_id, query).await
    }

    /// `GET /channels/{id}/webhooks` – list webhooks in a channel.
    pub async fn list_channel_webhooks(
        &self,
        channel_id: Snowflake,
    ) -> Result<Vec<crate::webhook::Webhook>, Error> {
        let path = format!("/channels/{channel_id}/webhooks");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// Alias for [`list_channel_webhooks`](Self::list_channel_webhooks).
    pub async fn get_channel_webhooks(
        &self,
        channel_id: Snowflake,
    ) -> Result<Vec<crate::webhook::Webhook>, Error> {
        self.list_channel_webhooks(channel_id).await
    }

    /// `POST /channels/{id}/webhooks` – create a webhook in a channel.
    pub async fn create_webhook(
        &self,
        channel_id: Snowflake,
        body: &CreateWebhook,
        reason: Option<&str>,
    ) -> Result<crate::webhook::Webhook, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason.map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())]).unwrap_or_default();
        let path = format!("/channels/{channel_id}/webhooks");
        self.request_with_headers("POST", &path, Some(b), headers).await?.expect_success()?.json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};
    use serde_json::json;

    fn sf(n: u64) -> Snowflake { Snowflake::new(n) }

    fn chan_json(kind: u8) -> String {
        json!({ "id": "1", "type": kind }).to_string()
    }

    #[test]
    fn channel_types_round_trip() {
        for (v, t) in [
            (0, ChannelType::GuildText), (1, ChannelType::Dm),
            (11, ChannelType::PublicThread), (15, ChannelType::GuildForum),
        ] {
            assert_eq!(ChannelType::from_u8(v), Some(t));
            assert_eq!(t.as_u8(), v);
        }
        assert!(ChannelType::PublicThread.is_thread());
        assert!(ChannelType::GuildForum.is_forum_like());
        assert!(!ChannelType::GuildText.is_thread());
    }

    #[test]
    fn obfuscated_flag_detection() {
        let mut c: Channel = serde_json::from_str(&json!({"id":"1","type":0,"flags": flags::CHANNEL_OBFUSCATED}).to_string()).unwrap();
        assert!(c.is_obfuscated());
        c.flags = Some(0);
        assert!(!c.is_obfuscated());
        c.flags = None;
        assert!(!c.is_obfuscated());
    }

    #[test]
    fn permission_overwrite_parses_decimal_strings_from_discord() {
        let raw = json!({ "id": "123", "type": 0, "allow": "1024", "deny": "0" }).to_string();
        let ow: PermissionOverwrite = serde_json::from_str(&raw).unwrap();
        assert_eq!(ow.allow, Permissions::from_bits_retain(1024));
        assert_eq!(ow.deny, Permissions::NONE);
        assert_eq!(serde_json::to_value(&ow).unwrap()["allow"], "1024");
    }

    #[test]
    fn channel_error_validation() {
        assert!(matches!(check_thread_name(""), Err(ChannelError::BadThreadName { len: 0 })));
        assert!(matches!(check_thread_name(&"x".repeat(101)), Err(ChannelError::BadThreadName { len: 101 })));
        assert!(check_thread_name("ok").is_ok());
        assert!(matches!(check_slowmode(21601), Err(ChannelError::BadSlowmode { .. })));
        assert!(check_slowmode(21600).is_ok());
        assert!(matches!(check_archive_duration(30), Err(ChannelError::BadArchiveDuration { value: 30 })));
        assert!(check_archive_duration(1440).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn get_channel_uses_the_right_route() {
        let c = client(vec![resp(200, &chan_json(0))]);
        let ch = c.get_channel(sf(1)).await.unwrap();
        assert_eq!((ch.id, ch.kind), (sf(1), ChannelType::GuildText));
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!((seen[0].method.as_str(), seen[0].url.as_str()), ("GET", "https://discord.com/api/v10/channels/1"));
    }

    #[tokio::test(start_paused = true)]
    async fn delete_channel_passes_audit_reason() {
        let c = client(vec![resp(200, &chan_json(0))]);
        c.delete_channel(sf(1), Some("cleanup")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "cleanup"));
    }

    #[tokio::test(start_paused = true)]
    async fn thread_creation_validates_name_and_archive_duration() {
        let c = client(vec![resp(201, &chan_json(11))]);
        let ok = CreateThread { name: "ok".into(), auto_archive_duration: Some(1440), kind: None, invitable: None, rate_limit_per_user: None };
        assert!(c.start_thread(sf(1), &ok, None).await.is_ok());

        let c2 = client(vec![]);
        let bad = CreateThread { name: "".into(), ..ok.clone() };
        assert!(matches!(c2.start_thread(sf(1), &bad, None).await, Err(Error::Transport(_))));

        let c3 = client(vec![]);
        let bad_dur = CreateThread { auto_archive_duration: Some(999), ..ok };
        assert!(matches!(c3.start_thread(sf(1), &bad_dur, None).await, Err(Error::Transport(_))));
    }

    #[tokio::test(start_paused = true)]
    async fn channel_permission_endpoints_use_the_right_routes() {
        let perm_resp = resp(200, r#"{"permissions":"1024"}"#);
        let c = client(vec![perm_resp.clone(), perm_resp, resp(204, ""), resp(204, ""), resp(204, "")]);

        c.get_channel_member_permissions(sf(1), sf(2)).await.unwrap();
        c.get_channel_role_permissions(sf(1), sf(3)).await.unwrap();
        c.edit_channel_permissions(sf(1), sf(4), Some(Permissions::VIEW_CHANNEL), None, 0, None).await.unwrap();
        c.delete_channel_permission(sf(1), sf(4), None).await.unwrap();
        c.set_voice_channel_status(sf(1), Some("🎵 now playing"), None).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/permissions/member/2");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/permissions/role/3");
        assert_eq!((seen[2].method.as_str(), seen[2].url.as_str()), ("PUT", "https://discord.com/api/v10/channels/1/permissions/4"));
        assert_eq!((seen[3].method.as_str(), seen[3].url.as_str()), ("DELETE", "https://discord.com/api/v10/channels/1/permissions/4"));
        assert_eq!((seen[4].method.as_str(), seen[4].url.as_str()), ("PUT", "https://discord.com/api/v10/channels/1/voice-status"));
    }

    #[tokio::test(start_paused = true)]
    async fn thread_member_routes_match_the_docs() {
        let c = client(vec![resp(204,""), resp(204,""), resp(204,""), resp(204,"")]);
        c.join_thread(sf(1)).await.unwrap();
        c.leave_thread(sf(1)).await.unwrap();
        c.add_thread_member(sf(1), sf(9)).await.unwrap();
        c.remove_thread_member(sf(1), sf(9)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/thread-members/@me");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/channels/1/thread-members/9");
    }

    fn recent_sf(offset_ms: u64) -> Snowflake {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let id = (now_ms - offset_ms - rift_core::snowflake::DISCORD_EPOCH_MS) << 22;
        Snowflake::new(id)
    }

    #[tokio::test(start_paused = true)]
    async fn bulk_delete_validation_and_route() {
        let c = client(vec![resp(204, "")]);
        let old_msg = sf(1); // 2015 Discord epoch, > 2 weeks old
        let r1 = recent_sf(10_000);
        let r2 = recent_sf(20_000);

        // Less than 2 messages
        assert!(matches!(
            c.bulk_delete_messages(sf(1), &[r1], None).await,
            Err(Error::Transport(_))
        ));

        // Message older than 2 weeks
        assert!(matches!(
            c.bulk_delete_messages(sf(1), &[r1, old_msg], None).await,
            Err(Error::Transport(_))
        ));

        // Valid messages
        c.bulk_delete_messages(sf(1), &[r1, r2], Some("cleanup")).await.unwrap();
        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/messages/bulk-delete");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "cleanup"));
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn get_and_list_thread_members_routes() {
        let member_json = json!({
            "id": "1",
            "user_id": "2",
            "join_timestamp": "2026-01-01T00:00:00Z",
            "flags": 0
        }).to_string();
        let list_json = json!([{
            "id": "1",
            "user_id": "2",
            "join_timestamp": "2026-01-01T00:00:00Z",
            "flags": 0
        }]).to_string();

        let c = client(vec![resp(200, &member_json), resp(200, &list_json)]);
        let m = c.get_thread_member(sf(1), sf(2), Some(true)).await.unwrap();
        assert_eq!(m.user_id, Some(sf(2)));

        let q = ListThreadMembers::new().with_member(true).after(sf(10)).limit(50);
        let list = c.list_thread_members(sf(1), Some(&q)).await.unwrap();
        assert_eq!(list.len(), 1);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/thread-members/2?with_member=true");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/thread-members?with_member=true&after=10&limit=50");
    }

    #[tokio::test(start_paused = true)]
    async fn start_forum_thread_and_with_files_routes() {
        let chan_json = json!({ "id": "1", "type": 11, "name": "test-thread" }).to_string();
        let c = client(vec![resp(201, &chan_json), resp(201, &chan_json)]);

        let msg = CreateMessage::new().content("initial post");
        let forum = CreateForumThread::new("test-thread", msg)
            .auto_archive_duration(1440)
            .tag(sf(99));

        let ch = c.start_forum_thread(sf(1), &forum, Some("new forum thread")).await.unwrap();
        assert_eq!(ch.name.as_deref(), Some("test-thread"));

        let file = UploadFile::new("file.txt", b"hello world".to_vec()).content_type("text/plain");
        let ch2 = c.start_forum_thread_with_files(sf(1), &forum, vec![file], None).await.unwrap();
        assert_eq!(ch2.name.as_deref(), Some("test-thread"));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/threads");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "new forum thread"));

        // Second request was multipart
        assert_eq!(seen[1].method, "POST");
        assert!(seen[1].headers.iter().any(|(k, v)| k == "Content-Type" && v.starts_with("multipart/form-data; boundary=")));
        let body_str = String::from_utf8_lossy(seen[1].body.as_ref().unwrap());
        assert!(body_str.contains("Content-Disposition: form-data; name=\"payload_json\""));
        assert!(body_str.contains("Content-Disposition: form-data; name=\"files[0]\"; filename=\"file.txt\""));
        assert!(body_str.contains("hello world"));
    }

    #[tokio::test(start_paused = true)]
    async fn list_pins_routes_and_limits() {
        let resp_json = json!({
            "items": [{
                "pinned_at": "2026-01-01T00:00:00Z",
                "message": { "id": "2", "channel_id": "1", "content": "pinned!" }
            }],
            "has_more": false
        }).to_string();

        let c = client(vec![resp(200, &resp_json)]);

        // Limit validation
        assert!(matches!(c.list_pins(sf(1), None, Some(0)).await, Err(Error::Transport(_))));
        assert!(matches!(c.list_pins(sf(1), None, Some(51)).await, Err(Error::Transport(_))));

        let res = c.list_pins(sf(1), Some("2026-01-01T00:00:00Z"), Some(25)).await.unwrap();
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].pinned_at, "2026-01-01T00:00:00Z");
        assert_eq!(res.items[0].message.content, "pinned!");
        assert!(!res.has_more);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/channels/1/messages/pins?before=2026-01-01T00:00:00Z&limit=25"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn create_and_delete_pin_routes() {
        let c = client(vec![resp(204, ""), resp(204, "")]);
        c.create_pin(sf(1), sf(2), Some("important")).await.unwrap();
        c.delete_pin(sf(1), sf(2), Some("cleanup")).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/messages/pins/2");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "important"));

        assert_eq!(seen[1].method, "DELETE");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/messages/pins/2");
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "cleanup"));
    }

    #[tokio::test(start_paused = true)]
    async fn delete_my_message_reaction_routes() {
        let c = client(vec![resp(204, ""), resp(204, "")]);
        c.delete_my_message_reaction(sf(1), sf(2), "🔥").await.unwrap();
        c.delete_own_reaction(sf(1), sf(2), "custom:12345").await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "DELETE");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/channels/1/messages/2/reactions/%F0%9F%94%A5/@me"
        );

        assert_eq!(seen[1].method, "DELETE");
        assert_eq!(
            seen[1].url,
            "https://discord.com/api/v10/channels/1/messages/2/reactions/custom%3A12345/@me"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn poll_voters_and_expire_routes() {
        let voters_json = json!({
            "users": [{
                "id": "10",
                "username": "voter1"
            }]
        }).to_string();
        let msg_json = json!({
            "id": "2",
            "channel_id": "1",
            "content": "poll message"
        }).to_string();

        let c = client(vec![resp(200, &voters_json), resp(200, &msg_json)]);

        // Limit validation
        assert!(matches!(c.get_answer_voters(sf(1), sf(2), 1, None, Some(0)).await, Err(Error::Transport(_))));
        assert!(matches!(c.get_answer_voters(sf(1), sf(2), 1, None, Some(101)).await, Err(Error::Transport(_))));

        let voters = c.get_answer_voters(sf(1), sf(2), 1, Some(sf(100)), Some(50)).await.unwrap();
        assert_eq!(voters.users.len(), 1);
        assert_eq!(voters.users[0].id, sf(10));

        let msg = c.poll_expire(sf(1), sf(2)).await.unwrap();
        assert_eq!(msg.id, sf(2));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/channels/1/polls/2/answers/1?after=100&limit=50"
        );

        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/polls/2/expire");
    }

    #[tokio::test(start_paused = true)]
    async fn group_dm_recipient_routes() {
        let c = client(vec![resp(204, ""), resp(204, "")]);
        let add = AddGroupDmUser::new("secret_token").nick("Buddy");
        c.add_group_dm_user(sf(1), sf(2), &add).await.unwrap();
        c.delete_group_dm_user(sf(1), sf(2)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/recipients/2");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["access_token"], "secret_token");
        assert_eq!(body["nick"], "Buddy");

        assert_eq!(seen[1].method, "DELETE");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/recipients/2");
    }

    #[tokio::test(start_paused = true)]
    async fn soundboard_sound_route() {
        let c = client(vec![resp(204, "")]);
        let sound = SendSoundboardSound::new(sf(999)).source_guild_id(sf(888));
        c.send_soundboard_sound(sf(1), &sound).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/send-soundboard-sound");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["sound_id"], "999");
        assert_eq!(body["source_guild_id"], "888");
    }

    #[tokio::test(start_paused = true)]
    async fn thread_search_routes_and_validation() {
        let search_json = json!({
            "threads": [{ "id": "10", "type": 11, "name": "found-thread" }],
            "members": [{ "id": "10", "user_id": "1", "join_timestamp": "2026-01-01T00:00:00Z", "flags": 0 }],
            "has_more": false,
            "total_results": 1
        }).to_string();

        let c = client(vec![resp(200, &search_json)]);

        // Validation tests
        let bad_name = ThreadSearchQuery::new().name("a".repeat(101));
        assert!(matches!(bad_name.validate(), Err(ChannelError::BadThreadSearchName { len: 101 })));

        let bad_slop = ThreadSearchQuery::new().slop(101);
        assert!(matches!(bad_slop.validate(), Err(ChannelError::BadThreadSearchSlop { value: 101 })));

        let mut bad_tags = ThreadSearchQuery::new();
        for i in 0..21 { bad_tags = bad_tags.tag(sf(i)); }
        assert!(matches!(bad_tags.validate(), Err(ChannelError::TooManyThreadSearchTags { count: 21 })));

        let bad_limit = ThreadSearchQuery::new().limit(26);
        assert!(matches!(bad_limit.validate(), Err(ChannelError::BadThreadSearchLimit { value: 26 })));

        let bad_offset = ThreadSearchQuery::new().offset(9976);
        assert!(matches!(bad_offset.validate(), Err(ChannelError::BadThreadSearchOffset { value: 9976 })));

        // Valid search
        let query = ThreadSearchQuery::new()
            .name("test thread")
            .slop(10)
            .min_id(sf(1))
            .max_id(sf(2))
            .tag(sf(3))
            .tag(sf(4))
            .tag_setting(ThreadSearchTagSetting::MatchAll)
            .archived(false)
            .sort_by(ThreadSortingMode::Relevance)
            .sort_order(SortingOrder::Desc)
            .limit(15)
            .offset(30);

        let res = c.thread_search(sf(1), Some(&query)).await.unwrap();
        assert_eq!(res.threads.len(), 1);
        assert_eq!(res.threads[0].name.as_deref(), Some("found-thread"));
        assert_eq!(res.total_results, 1);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/channels/1/threads/search?name=test%20thread&slop=10&min_id=1&max_id=2&tag=3,4&tag_setting=match_all&archived=false&sort_by=relevance&sort_order=desc&limit=15&offset=30"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn channel_webhooks_routes_and_validation() {
        let webhook_json = json!({
            "id": "100",
            "type": 1,
            "channel_id": "1",
            "name": "Captain Hook"
        }).to_string();
        let list_json = json!([{
            "id": "100",
            "type": 1,
            "channel_id": "1",
            "name": "Captain Hook"
        }]).to_string();

        let c = client(vec![resp(200, &list_json), resp(200, &webhook_json)]);

        // Webhook name validation
        assert!(matches!(CreateWebhook::new("").validate(), Err(ChannelError::BadWebhookName { len: 0 })));
        assert!(matches!(CreateWebhook::new("a".repeat(81)).validate(), Err(ChannelError::BadWebhookName { len: 81 })));
        assert!(matches!(CreateWebhook::new("Hey Clyde!").validate(), Err(ChannelError::ProhibitedWebhookName { .. })));
        assert!(matches!(CreateWebhook::new("Official Discord Bot").validate(), Err(ChannelError::ProhibitedWebhookName { .. })));

        // List webhooks
        let list = c.list_channel_webhooks(sf(1)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name.as_deref(), Some("Captain Hook"));

        // Create webhook
        let create = CreateWebhook::new("Captain Hook").avatar("data:image/png;base64,abc");
        let hook = c.create_webhook(sf(1), &create, Some("audit hook")).await.unwrap();
        assert_eq!(hook.id, sf(100));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/webhooks");

        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/webhooks");
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit hook"));
        let body: serde_json::Value = serde_json::from_slice(seen[1].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["name"], "Captain Hook");
        assert_eq!(body["avatar"], "data:image/png;base64,abc");
    }
}
