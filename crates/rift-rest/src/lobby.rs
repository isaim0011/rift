//! Discord Lobbies and Partner SDK API endpoints.
//!
//! Provides typed representations, validations, and client methods for all 15
//! Discord Lobby operations and all 5 Partner SDK endpoints.

use crate::channel::Channel;
use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake, ser_snowflake, Embed};
use crate::transport::Transport;
use crate::user::UserResponse;
use rift_core::Snowflake;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// Errors & Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Lobby and Partner SDK validation errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LobbyError {
    /// Lobby secret must be 1-250 characters.
    BadSecret {
        /// Provided length.
        len: usize,
    },
    /// Idle timeout must be between 5 and 604800 seconds.
    BadIdleTimeout {
        /// Provided timeout in seconds.
        seconds: u32,
    },
    /// Lobby name must be 1-100 characters.
    BadName {
        /// Provided length.
        len: usize,
    },
    /// Members list cannot exceed 25 items.
    TooManyMembers {
        /// Provided count.
        count: usize,
    },
    /// Bulk members update list must contain 1-25 items.
    InvalidBulkMembersCount {
        /// Provided count.
        count: usize,
    },
    /// Metadata map cannot exceed 25 key-value pairs.
    TooManyMetadataKeys {
        /// Provided count.
        count: usize,
    },
    /// Metadata values cannot exceed 1024 characters.
    MetadataValueTooLong {
        /// Key name.
        key: String,
        /// Provided length.
        len: usize,
    },
    /// Moderation metadata map cannot exceed 5 key-value pairs.
    TooManyModerationMetadataKeys {
        /// Provided count.
        count: usize,
    },
    /// Moderation metadata values cannot exceed 2000 characters.
    ModerationMetadataValueTooLong {
        /// Key name.
        key: String,
        /// Provided length.
        len: usize,
    },
    /// Member additional name must be 1-80 characters.
    BadAdditionalName {
        /// Provided length.
        len: usize,
    },
    /// Messages limit must be between 1 and 100.
    BadLimit {
        /// Provided limit.
        limit: u16,
    },
    /// External user id must be 1-1024 characters.
    BadExternalUserId {
        /// Provided length.
        len: usize,
    },
    /// External auth token must be 1-10240 characters.
    BadExternalAuthToken {
        /// Provided length.
        len: usize,
    },
    /// Client secret cannot exceed 1024 characters.
    BadClientSecret {
        /// Provided length.
        len: usize,
    },
    /// Preferred global name must be 1-32 characters.
    BadPreferredGlobalName {
        /// Provided length.
        len: usize,
    },
    /// Message content cannot exceed 4000 characters.
    BadContent {
        /// Provided length.
        len: usize,
    },
    /// Cannot exceed 10 embeds.
    TooManyEmbeds {
        /// Provided count.
        count: usize,
    },
    /// Cannot exceed 3 stickers.
    TooManyStickers {
        /// Provided count.
        count: usize,
    },
    /// Nonce cannot exceed 25 characters.
    BadNonce {
        /// Provided length.
        len: usize,
    },
    /// Webhook URL cannot exceed 512 characters.
    BadWebhookUrl {
        /// Provided length.
        len: usize,
    },
}

impl fmt::Display for LobbyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadSecret { len } => write!(f, "secret must be 1-250 characters, got {len}"),
            Self::BadIdleTimeout { seconds } => write!(f, "idle_timeout_seconds must be between 5 and 604800, got {seconds}"),
            Self::BadName { len } => write!(f, "lobby name must be 1-100 characters, got {len}"),
            Self::TooManyMembers { count } => write!(f, "members list cannot exceed 25 items, got {count}"),
            Self::InvalidBulkMembersCount { count } => write!(f, "bulk members update must contain 1-25 items, got {count}"),
            Self::TooManyMetadataKeys { count } => write!(f, "metadata cannot exceed 25 keys, got {count}"),
            Self::MetadataValueTooLong { key, len } => write!(f, "metadata key '{key}' value cannot exceed 1024 characters, got {len}"),
            Self::TooManyModerationMetadataKeys { count } => write!(f, "moderation metadata cannot exceed 5 keys, got {count}"),
            Self::ModerationMetadataValueTooLong { key, len } => write!(f, "moderation metadata key '{key}' value cannot exceed 2000 characters, got {len}"),
            Self::BadAdditionalName { len } => write!(f, "additional_name must be 1-80 characters, got {len}"),
            Self::BadLimit { limit } => write!(f, "message limit must be between 1 and 100, got {limit}"),
            Self::BadExternalUserId { len } => write!(f, "external_user_id must be 1-1024 characters, got {len}"),
            Self::BadExternalAuthToken { len } => write!(f, "external_auth_token must be 1-10240 characters, got {len}"),
            Self::BadClientSecret { len } => write!(f, "client_secret cannot exceed 1024 characters, got {len}"),
            Self::BadPreferredGlobalName { len } => write!(f, "preferred_global_name must be 1-32 characters, got {len}"),
            Self::BadContent { len } => write!(f, "message content cannot exceed 4000 characters, got {len}"),
            Self::TooManyEmbeds { count } => write!(f, "embeds cannot exceed 10 items, got {count}"),
            Self::TooManyStickers { count } => write!(f, "stickers cannot exceed 3 items, got {count}"),
            Self::BadNonce { len } => write!(f, "nonce cannot exceed 25 characters, got {len}"),
            Self::BadWebhookUrl { len } => write!(f, "override_event_webhooks_url cannot exceed 512 characters, got {len}"),
        }
    }
}

