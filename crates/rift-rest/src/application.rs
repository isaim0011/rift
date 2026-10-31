//! Application resource endpoints.
//!
//! Complete coverage for Discord applications according to the official documentation:
//! - Get/Modify Current Application (@me), Get/Modify Application by ID
//! - Get Activity Instance, Upload Activity Attachment
//! - Get/Edit Global and Guild Application Commands
//! - List/Create/Get/Modify/Delete Application Emojis
//! - List/Create/Get/Delete/Consume Entitlements
//! - List/Get/Set Guild Command Permissions
//! - Get/Update Application Role Connection Metadata Records

use crate::client::RestClient;
use crate::command::{IntegrationType, InteractionContext, OptionSpec, RegisteredCommand};
use crate::error::Error;
use crate::interaction::User;
use crate::message::{de_opt_snowflake, de_snowflake, ser_snowflake};
use crate::transport::Transport;
use rift_core::{Permissions, Snowflake};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt;

// ---------- Helper serializers/deserializers ----------

fn de_perm_str<'de, D: Deserializer<'de>>(d: D) -> Result<Permissions, D::Error> {
    let s = String::deserialize(d)?;
    s.parse::<Permissions>().map_err(serde::de::Error::custom)
}

fn ser_perm_str<S: Serializer>(v: &Permissions, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

fn ser_opt_perm_str<S: Serializer>(v: &Option<Option<Permissions>>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        Some(Some(p)) => s.collect_str(p),
        Some(None) => s.serialize_none(),
        None => s.serialize_none(),
    }
}

fn de_snowflakes<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let strings = Vec::<String>::deserialize(d)?;
    strings
        .into_iter()
        .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
        .collect()
}

fn append_query(path: &mut String, queries: &[(&str, Option<String>)]) {
    let mut first = true;
    for (key, val) in queries {
        if let Some(v) = val {
            if first {
                path.push('?');
                first = false;
            } else {
                path.push('&');
            }
            path.push_str(key);
            path.push('=');
            path.push_str(v);
        }
    }
}

// ---------- Application Data Structures ----------

/// Install parameters for an application.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct InstallParams {
    /// Scopes to authorize for this application.
    pub scopes: Vec<String>,
    /// Default permissions requested for the bot user.
    #[serde(deserialize_with = "de_perm_str", serialize_with = "ser_perm_str")]
    pub permissions: Permissions,
}

/// A member of a developer team.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TeamMember {
    /// User data for the team member.
    pub user: User,
    /// Team id.
    #[serde(deserialize_with = "de_snowflake")]
    pub team_id: Snowflake,
    /// Membership state (1 = Invited, 2 = Accepted).
    pub membership_state: u8,
    /// Role within the team.
    #[serde(default)]
    pub role: String,
    /// Permissions list.
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// A developer team owning an application.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Team {
    /// Team id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Team icon hash.
    pub icon: Option<String>,
    /// Team name.
    pub name: String,
    /// User id of the team owner.
    #[serde(deserialize_with = "de_snowflake")]
    pub owner_user_id: Snowflake,
    /// Members of the team.
    #[serde(default)]
    pub members: Vec<TeamMember>,
}

/// A Discord Application object.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Application {
    /// Application id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Name of the app.
    pub name: String,
    /// Icon hash.
    pub icon: Option<String>,
    /// Description of the app.
    #[serde(default)]
    pub description: String,
    /// Cover image hash.
    #[serde(default)]
    pub cover_image: Option<String>,
    /// Primary SKU id, if the app has SKUs.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub primary_sku_id: Option<Snowflake>,
    /// Public flags bitfield.
    #[serde(default)]
    pub flags: Option<u64>,
    /// Bot user associated with the app.
    #[serde(default)]
    pub bot: Option<User>,
    /// Slug for the app store.
    #[serde(default)]
    pub slug: Option<String>,
    /// Guild id if linked to a guild.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// RPC origins list.
    #[serde(default)]
    pub rpc_origins: Option<Vec<String>>,
    /// Whether the bot can be invited by anyone.
    #[serde(default)]
    pub bot_public: Option<bool>,
    /// Whether the bot requires OAuth2 code grant.
    #[serde(default)]
    pub bot_require_code_grant: Option<bool>,
    /// Terms of service URL.
    #[serde(default)]
    pub terms_of_service_url: Option<String>,
    /// Privacy policy URL.
    #[serde(default)]
    pub privacy_policy_url: Option<String>,
    /// Custom install URL.
    #[serde(default)]
    pub custom_install_url: Option<String>,
    /// Default install params.
    #[serde(default)]
    pub install_params: Option<InstallParams>,
    /// Public hex key for verifying interaction signatures.
    #[serde(default)]
    pub verify_key: Option<String>,
    /// Max participants in an activity.
    #[serde(default)]
    pub max_participants: Option<u32>,
    /// Tags for the app directory.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Redirect URIs for OAuth2.
    #[serde(default)]
    pub redirect_uris: Option<Vec<String>>,
    /// Interactions endpoint URL.
    #[serde(default)]
    pub interactions_endpoint_url: Option<String>,
    /// Role connections verification URL.
    #[serde(default)]
    pub role_connections_verification_url: Option<String>,
    /// Owner user.
    #[serde(default)]
    pub owner: Option<User>,
    /// Approximate count of guilds the bot is in.
    #[serde(default)]
    pub approximate_guild_count: Option<u32>,
    /// Approximate count of user installs.
    #[serde(default)]
    pub approximate_user_install_count: Option<u32>,
    /// Team owning the application.
    #[serde(default)]
    pub team: Option<Team>,
}

