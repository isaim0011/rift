//! Typed message endpoints with local validation.
//!
//! Every limit here is from the Message Resource docs. Invalid messages are
//! rejected before any network call.

use crate::client::RestClient;
use crate::error::Error;
use crate::transport::Transport;
use rift_core::{MessageFlags, Snowflake};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

const MAX_CONTENT: usize = 2000;
const MAX_EMBEDS: usize = 10;
const MAX_EMBED_TOTAL: usize = 6000;
const MAX_EMBED_FIELDS: usize = 25;
const MAX_STICKERS: usize = 3;
const MAX_NONCE: usize = 25;
const MAX_MENTION_IDS: usize = 100;
/// Maximum number of file attachments per message (10).
pub const MAX_FILES: usize = 10;
/// Maximum combined file upload size for bots (25 MiB = 26,214,400 bytes).
/// Boosted servers allow up to 50/100 MiB; 25 MiB is the safe baseline limit.
pub const MAX_TOTAL_FILE_SIZE: usize = 25 * 1024 * 1024;
/// Maximum filename length (1024 characters).
pub const MAX_FILENAME_LEN: usize = 1024;
/// Maximum description / alt-text length (1024 characters).
pub const MAX_DESCRIPTION_LEN: usize = 1024;

// ---------- snowflake (de)serialization: Discord uses JSON strings ----------

pub(crate) fn ser_snowflake<S: Serializer>(v: &Snowflake, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

pub(crate) fn ser_opt_snowflake<S: Serializer>(v: &Option<Snowflake>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(x) => s.collect_str(x),
        None => s.serialize_none(),
    }
}

pub(crate) fn ser_snowflakes<S: Serializer>(v: &[Snowflake], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(v.iter().map(|x| x.to_string()))
}

pub(crate) fn ser_flags<S: Serializer>(v: &Option<MessageFlags>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(f) => s.serialize_u32(f.bits()),
        None => s.serialize_none(),
    }
}

pub(crate) fn de_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Snowflake, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Snowflake>().map_err(serde::de::Error::custom)
}

pub(crate) fn de_opt_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Snowflake>, D::Error> {
    let s = Option::<String>::deserialize(d)?;
    s.map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom)).transpose()
}

// ---------- validation ----------

/// Why a message was rejected locally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    /// Nothing to send (need content, embeds, stickers or components).
    Empty,
    /// `content` over 2000 characters.
    ContentTooLong {
        /// Actual length.
        len: usize,
    },
    /// More than 10 embeds.
    TooManyEmbeds {
        /// Actual count.
        count: usize,
    },
    /// More than 25 fields in one embed.
    TooManyEmbedFields {
        /// Actual count.
        count: usize,
    },
    /// An embed text field is over its limit.
    EmbedFieldTooLong {
        /// Which field.
        field: &'static str,
        /// Actual length.
        len: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// All embed text combined is over 6000 characters.
    EmbedsTooLarge {
        /// Combined length.
        total: usize,
    },
    /// More than 3 stickers.
    TooManyStickers {
        /// Actual count.
        count: usize,
    },
    /// `nonce` over 25 characters.
    NonceTooLong {
        /// Actual length.
        len: usize,
    },
    /// Flags other than `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS`,
    /// `IS_VOICE_MESSAGE`, `IS_COMPONENTS_V2` were set.
    FlagNotSettable {
        /// The offending bits.
        bits: u32,
    },
    /// `IS_COMPONENTS_V2` messages can only contain components.
    ComponentsV2Conflict {
        /// The field that must not be present.
        field: &'static str,
    },
    /// `allowed_mentions.parse` can't be combined with non-empty `roles`/`users`.
    MentionsParseConflict,
    /// More than 100 role or user ids in `allowed_mentions`.
    TooManyMentionIds {
        /// `"roles"` or `"users"`.
        kind: &'static str,
    },
    /// A forward needs `channel_id` in its message reference.
    ForwardNeedsChannel,
    /// More than 10 files in a single message.
    TooManyFiles {
        /// Actual file count.
        count: usize,
    },
    /// More than 10 attachments specified in the attachments array.
    TooManyAttachments {
        /// Actual attachment count.
        count: usize,
    },
    /// Total size of uploaded files exceeds the limit (25 MiB).
    FilesTooLarge {
        /// Combined byte size of all files.
        total: usize,
        /// Maximum allowed bytes (25 MiB = 26,214,400).
        max: usize,
    },
    /// Filename is empty or only whitespace.
    EmptyFilename,
    /// Filename exceeds the 1024 character limit.
    FilenameTooLong {
        /// Actual length.
        len: usize,
    },
    /// Filename contains invalid control characters.
    InvalidFilename {
        /// The invalid filename.
        name: String,
    },
    /// Attachment description (alt text) exceeds 1024 characters.
    AttachmentDescriptionTooLong {
        /// Actual length.
        len: usize,
    },
    /// Poll question is empty.
    PollQuestionEmpty,
    /// Poll question exceeds 300 characters.
    PollQuestionTooLong {
        /// Actual length.
        len: usize,
    },
    /// Poll answer count is not 1-10.
    PollBadAnswerCount {
        /// Actual count.
        count: usize,
    },
    /// Poll answer text exceeds 55 characters.
    PollAnswerTextTooLong {
        /// Actual length.
        len: usize,
    },
    /// Poll duration is not 1-768 hours.
    PollBadDuration {
        /// Actual hours.
        hours: u32,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "message needs content, embeds, stickers or components"),
            Self::ContentTooLong { len } => write!(f, "content is {len} characters (max {MAX_CONTENT})"),
            Self::TooManyEmbeds { count } => write!(f, "{count} embeds (max {MAX_EMBEDS})"),
            Self::TooManyEmbedFields { count } => write!(f, "{count} embed fields (max {MAX_EMBED_FIELDS})"),
            Self::EmbedFieldTooLong { field, len, max } => {
                write!(f, "embed {field} is {len} characters (max {max})")
            }
            Self::EmbedsTooLarge { total } => {
                write!(f, "embeds total {total} characters (max {MAX_EMBED_TOTAL})")
            }
            Self::TooManyStickers { count } => write!(f, "{count} stickers (max {MAX_STICKERS})"),
            Self::NonceTooLong { len } => write!(f, "nonce is {len} characters (max {MAX_NONCE})"),
            Self::FlagNotSettable { bits } => write!(f, "flag bits {bits:#x} can't be set when creating a message"),
            Self::ComponentsV2Conflict { field } => {
                write!(f, "messages with IS_COMPONENTS_V2 can only contain components, not {field}")
            }
            Self::MentionsParseConflict => {
                write!(f, "allowed_mentions.parse can't be combined with non-empty roles/users")
            }
            Self::TooManyMentionIds { kind } => write!(f, "more than {MAX_MENTION_IDS} {kind} in allowed_mentions"),
            Self::ForwardNeedsChannel => write!(f, "a forward needs a channel_id in its message reference"),
            Self::TooManyFiles { count } => write!(f, "{count} files (max {MAX_FILES})"),
            Self::TooManyAttachments { count } => write!(f, "{count} attachments (max {MAX_FILES})"),
            Self::FilesTooLarge { total, max } => write!(f, "total file size {total} bytes exceeds limit of {max} bytes"),
            Self::EmptyFilename => write!(f, "filename cannot be empty"),
            Self::FilenameTooLong { len } => write!(f, "filename is {len} chars (max {MAX_FILENAME_LEN})"),
            Self::InvalidFilename { name } => write!(f, "filename contains invalid characters: {name:?}"),
            Self::AttachmentDescriptionTooLong { len } => write!(f, "attachment description is {len} chars (max {MAX_DESCRIPTION_LEN})"),
            Self::PollQuestionEmpty => write!(f, "poll question cannot be empty"),
            Self::PollQuestionTooLong { len } => write!(f, "poll question is {len} characters (max 300)"),
            Self::PollBadAnswerCount { count } => write!(f, "poll has {count} answers (must be 1-10)"),
            Self::PollAnswerTextTooLong { len } => write!(f, "poll answer text is {len} characters (max 55)"),
            Self::PollBadDuration { hours } => write!(f, "poll duration {hours} hours must be between 1 and 768"),
        }
    }
}

