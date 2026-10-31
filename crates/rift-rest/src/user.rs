//! User resource endpoints.
//!
//! Official Discord API reference:
//! <https://docs.discord.com/developers/resources/user>
//!
//! Implements all 12 user operations:
//! - Get Current User (`GET /users/@me`)
//! - Modify Current User (`PATCH /users/@me`)
//! - Get Current User Application Entitlements (`GET /users/@me/applications/{application_id}/entitlements`)
//! - Get Application Role Connection (`GET /users/@me/applications/{application_id}/role-connection`)
//! - Update Application Role Connection (`PUT /users/@me/applications/{application_id}/role-connection`)
//! - Delete Application Role Connection (`DELETE /users/@me/applications/{application_id}/role-connection`)
//! - Create DM (`POST /users/@me/channels`)
//! - Get Current User Connections (`GET /users/@me/connections`)
//! - Get Current User Guilds (`GET /users/@me/guilds`)
//! - Leave Guild (`DELETE /users/@me/guilds/{guild_id}`)
//! - Get Current User Guild Member (`GET /users/@me/guilds/{guild_id}/member`)
//! - Get User (`GET /users/{user_id}`)

use crate::channel::Channel;
use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake, ser_snowflake};
use crate::transport::Transport;
use rift_core::Snowflake;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

fn de_vec_snowflake<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Snowflake>, D::Error> {
    let strings = Vec::<String>::deserialize(d)?;
    strings
        .into_iter()
        .map(|s| s.parse::<Snowflake>().map_err(serde::de::Error::custom))
        .collect()
}

fn query_string<T: Serialize>(v: &T) -> String {
    let j = serde_json::to_value(v).unwrap_or_default();
    let Some(obj) = j.as_object() else { return String::new() };
    let parts: Vec<String> = obj
        .iter()
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
    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

// ───────────────────────── User Models ─────────────────────────

/// Current authenticated user (`/users/@me`) with PII fields.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CurrentUser {
    /// The user's snowflake id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The user's username.
    pub username: String,
    /// The user's 4-digit discord-tag or "0" for unique username system.
    pub discriminator: String,
    /// The user's display name, if set.
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
}

/// General public user object returned by `GET /users/{user_id}`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UserResponse {
    /// The user's snowflake id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The user's username.
    pub username: String,
    /// The user's 4-digit discord-tag or "0" for unique username system.
    pub discriminator: String,
    /// The user's display name, if set.
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
    /// The user's banner hash.
    #[serde(default)]
    pub banner: Option<String>,
    /// The user's banner color encoded as an integer representation of hexadecimal color code.
    #[serde(default)]
    pub accent_color: Option<u32>,
    /// The public flags on a user's account.
    #[serde(default)]
    pub public_flags: Option<u64>,
    /// The flags on a user's account.
    #[serde(default)]
    pub flags: Option<u64>,
}

/// Request payload for `PATCH /users/@me`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyCurrentUser {
    /// User's username, 2-32 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Data URI scheme base64 128x128 image for avatar.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
    /// Data URI scheme base64 image for banner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<String>>,
}

/// A partial guild returned by `GET /users/@me/guilds`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CurrentUserGuild {
    /// Guild snowflake id.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// Guild name (2-100 characters).
    pub name: String,
    /// Icon hash.
    #[serde(default)]
    pub icon: Option<String>,
    /// Whether the current user is the owner of this guild.
    #[serde(default)]
    pub owner: Option<bool>,
    /// Bitwise value of the current user's permissions in this guild.
    #[serde(default)]
    pub permissions: Option<String>,
    /// Array of enabled guild features.
    #[serde(default)]
    pub features: Option<Vec<String>>,
    /// Approximate member count, returned when `with_counts` is true.
    #[serde(default)]
    pub approximate_member_count: Option<u64>,
    /// Approximate presence count, returned when `with_counts` is true.
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,
}

/// Query parameters for `GET /users/@me/guilds`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct GetCurrentUserGuilds {
    /// Get guilds before this guild ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub before: Option<Snowflake>,
    /// Get guilds after this guild ID.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub after: Option<Snowflake>,
    /// Max number of guilds to return (1-200). Defaults to 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    /// Whether to include approximate member and presence counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_counts: Option<bool>,
}