/// Parameters for modifying an application.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyApplication {
    /// Custom install URL (max 2048 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_install_url: Option<String>,
    /// Description (max 400 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Role connections verification URL (max 2048 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_connections_verification_url: Option<String>,
    /// Interactions endpoint URL (max 2048 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions_endpoint_url: Option<String>,
    /// Event webhooks URL (max 2048 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_url: Option<String>,
    /// Tags for app directory (up to 5 tags, max 20 characters each).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Icon base64 data URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Cover image base64 data URI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    /// Application flags.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

impl ModifyApplication {
    /// Create empty modify request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set description (max 400 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set custom install URL (max 2048 characters).
    pub fn custom_install_url(mut self, url: impl Into<String>) -> Self {
        self.custom_install_url = Some(url.into());
        self
    }

    /// Set interactions endpoint URL.
    pub fn interactions_endpoint_url(mut self, url: impl Into<String>) -> Self {
        self.interactions_endpoint_url = Some(url.into());
        self
    }

    /// Set tags (up to 5 tags, max 20 chars each).
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Validate parameters before sending.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        if let Some(desc) = &self.description {
            if desc.chars().count() > 400 {
                return Err(ApplicationError::DescriptionTooLong { len: desc.chars().count() });
            }
        }
        if let Some(tags) = &self.tags {
            if tags.len() > 5 {
                return Err(ApplicationError::TooManyTags { count: tags.len() });
            }
            for tag in tags {
                if tag.chars().count() > 20 {
                    return Err(ApplicationError::TagTooLong { len: tag.chars().count() });
                }
            }
        }
        Ok(())
    }
}

// ---------- Activity Instances & Attachments ----------

/// Location of an activity instance.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ActivityLocation {
    /// Unique identifier for the location.
    pub id: String,
    /// Channel id where the activity is running.
    #[serde(deserialize_with = "de_snowflake")]
    pub channel_id: Snowflake,
    /// Guild id if the channel is in a guild.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
}

/// An embedded activity instance.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ActivityInstance {
    /// Application id of the activity.
    #[serde(deserialize_with = "de_snowflake")]
    pub application_id: Snowflake,
    /// Instance id.
    pub instance_id: String,
    /// Launch id.
    pub launch_id: String,
    /// Location of the activity.
    pub location: ActivityLocation,
    /// User IDs currently participating.
    #[serde(deserialize_with = "de_snowflakes")]
    pub users: Vec<Snowflake>,
}

/// Attachment data returned by the activities attachment endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    /// Attachment snowflake id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Name of the uploaded file.
    pub filename: String,
    /// Size of the file in bytes.
    pub size: u64,
    /// Source URL of file.
    pub url: String,
    /// Proxied URL of file.
    pub proxy_url: String,
    /// Description for the file.
    #[serde(default)]
    pub description: Option<String>,
    /// Media type of file.
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Response returned from uploading an application attachment.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ActivitiesAttachment {
    /// The uploaded attachment.
    pub attachment: Attachment,
}

// ---------- Application Emojis ----------

/// An application emoji.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ApplicationEmoji {
    /// Emoji snowflake id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Name of the emoji.
    pub name: String,
    /// User who created the emoji.
    #[serde(default)]
    pub user: Option<User>,
    /// Roles allowed to use this emoji.
    #[serde(default, deserialize_with = "de_snowflakes")]
    pub roles: Vec<Snowflake>,
    /// Whether this emoji must be wrapped in colons.
    #[serde(default)]
    pub require_colons: Option<bool>,
    /// Whether this emoji is managed.
    #[serde(default)]
    pub managed: Option<bool>,
    /// Whether this emoji is animated.
    #[serde(default)]
    pub animated: Option<bool>,
    /// Whether this emoji can currently be used.
    #[serde(default)]
    pub available: Option<bool>,
}

/// List of application emojis.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ApplicationEmojis {
    /// The list of emojis.
    pub items: Vec<ApplicationEmoji>,
}

/// Parameters for creating an application emoji.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateApplicationEmoji {
    /// Name of the emoji (2-32 characters).
    pub name: String,
    /// The 128x128 image data URI.
    pub image: String,
}

impl CreateApplicationEmoji {
    /// Create new emoji payload.
    pub fn new(name: impl Into<String>, image: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            image: image.into(),
        }
    }

    /// Validate emoji name length (2-32 characters).
    pub fn validate(&self) -> Result<(), ApplicationError> {
        let len = self.name.chars().count();
        if !(2..=32).contains(&len) {
            return Err(ApplicationError::BadEmojiName { len });
        }
        Ok(())
    }
}