/// Validate a filename locally according to Discord API limits.
pub(crate) fn validate_filename(name: &str) -> Result<(), ValidationError> {
    if name.trim().is_empty() {
        return Err(ValidationError::EmptyFilename);
    }
    let len = name.chars().count();
    if len > MAX_FILENAME_LEN {
        return Err(ValidationError::FilenameTooLong { len });
    }
    if name.chars().any(|c| c == '\0' || c == '\r' || c == '\n') {
        return Err(ValidationError::InvalidFilename { name: name.to_string() });
    }
    Ok(())
}

impl std::error::Error for ValidationError {}

// ---------- embeds ----------

/// Embed footer.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EmbedFooter {
    /// Footer text (max 2048).
    pub text: String,
    /// Footer icon URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// Embed image or thumbnail.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EmbedImage {
    /// Image URL (http(s) or `attachment://`).
    pub url: String,
}

/// Embed author.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EmbedAuthor {
    /// Author name (max 256).
    pub name: String,
    /// Author URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Author icon URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

/// One embed field.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EmbedField {
    /// Name (max 256).
    pub name: String,
    /// Value (max 1024).
    pub value: String,
    /// Show inline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<bool>,
}

/// A `rich` embed.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Embed {
    /// Title (max 256).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description (max 4096).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Title link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// ISO8601 timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Color as an integer (e.g. `0x5865F2`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    /// Footer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    /// Image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedImage>,
    /// Thumbnail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedImage>,
    /// Author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    /// Fields (max 25).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<EmbedField>,
}

fn check(field: &'static str, s: &str, max: usize, total: &mut usize) -> Result<(), ValidationError> {
    // Docs: limits ignore leading/trailing whitespace.
    let len = s.trim().chars().count();
    if len > max {
        return Err(ValidationError::EmbedFieldTooLong { field, len, max });
    }
    *total += len;
    Ok(())
}

impl Embed {
    /// Empty embed.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the title.
    pub fn title(mut self, v: impl Into<String>) -> Self {
        self.title = Some(v.into());
        self
    }
    /// Set the description.
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }
    /// Set the title URL.
    pub fn url(mut self, v: impl Into<String>) -> Self {
        self.url = Some(v.into());
        self
    }
    /// Set the color.
    pub fn color(mut self, v: u32) -> Self {
        self.color = Some(v);
        self
    }
    /// Set the footer text.
    pub fn footer(mut self, text: impl Into<String>) -> Self {
        self.footer = Some(EmbedFooter { text: text.into(), icon_url: None });
        self
    }
    /// Set the author name.
    pub fn author(mut self, name: impl Into<String>) -> Self {
        self.author = Some(EmbedAuthor { name: name.into(), url: None, icon_url: None });
        self
    }
    /// Set the image URL.
    pub fn image(mut self, url: impl Into<String>) -> Self {
        self.image = Some(EmbedImage { url: url.into() });
        self
    }
    /// Set the thumbnail URL.
    pub fn thumbnail(mut self, url: impl Into<String>) -> Self {
        self.thumbnail = Some(EmbedImage { url: url.into() });
        self
    }
    /// Add a field.
    pub fn field(mut self, name: impl Into<String>, value: impl Into<String>, inline: bool) -> Self {
        self.fields.push(EmbedField { name: name.into(), value: value.into(), inline: Some(inline) });
        self
    }

    /// Check per-field limits; returns this embed's contribution to the 6000 total.
    fn validate(&self) -> Result<usize, ValidationError> {
        let mut total = 0usize;
        if let Some(t) = &self.title {
            check("title", t, 256, &mut total)?;
        }
        if let Some(d) = &self.description {
            check("description", d, 4096, &mut total)?;
        }
        if let Some(f) = &self.footer {
            check("footer.text", &f.text, 2048, &mut total)?;
        }
        if let Some(a) = &self.author {
            check("author.name", &a.name, 256, &mut total)?;
        }
        if self.fields.len() > MAX_EMBED_FIELDS {
            return Err(ValidationError::TooManyEmbedFields { count: self.fields.len() });
        }
        for f in &self.fields {
            check("field.name", &f.name, 256, &mut total)?;
            check("field.value", &f.value, 1024, &mut total)?;
        }
        Ok(total)
    }
}

