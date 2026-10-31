//! Message models, embeds, reactions, attachments, and polls.

use crate::channel::Channel;
use crate::guild::GuildMember;
use crate::serde_util::{
    de_opt_snowflake, de_opt_vec_snowflake, de_snowflake, ser_opt_snowflake, ser_opt_vec_snowflake,
    ser_snowflake,
};
use crate::user::User;
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// An attachment in a message.
///
/// See <https://discord.com/developers/docs/resources/message#attachment-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Name of file attached.
    pub filename: String,

    /// The title of the file.
    #[serde(default)]
    pub title: Option<String>,

    /// Description for the file (max 1024 characters).
    #[serde(default)]
    pub description: Option<String>,

    /// The attachment's media type.
    #[serde(default)]
    pub content_type: Option<String>,

    /// Size of file in bytes.
    #[serde(default)]
    pub size: u64,

    /// Source url of file.
    pub url: String,

    /// A proxied url of file.
    pub proxy_url: String,

    /// Height of file (if image).
    #[serde(default)]
    pub height: Option<u32>,

    /// Width of file (if image).
    #[serde(default)]
    pub width: Option<u32>,

    /// Whether this attachment is ephemeral.
    #[serde(default)]
    pub ephemeral: Option<bool>,

    /// The duration of the audio file (currently for voice messages).
    #[serde(default)]
    pub duration_secs: Option<f64>,

    /// Base64 encoded bytearray representing a sampled waveform.
    #[serde(default)]
    pub waveform: Option<String>,

    /// Attachment flags.
    #[serde(default)]
    pub flags: Option<u32>,
}

/// An author of an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedAuthor {
    /// Name of author.
    pub name: String,

    /// Url of author.
    #[serde(default)]
    pub url: Option<String>,

    /// Url of author icon.
    #[serde(default)]
    pub icon_url: Option<String>,

    /// A proxied url of author icon.
    #[serde(default)]
    pub proxy_icon_url: Option<String>,
}

/// A footer of an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedFooter {
    /// Footer text.
    pub text: String,

    /// Url of footer icon.
    #[serde(default)]
    pub icon_url: Option<String>,

    /// A proxied url of footer icon.
    #[serde(default)]
    pub proxy_icon_url: Option<String>,
}

/// An image or thumbnail in an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedImage {
    /// Source url of image.
    pub url: String,

    /// A proxied url of the image.
    #[serde(default)]
    pub proxy_url: Option<String>,

    /// Height of image.
    #[serde(default)]
    pub height: Option<u32>,

    /// Width of image.
    #[serde(default)]
    pub width: Option<u32>,
}

/// Type alias for an embed thumbnail (same layout as [`EmbedImage`]).
pub type EmbedThumbnail = EmbedImage;

/// Sent with Rich Presence-related chat embeds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageActivity {
    /// Type of message activity.
    #[serde(rename = "type")]
    pub kind: u8,

    /// Party id from a Rich Presence event.
    #[serde(default)]
    pub party_id: Option<String>,
}

/// A field in an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedField {
    /// Name of the field.
    pub name: String,

    /// Value of the field.
    pub value: String,

    /// Whether or not this field should display inline.
    #[serde(default)]
    pub inline: Option<bool>,
}

/// Provider of an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedProvider {
    /// Name of provider.
    #[serde(default)]
    pub name: Option<String>,

    /// Url of provider.
    #[serde(default)]
    pub url: Option<String>,
}

/// Video in an embed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedVideo {
    /// Source url of video.
    #[serde(default)]
    pub url: Option<String>,

    /// A proxied url of the video.
    #[serde(default)]
    pub proxy_url: Option<String>,

    /// Height of video.
    #[serde(default)]
    pub height: Option<u32>,

    /// Width of video.
    #[serde(default)]
    pub width: Option<u32>,
}