impl std::error::Error for LobbyError {}

impl From<LobbyError> for Error {
    fn from(e: LobbyError) -> Self {
        Error::Transport(format!("lobby validation: {e}"))
    }
}

fn validate_metadata(map: &BTreeMap<String, String>) -> Result<(), LobbyError> {
    if map.len() > 25 {
        return Err(LobbyError::TooManyMetadataKeys { count: map.len() });
    }
    for (k, v) in map {
        let len = v.chars().count();
        if len > 1024 {
            return Err(LobbyError::MetadataValueTooLong { key: k.clone(), len });
        }
    }
    Ok(())
}

fn validate_moderation_metadata(map: &BTreeMap<String, String>) -> Result<(), LobbyError> {
    if map.len() > 5 {
        return Err(LobbyError::TooManyModerationMetadataKeys { count: map.len() });
    }
    for (k, v) in map {
        let len = v.chars().count();
        if len > 2000 {
            return Err(LobbyError::ModerationMetadataValueTooLong { key: k.clone(), len });
        }
    }
    Ok(())
}

fn validate_timeout(timeout: u32) -> Result<(), LobbyError> {
    if !(5..=604800).contains(&timeout) {
        return Err(LobbyError::BadIdleTimeout { seconds: timeout });
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), LobbyError> {
    let len = name.chars().count();
    if !(1..=100).contains(&len) {
        return Err(LobbyError::BadName { len });
    }
    Ok(())
}

fn validate_additional_name(name: &str) -> Result<(), LobbyError> {
    let len = name.chars().count();
    if !(1..=80).contains(&len) {
        return Err(LobbyError::BadAdditionalName { len });
    }
    Ok(())
}

fn ser_opt_snowflakes<S: Serializer>(v: &Option<Vec<Snowflake>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(list) => s.collect_seq(list.iter().map(|id| id.to_string())),
        None => s.serialize_none(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Enums & Models
// ─────────────────────────────────────────────────────────────────────────────

/// Supported third-party authentication provider identity types for partner SDK provisional accounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApplicationIdentityProviderAuthType {
    /// OpenID Connect identity provider token.
    #[serde(rename = "OIDC")]
    Oidc,
    /// Epic Online Services Access Token.
    #[serde(rename = "EPIC_ONLINE_SERVICES_ACCESS_TOKEN")]
    EpicOnlineServicesAccessToken,
    /// Epic Online Services ID Token.
    #[serde(rename = "EPIC_ONLINE_SERVICES_ID_TOKEN")]
    EpicOnlineServicesIdToken,
    /// Steam session ticket.
    #[serde(rename = "STEAM_SESSION_TICKET")]
    SteamSessionTicket,
    /// Unity Services ID Token.
    #[serde(rename = "UNITY_SERVICES_ID_TOKEN")]
    UnityServicesIdToken,
    /// Discord bot issued access token.
    #[serde(rename = "DISCORD_BOT_ISSUED_ACCESS_TOKEN")]
    DiscordBotIssuedAccessToken,
    /// Apple ID Token.
    #[serde(rename = "APPLE_ID_TOKEN")]
    AppleIdToken,
    /// PlayStation Network ID Token.
    #[serde(rename = "PLAYSTATION_NETWORK_ID_TOKEN")]
    PlaystationNetworkIdToken,
}

/// A member of a lobby.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LobbyMemberResponse {
    /// User id of the member.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Arbitrary member metadata.
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Flags for this member.
    #[serde(default)]
    pub flags: i32,
    /// Additional display name for this member in the lobby.
    #[serde(default)]
    pub additional_name: Option<String>,
}

/// A lobby object returned by Discord Lobby endpoints.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LobbyResponse {
    /// Unique identifier for the lobby.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Application id owning this lobby.
    #[serde(deserialize_with = "de_snowflake")]
    pub application_id: Snowflake,
    /// Arbitrary key-value metadata attached to the lobby.
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Members currently in the lobby.
    #[serde(default)]
    pub members: Vec<LobbyMemberResponse>,
    /// Linked guild channel if connected.
    #[serde(default)]
    pub linked_channel: Option<Channel>,
    /// Bitfield flags for the lobby.
    #[serde(default)]
    pub flags: u32,
    /// Webhook URL to receive lobby events.
    #[serde(default)]
    pub override_event_webhooks_url: Option<String>,
}

/// Response for creating a linked lobby guild invite.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LobbyGuildInviteResponse {
    /// The invite code.
    pub code: String,
}

/// Member information embedded in a lobby message.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MessageLobbyMemberResponse {
    /// Additional display name for the member.
    pub additional_name: String,
}

/// A message sent in a lobby chat channel.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct LobbyMessageResponse {
    /// Message snowflake ID.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Message type code.
    #[serde(rename = "type")]
    pub kind: u32,
    /// Message text contents.
    pub content: String,
    /// ID of the lobby.
    #[serde(deserialize_with = "de_snowflake")]
    pub lobby_id: Snowflake,
    /// ID of the channel.
    #[serde(deserialize_with = "de_snowflake")]
    pub channel_id: Snowflake,
    /// Message author.
    pub author: UserResponse,
    /// Lobby member metadata if available.
    #[serde(default)]
    pub lobby_member: Option<MessageLobbyMemberResponse>,
    /// Custom message metadata.
    #[serde(default)]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Moderation metadata.
    #[serde(default)]
    pub moderation_metadata: Option<BTreeMap<String, String>>,
    /// Message flags bitfield.
    #[serde(default)]
    pub flags: i32,
    /// Application ID if sent by a bot / application.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub application_id: Option<Snowflake>,
}