/// Parameters for updating an application emoji.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ModifyApplicationEmoji {
    /// New name of the emoji (2-32 characters).
    pub name: String,
}

impl ModifyApplicationEmoji {
    /// Create new modify emoji payload.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Validate emoji name length (2-32 characters).
    pub fn validate(&self) -> Result<(), ApplicationError> {
        let len = self.name.chars().count();
        if !(2..=32).contains(&len) {
            return Err(ApplicationError::BadEmojiName { len });
        }
        Ok(())
    }
}

// ---------- Entitlements ----------

/// The owner type for an entitlement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum EntitlementOwnerType {
    /// Guild subscription.
    Guild = 1,
    /// User subscription.
    User = 2,
}

impl Serialize for EntitlementOwnerType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for EntitlementOwnerType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            1 => Ok(Self::Guild),
            2 => Ok(Self::User),
            other => Err(serde::de::Error::custom(format!("invalid owner type: {other}"))),
        }
    }
}

pub use crate::user::Entitlement;

/// Parameters for creating a test entitlement.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateEntitlement {
    /// ID of the SKU to grant.
    #[serde(serialize_with = "ser_snowflake")]
    pub sku_id: Snowflake,
    /// ID of the user or guild.
    #[serde(serialize_with = "ser_snowflake")]
    pub owner_id: Snowflake,
    /// 1 for guild subscription, 2 for user subscription.
    pub owner_type: EntitlementOwnerType,
}

/// Query parameters for listing entitlements.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetEntitlements {
    /// Filter by user id.
    pub user_id: Option<Snowflake>,
    /// Filter by SKU IDs.
    pub sku_ids: Vec<Snowflake>,
    /// Filter by guild id.
    pub guild_id: Option<Snowflake>,
    /// Retrieve entitlements before this id.
    pub before: Option<Snowflake>,
    /// Retrieve entitlements after this id.
    pub after: Option<Snowflake>,
    /// Number of entitlements to return (1-100, default 100).
    pub limit: Option<u8>,
    /// Whether to exclude ended subscriptions.
    pub exclude_ended: Option<bool>,
    /// Whether to exclude deleted entitlements.
    pub exclude_deleted: Option<bool>,
    /// Only active entitlements.
    pub only_active: Option<bool>,
}

impl GetEntitlements {
    /// Create new query parameters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by user id.
    pub fn user_id(mut self, uid: Snowflake) -> Self {
        self.user_id = Some(uid);
        self
    }

    /// Filter by guild id.
    pub fn guild_id(mut self, gid: Snowflake) -> Self {
        self.guild_id = Some(gid);
        self
    }

    /// Set limit (1-100).
    pub fn limit(mut self, l: u8) -> Self {
        self.limit = Some(l);
        self
    }

    /// Convert to query parameter pairs.
    pub fn to_query_pairs(&self) -> Vec<(&'static str, Option<String>)> {
        let sku_ids_str = if self.sku_ids.is_empty() {
            None
        } else {
            Some(
                self.sku_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };
        vec![
            ("user_id", self.user_id.map(|id| id.to_string())),
            ("sku_ids", sku_ids_str),
            ("guild_id", self.guild_id.map(|id| id.to_string())),
            ("before", self.before.map(|id| id.to_string())),
            ("after", self.after.map(|id| id.to_string())),
            ("limit", self.limit.map(|l| l.to_string())),
            ("exclude_ended", self.exclude_ended.map(|b| b.to_string())),
            ("exclude_deleted", self.exclude_deleted.map(|b| b.to_string())),
            ("only_active", self.only_active.map(|b| b.to_string())),
        ]
    }
}

// ---------- Command Permissions ----------

/// The type of permission overwrite for a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CommandPermissionType {
    /// Role permission.
    Role = 1,
    /// User permission.
    User = 2,
    /// Channel permission.
    Channel = 3,
}

impl Serialize for CommandPermissionType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for CommandPermissionType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            1 => Ok(Self::Role),
            2 => Ok(Self::User),
            3 => Ok(Self::Channel),
            other => Err(serde::de::Error::custom(format!("invalid permission type: {other}"))),
        }
    }
}

/// A permission overwrite entry for an application command in a guild.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CommandPermission {
    /// ID of the role, user, or channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// Type of permission target.
    #[serde(rename = "type")]
    pub kind: CommandPermissionType,
    /// True to allow, false to disallow.
    pub permission: bool,
}

/// Command permissions for a command or application in a guild.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct GuildCommandPermissions {
    /// Command id (or application id for all commands).
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Application id.
    #[serde(deserialize_with = "de_snowflake")]
    pub application_id: Snowflake,
    /// Guild id.
    #[serde(deserialize_with = "de_snowflake")]
    pub guild_id: Snowflake,
    /// Array of permissions.
    pub permissions: Vec<CommandPermission>,
}

#[derive(Serialize)]
struct SetPermissionsBody<'a> {
    permissions: &'a [CommandPermission],
}

// ---------- Role Connection Metadata ----------

