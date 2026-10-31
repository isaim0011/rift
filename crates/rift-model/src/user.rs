//! User models.

use crate::serde_util::{de_snowflake, ser_snowflake};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

/// A Discord user object.
///
/// See <https://discord.com/developers/docs/resources/user#user-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// The user's id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The user's username, not unique across the platform.
    #[serde(default)]
    pub username: String,

    /// The user's 4-digit discord-tag.
    #[serde(default)]
    pub discriminator: Option<String>,

    /// The user's display name, if it is set.
    #[serde(default)]
    pub global_name: Option<String>,

    /// The user's avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,

    /// Whether the user belongs to an OAuth2 application.
    #[serde(default)]
    pub bot: Option<bool>,

    /// Whether the user is an Official Discord System user.
    #[serde(default)]
    pub system: Option<bool>,

    /// Whether the user has two factor authentication enabled on their account.
    #[serde(default)]
    pub mfa_enabled: Option<bool>,

    /// The user's banner hash.
    #[serde(default)]
    pub banner: Option<String>,

    /// The user's banner color encoded as an integer representation of hexadecimal color code.
    #[serde(default)]
    pub accent_color: Option<u32>,

    /// The user's chosen language option.
    #[serde(default)]
    pub locale: Option<String>,

    /// Whether the email on this account has been verified.
    #[serde(default)]
    pub verified: Option<bool>,

    /// The user's email.
    #[serde(default)]
    pub email: Option<String>,

    /// The flags on a user's account.
    #[serde(default)]
    pub flags: Option<u64>,

    /// The type of Nitro subscription on a user's account.
    #[serde(default)]
    pub premium_type: Option<u8>,

    /// The public flags on a user's account.
    #[serde(default)]
    pub public_flags: Option<u64>,

    /// Data for the user's avatar decoration.
    #[serde(default)]
    pub avatar_decoration_data: Option<serde_json::Value>,
}

/// Partial user information sent inside presence updates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceUser {
    /// The user's id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// The user's username.
    #[serde(default)]
    pub username: Option<String>,

    /// The user's discriminator.
    #[serde(default)]
    pub discriminator: Option<String>,

    /// The user's avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,

    /// The user's display name.
    #[serde(default)]
    pub global_name: Option<String>,
}
