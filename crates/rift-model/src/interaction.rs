//! Interaction models for gateway dispatch events.

use crate::channel::Channel;
use crate::guild::GuildMember;
use crate::message::Message;
use crate::monetization::Entitlement;
use crate::serde_util::{
    de_opt_permissions, de_opt_snowflake, de_snowflake, ser_opt_permissions, ser_opt_snowflake,
    ser_snowflake,
};
use crate::user::User;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

/// What kind of interaction this is.
///
/// See <https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-type>
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionType {
    /// Ping.
    Ping,
    /// Slash, user, or message command.
    ApplicationCommand,
    /// Button, select menu, etc.
    MessageComponent,
    /// Autocomplete for application commands.
    ApplicationCommandAutocomplete,
    /// Modal submission.
    ModalSubmit,
    /// Unknown or future interaction type.
    Unknown(u8),
}

impl InteractionType {
    /// Convert from a raw integer.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Ping,
            2 => Self::ApplicationCommand,
            3 => Self::MessageComponent,
            4 => Self::ApplicationCommandAutocomplete,
            5 => Self::ModalSubmit,
            other => Self::Unknown(other),
        }
    }

    /// Convert to raw integer.
    pub const fn to_u8(self) -> u8 {
        match self {
            Self::Ping => 1,
            Self::ApplicationCommand => 2,
            Self::MessageComponent => 3,
            Self::ApplicationCommandAutocomplete => 4,
            Self::ModalSubmit => 5,
            Self::Unknown(other) => other,
        }
    }
}

impl<'de> Deserialize<'de> for InteractionType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::from_u8(u8::deserialize(d)?))
    }
}

impl Serialize for InteractionType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.to_u8())
    }
}

/// An interaction received over the Gateway (`INTERACTION_CREATE`).
///
/// See <https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object>
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interaction {
    /// Interaction id.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Application id this interaction is for.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub application_id: Snowflake,

    /// Type of interaction.
    #[serde(rename = "type")]
    pub kind: InteractionType,

    /// Interaction data payload (kept raw for lazy deserialization).
    #[serde(default)]
    pub data: Option<Box<RawValue>>,

    /// Guild that the interaction was sent from.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub guild_id: Option<Snowflake>,

    /// Channel that the interaction was sent from.
    #[serde(
        default,
        deserialize_with = "de_opt_snowflake",
        serialize_with = "ser_opt_snowflake"
    )]
    pub channel_id: Option<Snowflake>,

    /// Channel object where the interaction was invoked.
    #[serde(default)]
    pub channel: Option<Channel>,

    /// Guild member data for the invoking user, including permissions (present in guild interactions).
    #[serde(default)]
    pub member: Option<GuildMember>,

    /// User data for the invoking user (present in DM interactions).
    #[serde(default)]
    pub user: Option<User>,

    /// Continuation token for responding to the interaction.
    pub token: String,

    /// Read-only property, always `1`.
    #[serde(default = "default_version")]
    pub version: u8,

    /// For components, the message they were attached to.
    #[serde(default)]
    pub message: Option<Box<Message>>,

    /// Bitwise set of permissions the app has in the source channel.
    #[serde(
        default,
        deserialize_with = "de_opt_permissions",
        serialize_with = "ser_opt_permissions"
    )]
    pub app_permissions: Option<Permissions>,

    /// Selected language of the invoking user.
    #[serde(default)]
    pub locale: Option<String>,

    /// Guild's preferred locale, if invoked in a guild.
    #[serde(default)]
    pub guild_locale: Option<String>,

    /// For monetized apps, any entitlements for the invoking user.
    #[serde(default)]
    pub entitlements: Option<Vec<Entitlement>>,

    /// Mapping of installation contexts to authorized snowflake IDs.
    #[serde(default)]
    pub authorizing_integration_owners: Option<serde_json::Value>,

    /// Context where the interaction was triggered.
    #[serde(default)]
    pub context: Option<u8>,
}

fn default_version() -> u8 {
    1
}

impl Interaction {
    /// Returns the invoking user, whether from `member` (guild) or `user` (DM).
    pub fn invoking_user(&self) -> Option<&User> {
        self.member
            .as_ref()
            .and_then(|m| m.user.as_ref())
            .or(self.user.as_ref())
    }

    /// The invoking member's permissions in this channel (if present).
    pub fn member_permissions(&self) -> Option<Permissions> {
        self.member.as_ref().and_then(|m| m.permissions)
    }

    /// Permissions the application has in this channel (if present).
    pub fn app_permissions(&self) -> Option<Permissions> {
        self.app_permissions
    }
}

impl PartialEq for Interaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.application_id == other.application_id
            && self.kind == other.kind
            && self.token == other.token
            && self.version == other.version
            && self.guild_id == other.guild_id
            && self.channel_id == other.channel_id
            && self.channel == other.channel
            && self.member == other.member
            && self.user == other.user
            && self.message == other.message
            && self.app_permissions == other.app_permissions
            && self.locale == other.locale
            && self.guild_locale == other.guild_locale
            && self.entitlements == other.entitlements
            && self.authorizing_integration_owners == other.authorizing_integration_owners
            && self.context == other.context
            && match (&self.data, &other.data) {
                (Some(a), Some(b)) => a.get() == b.get(),
                (None, None) => true,
                _ => false,
            }
    }
}

/// An application command permission overwrite.
///
/// See <https://discord.com/developers/docs/interactions/application-commands#application-command-permissions-object>
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationCommandPermission {
    /// ID of the role, user, or channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,

    /// Role (`1`), user (`2`), or channel (`3`).
    #[serde(rename = "type")]
    pub kind: u8,

    /// `true` to allow, `false`, to disallow.
    pub permission: bool,
}