// ---------- allowed mentions & references ----------

/// Which mention kinds Discord should parse from the content.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MentionType {
    /// Role mentions.
    Roles,
    /// User mentions.
    Users,
    /// `@everyone` and `@here`.
    Everyone,
}

/// Controls who a message may notify. Regular messages default to parsing
/// every mention type, so user-generated text should usually be sent with
/// [`AllowedMentions::none`].
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct AllowedMentions {
    /// Mention kinds to parse. Mutually exclusive with non-empty `roles`/`users`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse: Option<Vec<MentionType>>,
    /// Specific role ids (max 100).
    #[serde(skip_serializing_if = "Vec::is_empty", serialize_with = "ser_snowflakes")]
    pub roles: Vec<Snowflake>,
    /// Specific user ids (max 100).
    #[serde(skip_serializing_if = "Vec::is_empty", serialize_with = "ser_snowflakes")]
    pub users: Vec<Snowflake>,
    /// For replies, whether to ping the replied-to author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replied_user: Option<bool>,
}

impl AllowedMentions {
    /// Mention nobody (`"parse": []`).
    pub fn none() -> Self {
        Self { parse: Some(Vec::new()), ..Self::default() }
    }

    /// Only user mentions (the default for interactions and webhooks).
    pub fn users_only() -> Self {
        Self { parse: Some(vec![MentionType::Users]), ..Self::default() }
    }

    fn validate(&self) -> Result<(), ValidationError> {
        if self.parse.is_some() && (!self.roles.is_empty() || !self.users.is_empty()) {
            return Err(ValidationError::MentionsParseConflict);
        }
        if self.roles.len() > MAX_MENTION_IDS {
            return Err(ValidationError::TooManyMentionIds { kind: "roles" });
        }
        if self.users.len() > MAX_MENTION_IDS {
            return Err(ValidationError::TooManyMentionIds { kind: "users" });
        }
        Ok(())
    }
}

/// Makes a message a reply or a forward.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct MessageReference {
    /// `0` = reply (the default when unset), `1` = forward.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    /// The message being replied to / forwarded.
    #[serde(serialize_with = "ser_snowflake")]
    pub message_id: Snowflake,
    /// Its channel (required for forwards).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// Its guild.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Error instead of sending a normal message if the target is gone (default true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_not_exists: Option<bool>,
}

impl MessageReference {
    /// Reply to `message_id` (only `message_id` is required for replies).
    pub fn reply(message_id: Snowflake) -> Self {
        Self { kind: None, message_id, channel_id: None, guild_id: None, fail_if_not_exists: None }
    }

    /// Forward `message_id` from `channel_id` (type, message and channel are required).
    pub fn forward(message_id: Snowflake, channel_id: Snowflake) -> Self {
        Self { kind: Some(1), message_id, channel_id: Some(channel_id), guild_id: None, fail_if_not_exists: None }
    }

    fn is_forward(&self) -> bool {
        self.kind == Some(1)
    }
}

/// The checks Create Message and interaction responses share.
pub(crate) fn validate_text_parts(
    content: Option<&str>,
    embeds: &[Embed],
    mentions: Option<&AllowedMentions>,
) -> Result<(), ValidationError> {
    if let Some(c) = content {
        let len = c.chars().count();
        if len > MAX_CONTENT {
            return Err(ValidationError::ContentTooLong { len });
        }
    }
    if embeds.len() > MAX_EMBEDS {
        return Err(ValidationError::TooManyEmbeds { count: embeds.len() });
    }
    let mut total = 0usize;
    for e in embeds {
        total += e.validate()?;
    }
    if total > MAX_EMBED_TOTAL {
        return Err(ValidationError::EmbedsTooLarge { total });
    }
    if let Some(a) = mentions {
        a.validate()?;
    }
    Ok(())
}

// ---------- attachments ----------

/// An attachment metadata object for creating or editing messages.
///
/// When creating a message with files, attachments reference uploaded files
/// via `id = "0"`, `"1"`, etc. matching the index in `files[n]`.
/// When editing a message, existing attachments to keep reference their snowflake ID.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    /// Attachment ID (`0`, `1`, ... for new uploads or snowflake ID for existing files).
    pub id: String,
    /// Filename (1-1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional description / alt-text (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional title (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Whether this attachment is marked as a spoiler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_spoiler: Option<bool>,
}

impl Attachment {
    /// Create an attachment metadata reference by ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            filename: None,
            description: None,
            title: None,
            is_spoiler: None,
        }
    }

    /// Set the filename.
    pub fn filename(mut self, name: impl Into<String>) -> Self {
        self.filename = Some(name.into());
        self
    }

    /// Set the description / alt text.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Mark as spoiler.
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.is_spoiler = Some(spoiler);
        self
    }

    /// Validate attachment fields locally.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(f) = &self.filename {
            validate_filename(f)?;
        }
        if let Some(d) = &self.description {
            let len = d.chars().count();
            if len > MAX_DESCRIPTION_LEN {
                return Err(ValidationError::AttachmentDescriptionTooLong { len });
            }
        }
        Ok(())
    }
}