/// Type of role connection metadata item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RoleConnectionMetadataType {
    /// Integer <= guild configured value.
    IntegerLessThanOrEqual = 1,
    /// Integer >= guild configured value.
    IntegerGreaterThanOrEqual = 2,
    /// Integer == guild configured value.
    IntegerEqual = 3,
    /// Integer != guild configured value.
    IntegerNotEqual = 4,
    /// Datetime <= guild configured value (days before current date).
    DatetimeLessThanOrEqual = 5,
    /// Datetime >= guild configured value (days before current date).
    DatetimeGreaterThanOrEqual = 6,
    /// Boolean == guild configured value (1).
    BooleanEqual = 7,
    /// Boolean != guild configured value (1).
    BooleanNotEqual = 8,
}

impl Serialize for RoleConnectionMetadataType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for RoleConnectionMetadataType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        match v {
            1 => Ok(Self::IntegerLessThanOrEqual),
            2 => Ok(Self::IntegerGreaterThanOrEqual),
            3 => Ok(Self::IntegerEqual),
            4 => Ok(Self::IntegerNotEqual),
            5 => Ok(Self::DatetimeLessThanOrEqual),
            6 => Ok(Self::DatetimeGreaterThanOrEqual),
            7 => Ok(Self::BooleanEqual),
            8 => Ok(Self::BooleanNotEqual),
            other => Err(serde::de::Error::custom(format!("invalid metadata type: {other}"))),
        }
    }
}

/// An application role connection metadata record.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RoleConnectionMetadata {
    /// Type of metadata item.
    #[serde(rename = "type")]
    pub kind: RoleConnectionMetadataType,
    /// Dictionary key for the metadata item (1-50 characters: a-z, 0-9, _).
    pub key: String,
    /// Name of the metadata field (1-100 characters).
    pub name: String,
    /// Translations for the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,
    /// Description of the metadata field (1-200 characters).
    pub description: String,
    /// Translations for the description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
}

impl RoleConnectionMetadata {
    /// Create a new metadata item.
    pub fn new(
        kind: RoleConnectionMetadataType,
        key: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            key: key.into(),
            name: name.into(),
            name_localizations: None,
            description: description.into(),
            description_localizations: None,
        }
    }

    /// Local validation.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        let klen = self.key.chars().count();
        if klen == 0 || klen > 50 {
            return Err(ApplicationError::BadMetadataKey { len: klen });
        }
        let nlen = self.name.chars().count();
        if nlen == 0 || nlen > 100 {
            return Err(ApplicationError::BadMetadataName { len: nlen });
        }
        let dlen = self.description.chars().count();
        if dlen == 0 || dlen > 200 {
            return Err(ApplicationError::BadMetadataDescription { len: dlen });
        }
        Ok(())
    }
}

// ---------- Command Editing ----------

/// Parameters for updating an existing application command.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct EditCommand {
    /// Name of the command (1-32 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Name localizations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,
    /// Description of the command (1-100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Description localizations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
    /// The parameters of the command (max 25).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<OptionSpec>>,
    /// Permissions required to run the command in a guild.
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_perm_str")]
    pub default_member_permissions: Option<Option<Permissions>>,
    /// Whether the command is enabled in DMs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,
    /// Contexts where the command can be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContext>>,
    /// Installation contexts where the command is available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<IntegrationType>>,
}

impl EditCommand {
    /// Create empty edit request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name (1-32 characters).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set description (1-100 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set options (max 25).
    pub fn options(mut self, opts: Vec<OptionSpec>) -> Self {
        self.options = Some(opts);
        self
    }

    /// Validate the edit request.
    pub fn validate(&self) -> Result<(), ApplicationError> {
        if let Some(n) = &self.name {
            let len = n.chars().count();
            if len == 0 || len > 32 {
                return Err(ApplicationError::BadCommandName { len });
            }
        }
        if let Some(d) = &self.description {
            let len = d.chars().count();
            if len > 100 {
                return Err(ApplicationError::BadCommandDescription { len });
            }
        }
        if let Some(opts) = &self.options {
            if opts.len() > 25 {
                return Err(ApplicationError::TooManyOptions { count: opts.len() });
            }
        }
        Ok(())
    }
}

// ---------- Validation Errors ----------