/// Token response for Partner SDK provisional accounts.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProvisionalTokenResponse {
    /// Type of token (typically "Bearer").
    pub token_type: String,
    /// The access token string.
    pub access_token: String,
    /// Expiration time in seconds.
    pub expires_in: i32,
    /// OAuth2 scope granted.
    pub scope: String,
    /// ID token issued for this account.
    pub id_token: String,
    /// Refresh token if issued.
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Array of scopes granted.
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    /// Expiry timestamp in seconds (Unix epoch).
    #[serde(default)]
    pub expires_at_s: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request Payloads
// ─────────────────────────────────────────────────────────────────────────────

/// Member specification when creating or editing a lobby.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LobbyMemberRequest {
    /// User snowflake id.
    #[serde(serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Key-value metadata for the member (max 25 entries, max 1024 chars per value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Flags for this member.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Additional display name (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_name: Option<String>,
}

impl LobbyMemberRequest {
    /// Validate member request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(m) = &self.metadata {
            validate_metadata(m)?;
        }
        if let Some(n) = &self.additional_name {
            validate_additional_name(n)?;
        }
        Ok(())
    }
}

/// Member specification for bulk member update endpoint.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BulkLobbyMemberRequest {
    /// User snowflake id.
    #[serde(serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Key-value metadata for the member (max 25 entries, max 1024 chars per value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Flags for this member.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Additional display name (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_name: Option<String>,
    /// Whether to remove the member from the lobby.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_member: Option<bool>,
}

impl BulkLobbyMemberRequest {
    /// Validate bulk member request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(m) = &self.metadata {
            validate_metadata(m)?;
        }
        if let Some(n) = &self.additional_name {
            validate_additional_name(n)?;
        }
        Ok(())
    }
}

/// Request payload for creating or joining a lobby via `PUT /lobbies`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateOrJoinLobby {
    /// Unique secret token for the lobby (1-250 characters).
    pub secret: String,
    /// Timeout in seconds before an idle lobby is cleaned up (5 to 604800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
    /// Key-value metadata for the lobby (max 25 entries, values max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lobby_metadata: Option<BTreeMap<String, String>>,
    /// Key-value metadata for the joining member (max 25 entries, values max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_metadata: Option<BTreeMap<String, String>>,
    /// Bitfield flags for the lobby.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Name of the lobby (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl CreateOrJoinLobby {
    /// Validate create or join lobby request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        let sec_len = self.secret.chars().count();
        if !(1..=250).contains(&sec_len) {
            return Err(LobbyError::BadSecret { len: sec_len });
        }
        if let Some(t) = self.idle_timeout_seconds {
            validate_timeout(t)?;
        }
        if let Some(m) = &self.lobby_metadata {
            validate_metadata(m)?;
        }
        if let Some(m) = &self.member_metadata {
            validate_metadata(m)?;
        }
        if let Some(n) = &self.name {
            validate_name(n)?;
        }
        Ok(())
    }
}

/// Request payload for creating a lobby via `POST /lobbies`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CreateLobby {
    /// Name of the lobby (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Timeout in seconds before an idle lobby is cleaned up (5 to 604800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
    /// Initial members to populate the lobby (max 25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<LobbyMemberRequest>>,
    /// Key-value metadata for the lobby (max 25 entries, values max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Bitfield flags for the lobby.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Custom webhook URL to receive lobby lifecycle events (max 512 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_event_webhooks_url: Option<String>,
}

impl CreateLobby {
    /// Validate create lobby request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(n) = &self.name {
            validate_name(n)?;
        }
        if let Some(t) = self.idle_timeout_seconds {
            validate_timeout(t)?;
        }
        if let Some(members) = &self.members {
            if members.len() > 25 {
                return Err(LobbyError::TooManyMembers { count: members.len() });
            }
            for m in members {
                m.validate()?;
            }
        }
        if let Some(m) = &self.metadata {
            validate_metadata(m)?;
        }
        if let Some(u) = &self.override_event_webhooks_url {
            let len = u.chars().count();
            if len > 512 {
                return Err(LobbyError::BadWebhookUrl { len });
            }
        }
        Ok(())
    }
}

/// Request payload for editing an existing lobby via `PATCH /lobbies/{lobby_id}`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct EditLobby {
    /// Name of the lobby (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Timeout in seconds before an idle lobby is cleaned up (5 to 604800).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,
    /// Updated members of the lobby (max 25).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<LobbyMemberRequest>>,
    /// Key-value metadata for the lobby (max 25 entries, values max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Bitfield flags for the lobby.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Custom webhook URL to receive lobby lifecycle events (max 512 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_event_webhooks_url: Option<String>,
}

