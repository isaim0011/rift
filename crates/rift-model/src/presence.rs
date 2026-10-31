//! Presence and activity models.

use crate::serde_util::{de_opt_snowflake, ser_opt_snowflake};
use crate::user::PresenceUser;
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// Active sessions on different platforms.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientStatus {
    /// The user's status on desktop (online, dnd, idle, offline).
    #[serde(default)]
    pub desktop: Option<String>,

    /// The user's status on mobile.
    #[serde(default)]
    pub mobile: Option<String>,

    /// The user's status on web.
    #[serde(default)]
    pub web: Option<String>,
}

/// Timestamps for an activity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityTimestamps {
    /// Unix time (in milliseconds) of when the activity started.
    #[serde(default)]
    pub start: Option<u64>,

    /// Unix time (in milliseconds) of when the activity ends.
    #[serde(default)]
    pub end: Option<u64>,
}

/// Emoji for an activity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEmoji {
    /// The name of the emoji.
    pub name: String,

    /// The id of the emoji.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub id: Option<Snowflake>,

    /// Whether this emoji is animated.
    #[serde(default)]
    pub animated: Option<bool>,
}

/// Party information for an activity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityParty {
    /// The id of the party.
    #[serde(default)]
    pub id: Option<String>,

    /// Used to show the party's current and maximum size.
    #[serde(default)]
    pub size: Option<[u32; 2]>,
}

/// Assets for an activity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityAssets {
    /// See Activity Asset Image.
    #[serde(default)]
    pub large_image: Option<String>,

    /// Text displayed when hovering over the large image of the activity.
    #[serde(default)]
    pub large_text: Option<String>,

    /// See Activity Asset Image.
    #[serde(default)]
    pub small_image: Option<String>,

    /// Text displayed when hovering over the small image of the activity.
    #[serde(default)]
    pub small_text: Option<String>,
}

/// Secrets for Rich Presence joining and spectating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivitySecrets {
    /// The secret for joining a party.
    #[serde(default)]
    pub join: Option<String>,

    /// The secret for spectating a game.
    #[serde(default)]
    pub spectate: Option<String>,

    /// The secret for a specific instanced match.
    #[serde(rename = "match", default)]
    pub r#match: Option<String>,
}

/// A button shown in Rich Presence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityButton {
    /// The text on the button (1-32 characters).
    pub label: String,

    /// The URL opened when clicking the button (1-512 characters).
    pub url: String,
}

/// A user's activity.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#activity-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// The activity's name.
    pub name: String,

    /// Activity type.
    #[serde(rename = "type")]
    pub kind: u8,

    /// Stream url, is validated when type=1 (Streaming).
    #[serde(default)]
    pub url: Option<String>,

    /// Unix timestamp (in milliseconds) of when the activity was added to the user's session.
    #[serde(default)]
    pub created_at: Option<u64>,

    /// Unix timestamps for start and/or end of the game.
    #[serde(default)]
    pub timestamps: Option<ActivityTimestamps>,

    /// Application id for the game.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub application_id: Option<Snowflake>,

    /// What the player is currently doing.
    #[serde(default)]
    pub details: Option<String>,

    /// The user's current party status.
    #[serde(default)]
    pub state: Option<String>,

    /// The emoji used for a custom status.
    #[serde(default)]
    pub emoji: Option<ActivityEmoji>,

    /// Information for the current party of the player.
    #[serde(default)]
    pub party: Option<ActivityParty>,

    /// Images for the presence and their hover texts.
    #[serde(default)]
    pub assets: Option<ActivityAssets>,

    /// Secrets for Rich Presence joining and spectating.
    #[serde(default)]
    pub secrets: Option<ActivitySecrets>,

    /// Whether or not the activity is an instanced game session.
    #[serde(default)]
    pub instance: Option<bool>,

    /// Activity flags bitfield.
    #[serde(default)]
    pub flags: Option<u32>,

    /// Custom buttons shown in the Rich Presence (max 2).
    #[serde(default)]
    pub buttons: Option<Vec<ActivityButton>>,
}

/// A presence update dispatched over the Gateway.
///
/// See <https://discord.com/developers/docs/topics/gateway-events#presence-update>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceUpdate {
    /// The user presence is being updated for.
    pub user: PresenceUser,

    /// ID of the guild.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Either "idle", "dnd", "online", or "offline".
    pub status: String,

    /// User's current activities.
    #[serde(default)]
    pub activities: Option<Vec<Activity>>,

    /// User's platform-dependent status.
    #[serde(default)]
    pub client_status: Option<ClientStatus>,
}