// ---------- polls ----------

/// An emoji attached to a poll answer.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PollEmoji {
    /// Snowflake ID of a custom emoji.
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake", deserialize_with = "de_opt_snowflake")]
    pub id: Option<Snowflake>,
    /// Emoji name (or unicode character).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the emoji is animated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animated: Option<bool>,
}

impl PollEmoji {
    /// Create a standard unicode emoji for a poll answer.
    pub fn unicode(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: Some(name.into()),
            animated: None,
        }
    }

    /// Create a custom guild emoji for a poll answer.
    pub fn custom(id: Snowflake, name: impl Into<String>, animated: bool) -> Self {
        Self {
            id: Some(id),
            name: Some(name.into()),
            animated: Some(animated),
        }
    }
}

/// Media data for a poll question or answer.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct PollMedia {
    /// The text of the field (question max 300 chars, answer max 55 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The emoji of the answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PollEmoji>,
}

impl PollMedia {
    /// Create poll media with text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            emoji: None,
        }
    }

    /// Set the emoji for this media.
    pub fn emoji(mut self, emoji: PollEmoji) -> Self {
        self.emoji = Some(emoji);
        self
    }
}

impl From<&str> for PollMedia {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for PollMedia {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// One answer choice in a poll.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PollAnswer {
    /// The media (text and optional emoji) of the answer.
    pub poll_media: PollMedia,
}

impl PollAnswer {
    /// Create a poll answer with text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            poll_media: PollMedia::new(text),
        }
    }

    /// Create a poll answer with text and an emoji.
    pub fn with_emoji(text: impl Into<String>, emoji: PollEmoji) -> Self {
        Self {
            poll_media: PollMedia::new(text).emoji(emoji),
        }
    }
}

impl From<&str> for PollAnswer {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for PollAnswer {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// A poll to be created within a message.
///
/// Follows Discord API limits:
/// - Question: max 300 characters
/// - Answers: 1 to 10 choices, each answer text max 55 characters
/// - Duration: 1 to 768 hours (32 days, default 24)
/// - Layout type: defaults to 1 (Default)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CreatePoll {
    /// The question of the poll. Only `text` is supported.
    pub question: PollMedia,
    /// Each of the answers available in the poll (1 to 10).
    pub answers: Vec<PollAnswer>,
    /// Number of hours the poll should be open for (1-768, default 24).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    /// Whether a user can select multiple answers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_multiselect: Option<bool>,
    /// The layout type of the poll. Defaults to 1 (Default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_type: Option<u8>,
}

impl CreatePoll {
    /// Create a new poll with the specified question text.
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: PollMedia::new(question),
            answers: Vec::new(),
            duration: None,
            allow_multiselect: None,
            layout_type: None,
        }
    }

    /// Add an answer choice (max 10).
    pub fn answer(mut self, answer: impl Into<PollAnswer>) -> Self {
        self.answers.push(answer.into());
        self
    }

    /// Set duration in hours (1..=768).
    pub fn duration(mut self, hours: u32) -> Self {
        self.duration = Some(hours);
        self
    }

    /// Set whether multiple answers can be selected.
    pub fn allow_multiselect(mut self, allow: bool) -> Self {
        self.allow_multiselect = Some(allow);
        self
    }

    /// Set layout type (1 = Default).
    pub fn layout_type(mut self, layout: u8) -> Self {
        self.layout_type = Some(layout);
        self
    }

    /// Validate the poll against Discord API rules.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let q_len = self.question.text.as_deref().map(|t| t.chars().count()).unwrap_or(0);
        if q_len == 0 {
            return Err(ValidationError::PollQuestionEmpty);
        }
        if q_len > 300 {
            return Err(ValidationError::PollQuestionTooLong { len: q_len });
        }
        if self.answers.is_empty() || self.answers.len() > 10 {
            return Err(ValidationError::PollBadAnswerCount { count: self.answers.len() });
        }
        for ans in &self.answers {
            if let Some(t) = &ans.poll_media.text {
                let len = t.chars().count();
                if len > 55 {
                    return Err(ValidationError::PollAnswerTextTooLong { len });
                }
            }
        }
        if let Some(d) = self.duration {
            if !(1..=768).contains(&d) {
                return Err(ValidationError::PollBadDuration { hours: d });
            }
        }
        Ok(())
    }
}

// ---------- create message ----------

/// Body of Create Message. Build with the fluent methods, then send with
/// [`RestClient::create_message`], which validates first.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tts: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    embeds: Vec<Embed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Vec::is_empty", serialize_with = "ser_snowflakes")]
    sticker_ids: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_flags")]
    flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enforce_nonce: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<Attachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    poll: Option<CreatePoll>,
}