/// Validation errors for application endpoints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationError {
    /// Description is too long.
    DescriptionTooLong {
        /// Actual length.
        len: usize,
    },
    /// Too many tags (max 5).
    TooManyTags {
        /// Actual count.
        count: usize,
    },
    /// A tag is too long (max 20 characters).
    TagTooLong {
        /// Actual length.
        len: usize,
    },
    /// Emoji name must be 2-32 characters.
    BadEmojiName {
        /// Actual length.
        len: usize,
    },
    /// Command name must be 1-32 characters.
    BadCommandName {
        /// Actual length.
        len: usize,
    },
    /// Command description must be 1-100 characters.
    BadCommandDescription {
        /// Actual length.
        len: usize,
    },
    /// Too many command options (max 25).
    TooManyOptions {
        /// Actual count.
        count: usize,
    },
    /// Too many role connection metadata records (max 5).
    TooManyMetadataRecords {
        /// Actual count.
        count: usize,
    },
    /// Role connection metadata key must be 1-50 characters.
    BadMetadataKey {
        /// Actual length.
        len: usize,
    },
    /// Role connection metadata name must be 1-100 characters.
    BadMetadataName {
        /// Actual length.
        len: usize,
    },
    /// Role connection metadata description must be 1-200 characters.
    BadMetadataDescription {
        /// Actual length.
        len: usize,
    },
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptionTooLong { len } => write!(f, "application description {len} chars (max 400)"),
            Self::TooManyTags { count } => write!(f, "{count} tags provided (max 5)"),
            Self::TagTooLong { len } => write!(f, "tag {len} chars (max 20)"),
            Self::BadEmojiName { len } => write!(f, "emoji name {len} chars (must be 2-32)"),
            Self::BadCommandName { len } => write!(f, "command name {len} chars (must be 1-32)"),
            Self::BadCommandDescription { len } => write!(f, "command description {len} chars (max 100)"),
            Self::TooManyOptions { count } => write!(f, "{count} options provided (max 25)"),
            Self::TooManyMetadataRecords { count } => write!(f, "{count} metadata records provided (max 5)"),
            Self::BadMetadataKey { len } => write!(f, "metadata key {len} chars (must be 1-50)"),
            Self::BadMetadataName { len } => write!(f, "metadata name {len} chars (must be 1-100)"),
            Self::BadMetadataDescription { len } => {
                write!(f, "metadata description {len} chars (must be 1-200)")
            }
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<ApplicationError> for Error {
    fn from(e: ApplicationError) -> Self {
        Error::Transport(format!("application validation: {e}"))
    }
}

// ---------- RestClient Implementation ----------

impl<T: Transport> RestClient<T> {
    /// `GET /applications/@me`: get current application.
    pub async fn get_current_application(&self) -> Result<Application, Error> {
        self.request("GET", "/applications/@me", None).await?.expect_success()?.json()
    }