/// Member object returned by `GET /users/@me/guilds/{guild_id}/member`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct CurrentUserGuildMember {
    /// The user this guild member represents.
    #[serde(default)]
    pub user: Option<crate::interaction::User>,
    /// This user's guild nickname.
    #[serde(default)]
    pub nick: Option<String>,
    /// The member's guild avatar hash.
    #[serde(default)]
    pub avatar: Option<String>,
    /// Array of role snowflake IDs.
    #[serde(default, deserialize_with = "de_vec_snowflake")]
    pub roles: Vec<Snowflake>,
    /// When the user joined the guild (ISO8601 timestamp).
    pub joined_at: String,
    /// When the user started boosting the guild.
    #[serde(default)]
    pub premium_since: Option<String>,
    /// Whether the user is deafened in voice channels.
    #[serde(default)]
    pub deaf: Option<bool>,
    /// Whether the user is muted in voice channels.
    #[serde(default)]
    pub mute: Option<bool>,
    /// Guild member flags.
    #[serde(default)]
    pub flags: Option<u64>,
    /// Whether the user has not yet passed the guild's Membership Screening requirements.
    #[serde(default)]
    pub pending: Option<bool>,
    /// Total permissions of the member in the channel, including overwrites, returned when in interaction context.
    #[serde(default)]
    pub permissions: Option<String>,
    /// When the user's timeout will expire and the user will be able to communicate again.
    #[serde(default)]
    pub communication_disabled_until: Option<String>,
}

/// A third-party service connection object (`GET /users/@me/connections`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct UserConnection {
    /// ID of the connection account.
    pub id: String,
    /// The username of the connection account.
    pub name: String,
    /// The service of this connection (e.g. twitch, youtube, spotify, steam, github).
    #[serde(rename = "type")]
    pub kind: String,
    /// Whether the connection is revoked.
    #[serde(default)]
    pub revoked: Option<bool>,
    /// Whether the connection is verified.
    #[serde(default)]
    pub verified: Option<bool>,
    /// Whether friend sync is enabled for this connection.
    #[serde(default)]
    pub friend_sync: Option<bool>,
    /// Whether activities related to this connection will be shown in presence updates.
    #[serde(default)]
    pub show_activity: Option<bool>,
    /// Whether this connection has a corresponding third party OAuth2 token.
    #[serde(default)]
    pub two_way_link: Option<bool>,
    /// Visibility of this connection (0 = None, 1 = Everyone).
    #[serde(default)]
    pub visibility: Option<u8>,
}

/// Application role connection for the current user.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ApplicationRoleConnection {
    /// The vanity name of the platform a bot has connected.
    #[serde(default)]
    pub platform_name: Option<String>,
    /// The username on the platform a bot has connected.
    #[serde(default)]
    pub platform_username: Option<String>,
    /// Object mapping application role connection metadata keys to their string-serialized values.
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Update payload for `PUT /users/@me/applications/{application_id}/role-connection`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct UpdateApplicationRoleConnection {
    /// The vanity name of the platform (max 50 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_name: Option<String>,
    /// The username on the platform (max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_username: Option<String>,
    /// Key-value mapping for metadata (max 5 keys, key/val max 100 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

/// An entitlement record (`GET /users/@me/applications/{application_id}/entitlements`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Entitlement {
    /// ID of the entitlement.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// ID of the SKU.
    #[serde(deserialize_with = "de_snowflake")]
    pub sku_id: Snowflake,
    /// ID of the parent application.
    #[serde(deserialize_with = "de_snowflake")]
    pub application_id: Snowflake,
    /// ID of the user that is granted access to the entitlement's SKU.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub user_id: Option<Snowflake>,
    /// Type of entitlement.
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
    /// Whether the entitlement was deleted.
    #[serde(default)]
    pub deleted: bool,
    /// Start date at which the entitlement is valid (ISO8601).
    #[serde(default)]
    pub starts_at: Option<String>,
    /// Date at which the entitlement is no longer valid (ISO8601).
    #[serde(default)]
    pub ends_at: Option<String>,
    /// ID of the guild that is granted access to the entitlement's SKU.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// Whether this entitlement has been consumed for one-time purchases.
    #[serde(default)]
    pub consumed: Option<bool>,
}

/// Query parameters for `GET /users/@me/applications/{application_id}/entitlements`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GetCurrentUserEntitlements {
    /// Filter entitlements for specific SKU IDs.
    pub sku_ids: Option<Vec<Snowflake>>,
    /// Whether to exclude consumed entitlements.
    pub exclude_consumed: Option<bool>,
}

impl GetCurrentUserEntitlements {
    fn to_query_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ids) = &self.sku_ids {
            if !ids.is_empty() {
                let joined = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
                parts.push(format!("sku_ids={joined}"));
            }
        }
        if let Some(exc) = self.exclude_consumed {
            parts.push(format!("exclude_consumed={exc}"));
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!("?{}", parts.join("&"))
        }
    }
}