impl CreateMessage {
    /// Empty message (add something before sending).
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the text (max 2000 characters).
    pub fn content(mut self, v: impl Into<String>) -> Self {
        self.content = Some(v.into());
        self
    }
    /// Add an embed (max 10).
    pub fn embed(mut self, e: Embed) -> Self {
        self.embeds.push(e);
        self
    }
    /// Text-to-speech.
    pub fn tts(mut self, v: bool) -> Self {
        self.tts = Some(v);
        self
    }
    /// Nonce for de-duplication (max 25 characters).
    pub fn nonce(mut self, v: impl Into<String>) -> Self {
        self.nonce = Some(v.into());
        self
    }
    /// Make the nonce unique-checked.
    pub fn enforce_nonce(mut self, v: bool) -> Self {
        self.enforce_nonce = Some(v);
        self
    }
    /// Control who gets pinged.
    pub fn allowed_mentions(mut self, a: AllowedMentions) -> Self {
        self.allowed_mentions = Some(a);
        self
    }
    /// Reply to a message.
    pub fn reply_to(mut self, message_id: Snowflake) -> Self {
        self.message_reference = Some(MessageReference::reply(message_id));
        self
    }
    /// Set a full message reference (reply or forward).
    pub fn message_reference(mut self, r: MessageReference) -> Self {
        self.message_reference = Some(r);
        self
    }
    /// Add raw component JSON (typed components come later).
    pub fn components(mut self, c: Vec<serde_json::Value>) -> Self {
        self.components = c;
        self
    }
    /// Add a sticker id (max 3).
    pub fn sticker_id(mut self, id: Snowflake) -> Self {
        self.sticker_ids.push(id);
        self
    }
    /// Set flags (only `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS`,
    /// `IS_VOICE_MESSAGE`, `IS_COMPONENTS_V2` are accepted).
    pub fn flags(mut self, f: MessageFlags) -> Self {
        self.flags = Some(f);
        self
    }
    /// Send without a push/desktop notification.
    pub fn silent(mut self) -> Self {
        self.flags = Some(self.flags.unwrap_or(MessageFlags::NONE) | MessageFlags::SUPPRESS_NOTIFICATIONS);
        self
    }
    /// Don't show link embeds.
    pub fn suppress_embeds(mut self) -> Self {
        self.flags = Some(self.flags.unwrap_or(MessageFlags::NONE) | MessageFlags::SUPPRESS_EMBEDS);
        self
    }
    /// Add an attachment description or reference.
    pub fn attachment(mut self, a: Attachment) -> Self {
        self.attachments.push(a);
        self
    }
    /// Set attachments.
    pub fn attachments(mut self, a: Vec<Attachment>) -> Self {
        self.attachments = a;
        self
    }
    /// Add a poll to the message.
    pub fn poll(mut self, p: CreatePoll) -> Self {
        self.poll = Some(p);
        self
    }

    /// Check everything the docs let us check without a network call.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let flags = self.flags.unwrap_or(MessageFlags::NONE);
        let bad = flags.unsettable_on_create();
        if bad != 0 {
            return Err(ValidationError::FlagNotSettable { bits: bad });
        }

        let is_forward = self.message_reference.as_ref().is_some_and(MessageReference::is_forward);
        if is_forward && self.message_reference.as_ref().is_some_and(|r| r.channel_id.is_none()) {
            return Err(ValidationError::ForwardNeedsChannel);
        }

        if flags.contains(MessageFlags::IS_COMPONENTS_V2) {
            if self.content.is_some() {
                return Err(ValidationError::ComponentsV2Conflict { field: "content" });
            }
            if !self.embeds.is_empty() {
                return Err(ValidationError::ComponentsV2Conflict { field: "embeds" });
            }
            if !self.sticker_ids.is_empty() {
                return Err(ValidationError::ComponentsV2Conflict { field: "sticker_ids" });
            }
        }

        let has_text = self.content.as_deref().is_some_and(|c| !c.is_empty());
        if !is_forward
            && !has_text
            && self.embeds.is_empty()
            && self.sticker_ids.is_empty()
            && self.components.is_empty()
            && self.attachments.is_empty()
            && self.poll.is_none()
        {
            return Err(ValidationError::Empty);
        }

        if let Some(c) = &self.content {
            let len = c.chars().count();
            if len > MAX_CONTENT {
                return Err(ValidationError::ContentTooLong { len });
            }
        }
        if self.embeds.len() > MAX_EMBEDS {
            return Err(ValidationError::TooManyEmbeds { count: self.embeds.len() });
        }
        let mut total = 0usize;
        for e in &self.embeds {
            total += e.validate()?;
        }
        if total > MAX_EMBED_TOTAL {
            return Err(ValidationError::EmbedsTooLarge { total });
        }
        if self.sticker_ids.len() > MAX_STICKERS {
            return Err(ValidationError::TooManyStickers { count: self.sticker_ids.len() });
        }
        if let Some(n) = &self.nonce {
            let len = n.chars().count();
            if len > MAX_NONCE {
                return Err(ValidationError::NonceTooLong { len });
            }
        }
        if let Some(a) = &self.allowed_mentions {
            a.validate()?;
        }
        if self.attachments.len() > MAX_FILES {
            return Err(ValidationError::TooManyAttachments { count: self.attachments.len() });
        }
        for a in &self.attachments {
            a.validate()?;
        }
        if let Some(poll) = &self.poll {
            poll.validate()?;
        }
        Ok(())
    }
}

// ---------- edit message ----------

/// Body of Edit Message (`PATCH /channels/{channel_id}/messages/{message_id}`).
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct EditMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<Embed>>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_flags")]
    flags: Option<MessageFlags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mentions: Option<AllowedMentions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    components: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<Attachment>>,
}

impl EditMessage {
    /// Create an empty edit request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new message content.
    pub fn content(mut self, c: impl Into<String>) -> Self {
        self.content = Some(Some(c.into()));
        self
    }

    /// Clear message content explicitly.
    pub fn clear_content(mut self) -> Self {
        self.content = Some(None);
        self
    }

    /// Replace the message's embeds (up to 10).
    pub fn embeds(mut self, embeds: Vec<Embed>) -> Self {
        self.embeds = Some(embeds);
        self
    }

    /// Add an embed to the edit request.
    pub fn embed(mut self, e: Embed) -> Self {
        self.embeds.get_or_insert_with(Vec::new).push(e);
        self
    }

    /// Set flags (only `SUPPRESS_EMBEDS` can be set when editing).
    pub fn flags(mut self, f: MessageFlags) -> Self {
        self.flags = Some(f);
        self
    }