impl EditLobby {
    /// Validate edit lobby request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(n) = &self.name {
            validate_name(n)?;
        }
        if let Some(t) = self.idle_timeout_seconds {
            validate_timeout(t)?;
        }
        if let Some(members) = &self.members {
            if members.len() > 25 {
                return Err(LobbyError::TooManyMembers { count: members.len() });
            }
            for m in members {
                m.validate()?;
            }
        }
        if let Some(m) = &self.metadata {
            validate_metadata(m)?;
        }
        if let Some(u) = &self.override_event_webhooks_url {
            let len = u.chars().count();
            if len > 512 {
                return Err(LobbyError::BadWebhookUrl { len });
            }
        }
        Ok(())
    }
}

/// Request payload for linking a channel to a lobby via `PATCH /lobbies/{lobby_id}/channel-linking`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct EditLobbyChannelLink {
    /// The guild channel id to link, or `None` to unlink.
    #[serde(serialize_with = "crate::message::ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
}

/// Request payload for adding or updating a member via `PUT /lobbies/{lobby_id}/members/{user_id}`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct AddLobbyMember {
    /// Key-value metadata for the member (max 25 entries, values max 1024 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
    /// Flags for this member.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Additional display name (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_name: Option<String>,
}

impl AddLobbyMember {
    /// Validate add lobby member request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(m) = &self.metadata {
            validate_metadata(m)?;
        }
        if let Some(n) = &self.additional_name {
            validate_additional_name(n)?;
        }
        Ok(())
    }
}

/// Request payload for creating a lobby message via `POST /lobbies/{lobby_id}/messages`.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct CreateLobbyMessage {
    /// Text content (max 4000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Up to 10 rich embeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    /// Allowed mentions configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<crate::message::AllowedMentions>,
    /// Up to 3 sticker IDs.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflakes")]
    pub sticker_ids: Option<Vec<Snowflake>>,
    /// Message flags bitfield.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
    /// Nonce for message deduplication (max 25 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Whether to enforce nonce uniqueness.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_nonce: Option<bool>,
    /// Text-to-speech flag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
}

impl CreateLobbyMessage {
    /// Validate create lobby message payload.
    pub fn validate(&self) -> Result<(), LobbyError> {
        if let Some(c) = &self.content {
            let len = c.chars().count();
            if len > 4000 {
                return Err(LobbyError::BadContent { len });
            }
        }
        if let Some(embeds) = &self.embeds {
            if embeds.len() > 10 {
                return Err(LobbyError::TooManyEmbeds { count: embeds.len() });
            }
        }
        if let Some(stickers) = &self.sticker_ids {
            if stickers.len() > 3 {
                return Err(LobbyError::TooManyStickers { count: stickers.len() });
            }
        }
        if let Some(n) = &self.nonce {
            let len = n.chars().count();
            if len > 25 {
                return Err(LobbyError::BadNonce { len });
            }
        }
        Ok(())
    }
}

/// Request body for unmerging a provisional account via `POST /partner-sdk/provisional-accounts/unmerge`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct UnmergeProvisionalAccount {
    /// Client ID of your application.
    #[serde(serialize_with = "ser_snowflake")]
    pub client_id: Snowflake,
    /// Client secret of your application (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// External authentication token (1-10240 characters).
    pub external_auth_token: String,
    /// Type of external authentication identity provider.
    pub external_auth_type: ApplicationIdentityProviderAuthType,
}

impl UnmergeProvisionalAccount {
    /// Validate unmerge provisional account request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        let tok_len = self.external_auth_token.chars().count();
        if !(1..=10240).contains(&tok_len) {
            return Err(LobbyError::BadExternalAuthToken { len: tok_len });
        }
        if let Some(sec) = &self.client_secret {
            let len = sec.chars().count();
            if len > 1024 {
                return Err(LobbyError::BadClientSecret { len });
            }
        }
        Ok(())
    }
}

/// Request body for bot unmerging a provisional account via `POST /partner-sdk/provisional-accounts/unmerge/bot`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BotUnmergeProvisionalAccount {
    /// External third-party user ID (1-1024 characters).
    pub external_user_id: String,
}

impl BotUnmergeProvisionalAccount {
    /// Validate bot unmerge provisional account request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        let len = self.external_user_id.chars().count();
        if !(1..=1024).contains(&len) {
            return Err(LobbyError::BadExternalUserId { len });
        }
        Ok(())
    }
}

/// Request body for acquiring a partner SDK token via `POST /partner-sdk/token`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PartnerSdkToken {
    /// Client ID of your application.
    #[serde(serialize_with = "ser_snowflake")]
    pub client_id: Snowflake,
    /// Client secret of your application (max 1024 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    /// External authentication token (1-10240 characters).
    pub external_auth_token: String,
    /// Type of external authentication identity provider.
    pub external_auth_type: ApplicationIdentityProviderAuthType,
}

impl PartnerSdkToken {
    /// Validate partner SDK token request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        let tok_len = self.external_auth_token.chars().count();
        if !(1..=10240).contains(&tok_len) {
            return Err(LobbyError::BadExternalAuthToken { len: tok_len });
        }
        if let Some(sec) = &self.client_secret {
            let len = sec.chars().count();
            if len > 1024 {
                return Err(LobbyError::BadClientSecret { len });
            }
        }
        Ok(())
    }
}

/// Request body for bot-acquired partner SDK token via `POST /partner-sdk/token/bot`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct BotPartnerSdkToken {
    /// External third-party user ID (1-1024 characters).
    pub external_user_id: String,
    /// Provisional user snowflake ID if linking to an existing account.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub provisional_user_id: Option<Snowflake>,
    /// Preferred global display name (1-32 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_global_name: Option<String>,
}