    /// `PATCH /applications/@me`: modify current application.
    pub async fn modify_current_application(&self, body: &ModifyApplication) -> Result<Application, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", "/applications/@me", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /applications/{application_id}`: get an application by id.
    pub async fn get_application(&self, application_id: Snowflake) -> Result<Application, Error> {
        let path = format!("/applications/{application_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /applications/{application_id}`: modify an application by id.
    pub async fn modify_application(
        &self,
        application_id: Snowflake,
        body: &ModifyApplication,
    ) -> Result<Application, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}");
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/activity-instances/{instance_id}`: get activity instance.
    pub async fn get_activity_instance(
        &self,
        application_id: Snowflake,
        instance_id: &str,
    ) -> Result<ActivityInstance, Error> {
        let path = format!("/applications/{application_id}/activity-instances/{instance_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `POST /applications/{application_id}/attachment`: upload application attachment for activities.
    pub async fn upload_application_attachment(
        &self,
        application_id: Snowflake,
        filename: &str,
        file_data: &[u8],
        content_type: Option<&str>,
    ) -> Result<ActivitiesAttachment, Error> {
        let boundary = "----RiftMultipartBoundary7MA4YWxkTrZu0gW";
        let ct = content_type.unwrap_or("application/octet-stream");
        let mut body = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {ct}\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(file_data);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let headers = vec![(
            "Content-Type".to_owned(),
            format!("multipart/form-data; boundary={boundary}"),
        )];
        let path = format!("/applications/{application_id}/attachment");
        self.request_with_headers("POST", &path, Some(body), headers)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /applications/{application_id}/commands/{command_id}`: get a global command.
    pub async fn get_global_command(
        &self,
        application_id: Snowflake,
        command_id: Snowflake,
    ) -> Result<RegisteredCommand, Error> {
        let path = format!("/applications/{application_id}/commands/{command_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /applications/{application_id}/commands/{command_id}`: edit a global command.
    pub async fn edit_global_command(
        &self,
        application_id: Snowflake,
        command_id: Snowflake,
        edit: &EditCommand,
    ) -> Result<RegisteredCommand, Error> {
        edit.validate()?;
        let b = serde_json::to_vec(edit).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/commands/{command_id}");
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/emojis`: list application emojis.
    pub async fn list_application_emojis(
        &self,
        application_id: Snowflake,
    ) -> Result<ApplicationEmojis, Error> {
        let path = format!("/applications/{application_id}/emojis");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `POST /applications/{application_id}/emojis`: create an application emoji.
    pub async fn create_application_emoji(
        &self,
        application_id: Snowflake,
        emoji: &CreateApplicationEmoji,
    ) -> Result<ApplicationEmoji, Error> {
        emoji.validate()?;
        let b = serde_json::to_vec(emoji).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/emojis");
        self.request("POST", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/emojis/{emoji_id}`: get an application emoji.
    pub async fn get_application_emoji(
        &self,
        application_id: Snowflake,
        emoji_id: Snowflake,
    ) -> Result<ApplicationEmoji, Error> {
        let path = format!("/applications/{application_id}/emojis/{emoji_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /applications/{application_id}/emojis/{emoji_id}`: modify an application emoji.
    pub async fn modify_application_emoji(
        &self,
        application_id: Snowflake,
        emoji_id: Snowflake,
        edit: &ModifyApplicationEmoji,
    ) -> Result<ApplicationEmoji, Error> {
        edit.validate()?;
        let b = serde_json::to_vec(edit).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/emojis/{emoji_id}");
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `DELETE /applications/{application_id}/emojis/{emoji_id}`: delete an application emoji.
    pub async fn delete_application_emoji(
        &self,
        application_id: Snowflake,
        emoji_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/applications/{application_id}/emojis/{emoji_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `GET /applications/{application_id}/entitlements`: list entitlements.
    pub async fn get_entitlements(
        &self,
        application_id: Snowflake,
        params: &GetEntitlements,
    ) -> Result<Vec<Entitlement>, Error> {
        let mut path = format!("/applications/{application_id}/entitlements");
        append_query(&mut path, &params.to_query_pairs());
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `POST /applications/{application_id}/entitlements`: create a test entitlement.
    pub async fn create_test_entitlement(
        &self,
        application_id: Snowflake,
        body: &CreateEntitlement,
    ) -> Result<Entitlement, Error> {
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/entitlements");
        self.request("POST", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/entitlements/{entitlement_id}`: get an entitlement.
    pub async fn get_entitlement(
        &self,
        application_id: Snowflake,
        entitlement_id: Snowflake,
    ) -> Result<Entitlement, Error> {
        let path = format!("/applications/{application_id}/entitlements/{entitlement_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `DELETE /applications/{application_id}/entitlements/{entitlement_id}`: delete a test entitlement.
    pub async fn delete_test_entitlement(
        &self,
        application_id: Snowflake,
        entitlement_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/applications/{application_id}/entitlements/{entitlement_id}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `POST /applications/{application_id}/entitlements/{entitlement_id}/consume`: consume an entitlement.
    pub async fn consume_entitlement(
        &self,
        application_id: Snowflake,
        entitlement_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/applications/{application_id}/entitlements/{entitlement_id}/consume");
        self.request("POST", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `GET /applications/{application_id}/guilds/{guild_id}/commands/permissions`: list all command permissions for a guild.
    pub async fn list_guild_commands_permissions(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Vec<GuildCommandPermissions>, Error> {
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/permissions");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/guilds/{guild_id}/commands/{command_id}`: get a guild command.
    pub async fn get_guild_command(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command_id: Snowflake,
    ) -> Result<RegisteredCommand, Error> {
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /applications/{application_id}/guilds/{guild_id}/commands/{command_id}`: edit a guild command.
    pub async fn edit_guild_command(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command_id: Snowflake,
        edit: &EditCommand,
    ) -> Result<RegisteredCommand, Error> {
        edit.validate()?;
        let b = serde_json::to_vec(edit).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}");
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions`: get permissions for a command in a guild.
    pub async fn get_guild_command_permissions(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command_id: Snowflake,
    ) -> Result<GuildCommandPermissions, Error> {
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PUT /applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions`: set permissions for a command in a guild.
    pub async fn set_guild_command_permissions(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
        command_id: Snowflake,
        permissions: &[CommandPermission],
    ) -> Result<GuildCommandPermissions, Error> {
        let body = SetPermissionsBody { permissions };
        let b = serde_json::to_vec(&body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions");
        self.request("PUT", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `GET /applications/{application_id}/role-connections/metadata`: get role connection metadata records.
    pub async fn get_role_connection_metadata(
        &self,
        application_id: Snowflake,
    ) -> Result<Vec<RoleConnectionMetadata>, Error> {
        let path = format!("/applications/{application_id}/role-connections/metadata");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PUT /applications/{application_id}/role-connections/metadata`: update role connection metadata records.
    pub async fn update_role_connection_metadata(
        &self,
        application_id: Snowflake,
        records: &[RoleConnectionMetadata],
    ) -> Result<Vec<RoleConnectionMetadata>, Error> {
        if records.len() > 5 {
            return Err(ApplicationError::TooManyMetadataRecords { count: records.len() }.into());
        }
        for r in records {
            r.validate()?;
        }
        let b = serde_json::to_vec(records).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/applications/{application_id}/role-connections/metadata");
        self.request("PUT", &path, Some(b)).await?.expect_success()?.json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{client, resp};

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    const SAMPLE_APP: &str = r#"{
        "id": "111",
        "name": "Rift App",
        "icon": null,
        "description": "A rift app description"
    }"#;

    const SAMPLE_COMMAND: &str = r#"{
        "id": "222",
        "application_id": "111",
        "version": "1",
        "name": "testcmd",
        "description": "A test command",
        "type": 1
    }"#;

    const SAMPLE_EMOJI: &str = r#"{
        "id": "333",
        "name": "rifteyes",
        "roles": []
    }"#;

    const SAMPLE_ENTITLEMENT: &str = r#"{
        "id": "444",
        "sku_id": "555",
        "application_id": "111",
        "user_id": "777",
        "deleted": false,
        "type": 8
    }"#;

    const SAMPLE_PERMS: &str = r#"{
        "id": "222",
        "application_id": "111",
        "guild_id": "888",
        "permissions": [
            { "id": "999", "type": 1, "permission": true }
        ]
    }"#;

    #[tokio::test(start_paused = true)]
    async fn application_me_and_by_id_routes() {
        let c = client(vec![
            resp(200, SAMPLE_APP),
            resp(200, SAMPLE_APP),
            resp(200, SAMPLE_APP),
            resp(200, SAMPLE_APP),
        ]);

        let app = c.get_current_application().await.unwrap();
        assert_eq!(app.id, sf(111));

        let mod_req = ModifyApplication::new().description("Updated desc");
        let app_mod = c.modify_current_application(&mod_req).await.unwrap();
        assert_eq!(app_mod.id, sf(111));

        let app2 = c.get_application(sf(111)).await.unwrap();
        assert_eq!(app2.id, sf(111));

        let app3 = c.modify_application(sf(111), &mod_req).await.unwrap();
        assert_eq!(app3.id, sf(111));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/applications/@me");
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/applications/@me");
        assert_eq!(seen[2].method, "GET");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/applications/111");
        assert_eq!(seen[3].method, "PATCH");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/applications/111");
    }

    #[tokio::test(start_paused = true)]
    async fn activity_instance_and_attachment_routes() {
        let sample_instance = r#"{
            "application_id": "111",
            "instance_id": "inst_1",
            "launch_id": "launch_1",
            "location": { "id": "loc_1", "channel_id": "222" },
            "users": ["333"]
        }"#;
        let sample_att = r#"{
            "attachment": {
                "id": "777",
                "filename": "state.bin",
                "size": 12,
                "url": "https://cdn.discordapp.com/state.bin",
                "proxy_url": "https://media.discordapp.net/state.bin"
            }
        }"#;

        let c = client(vec![resp(200, sample_instance), resp(200, sample_att)]);

        let inst = c.get_activity_instance(sf(111), "inst_1").await.unwrap();
        assert_eq!(inst.instance_id, "inst_1");

        let att = c
            .upload_application_attachment(sf(111), "state.bin", b"hello world!", Some("application/octet-stream"))
            .await
            .unwrap();
        assert_eq!(att.attachment.filename, "state.bin");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/applications/111/activity-instances/inst_1"
        );
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/applications/111/attachment");
        assert!(seen[1]
            .headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v.starts_with("multipart/form-data; boundary=")));
    }

    #[tokio::test(start_paused = true)]
    async fn command_get_and_patch_routes() {
        let c = client(vec![
            resp(200, SAMPLE_COMMAND),
            resp(200, SAMPLE_COMMAND),
            resp(200, SAMPLE_COMMAND),
            resp(200, SAMPLE_COMMAND),
        ]);

        let edit = EditCommand::new().description("updated description");

        let global_cmd = c.get_global_command(sf(111), sf(222)).await.unwrap();
        assert_eq!(global_cmd.id, sf(222));

        let edited_global = c.edit_global_command(sf(111), sf(222), &edit).await.unwrap();
        assert_eq!(edited_global.id, sf(222));

        let guild_cmd = c.get_guild_command(sf(111), sf(888), sf(222)).await.unwrap();
        assert_eq!(guild_cmd.id, sf(222));

        let edited_guild = c.edit_guild_command(sf(111), sf(888), sf(222), &edit).await.unwrap();
        assert_eq!(edited_guild.id, sf(222));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/applications/111/commands/222");
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/applications/111/commands/222");
        assert_eq!(
            seen[2].url,
            "https://discord.com/api/v10/applications/111/guilds/888/commands/222"
        );
        assert_eq!(seen[2].method, "GET");
        assert_eq!(
            seen[3].url,
            "https://discord.com/api/v10/applications/111/guilds/888/commands/222"
        );
        assert_eq!(seen[3].method, "PATCH");
    }

    #[tokio::test(start_paused = true)]
    async fn emoji_crud_routes() {
        let c = client(vec![
            resp(200, r#"{"items":[]}"#),
            resp(201, SAMPLE_EMOJI),
            resp(200, SAMPLE_EMOJI),
            resp(200, SAMPLE_EMOJI),
            resp(204, ""),
        ]);

        let emojis = c.list_application_emojis(sf(111)).await.unwrap();
        assert_eq!(emojis.items.len(), 0);

        let create = CreateApplicationEmoji::new("rifteyes", "data:image/png;base64,1234");
        let created = c.create_application_emoji(sf(111), &create).await.unwrap();
        assert_eq!(created.name, "rifteyes");

        let fetched = c.get_application_emoji(sf(111), sf(333)).await.unwrap();
        assert_eq!(fetched.id, sf(333));

        let modify = ModifyApplicationEmoji::new("rifteyes2");
        let modified = c.modify_application_emoji(sf(111), sf(333), &modify).await.unwrap();
        assert_eq!(modified.id, sf(333));

        c.delete_application_emoji(sf(111), sf(333)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/applications/111/emojis");
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/applications/111/emojis");
        assert_eq!(seen[2].method, "GET");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/applications/111/emojis/333");
        assert_eq!(seen[3].method, "PATCH");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/applications/111/emojis/333");
        assert_eq!(seen[4].method, "DELETE");
        assert_eq!(seen[4].url, "https://discord.com/api/v10/applications/111/emojis/333");
    }

    #[tokio::test(start_paused = true)]
    async fn entitlement_routes() {
        let c = client(vec![
            resp(200, &format!("[{SAMPLE_ENTITLEMENT}]")),
            resp(200, SAMPLE_ENTITLEMENT),
            resp(200, SAMPLE_ENTITLEMENT),
            resp(204, ""),
            resp(204, ""),
        ]);

        let query = GetEntitlements::new().user_id(sf(777)).limit(10);
        let list = c.get_entitlements(sf(111), &query).await.unwrap();
        assert_eq!(list.len(), 1);

        let create = CreateEntitlement {
            sku_id: sf(555),
            owner_id: sf(777),
            owner_type: EntitlementOwnerType::User,
        };
        let created = c.create_test_entitlement(sf(111), &create).await.unwrap();
        assert_eq!(created.id, sf(444));

        let fetched = c.get_entitlement(sf(111), sf(444)).await.unwrap();
        assert_eq!(fetched.id, sf(444));

        c.delete_test_entitlement(sf(111), sf(444)).await.unwrap();
        c.consume_entitlement(sf(111), sf(444)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/applications/111/entitlements?user_id=777&limit=10"
        );
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/applications/111/entitlements");
        assert_eq!(seen[2].method, "GET");
        assert_eq!(
            seen[2].url,
            "https://discord.com/api/v10/applications/111/entitlements/444"
        );
        assert_eq!(seen[3].method, "DELETE");
        assert_eq!(
            seen[3].url,
            "https://discord.com/api/v10/applications/111/entitlements/444"
        );
        assert_eq!(seen[4].method, "POST");
        assert_eq!(
            seen[4].url,
            "https://discord.com/api/v10/applications/111/entitlements/444/consume"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn guild_command_permissions_routes() {
        let c = client(vec![
            resp(200, &format!("[{SAMPLE_PERMS}]")),
            resp(200, SAMPLE_PERMS),
            resp(200, SAMPLE_PERMS),
        ]);

        let all_perms = c.list_guild_commands_permissions(sf(111), sf(888)).await.unwrap();
        assert_eq!(all_perms.len(), 1);

        let cmd_perms = c.get_guild_command_permissions(sf(111), sf(888), sf(222)).await.unwrap();
        assert_eq!(cmd_perms.id, sf(222));

        let new_perms = vec![CommandPermission {
            id: sf(999),
            kind: CommandPermissionType::Role,
            permission: false,
        }];
        let set_perms = c
            .set_guild_command_permissions(sf(111), sf(888), sf(222), &new_perms)
            .await
            .unwrap();
        assert_eq!(set_perms.id, sf(222));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/applications/111/guilds/888/commands/permissions"
        );
        assert_eq!(seen[1].method, "GET");
        assert_eq!(
            seen[1].url,
            "https://discord.com/api/v10/applications/111/guilds/888/commands/222/permissions"
        );
        assert_eq!(seen[2].method, "PUT");
        assert_eq!(
            seen[2].url,
            "https://discord.com/api/v10/applications/111/guilds/888/commands/222/permissions"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn role_connections_metadata_routes() {
        let sample_meta = r#"[
            {
                "type": 1,
                "key": "subscribers",
                "name": "Subscribers",
                "description": "Subscriber count"
            }
        ]"#;

        let c = client(vec![resp(200, sample_meta), resp(200, sample_meta)]);

        let list = c.get_role_connection_metadata(sf(111)).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key, "subscribers");

        let updated = c.update_role_connection_metadata(sf(111), &list).await.unwrap();
        assert_eq!(updated.len(), 1);

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/applications/111/role-connections/metadata"
        );
        assert_eq!(seen[1].method, "PUT");
        assert_eq!(
            seen[1].url,
            "https://discord.com/api/v10/applications/111/role-connections/metadata"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn validation_guards_prevent_invalid_requests() {
        let c = client(vec![]);

        // Emoji name too short
        let bad_emoji = CreateApplicationEmoji::new("a", "data:image/png;base64,1234");
        assert!(c.create_application_emoji(sf(111), &bad_emoji).await.is_err());

        // Application description too long
        let long_desc = "a".repeat(401);
        let bad_mod = ModifyApplication::new().description(&long_desc);
        assert!(c.modify_current_application(&bad_mod).await.is_err());

        // Too many metadata records (> 5)
        let too_many_meta: Vec<RoleConnectionMetadata> = (0..6)
            .map(|i| {
                RoleConnectionMetadata::new(
                    RoleConnectionMetadataType::IntegerEqual,
                    format!("key_{i}"),
                    "Name",
                    "Description",
                )
            })
            .collect();
        assert!(c
            .update_role_connection_metadata(sf(111), &too_many_meta)
            .await
            .is_err());

        // Zero network requests sent
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }
}