/// A Discord embed object.
///
/// See <https://discord.com/developers/docs/resources/message#embed-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Embed {
    /// Title of embed.
    #[serde(default)]
    pub title: Option<String>,

    /// Type of embed (always "rich" for webhook embeds).
    #[serde(rename = "type", default)]
    pub kind: Option<String>,

    /// Description of embed.
    #[serde(default)]
    pub description: Option<String>,

    /// Url of embed.
    #[serde(default)]
    pub url: Option<String>,

    /// Timestamp of embed content.
    #[serde(default)]
    pub timestamp: Option<String>,

    /// Color code of the embed.
    #[serde(default)]
    pub color: Option<u32>,

    /// Footer information.
    #[serde(default)]
    pub footer: Option<EmbedFooter>,

    /// Image information.
    #[serde(default)]
    pub image: Option<EmbedImage>,

    /// Thumbnail information.
    #[serde(default)]
    pub thumbnail: Option<EmbedImage>,

    /// Video information.
    #[serde(default)]
    pub video: Option<EmbedVideo>,

    /// Provider information.
    #[serde(default)]
    pub provider: Option<EmbedProvider>,

    /// Author information.
    #[serde(default)]
    pub author: Option<EmbedAuthor>,

    /// Fields information.
    #[serde(default)]
    pub fields: Option<Vec<EmbedField>>,
}

/// Emoji used in reactions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionEmoji {
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

    /// Whether this emoji is animated.
    #[serde(default)]
    pub animated: Option<bool>,
}

/// Reaction count details.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionCountDetails {
    /// Count of super reactions.
    #[serde(default)]
    pub burst: u32,

    /// Count of normal reactions.
    #[serde(default)]
    pub normal: u32,
}

/// A reaction on a message.
///
/// See <https://discord.com/developers/docs/resources/message#reaction-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    /// Total number of times this emoji has been used to react.
    pub count: u32,

    /// Reaction count details.
    #[serde(default)]
    pub count_details: Option<ReactionCountDetails>,

    /// Whether the current user reacted using this emoji.
    #[serde(default)]
    pub me: bool,

    /// Whether the current user super-reacted using this emoji.
    #[serde(default)]
    pub me_burst: Option<bool>,

    /// Emoji information.
    pub emoji: ReactionEmoji,

    /// HEX colors used for super reaction.
    #[serde(default)]
    pub burst_colors: Option<Vec<String>>,
}

/// Reference data sent with crossposted messages and replies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageReference {
    /// Type of reference.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,

    /// Id of the originating message.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub message_id: Option<Snowflake>,

    /// Id of the originating message's channel.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// Id of the originating message's guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// When sending, whether to error if the referenced message doesn't exist instead of sending as a normal message.
    #[serde(default)]
    pub fail_if_not_exists: Option<bool>,
}

/// Partial sticker object in messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickerItem {
    /// ID of the sticker.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Name of the sticker.
    pub name: String,

    /// Type of sticker format.
    pub format_type: u8,
}

/// Mentioned channel in a message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMention {
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// ID of the guild that contains the channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,

    /// The type of channel.
    #[serde(rename = "type")]
    pub kind: u8,

    /// The name of the channel.
    pub name: String,
}

/// Interaction info in a message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MessageInteraction {
    /// ID of the interaction.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Type of interaction.
    #[serde(rename = "type")]
    pub kind: u8,

    /// Name of the application command.
    pub name: String,

    /// User who invoked the interaction.
    pub user: User,

    /// Member who invoked the interaction in the guild.
    #[serde(default)]
    pub member: Option<GuildMember>,
}

/// Media data for a poll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollMedia {
    /// Text of the field.
    #[serde(default)]
    pub text: Option<String>,

    /// The emoji of the field.
    #[serde(default)]
    pub emoji: Option<ReactionEmoji>,
}

/// Answer in a poll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollAnswer {
    /// The ID of the answer.
    pub answer_id: u32,

    /// The data of the answer.
    pub poll_media: PollMedia,
}

/// Count of votes for an answer in a poll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollAnswerCount {
    /// The answer ID.
    pub id: u32,

    /// The number of votes for this answer.
    pub count: u32,

    /// Whether the current user voted for this answer.
    #[serde(default)]
    pub me_voted: bool,
}

/// Results of a poll.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollResults {
    /// Whether the votes have been precisely counted.
    #[serde(default)]
    pub is_finalized: bool,

    /// The counts for each answer.
    #[serde(default)]
    pub answer_counts: Option<Vec<PollAnswerCount>>,
}