impl BotPartnerSdkToken {
    /// Validate bot partner SDK token request.
    pub fn validate(&self) -> Result<(), LobbyError> {
        let len = self.external_user_id.chars().count();
        if !(1..=1024).contains(&len) {
            return Err(LobbyError::BadExternalUserId { len });
        }
        if let Some(name) = &self.preferred_global_name {
            let n_len = name.chars().count();
            if !(1..=32).contains(&n_len) {
                return Err(LobbyError::BadPreferredGlobalName { len: n_len });
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Client Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl<T: Transport> RestClient<T> {
    // ── Lobbies (15 operations) ──────────────────────────────────────────────

    /// `PUT /lobbies` – Create or join a lobby with a secret token.
    pub async fn create_or_join_lobby(&self, body: &CreateOrJoinLobby) -> Result<LobbyResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PUT", "/lobbies", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /lobbies` – Create a new lobby.
    pub async fn create_lobby(&self, body: &CreateLobby) -> Result<LobbyResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/lobbies", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /lobbies/{lobby_id}` – Fetch a lobby by ID.
    pub async fn get_lobby(&self, lobby_id: Snowflake) -> Result<LobbyResponse, Error> {
        let path = format!("/lobbies/{lobby_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /lobbies/{lobby_id}` – Delete a lobby.
    pub async fn delete_lobby(&self, lobby_id: Snowflake) -> Result<(), Error> {
        let path = format!("/lobbies/{lobby_id}");
        self.request("DELETE", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `PATCH /lobbies/{lobby_id}` – Edit an existing lobby.
    pub async fn edit_lobby(&self, lobby_id: Snowflake, body: &EditLobby) -> Result<LobbyResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}");
        self.request("PATCH", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `PATCH /lobbies/{lobby_id}/channel-linking` – Link or unlink a guild channel from a lobby.
    pub async fn edit_lobby_channel_link(
        &self,
        lobby_id: Snowflake,
        body: &EditLobbyChannelLink,
    ) -> Result<LobbyResponse, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}/channel-linking");
        self.request("PATCH", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /lobbies/{lobby_id}/members/@me` – Leave a lobby.
    pub async fn leave_lobby(&self, lobby_id: Snowflake) -> Result<(), Error> {
        let path = format!("/lobbies/{lobby_id}/members/@me");
        self.request("DELETE", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /lobbies/{lobby_id}/members/@me/invites` – Create a linked lobby guild invite for the current user.
    pub async fn create_linked_lobby_guild_invite_for_self(
        &self,
        lobby_id: Snowflake,
    ) -> Result<LobbyGuildInviteResponse, Error> {
        let path = format!("/lobbies/{lobby_id}/members/@me/invites");
        self.request("POST", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /lobbies/{lobby_id}/members/bulk` – Bulk update lobby members.
    pub async fn bulk_update_lobby_members(
        &self,
        lobby_id: Snowflake,
        members: &[BulkLobbyMemberRequest],
    ) -> Result<Vec<LobbyMemberResponse>, Error> {
        if !(1..=25).contains(&members.len()) {
            return Err(LobbyError::InvalidBulkMembersCount { count: members.len() }.into());
        }
        for m in members {
            m.validate()?;
        }
        let b = serde_json::to_vec(members).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}/members/bulk");
        self.request("POST", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `PUT /lobbies/{lobby_id}/members/{user_id}` – Add or modify a lobby member.
    pub async fn add_lobby_member(
        &self,
        lobby_id: Snowflake,
        user_id: Snowflake,
        body: &AddLobbyMember,
    ) -> Result<LobbyMemberResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}/members/{user_id}");
        self.request("PUT", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /lobbies/{lobby_id}/members/{user_id}` – Remove a member from a lobby.
    pub async fn delete_lobby_member(
        &self,
        lobby_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/lobbies/{lobby_id}/members/{user_id}");
        self.request("DELETE", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /lobbies/{lobby_id}/members/{user_id}/invites` – Create a linked lobby guild invite for a specific user.
    pub async fn create_linked_lobby_guild_invite_for_user(
        &self,
        lobby_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<LobbyGuildInviteResponse, Error> {
        let path = format!("/lobbies/{lobby_id}/members/{user_id}/invites");
        self.request("POST", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /lobbies/{lobby_id}/messages` – Fetch messages in a lobby channel.
    pub async fn get_lobby_messages(
        &self,
        lobby_id: Snowflake,
        limit: Option<u16>,
    ) -> Result<Vec<LobbyMessageResponse>, Error> {
        if let Some(l) = limit {
            if !(1..=100).contains(&l) {
                return Err(LobbyError::BadLimit { limit: l }.into());
            }
        }
        let path = match limit {
            Some(l) => format!("/lobbies/{lobby_id}/messages?limit={l}"),
            None => format!("/lobbies/{lobby_id}/messages"),
        };
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /lobbies/{lobby_id}/messages` – Send a message in a lobby channel.
    pub async fn create_lobby_message(
        &self,
        lobby_id: Snowflake,
        body: &CreateLobbyMessage,
    ) -> Result<LobbyMessageResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}/messages");
        self.request("POST", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `PUT /lobbies/{lobby_id}/messages/{message_id}/moderation-metadata` – Attach external moderation metadata to a lobby message.
    pub async fn update_lobby_message_external_moderation_metadata(
        &self,
        lobby_id: Snowflake,
        message_id: Snowflake,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(), Error> {
        validate_moderation_metadata(metadata)?;
        let b = serde_json::to_vec(metadata).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/lobbies/{lobby_id}/messages/{message_id}/moderation-metadata");
        self.request("PUT", &path, Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    // ── Partner SDK (5 operations) ───────────────────────────────────────────

    /// `PUT /partner-sdk/dms/{user_id_1}/{user_id_2}/messages/{message_id}/moderation-metadata` – Attach external moderation metadata to a user DM message.
    pub async fn update_user_message_external_moderation_metadata(
        &self,
        user_id_1: Snowflake,
        user_id_2: Snowflake,
        message_id: Snowflake,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(), Error> {
        validate_moderation_metadata(metadata)?;
        let b = serde_json::to_vec(metadata).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/partner-sdk/dms/{user_id_1}/{user_id_2}/messages/{message_id}/moderation-metadata");
        self.request("PUT", &path, Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /partner-sdk/provisional-accounts/unmerge` – Unmerge a provisional partner SDK account using client credentials.
    pub async fn partner_sdk_unmerge_provisional_account(
        &self,
        body: &UnmergeProvisionalAccount,
    ) -> Result<(), Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/partner-sdk/provisional-accounts/unmerge", Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /partner-sdk/provisional-accounts/unmerge/bot` – Unmerge a provisional partner SDK account using bot authentication.
    pub async fn bot_partner_sdk_unmerge_provisional_account(
        &self,
        body: &BotUnmergeProvisionalAccount,
    ) -> Result<(), Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/partner-sdk/provisional-accounts/unmerge/bot", Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /partner-sdk/token` – Exchange an external auth token for a partner SDK token.
    pub async fn partner_sdk_token(
        &self,
        body: &PartnerSdkToken,
    ) -> Result<ProvisionalTokenResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/partner-sdk/token", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /partner-sdk/token/bot` – Obtain a partner SDK token using bot authorization.
    pub async fn bot_partner_sdk_token(
        &self,
        body: &BotPartnerSdkToken,
    ) -> Result<ProvisionalTokenResponse, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/partner-sdk/token/bot", Some(b))
            .await?
            .expect_success()?
            .json()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};
    use serde_json::json;

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    fn sample_lobby_json() -> String {
        json!({
            "id": "111111111111111111",
            "application_id": "222222222222222222",
            "metadata": { "game_mode": "battle_royale" },
            "members": [
                {
                    "id": "333333333333333333",
                    "metadata": { "ready": "true" },
                    "flags": 1,
                    "additional_name": "PlayerOne"
                }
            ],
            "flags": 1,
            "override_event_webhooks_url": "https://example.com/webhook"
        })
        .to_string()
    }

    fn sample_member_json() -> String {
        json!({
            "id": "333333333333333333",
            "metadata": { "ready": "true" },
            "flags": 1,
            "additional_name": "PlayerOne"
        })
        .to_string()
    }

    fn sample_invite_json() -> String {
        json!({ "code": "rift-invite-xyz" }).to_string()
    }

    fn sample_message_json() -> String {
        json!({
            "id": "444444444444444444",
            "type": 0,
            "content": "Hello lobby!",
            "lobby_id": "111111111111111111",
            "channel_id": "555555555555555555",
            "author": {
                "id": "333333333333333333",
                "username": "tester",
                "discriminator": "0001",
                "avatar": null
            },
            "lobby_member": {
                "additional_name": "PlayerOne"
            },
            "flags": 0
        })
        .to_string()
    }

    fn sample_token_json() -> String {
        json!({
            "token_type": "Bearer",
            "access_token": "mock_access_token_123",
            "expires_in": 3600,
            "scope": "identify",
            "id_token": "mock_id_token_456"
        })
        .to_string()
    }

    #[tokio::test(start_paused = true)]
    async fn test_all_15_lobby_routes_pinned() {
        let c = client(vec![
            resp(200, &sample_lobby_json()),   // 1. PUT /lobbies
            resp(201, &sample_lobby_json()),   // 2. POST /lobbies
            resp(200, &sample_lobby_json()),   // 3. GET /lobbies/{id}
            resp(204, ""),                     // 4. DELETE /lobbies/{id}
            resp(200, &sample_lobby_json()),   // 5. PATCH /lobbies/{id}
            resp(200, &sample_lobby_json()),   // 6. PATCH /lobbies/{id}/channel-linking
            resp(204, ""),                     // 7. DELETE /lobbies/{id}/members/@me
            resp(200, &sample_invite_json()),  // 8. POST /lobbies/{id}/members/@me/invites
            resp(200, &format!("[{}]", sample_member_json())), // 9. POST /lobbies/{id}/members/bulk
            resp(200, &sample_member_json()),  // 10. PUT /lobbies/{id}/members/{user_id}
            resp(204, ""),                     // 11. DELETE /lobbies/{id}/members/{user_id}
            resp(200, &sample_invite_json()),  // 12. POST /lobbies/{id}/members/{user_id}/invites
            resp(200, &format!("[{}]", sample_message_json())), // 13. GET /lobbies/{id}/messages
            resp(201, &sample_message_json()), // 14. POST /lobbies/{id}/messages
            resp(204, ""),                     // 15. PUT /lobbies/{id}/messages/{message_id}/moderation-metadata
        ]);

        let lobby_id = sf(111111111111111111);
        let user_id = sf(333333333333333333);
        let msg_id = sf(444444444444444444);

        // 1. PUT /lobbies
        let r1 = c
            .create_or_join_lobby(&CreateOrJoinLobby {
                secret: "secret-123".into(),
                idle_timeout_seconds: Some(60),
                lobby_metadata: None,
                member_metadata: None,
                flags: Some(1),
                name: Some("Custom Match".into()),
            })
            .await
            .unwrap();
        assert_eq!(r1.id, lobby_id);

        // 2. POST /lobbies
        let r2 = c
            .create_lobby(&CreateLobby {
                name: Some("Lobby Alpha".into()),
                idle_timeout_seconds: Some(120),
                members: Some(vec![LobbyMemberRequest {
                    id: user_id,
                    metadata: None,
                    flags: Some(1),
                    additional_name: Some("Lead".into()),
                }]),
                metadata: None,
                flags: Some(1),
                override_event_webhooks_url: Some("https://example.com/hook".into()),
            })
            .await
            .unwrap();
        assert_eq!(r2.id, lobby_id);

        // 3. GET /lobbies/{id}
        let r3 = c.get_lobby(lobby_id).await.unwrap();
        assert_eq!(r3.id, lobby_id);

        // 4. DELETE /lobbies/{id}
        c.delete_lobby(lobby_id).await.unwrap();

        // 5. PATCH /lobbies/{id}
        let r5 = c
            .edit_lobby(
                lobby_id,
                &EditLobby {
                    name: Some("Lobby Beta".into()),
                    idle_timeout_seconds: Some(300),
                    members: None,
                    metadata: None,
                    flags: None,
                    override_event_webhooks_url: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(r5.id, lobby_id);

        // 6. PATCH /lobbies/{id}/channel-linking
        let r6 = c
            .edit_lobby_channel_link(
                lobby_id,
                &EditLobbyChannelLink {
                    channel_id: Some(sf(999999999999999999)),
                },
            )
            .await
            .unwrap();
        assert_eq!(r6.id, lobby_id);

        // 7. DELETE /lobbies/{id}/members/@me
        c.leave_lobby(lobby_id).await.unwrap();

        // 8. POST /lobbies/{id}/members/@me/invites
        let r8 = c.create_linked_lobby_guild_invite_for_self(lobby_id).await.unwrap();
        assert_eq!(r8.code, "rift-invite-xyz");

        // 9. POST /lobbies/{id}/members/bulk
        let r9 = c
            .bulk_update_lobby_members(
                lobby_id,
                &[BulkLobbyMemberRequest {
                    id: user_id,
                    metadata: None,
                    flags: Some(1),
                    additional_name: Some("PlayerOne".into()),
                    remove_member: Some(false),
                }],
            )
            .await
            .unwrap();
        assert_eq!(r9.len(), 1);

        // 10. PUT /lobbies/{id}/members/{user_id}
        let r10 = c
            .add_lobby_member(
                lobby_id,
                user_id,
                &AddLobbyMember {
                    metadata: None,
                    flags: Some(1),
                    additional_name: Some("PlayerOne".into()),
                },
            )
            .await
            .unwrap();
        assert_eq!(r10.id, user_id);

        // 11. DELETE /lobbies/{id}/members/{user_id}
        c.delete_lobby_member(lobby_id, user_id).await.unwrap();

        // 12. POST /lobbies/{id}/members/{user_id}/invites
        let r12 = c
            .create_linked_lobby_guild_invite_for_user(lobby_id, user_id)
            .await
            .unwrap();
        assert_eq!(r12.code, "rift-invite-xyz");

        // 13. GET /lobbies/{id}/messages
        let r13 = c.get_lobby_messages(lobby_id, Some(50)).await.unwrap();
        assert_eq!(r13.len(), 1);

        // 14. POST /lobbies/{id}/messages
        let r14 = c
            .create_lobby_message(
                lobby_id,
                &CreateLobbyMessage {
                    content: Some("Hello lobby!".into()),
                    embeds: None,
                    allowed_mentions: None,
                    sticker_ids: None,
                    flags: None,
                    nonce: Some("nonce123".into()),
                    enforce_nonce: Some(true),
                    tts: Some(false),
                },
            )
            .await
            .unwrap();
        assert_eq!(r14.id, msg_id);

        // 15. PUT /lobbies/{id}/messages/{message_id}/moderation-metadata
        let mut mod_meta = BTreeMap::new();
        mod_meta.insert("flagged".into(), "false".into());
        c.update_lobby_message_external_moderation_metadata(lobby_id, msg_id, &mod_meta)
            .await
            .unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 15);
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/lobbies");

        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/lobbies");

        assert_eq!(seen[2].method, "GET");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/lobbies/111111111111111111");

        assert_eq!(seen[3].method, "DELETE");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/lobbies/111111111111111111");

        assert_eq!(seen[4].method, "PATCH");
        assert_eq!(seen[4].url, "https://discord.com/api/v10/lobbies/111111111111111111");

        assert_eq!(seen[5].method, "PATCH");
        assert_eq!(seen[5].url, "https://discord.com/api/v10/lobbies/111111111111111111/channel-linking");

        assert_eq!(seen[6].method, "DELETE");
        assert_eq!(seen[6].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/@me");

        assert_eq!(seen[7].method, "POST");
        assert_eq!(seen[7].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/@me/invites");

        assert_eq!(seen[8].method, "POST");
        assert_eq!(seen[8].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/bulk");

        assert_eq!(seen[9].method, "PUT");
        assert_eq!(seen[9].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/333333333333333333");

        assert_eq!(seen[10].method, "DELETE");
        assert_eq!(seen[10].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/333333333333333333");

        assert_eq!(seen[11].method, "POST");
        assert_eq!(seen[11].url, "https://discord.com/api/v10/lobbies/111111111111111111/members/333333333333333333/invites");

        assert_eq!(seen[12].method, "GET");
        assert_eq!(seen[12].url, "https://discord.com/api/v10/lobbies/111111111111111111/messages?limit=50");

        assert_eq!(seen[13].method, "POST");
        assert_eq!(seen[13].url, "https://discord.com/api/v10/lobbies/111111111111111111/messages");

        assert_eq!(seen[14].method, "PUT");
        assert_eq!(seen[14].url, "https://discord.com/api/v10/lobbies/111111111111111111/messages/444444444444444444/moderation-metadata");
    }

    #[tokio::test(start_paused = true)]
    async fn test_all_5_partner_sdk_routes_pinned() {
        let c = client(vec![
            resp(204, ""),                     // 1. PUT /partner-sdk/dms/...
            resp(204, ""),                     // 2. POST /partner-sdk/provisional-accounts/unmerge
            resp(204, ""),                     // 3. POST /partner-sdk/provisional-accounts/unmerge/bot
            resp(200, &sample_token_json()),   // 4. POST /partner-sdk/token
            resp(200, &sample_token_json()),   // 5. POST /partner-sdk/token/bot
        ]);

        let u1 = sf(1001);
        let u2 = sf(1002);
        let msg_id = sf(2001);

        // 1. PUT moderation-metadata
        let mut meta = BTreeMap::new();
        meta.insert("rule".into(), "safety".into());
        c.update_user_message_external_moderation_metadata(u1, u2, msg_id, &meta)
            .await
            .unwrap();

        // 2. POST unmerge
        c.partner_sdk_unmerge_provisional_account(&UnmergeProvisionalAccount {
            client_id: sf(5001),
            client_secret: Some("secret".into()),
            external_auth_token: "auth_token_xyz".into(),
            external_auth_type: ApplicationIdentityProviderAuthType::Oidc,
        })
        .await
        .unwrap();

        // 3. POST unmerge bot
        c.bot_partner_sdk_unmerge_provisional_account(&BotUnmergeProvisionalAccount {
            external_user_id: "ext_user_123".into(),
        })
        .await
        .unwrap();

        // 4. POST token
        let t1 = c
            .partner_sdk_token(&PartnerSdkToken {
                client_id: sf(5001),
                client_secret: Some("secret".into()),
                external_auth_token: "auth_token_xyz".into(),
                external_auth_type: ApplicationIdentityProviderAuthType::SteamSessionTicket,
            })
            .await
            .unwrap();
        assert_eq!(t1.access_token, "mock_access_token_123");

        // 5. POST token bot
        let t2 = c
            .bot_partner_sdk_token(&BotPartnerSdkToken {
                external_user_id: "ext_user_123".into(),
                provisional_user_id: Some(sf(8001)),
                preferred_global_name: Some("GamerTag".into()),
            })
            .await
            .unwrap();
        assert_eq!(t2.access_token, "mock_access_token_123");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 5);
        assert_eq!(seen[0].method, "PUT");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/partner-sdk/dms/1001/1002/messages/2001/moderation-metadata");

        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/partner-sdk/provisional-accounts/unmerge");

        assert_eq!(seen[2].method, "POST");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/partner-sdk/provisional-accounts/unmerge/bot");

        assert_eq!(seen[3].method, "POST");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/partner-sdk/token");

        assert_eq!(seen[4].method, "POST");
        assert_eq!(seen[4].url, "https://discord.com/api/v10/partner-sdk/token/bot");
    }

    #[tokio::test(start_paused = true)]
    async fn test_validation_guards_prevent_bad_requests() {
        let c = client(vec![]);

        // Secret validation
        let bad_join = CreateOrJoinLobby {
            secret: "".into(),
            idle_timeout_seconds: None,
            lobby_metadata: None,
            member_metadata: None,
            flags: None,
            name: None,
        };
        assert!(c.create_or_join_lobby(&bad_join).await.is_err());

        // Timeout validation
        let bad_timeout = CreateLobby {
            name: None,
            idle_timeout_seconds: Some(2), // < 5
            members: None,
            metadata: None,
            flags: None,
            override_event_webhooks_url: None,
        };
        assert!(c.create_lobby(&bad_timeout).await.is_err());

        // Bulk members count validation
        assert!(c.bulk_update_lobby_members(sf(1), &[]).await.is_err());

        // Message limit validation
        assert!(c.get_lobby_messages(sf(1), Some(0)).await.is_err());
        assert!(c.get_lobby_messages(sf(1), Some(150)).await.is_err());

        // External user id validation
        assert!(c.bot_partner_sdk_unmerge_provisional_account(&BotUnmergeProvisionalAccount {
            external_user_id: "".into()
        }).await.is_err());
    }
}