#[derive(Serialize)]
struct CreateDmPayload {
    #[serde(serialize_with = "ser_snowflake")]
    recipient_id: Snowflake,
}

// ───────────────────────── Validation Errors ─────────────────────────

/// Validation error for user endpoint payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserError {
    /// Username length is invalid (must be 2-32 characters).
    BadUsername {
        /// Provided character length.
        len: usize,
    },
    /// Guild list limit out of bounds (1-200).
    BadGuildLimit {
        /// Provided limit value.
        value: u16,
    },
    /// Role connection platform name exceeds 50 characters.
    BadPlatformName {
        /// Provided character length.
        len: usize,
    },
    /// Role connection platform username exceeds 100 characters.
    BadPlatformUsername {
        /// Provided character length.
        len: usize,
    },
    /// Role connection metadata has too many entries (max 5).
    BadMetadataCount {
        /// Provided entries count.
        count: usize,
    },
    /// Role connection metadata key or value exceeds 100 characters.
    BadMetadataLength {
        /// Length of the offending key or value.
        len: usize,
    },
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadUsername { len } => write!(f, "username length {len} is invalid; must be between 2 and 32 chars"),
            Self::BadGuildLimit { value } => write!(f, "guild limit {value} is out of bounds (must be 1-200)"),
            Self::BadPlatformName { len } => write!(f, "platform_name is {len} chars; max allowed is 50"),
            Self::BadPlatformUsername { len } => write!(f, "platform_username is {len} chars; max allowed is 100"),
            Self::BadMetadataCount { count } => write!(f, "metadata has {count} entries; max allowed is 5"),
            Self::BadMetadataLength { len } => write!(f, "metadata key/value length {len} exceeds max 100 chars"),
        }
    }
}

impl std::error::Error for UserError {}