    /// Set allowed mentions.
    pub fn allowed_mentions(mut self, a: AllowedMentions) -> Self {
        self.allowed_mentions = Some(a);
        self
    }

    /// Set components.
    pub fn components(mut self, comps: Vec<serde_json::Value>) -> Self {
        self.components = Some(comps);
        self
    }

    /// Set the attachments array.
    ///
    /// When editing, Discord requires the attachments array to specify which existing
    /// attachments to keep (referencing their snowflake ID) and which new ones to add.
    pub fn attachments(mut self, atts: Vec<Attachment>) -> Self {
        self.attachments = Some(atts);
        self
    }

    /// Retain an existing attachment on the message by its snowflake ID.
    pub fn keep_attachment(mut self, id: Snowflake) -> Self {
        self.attachments.get_or_insert_with(Vec::new).push(Attachment::new(id.to_string()));
        self
    }

    /// Add an attachment description or reference.
    pub fn attachment(mut self, a: Attachment) -> Self {
        self.attachments.get_or_insert_with(Vec::new).push(a);
        self
    }

    /// Validate the edit payload against documented limits.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(Some(c)) = &self.content {
            let len = c.chars().count();
            if len > MAX_CONTENT {
                return Err(ValidationError::ContentTooLong { len });
            }
        }
        if let Some(embeds) = &self.embeds {
            if embeds.len() > MAX_EMBEDS {
                return Err(ValidationError::TooManyEmbeds { count: embeds.len() });
            }
            let mut total = 0usize;
            for e in embeds {
                total += e.validate()?;
            }
            if total > MAX_EMBED_TOTAL {
                return Err(ValidationError::EmbedsTooLarge { total });
            }
        }
        if let Some(a) = &self.allowed_mentions {
            a.validate()?;
        }
        if let Some(atts) = &self.attachments {
            if atts.len() > MAX_FILES {
                return Err(ValidationError::TooManyAttachments { count: atts.len() });
            }
            for att in atts {
                att.validate()?;
            }
        }
        Ok(())
    }
}

/// The parts of a Message object most bots need. Unknown fields are ignored;
/// use [`Response::json`](crate::Response::json) with your own type for more.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Message {
    /// Message id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Channel id.
    #[serde(deserialize_with = "de_snowflake")]
    pub channel_id: Snowflake,
    /// Text content (empty without the `MESSAGE_CONTENT` intent, except for your own messages).
    #[serde(default)]
    pub content: String,
    /// Message flags.
    #[serde(default)]
    pub flags: Option<u32>,
}