/// A poll in a message.
///
/// See <https://discord.com/developers/docs/resources/poll#poll-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Poll {
    /// The question of the poll.
    pub question: PollMedia,

    /// Each of the answers available in the poll.
    pub answers: Vec<PollAnswer>,

    /// The time when the poll ends.
    #[serde(default)]
    pub expiry: Option<String>,

    /// Whether a user can select more than one answer.
    #[serde(default)]
    pub allow_multiselect: bool,

    /// The layout type of the poll.
    #[serde(default)]
    pub layout_type: u8,

    /// The results of the poll.
    #[serde(default)]
    pub results: Option<PollResults>,
}

/// A Discord message object.
///
/// See <https://discord.com/developers/docs/resources/message#message-object>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Id of the message.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Id of the channel the message was sent in.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,

    /// The author of this message (not guaranteed to be a user, e.g. for webhooks).
    #[serde(default)]
    pub author: Option<User>,

    /// Contents of the message.
    #[serde(default)]
    pub content: String,

    /// When this message was sent.
    #[serde(default)]
    pub timestamp: String,

    /// When this message was edited (or null if never).
    #[serde(default)]
    pub edited_timestamp: Option<String>,

    /// Whether this was a TTS message.
    #[serde(default)]
    pub tts: bool,

    /// Whether this message mentions everyone.
    #[serde(default)]
    pub mention_everyone: bool,

    /// Users specifically mentioned in the message.
    #[serde(default)]
    pub mentions: Option<Vec<User>>,

    /// Roles specifically mentioned in this message.
    #[serde(
        default,
        deserialize_with = "de_opt_vec_snowflake",
        serialize_with = "ser_opt_vec_snowflake"
    )]
    pub mention_roles: Option<Vec<Snowflake>>,

    /// Channels specifically mentioned in this message.
    #[serde(default)]
    pub mention_channels: Option<Vec<ChannelMention>>,

    /// Any attached files.
    #[serde(default)]
    pub attachments: Option<Vec<Attachment>>,

    /// Any embedded content.
    #[serde(default)]
    pub embeds: Option<Vec<Embed>>,

    /// Reactions to the message.
    #[serde(default)]
    pub reactions: Option<Vec<Reaction>>,

    /// Used for validating a message was sent.
    #[serde(default)]
    pub nonce: Option<serde_json::Value>,

    /// Whether this message is pinned.
    #[serde(default)]
    pub pinned: bool,

    /// If the message is generated by a webhook, this is the webhook's id.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub webhook_id: Option<Snowflake>,

    /// Type of message.
    #[serde(rename = "type", default)]
    pub kind: u8,

    /// Sent with Rich Presence-related chat embeds.
    #[serde(default)]
    pub activity: Option<MessageActivity>,

    /// Sent with Rich Presence-related chat embeds.
    #[serde(default)]
    pub application: Option<serde_json::Value>,

    /// If the message is an Interaction or application-owned webhook, this is the id of the application.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub application_id: Option<Snowflake>,

    /// Message flags combined as a bitfield.
    #[serde(default)]
    pub flags: Option<u64>,

    /// Data showing the source of a crosspost, channel follow add, pin, or reply message.
    #[serde(default)]
    pub message_reference: Option<MessageReference>,

    /// The message associated with the message_reference.
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,

    /// Sent if the message is sent as a result of an interaction.
    #[serde(default)]
    pub interaction_metadata: Option<serde_json::Value>,

    /// Deprecated interaction information.
    #[serde(default)]
    pub interaction: Option<MessageInteraction>,

    /// The thread that was started from this message, includes thread member object.
    #[serde(default)]
    pub thread: Option<Channel>,

    /// Sent if the message contains components like buttons, select menus, etc.
    #[serde(default)]
    pub components: Option<Vec<serde_json::Value>>,

    /// Sent if the message contains stickers.
    #[serde(default)]
    pub sticker_items: Option<Vec<StickerItem>>,

    /// A generally increasing integer (there may be gaps or duplicates) that represents the approximate position of the message in a thread.
    #[serde(default)]
    pub position: Option<u64>,

    /// Data of the role subscription purchase or renewal that prompted this `ROLE_SUBSCRIPTION_PURCHASE` message.
    #[serde(default)]
    pub role_subscription_data: Option<serde_json::Value>,

    /// The poll object.
    #[serde(default)]
    pub poll: Option<Poll>,

    // Additional fields sent over Gateway in `MESSAGE_CREATE` / `MESSAGE_UPDATE`:
    /// ID of the guild the message was sent in.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Member properties for this message's author.
    #[serde(default)]
    pub member: Option<GuildMember>,
}