impl From<UserError> for Error {
    fn from(e: UserError) -> Self {
        Error::Transport(format!("user validation: {e}"))
    }
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    /// `GET /users/@me` – Returns the user object of the requester's account.
    pub async fn get_current_user(&self) -> Result<CurrentUser, Error> {
        self.request("GET", "/users/@me", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `PATCH /users/@me` – Modify the requester's user account settings.
    pub async fn modify_current_user(&self, body: &ModifyCurrentUser) -> Result<CurrentUser, Error> {
        if let Some(u) = &body.username {
            let count = u.chars().count();
            if !(2..=32).contains(&count) {
                return Err(UserError::BadUsername { len: count }.into());
            }
        }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("PATCH", "/users/@me", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /users/@me/applications/{application_id}/entitlements` – Get entitlements for the current user.
    pub async fn get_current_user_application_entitlements(
        &self,
        application_id: Snowflake,
        query: &GetCurrentUserEntitlements,
    ) -> Result<Vec<Entitlement>, Error> {
        let qs = query.to_query_string();
        let path = format!("/users/@me/applications/{application_id}/entitlements{qs}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /users/@me/applications/{application_id}/role-connection` – Get application role connection for the current user.
    pub async fn get_user_application_role_connection(
        &self,
        application_id: Snowflake,
    ) -> Result<ApplicationRoleConnection, Error> {
        let path = format!("/users/@me/applications/{application_id}/role-connection");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `PUT /users/@me/applications/{application_id}/role-connection` – Update application role connection for the current user.
    pub async fn update_user_application_role_connection(
        &self,
        application_id: Snowflake,
        body: &UpdateApplicationRoleConnection,
    ) -> Result<ApplicationRoleConnection, Error> {
        if let Some(p) = &body.platform_name {
            let count = p.chars().count();
            if count > 50 {
                return Err(UserError::BadPlatformName { len: count }.into());
            }
        }
        if let Some(u) = &body.platform_username {
            let count = u.chars().count();
            if count > 100 {
                return Err(UserError::BadPlatformUsername { len: count }.into());
            }
        }
        if let Some(m) = &body.metadata {
            if m.len() > 5 {
                return Err(UserError::BadMetadataCount { count: m.len() }.into());
            }
            for (k, v) in m {
                let k_len = k.chars().count();
                if k_len > 100 {
                    return Err(UserError::BadMetadataLength { len: k_len }.into());
                }
                let v_len = v.chars().count();
                if v_len > 100 {
                    return Err(UserError::BadMetadataLength { len: v_len }.into());
                }
            }
        }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/users/@me/applications/{application_id}/role-connection");
        self.request("PUT", &path, Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /users/@me/applications/{application_id}/role-connection` – Delete application role connection for the current user.
    pub async fn delete_user_application_role_connection(
        &self,
        application_id: Snowflake,
    ) -> Result<(), Error> {
        let path = format!("/users/@me/applications/{application_id}/role-connection");
        self.request("DELETE", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /users/@me/channels` – Create a new DM channel with a user.
    pub async fn create_dm(&self, recipient_id: Snowflake) -> Result<Channel, Error> {
        let payload = CreateDmPayload { recipient_id };
        let b = serde_json::to_vec(&payload).map_err(|e| Error::Decode(e.to_string()))?;
        self.request("POST", "/users/@me/channels", Some(b))
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /users/@me/connections` – Returns a list of connection objects.
    pub async fn get_current_user_connections(&self) -> Result<Vec<UserConnection>, Error> {
        self.request("GET", "/users/@me/connections", None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /users/@me/guilds` – Returns a list of partial guild objects the current user is a member of.
    pub async fn get_current_user_guilds(
        &self,
        query: &GetCurrentUserGuilds,
    ) -> Result<Vec<CurrentUserGuild>, Error> {
        if let Some(lim) = query.limit {
            if !(1..=200).contains(&lim) {
                return Err(UserError::BadGuildLimit { value: lim }.into());
            }
        }
        let qs = query_string(query);
        let path = format!("/users/@me/guilds{qs}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /users/@me/guilds/{guild_id}` – Leave a guild.
    pub async fn leave_guild(&self, guild_id: Snowflake) -> Result<(), Error> {
        let path = format!("/users/@me/guilds/{guild_id}");
        self.request("DELETE", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `GET /users/@me/guilds/{guild_id}/member` – Returns a guild member object for the current user.
    pub async fn get_current_user_guild_member(
        &self,
        guild_id: Snowflake,
    ) -> Result<CurrentUserGuildMember, Error> {
        let path = format!("/users/@me/guilds/{guild_id}/member");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /users/{user_id}` – Returns a user object for a given user ID.
    pub async fn get_user(&self, user_id: Snowflake) -> Result<UserResponse, Error> {
        let path = format!("/users/{user_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }
}

// ───────────────────────── Tests ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::RestClient;
    use crate::transport::{RawResponse, Request, Transport};
    use rift_core::Snowflake;
    use rift_http::Config;
    use serde_json::json;
    use std::collections::VecDeque;
    use std::future::Future;
    use std::sync::Mutex;

    struct Fake {
        queue: Mutex<VecDeque<Result<RawResponse, String>>>,
        seen: Mutex<Vec<Request>>,
    }

    impl Fake {
        fn new(items: Vec<Result<RawResponse, String>>) -> Self {
            Self {
                queue: Mutex::new(items.into()),
                seen: Mutex::new(Vec::new()),
            }
        }
    }

    impl Transport for Fake {
        fn send(&self, req: Request) -> impl Future<Output = Result<RawResponse, String>> + Send {
            self.seen.lock().unwrap().push(req);
            let r = self
                .queue
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Err("queue empty".into()));
            async move { r }
        }
    }

    fn client(responses: Vec<Result<RawResponse, String>>) -> RestClient<Fake> {
        RestClient::with_transport(
            Fake::new(responses),
            "tok",
            "DiscordBot (https://example.com, 1.0.0)".into(),
            Config::default(),
        )
    }

    fn resp(status: u16, body: &str) -> Result<RawResponse, String> {
        Ok(RawResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        })
    }

    fn sf(n: u64) -> Snowflake {
        Snowflake::new(n)
    }

    #[test]
    fn user_error_display() {
        let err = UserError::BadUsername { len: 1 };
        assert!(err.to_string().contains("username length 1"));
        let err2 = UserError::BadGuildLimit { value: 250 };
        assert!(err2.to_string().contains("guild limit 250"));
    }

    #[tokio::test(start_paused = true)]
    async fn get_and_modify_current_user_routes() {
        let user_json = json!({
            "id": "100",
            "username": "botuser",
            "discriminator": "0001"
        })
        .to_string();

        let c = client(vec![resp(200, &user_json), resp(200, &user_json)]);

        let me = c.get_current_user().await.unwrap();
        assert_eq!(me.id, sf(100));
        assert_eq!(me.username, "botuser");

        let mod_body = ModifyCurrentUser {
            username: Some("newname".into()),
            avatar: None,
            banner: None,
        };
        let updated = c.modify_current_user(&mod_body).await.unwrap();
        assert_eq!(updated.username, "botuser");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/users/@me");
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/users/@me");
    }

    #[tokio::test(start_paused = true)]
    async fn modify_current_user_validates_username_length() {
        let c = client(vec![]);
        let bad_short = ModifyCurrentUser {
            username: Some("a".into()),
            ..Default::default()
        };
        assert!(c.modify_current_user(&bad_short).await.is_err());

        let bad_long = ModifyCurrentUser {
            username: Some("a".repeat(33)),
            ..Default::default()
        };
        assert!(c.modify_current_user(&bad_long).await.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn user_role_connection_routes() {
        let rc_json = json!({
            "platform_name": "Steam",
            "platform_username": "player1",
            "metadata": {}
        })
        .to_string();

        let c = client(vec![resp(200, &rc_json), resp(200, &rc_json), resp(204, "")]);

        let rc = c.get_user_application_role_connection(sf(111)).await.unwrap();
        assert_eq!(rc.platform_name.as_deref(), Some("Steam"));

        let update = UpdateApplicationRoleConnection {
            platform_name: Some("SteamUpdated".into()),
            platform_username: Some("player2".into()),
            metadata: Some(HashMap::new()),
        };
        let updated = c.update_user_application_role_connection(sf(111), &update).await.unwrap();
        assert_eq!(updated.platform_username.as_deref(), Some("player1"));

        c.delete_user_application_role_connection(sf(111)).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/users/@me/applications/111/role-connection");
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/users/@me/applications/111/role-connection");
        assert_eq!(seen[1].method, "PUT");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/users/@me/applications/111/role-connection");
        assert_eq!(seen[2].method, "DELETE");
    }

    #[tokio::test(start_paused = true)]
    async fn user_entitlements_query_and_route() {
        let ent_json = json!([{
            "id": "500",
            "sku_id": "600",
            "application_id": "700",
            "deleted": false
        }])
        .to_string();

        let c = client(vec![resp(200, &ent_json)]);

        let q = GetCurrentUserEntitlements {
            sku_ids: Some(vec![sf(600), sf(601)]),
            exclude_consumed: Some(true),
        };
        let res = c.get_current_user_application_entitlements(sf(700), &q).await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, sf(500));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/users/@me/applications/700/entitlements?sku_ids=600,601&exclude_consumed=true"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn dm_and_connections_routes() {
        let chan_json = json!({
            "id": "200",
            "type": 1
        })
        .to_string();

        let conn_json = json!([{
            "id": "conn1",
            "name": "TwitchUser",
            "type": "twitch"
        }])
        .to_string();

        let c = client(vec![resp(200, &chan_json), resp(200, &conn_json)]);

        let dm = c.create_dm(sf(300)).await.unwrap();
        assert_eq!(dm.id, sf(200));

        let conns = c.get_current_user_connections().await.unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "TwitchUser");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/users/@me/channels");
        assert_eq!(seen[1].method, "GET");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/users/@me/connections");
    }

    #[tokio::test(start_paused = true)]
    async fn user_guild_endpoints_and_limits() {
        let guilds_json = json!([{
            "id": "999",
            "name": "My Guild"
        }])
        .to_string();

        let member_json = json!({
            "joined_at": "2026-01-01T00:00:00.000000+00:00",
            "roles": ["888"]
        })
        .to_string();

        let user_json = json!({
            "id": "42",
            "username": "otheruser",
            "discriminator": "0002"
        })
        .to_string();

        let c = client(vec![
            resp(200, &guilds_json),
            resp(204, ""),
            resp(200, &member_json),
            resp(200, &user_json),
        ]);

        let q = GetCurrentUserGuilds {
            limit: Some(100),
            with_counts: Some(true),
            ..Default::default()
        };
        let guilds = c.get_current_user_guilds(&q).await.unwrap();
        assert_eq!(guilds.len(), 1);
        assert_eq!(guilds[0].id, sf(999));

        c.leave_guild(sf(999)).await.unwrap();

        let member = c.get_current_user_guild_member(sf(999)).await.unwrap();
        assert_eq!(member.roles, vec![sf(888)]);

        let user = c.get_user(sf(42)).await.unwrap();
        assert_eq!(user.id, sf(42));
        assert_eq!(user.username, "otheruser");

        {
            let seen = c.transport().seen.lock().unwrap();
            assert_eq!(seen[0].url, "https://discord.com/api/v10/users/@me/guilds?limit=100&with_counts=true");
            assert_eq!(seen[1].url, "https://discord.com/api/v10/users/@me/guilds/999");
            assert_eq!(seen[1].method, "DELETE");
            assert_eq!(seen[2].url, "https://discord.com/api/v10/users/@me/guilds/999/member");
            assert_eq!(seen[3].url, "https://discord.com/api/v10/users/42");
        }

        let bad_limit = GetCurrentUserGuilds { limit: Some(250), ..Default::default() };
        assert!(c.get_current_user_guilds(&bad_limit).await.is_err());
    }
}