/// Percent-encode for URL path segments (emoji must be URL-encoded, custom
/// emoji as `name:id`).
pub(crate) fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl<T: Transport> RestClient<T> {
    /// `POST /channels/{id}/messages`. Validates locally first; an invalid
    /// message never reaches the network.
    pub async fn create_message(&self, channel_id: Snowflake, msg: &CreateMessage) -> Result<Message, Error> {
        msg.validate()?;
        let body = serde_json::to_vec(msg).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/channels/{channel_id}/messages");
        self.request("POST", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `PATCH /channels/{id}/messages/{message_id}`: edit a message. Validates locally first.
    pub async fn edit_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        edit: &EditMessage,
    ) -> Result<Message, Error> {
        edit.validate()?;
        let body = serde_json::to_vec(edit).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/channels/{channel_id}/messages/{message_id}");
        self.request("PATCH", &path, Some(body)).await?.expect_success()?.json()
    }

    /// `DELETE /channels/{id}/messages/{id}`.
    pub async fn delete_message(&self, channel_id: Snowflake, message_id: Snowflake) -> Result<(), Error> {
        let path = format!("/channels/{channel_id}/messages/{message_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `PUT /channels/{id}/messages/{id}/reactions/{emoji}/@me`. Pass a unicode
    /// emoji (`"🔥"`) or a custom one as `name:id`.
    pub async fn add_reaction(&self, channel_id: Snowflake, message_id: Snowflake, emoji: &str) -> Result<(), Error> {
        let path = format!(
            "/channels/{channel_id}/messages/{message_id}/reactions/{}/@me",
            percent_encode(emoji)
        );
        self.request("PUT", &path, None).await?.expect_success()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};
    use serde_json::json;

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    fn to_json(m: &CreateMessage) -> serde_json::Value {
        serde_json::to_value(m).unwrap()
    }

    #[test]
    fn reply_serializes_snowflakes_as_strings() {
        let m = CreateMessage::new()
            .content("hi")
            .reply_to(sf(5))
            .allowed_mentions(AllowedMentions::none());
        assert_eq!(
            to_json(&m),
            json!({
                "content": "hi",
                "allowed_mentions": { "parse": [] },
                "message_reference": { "message_id": "5" }
            })
        );
    }

    #[test]
    fn forward_has_type_message_and_channel() {
        let m = CreateMessage::new().message_reference(MessageReference::forward(sf(1), sf(2)));
        assert_eq!(
            to_json(&m),
            json!({ "message_reference": { "type": 1, "message_id": "1", "channel_id": "2" } })
        );
        assert!(m.validate().is_ok()); // a forward needs nothing else
    }

    #[test]
    fn forward_without_channel_is_rejected() {
        let mut r = MessageReference::forward(sf(1), sf(2));
        r.channel_id = None;
        let m = CreateMessage::new().message_reference(r);
        assert_eq!(m.validate(), Err(ValidationError::ForwardNeedsChannel));
    }

    #[test]
    fn empty_message_is_rejected() {
        assert_eq!(CreateMessage::new().validate(), Err(ValidationError::Empty));
        assert_eq!(CreateMessage::new().content("").validate(), Err(ValidationError::Empty));
    }

    #[test]
    fn content_limit_is_2000_characters() {
        let ok = CreateMessage::new().content("a".repeat(2000));
        assert!(ok.validate().is_ok());
        let long = CreateMessage::new().content("a".repeat(2001));
        assert_eq!(long.validate(), Err(ValidationError::ContentTooLong { len: 2001 }));
        // Characters, not bytes.
        assert!(CreateMessage::new().content("é".repeat(2000)).validate().is_ok());
    }

    #[test]
    fn components_v2_forbids_content_and_embeds() {
        let comps = vec![json!({ "type": 10, "content": "hello" })];
        let ok = CreateMessage::new().flags(MessageFlags::IS_COMPONENTS_V2).components(comps.clone());
        assert!(ok.validate().is_ok());

        let with_content = ok.clone().content("nope");
        assert_eq!(
            with_content.validate(),
            Err(ValidationError::ComponentsV2Conflict { field: "content" })
        );
        let with_embed = ok.embed(Embed::new().title("nope"));
        assert_eq!(
            with_embed.validate(),
            Err(ValidationError::ComponentsV2Conflict { field: "embeds" })
        );
    }

    #[test]
    fn unsettable_flags_are_rejected() {
        let m = CreateMessage::new().content("x").flags(MessageFlags::EPHEMERAL);
        assert_eq!(m.validate(), Err(ValidationError::FlagNotSettable { bits: 64 }));
        assert!(CreateMessage::new().content("x").silent().suppress_embeds().validate().is_ok());
    }

    #[test]
    fn embed_limits() {
        let m = |e: Embed| CreateMessage::new().embed(e);
        assert!(m(Embed::new().title("t".repeat(256))).validate().is_ok());
        assert!(matches!(
            m(Embed::new().title("t".repeat(257))).validate(),
            Err(ValidationError::EmbedFieldTooLong { field: "title", len: 257, max: 256 })
        ));

        let mut e = Embed::new();
        for i in 0..25 {
            e = e.field(format!("n{i}"), "v", false);
        }
        assert!(m(e.clone()).validate().is_ok());
        assert_eq!(
            m(e.field("one", "too many", false)).validate(),
            Err(ValidationError::TooManyEmbedFields { count: 26 })
        );

        // Whitespace is trimmed before counting.
        assert!(m(Embed::new().title(format!("  {}  ", "t".repeat(256)))).validate().is_ok());
    }

    #[test]
    fn embeds_have_a_combined_limit_of_6000() {
        let big = |n: usize| Embed::new().description("d".repeat(n));
        let ok = CreateMessage::new().embed(big(4096)).embed(big(1904)); // 6000 exactly
        assert!(ok.validate().is_ok());
        let over = CreateMessage::new().embed(big(4096)).embed(big(1905));
        assert_eq!(over.validate(), Err(ValidationError::EmbedsTooLarge { total: 6001 }));
    }

    #[test]
    fn at_most_ten_embeds_and_three_stickers() {
        let mut m = CreateMessage::new();
        for _ in 0..11 {
            m = m.embed(Embed::new().title("x"));
        }
        assert_eq!(m.validate(), Err(ValidationError::TooManyEmbeds { count: 11 }));

        let s = CreateMessage::new().sticker_id(sf(1)).sticker_id(sf(2)).sticker_id(sf(3)).sticker_id(sf(4));
        assert_eq!(s.validate(), Err(ValidationError::TooManyStickers { count: 4 }));
    }

    #[test]
    fn nonce_is_capped_at_25_characters() {
        let m = CreateMessage::new().content("x");
        assert!(m.clone().nonce("n".repeat(25)).validate().is_ok());
        assert_eq!(
            m.nonce("n".repeat(26)).validate(),
            Err(ValidationError::NonceTooLong { len: 26 })
        );
    }

    #[test]
    fn allowed_mentions_parse_is_exclusive_with_ids() {
        let conflict = AllowedMentions {
            parse: Some(vec![MentionType::Users]),
            users: vec![sf(1)],
            ..Default::default()
        };
        assert_eq!(
            CreateMessage::new().content("x").allowed_mentions(conflict).validate(),
            Err(ValidationError::MentionsParseConflict)
        );
        // An empty list next to parse is fine, per the docs.
        let fine = AllowedMentions { parse: Some(vec![MentionType::Users]), ..Default::default() };
        assert!(CreateMessage::new().content("x").allowed_mentions(fine).validate().is_ok());

        let many = AllowedMentions { users: (0..101).map(sf).collect(), ..Default::default() };
        assert_eq!(
            CreateMessage::new().content("x").allowed_mentions(many).validate(),
            Err(ValidationError::TooManyMentionIds { kind: "users" })
        );
    }

    #[test]
    fn mention_types_serialize_lowercase() {
        let a = AllowedMentions { parse: Some(vec![MentionType::Roles, MentionType::Everyone]), ..Default::default() };
        assert_eq!(serde_json::to_value(&a).unwrap(), json!({ "parse": ["roles", "everyone"] }));
        assert_eq!(
            serde_json::to_value(AllowedMentions { replied_user: Some(true), ..Default::default() }).unwrap(),
            json!({ "replied_user": true })
        );
    }

    #[test]
    fn percent_encoding_matches_the_emoji_rules() {
        assert_eq!(percent_encode("🔥"), "%F0%9F%94%A5");
        assert_eq!(percent_encode("name:123"), "name%3A123");
        assert_eq!(percent_encode("abc-_.~09"), "abc-_.~09");
    }

    #[tokio::test(start_paused = true)]
    async fn create_message_sends_the_right_request() {
        let reply = r#"{"id":"10","channel_id":"20","content":"hi","flags":0,"author":{"id":"1","username":"x"}}"#;
        let c = client(vec![resp(200, reply)]);

        let msg = c.create_message(sf(20), &CreateMessage::new().content("hi")).await.unwrap();
        assert_eq!(msg.id, sf(10));
        assert_eq!(msg.channel_id, sf(20));
        assert_eq!(msg.content, "hi");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/20/messages");
        assert!(seen[0].headers.contains(&("Content-Type".to_owned(), "application/json".to_owned())));
        let sent: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(sent, json!({ "content": "hi" }));
    }

    #[tokio::test(start_paused = true)]
    async fn invalid_messages_never_reach_the_network() {
        let c = client(vec![]);
        let r = c.create_message(sf(20), &CreateMessage::new()).await;
        assert!(matches!(r, Err(Error::Validation(ValidationError::Empty))));
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn discord_errors_are_parsed() {
        let c = client(vec![resp(403, r#"{"code":50013,"message":"Missing Permissions"}"#)]);
        let r = c.create_message(sf(20), &CreateMessage::new().content("hi")).await;
        match r {
            Err(Error::Api { status: 403, code: Some(50013), message }) => {
                assert_eq!(message, "Missing Permissions");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn reaction_and_delete_use_the_documented_routes() {
        let c = client(vec![resp(204, ""), resp(204, "")]);
        c.add_reaction(sf(1), sf(2), "🔥").await.unwrap();
        c.delete_message(sf(1), sf(2)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/channels/1/messages/2/reactions/%F0%9F%94%A5/@me"
        );
        assert_eq!(seen[1].method, "DELETE");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/channels/1/messages/2");
    }

    #[tokio::test(start_paused = true)]
    async fn edit_message_uses_the_documented_route() {
        let msg_json = json!({
            "id": "2",
            "channel_id": "1",
            "content": "edited content"
        }).to_string();
        let c = client(vec![resp(200, &msg_json)]);
        let edit = EditMessage::new()
            .content("edited content")
            .keep_attachment(sf(999))
            .flags(MessageFlags::SUPPRESS_EMBEDS);
        let m = c.edit_message(sf(1), sf(2), &edit).await.unwrap();
        assert_eq!(m.content, "edited content");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "PATCH");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/messages/2");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["content"], "edited content");
        assert_eq!(body["attachments"][0]["id"], "999");
    }

    #[test]
    fn attachment_and_files_validation() {
        assert_eq!(validate_filename(""), Err(ValidationError::EmptyFilename));
        assert_eq!(validate_filename("   "), Err(ValidationError::EmptyFilename));
        assert_eq!(
            validate_filename(&"a".repeat(1025)),
            Err(ValidationError::FilenameTooLong { len: 1025 })
        );
        assert_eq!(
            validate_filename("bad\nname"),
            Err(ValidationError::InvalidFilename { name: "bad\nname".into() })
        );
        assert!(validate_filename("valid_name.png").is_ok());

        let mut a = Attachment::new("0").filename("test.png");
        assert!(a.validate().is_ok());
        a.description = Some("d".repeat(1025));
        assert_eq!(
            a.validate(),
            Err(ValidationError::AttachmentDescriptionTooLong { len: 1025 })
        );

        let m = CreateMessage::new().attachment(Attachment::new("0").filename("img.png"));
        assert!(m.validate().is_ok()); // message with attachment is not empty
    }

    #[tokio::test(start_paused = true)]
    async fn poll_validation_and_creation() {
        // Empty question
        let poll = CreatePoll::new("").answer("A").answer("B");
        assert_eq!(poll.validate(), Err(ValidationError::PollQuestionEmpty));

        // Question too long (> 300 chars)
        let poll = CreatePoll::new("q".repeat(301)).answer("A").answer("B");
        assert_eq!(poll.validate(), Err(ValidationError::PollQuestionTooLong { len: 301 }));

        // No answers
        let poll = CreatePoll::new("Favorite color?");
        assert_eq!(poll.validate(), Err(ValidationError::PollBadAnswerCount { count: 0 }));

        // > 10 answers
        let mut poll = CreatePoll::new("Pick one:");
        for i in 0..11 {
            poll = poll.answer(format!("Option {i}"));
        }
        assert_eq!(poll.validate(), Err(ValidationError::PollBadAnswerCount { count: 11 }));

        // Answer text too long (> 55 chars)
        let poll = CreatePoll::new("Pick one:").answer("a".repeat(56)).answer("Valid");
        assert_eq!(poll.validate(), Err(ValidationError::PollAnswerTextTooLong { len: 56 }));

        // Invalid duration (< 1 or > 768)
        let poll = CreatePoll::new("Question?").answer("A").answer("B").duration(0);
        assert_eq!(poll.validate(), Err(ValidationError::PollBadDuration { hours: 0 }));
        let poll = CreatePoll::new("Question?").answer("A").answer("B").duration(769);
        assert_eq!(poll.validate(), Err(ValidationError::PollBadDuration { hours: 769 }));

        // Valid poll alone makes message non-empty
        let valid_poll = CreatePoll::new("Rust or Go?")
            .answer("Rust")
            .answer(PollAnswer::with_emoji("Go", PollEmoji::unicode("🚀")))
            .duration(48)
            .allow_multiselect(false)
            .layout_type(1);
        assert!(valid_poll.validate().is_ok());

        let msg = CreateMessage::new().poll(valid_poll);
        assert!(msg.validate().is_ok());

        // Test sending message with poll
        let msg_json = json!({
            "id": "100",
            "channel_id": "1",
            "content": ""
        }).to_string();
        let c = client(vec![resp(200, &msg_json)]);
        let valid_poll = CreatePoll::new("Rust or Go?").answer("Rust").answer("Go");
        let _m = c.create_message(sf(1), &CreateMessage::new().poll(valid_poll)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/channels/1/messages");
        let body: serde_json::Value = serde_json::from_slice(seen[0].body.as_ref().unwrap()).unwrap();
        assert_eq!(body["poll"]["question"]["text"], "Rust or Go?");
        assert_eq!(body["poll"]["answers"].as_array().unwrap().len(), 2);
    }
}
