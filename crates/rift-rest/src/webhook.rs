//! Webhook resource endpoints.
//!
//! Complete coverage for Discord webhooks according to the official documentation:
//! - Get Webhook, Modify Webhook, Delete Webhook
//! - Get Webhook with Token, Modify Webhook with Token, Delete Webhook with Token
//! - Execute Webhook, Execute GitHub-Compatible Webhook, Execute Slack-Compatible Webhook
//! - Get/Edit/Delete Webhook Message (including `@original`)
//!
//! Note: Webhook tokens live in URL paths. They are redacted in `Debug` implementations
//! and omitted from rate limiter route logging to avoid leaking credentials.

use crate::client::RestClient;
use crate::error::Error;
use crate::interaction::User;
use crate::message::{
    de_opt_snowflake, de_snowflake, ser_flags, ser_opt_snowflake, validate_text_parts,
    AllowedMentions, Embed, Message, ValidationError,
};
use crate::transport::Transport;
use rift_core::{MessageFlags, Snowflake};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

// ---------- Enums and Types ----------

/// The type of a webhook.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum WebhookType {
    /// Incoming webhooks post messages to channels with a token.
    Incoming = 1,
    /// Channel follower webhooks post published messages to another channel.
    ChannelFollower = 2,
    /// Application webhooks used with Discord Activities.
    Application = 3,
    /// Unknown webhook type for future compatibility.
    Unknown(u8),
}

impl WebhookType {
    /// Parse the raw integer.
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Incoming,
            2 => Self::ChannelFollower,
            3 => Self::Application,
            other => Self::Unknown(other),
        }
    }

    /// Raw integer representation.
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Incoming => 1,
            Self::ChannelFollower => 2,
            Self::Application => 3,
            Self::Unknown(v) => v,
        }
    }
}

impl<'de> Deserialize<'de> for WebhookType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(d)?;
        Ok(Self::from_u8(v))
    }
}

impl Serialize for WebhookType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(self.as_u8())
    }
}

/// Partial guild object for follower webhooks.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WebhookSourceGuild {
    /// The id of the source guild.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The name of the source guild.
    pub name: String,
    /// The icon hash of the source guild.
    pub icon: Option<String>,
}

/// Partial channel object for follower webhooks.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct WebhookSourceChannel {
    /// The id of the source channel.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The name of the source channel.
    pub name: String,
}

/// A Discord webhook.
#[derive(Clone, Deserialize, PartialEq)]
pub struct Webhook {
    /// The id of the webhook.
    #[serde(deserialize_with = "de_snowflake")]
    pub id: Snowflake,
    /// The type of the webhook.
    #[serde(rename = "type")]
    pub kind: WebhookType,
    /// The guild id this webhook belongs to.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub guild_id: Option<Snowflake>,
    /// The channel id this webhook belongs to.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
    /// The user that created this webhook.
    #[serde(default)]
    pub user: Option<User>,
    /// The default name of the webhook.
    #[serde(default)]
    pub name: Option<String>,
    /// The default avatar hash of the webhook.
    #[serde(default)]
    pub avatar: Option<String>,
    /// The secure token of the webhook (returned for Incoming webhooks).
    #[serde(default)]
    pub token: Option<String>,
    /// The bot/OAuth2 application that created this webhook.
    #[serde(default, deserialize_with = "de_opt_snowflake")]
    pub application_id: Option<Snowflake>,
    /// The url used for executing the webhook (contains token).
    #[serde(default)]
    pub url: Option<String>,
    /// The source guild (for Channel Follower webhooks).
    #[serde(default)]
    pub source_guild: Option<WebhookSourceGuild>,
    /// The source channel (for Channel Follower webhooks).
    #[serde(default)]
    pub source_channel: Option<WebhookSourceChannel>,
}

impl fmt::Debug for Webhook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let safe_url = self.url.as_ref().map(|u| {
            if let Some(tok) = &self.token {
                u.replace(tok, "<redacted>")
            } else {
                u.clone()
            }
        });
        f.debug_struct("Webhook")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("guild_id", &self.guild_id)
            .field("channel_id", &self.channel_id)
            .field("user", &self.user)
            .field("name", &self.name)
            .field("avatar", &self.avatar)
            .field("token", &self.token.as_ref().map(|_| "<redacted>"))
            .field("application_id", &self.application_id)
            .field("url", &safe_url)
            .field("source_guild", &self.source_guild)
            .field("source_channel", &self.source_channel)
            .finish()
    }
}

// ---------- Request Payloads & Validation ----------

/// Errors encountered when validating webhook parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebhookError {
    /// Webhook name is too short or too long (must be 1-80 characters).
    BadWebhookName {
        /// Actual length.
        len: usize,
    },
    /// Thread name is too long (max 100 characters).
    BadThreadName {
        /// Actual length.
        len: usize,
    },
    /// More than 5 applied tags specified.
    TooManyTags {
        /// Actual count.
        count: usize,
    },
    /// Generic validation error from message validation.
    Validation(ValidationError),
}

impl fmt::Display for WebhookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadWebhookName { len } => {
                write!(f, "webhook name is {len} characters (must be 1-80)")
            }
            Self::BadThreadName { len } => {
                write!(f, "thread name is {len} characters (max 100)")
            }
            Self::TooManyTags { count } => {
                write!(f, "applied_tags has {count} tags (max 5)")
            }
            Self::Validation(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for WebhookError {}

impl From<ValidationError> for WebhookError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<WebhookError> for Error {
    fn from(e: WebhookError) -> Self {
        match e {
            WebhookError::Validation(ve) => Error::Validation(ve),
            other => Error::Transport(format!("webhook validation error: {other}")),
        }
    }
}

/// Request body for modifying a webhook with bot authorization.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyWebhook {
    /// The default name of the webhook (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Image data for the default webhook avatar (data URI scheme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// The new channel id this webhook should be moved to.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_opt_snowflake")]
    pub channel_id: Option<Snowflake>,
}

impl ModifyWebhook {
    /// Create an empty modify request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name (1-80 characters).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set avatar image data URI.
    pub fn avatar(mut self, avatar: impl Into<String>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Move the webhook to a new channel.
    pub fn channel_id(mut self, channel_id: Snowflake) -> Self {
        self.channel_id = Some(channel_id);
        self
    }

    /// Local validation before sending.
    pub fn validate(&self) -> Result<(), WebhookError> {
        if let Some(name) = &self.name {
            let len = name.chars().count();
            if len == 0 || len > 80 {
                return Err(WebhookError::BadWebhookName { len });
            }
        }
        Ok(())
    }
}

/// Request body for modifying a webhook with token (no bot auth required).
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ModifyWebhookWithToken {
    /// The default name of the webhook (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Image data for the default webhook avatar (data URI scheme).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
}

impl ModifyWebhookWithToken {
    /// Create an empty modify request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name (1-80 characters).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set avatar image data URI.
    pub fn avatar(mut self, avatar: impl Into<String>) -> Self {
        self.avatar = Some(avatar.into());
        self
    }

    /// Local validation before sending.
    pub fn validate(&self) -> Result<(), WebhookError> {
        if let Some(name) = &self.name {
            let len = name.chars().count();
            if len == 0 || len > 80 {
                return Err(WebhookError::BadWebhookName { len });
            }
        }
        Ok(())
    }
}

fn ser_snowflakes<S: Serializer>(v: &[Snowflake], s: S) -> Result<S::Ok, S::Error> {
    s.collect_seq(v.iter().map(|x| x.to_string()))
}

/// Parameters for executing a webhook.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct ExecuteWebhook {
    /// Message contents (max 2000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Override the default username of the webhook (1-80 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Override the default avatar of the webhook (URL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// True if this is a text-to-speech message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    /// Embedded rich content (up to 10 embeds).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
    /// Allowed mentions object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    /// Message components (interactive buttons, select menus, etc.).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<serde_json::Value>,
    /// Message flags (only `SUPPRESS_EMBEDS` and `SUPPRESS_NOTIFICATIONS` can be set).
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_flags")]
    pub flags: Option<MessageFlags>,
    /// Name of thread to create (forum/media channels only; max 100 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<String>,
    /// Array of tag ids to apply to the thread (forum/media channels only; max 5).
    #[serde(skip_serializing_if = "Vec::is_empty", serialize_with = "ser_snowflakes")]
    pub applied_tags: Vec<Snowflake>,
    /// Query param: wait for server confirmation of message send before response.
    #[serde(skip)]
    pub wait: Option<bool>,
    /// Query param: send to a specific thread in the webhook's channel.
    #[serde(skip)]
    pub thread_id: Option<Snowflake>,
    /// Query param: send with components enabled.
    #[serde(skip)]
    pub with_components: Option<bool>,
}

impl ExecuteWebhook {
    /// Create an empty execute webhook request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set message content.
    pub fn content(mut self, c: impl Into<String>) -> Self {
        self.content = Some(c.into());
        self
    }

    /// Set custom username.
    pub fn username(mut self, u: impl Into<String>) -> Self {
        self.username = Some(u.into());
        self
    }

    /// Set custom avatar URL.
    pub fn avatar_url(mut self, url: impl Into<String>) -> Self {
        self.avatar_url = Some(url.into());
        self
    }

    /// Set text-to-speech.
    pub fn tts(mut self, t: bool) -> Self {
        self.tts = Some(t);
        self
    }

    /// Add an embed.
    pub fn embed(mut self, e: Embed) -> Self {
        self.embeds.push(e);
        self
    }

    /// Set allowed mentions.
    pub fn allowed_mentions(mut self, m: AllowedMentions) -> Self {
        self.allowed_mentions = Some(m);
        self
    }

    /// Set components.
    pub fn components(mut self, c: Vec<serde_json::Value>) -> Self {
        self.components = c;
        self
    }

    /// Set flags.
    pub fn flags(mut self, f: MessageFlags) -> Self {
        self.flags = Some(f);
        self
    }

    /// Set thread name (for creating thread in forum/media channels).
    pub fn thread_name(mut self, name: impl Into<String>) -> Self {
        self.thread_name = Some(name.into());
        self
    }

    /// Set applied tags.
    pub fn applied_tags(mut self, tags: Vec<Snowflake>) -> Self {
        self.applied_tags = tags;
        self
    }

    /// Wait for server confirmation (returns the created `Message`).
    pub fn wait(mut self, w: bool) -> Self {
        self.wait = Some(w);
        self
    }

    /// Target a specific thread.
    pub fn thread_id(mut self, t: Snowflake) -> Self {
        self.thread_id = Some(t);
        self
    }

    /// Validate the request before sending.
    pub fn validate(&self) -> Result<(), WebhookError> {
        let has_body = self.content.as_deref().is_some_and(|c| !c.is_empty())
            || !self.embeds.is_empty()
            || !self.components.is_empty();
        if !has_body {
            return Err(WebhookError::Validation(ValidationError::Empty));
        }
        if let Some(u) = &self.username {
            let len = u.chars().count();
            if len == 0 || len > 80 {
                return Err(WebhookError::BadWebhookName { len });
            }
        }
        if let Some(tn) = &self.thread_name {
            let len = tn.chars().count();
            if len > 100 {
                return Err(WebhookError::BadThreadName { len });
            }
        }
        if self.applied_tags.len() > 5 {
            return Err(WebhookError::TooManyTags { count: self.applied_tags.len() });
        }
        validate_text_parts(self.content.as_deref(), &self.embeds, self.allowed_mentions.as_ref())?;
        Ok(())
    }
}

/// Parameters for editing a message previously sent by a webhook.
#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct WebhookMessageEdit {
    /// New message contents (max 2000 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Embedded rich content (up to 10 embeds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,
    /// Allowed mentions object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    /// Message components.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<serde_json::Value>>,
    /// Flags.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "ser_flags")]
    pub flags: Option<MessageFlags>,
}

impl WebhookMessageEdit {
    /// Create an empty edit request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set new content.
    pub fn content(mut self, c: impl Into<String>) -> Self {
        self.content = Some(c.into());
        self
    }

    /// Set embeds.
    pub fn embeds(mut self, e: Vec<Embed>) -> Self {
        self.embeds = Some(e);
        self
    }

    /// Set allowed mentions.
    pub fn allowed_mentions(mut self, m: AllowedMentions) -> Self {
        self.allowed_mentions = Some(m);
        self
    }

    /// Set components.
    pub fn components(mut self, c: Vec<serde_json::Value>) -> Self {
        self.components = Some(c);
        self
    }

    /// Set flags.
    pub fn flags(mut self, f: MessageFlags) -> Self {
        self.flags = Some(f);
        self
    }

    /// Validate the edit request.
    pub fn validate(&self) -> Result<(), WebhookError> {
        let embeds_slice = self.embeds.as_deref().unwrap_or(&[]);
        validate_text_parts(self.content.as_deref(), embeds_slice, self.allowed_mentions.as_ref())?;
        Ok(())
    }
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

// ---------- Client Implementation ----------

impl<T: Transport> RestClient<T> {
    /// `GET /webhooks/{webhook_id}`: get a webhook with bot authorization.
    pub async fn get_webhook(&self, webhook_id: Snowflake) -> Result<Webhook, Error> {
        let path = format!("/webhooks/{webhook_id}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /webhooks/{webhook_id}`: modify a webhook with bot authorization.
    pub async fn modify_webhook(
        &self,
        webhook_id: Snowflake,
        body: &ModifyWebhook,
        reason: Option<&str>,
    ) -> Result<Webhook, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        let path = format!("/webhooks/{webhook_id}");
        self.request_with_headers("PATCH", &path, Some(b), headers)
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /webhooks/{webhook_id}`: delete a webhook with bot authorization.
    pub async fn delete_webhook(&self, webhook_id: Snowflake, reason: Option<&str>) -> Result<(), Error> {
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        let path = format!("/webhooks/{webhook_id}");
        self.request_with_headers("DELETE", &path, None, headers).await?.expect_success()?;
        Ok(())
    }

    /// `GET /webhooks/{webhook_id}/{webhook_token}`: get a webhook with its token (no bot auth).
    pub async fn get_webhook_with_token(
        &self,
        webhook_id: Snowflake,
        token: &str,
    ) -> Result<Webhook, Error> {
        let path = format!("/webhooks/{webhook_id}/{token}");
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /webhooks/{webhook_id}/{webhook_token}`: modify a webhook with its token (no bot auth).
    pub async fn modify_webhook_with_token(
        &self,
        webhook_id: Snowflake,
        token: &str,
        body: &ModifyWebhookWithToken,
    ) -> Result<Webhook, Error> {
        body.validate()?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/webhooks/{webhook_id}/{token}");
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `DELETE /webhooks/{webhook_id}/{webhook_token}`: delete a webhook with its token (no bot auth).
    pub async fn delete_webhook_with_token(
        &self,
        webhook_id: Snowflake,
        token: &str,
    ) -> Result<(), Error> {
        let path = format!("/webhooks/{webhook_id}/{token}");
        self.request("DELETE", &path, None).await?.expect_success()?;
        Ok(())
    }

    /// `POST /webhooks/{webhook_id}/{webhook_token}`: execute a webhook.
    ///
    /// If `exec.wait` is `Some(true)`, returns `Ok(Some(message))`.
    /// Otherwise returns `Ok(None)`.
    pub async fn execute_webhook(
        &self,
        webhook_id: Snowflake,
        token: &str,
        exec: &ExecuteWebhook,
    ) -> Result<Option<Message>, Error> {
        exec.validate()?;
        let b = serde_json::to_vec(exec).map_err(|e| Error::Decode(e.to_string()))?;
        let mut path = format!("/webhooks/{webhook_id}/{token}");
        append_query(
            &mut path,
            &[
                ("wait", exec.wait.map(|w| w.to_string())),
                ("thread_id", exec.thread_id.map(|t| t.to_string())),
                ("with_components", exec.with_components.map(|c| c.to_string())),
            ],
        );
        let resp = self.request("POST", &path, Some(b)).await?.expect_success()?;
        if exec.wait == Some(true) {
            Ok(Some(resp.json()?))
        } else {
            Ok(None)
        }
    }

    /// `POST /webhooks/{webhook_id}/{webhook_token}/github`: execute a GitHub-compatible webhook.
    pub async fn execute_github_compatible_webhook(
        &self,
        webhook_id: Snowflake,
        token: &str,
        payload: &serde_json::Value,
        wait: Option<bool>,
        thread_id: Option<Snowflake>,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(payload).map_err(|e| Error::Decode(e.to_string()))?;
        let mut path = format!("/webhooks/{webhook_id}/{token}/github");
        append_query(
            &mut path,
            &[
                ("wait", wait.map(|w| w.to_string())),
                ("thread_id", thread_id.map(|t| t.to_string())),
            ],
        );
        self.request("POST", &path, Some(b)).await?.expect_success()?;
        Ok(())
    }

    /// `POST /webhooks/{webhook_id}/{webhook_token}/slack`: execute a Slack-compatible webhook.
    pub async fn execute_slack_compatible_webhook(
        &self,
        webhook_id: Snowflake,
        token: &str,
        payload: &serde_json::Value,
        wait: Option<bool>,
        thread_id: Option<Snowflake>,
    ) -> Result<(), Error> {
        let b = serde_json::to_vec(payload).map_err(|e| Error::Decode(e.to_string()))?;
        let mut path = format!("/webhooks/{webhook_id}/{token}/slack");
        append_query(
            &mut path,
            &[
                ("wait", wait.map(|w| w.to_string())),
                ("thread_id", thread_id.map(|t| t.to_string())),
            ],
        );
        self.request("POST", &path, Some(b)).await?.expect_success()?;
        Ok(())
    }

    /// `GET /webhooks/{webhook_id}/{webhook_token}/messages/@original`: get the original message.
    pub async fn get_original_webhook_message(
        &self,
        webhook_id: Snowflake,
        token: &str,
        thread_id: Option<Snowflake>,
    ) -> Result<Message, Error> {
        let mut path = format!("/webhooks/{webhook_id}/{token}/messages/@original");
        append_query(&mut path, &[("thread_id", thread_id.map(|t| t.to_string()))]);
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `GET /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`: get a webhook message.
    pub async fn get_webhook_message(
        &self,
        webhook_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
    ) -> Result<Message, Error> {
        let mut path = format!("/webhooks/{webhook_id}/{token}/messages/{message_id}");
        append_query(&mut path, &[("thread_id", thread_id.map(|t| t.to_string()))]);
        self.request("GET", &path, None).await?.expect_success()?.json()
    }

    /// `PATCH /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`: edit a webhook message.
    pub async fn edit_webhook_message(
        &self,
        webhook_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        edit: &WebhookMessageEdit,
        thread_id: Option<Snowflake>,
    ) -> Result<Message, Error> {
        edit.validate()?;
        let b = serde_json::to_vec(edit).map_err(|e| Error::Decode(e.to_string()))?;
        let mut path = format!("/webhooks/{webhook_id}/{token}/messages/{message_id}");
        append_query(&mut path, &[("thread_id", thread_id.map(|t| t.to_string()))]);
        self.request("PATCH", &path, Some(b)).await?.expect_success()?.json()
    }

    /// `DELETE /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`: delete a webhook message.
    pub async fn delete_webhook_message(
        &self,
        webhook_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
    ) -> Result<(), Error> {
        let mut path = format!("/webhooks/{webhook_id}/{token}/messages/{message_id}");
        append_query(&mut path, &[("thread_id", thread_id.map(|t| t.to_string()))]);
        self.request("DELETE", &path, None).await?.expect_success()?;
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

    const SAMPLE_WEBHOOK: &str = r#"{
        "id": "12345",
        "type": 1,
        "guild_id": "67890",
        "channel_id": "11111",
        "name": "Captain Hook",
        "avatar": "abcdef",
        "token": "secret_token_123",
        "application_id": null,
        "url": "https://discord.com/api/webhooks/12345/secret_token_123"
    }"#;

    const SAMPLE_MESSAGE: &str = r#"{
        "id": "99999",
        "channel_id": "11111",
        "content": "Hello from webhook!"
    }"#;

    #[test]
    fn webhook_debug_redacts_token_and_url() {
        let wh: Webhook = serde_json::from_str(SAMPLE_WEBHOOK).unwrap();
        let debug = format!("{wh:?}");
        assert!(!debug.contains("secret_token_123"));
        assert!(debug.contains("<redacted>"));
    }

    #[tokio::test(start_paused = true)]
    async fn get_modify_delete_webhook_routes_match_spec() {
        let c = client(vec![
            resp(200, SAMPLE_WEBHOOK),
            resp(200, SAMPLE_WEBHOOK),
            resp(204, ""),
        ]);

        let wh = c.get_webhook(sf(12345)).await.unwrap();
        assert_eq!(wh.id, sf(12345));

        let mod_req = ModifyWebhook::new().name("New Hook Name");
        let updated = c.modify_webhook(sf(12345), &mod_req, Some("Audit Reason")).await.unwrap();
        assert_eq!(updated.id, sf(12345));

        c.delete_webhook(sf(12345), Some("Audit Reason")).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/webhooks/12345");
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/webhooks/12345");
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "Audit Reason"));
        assert_eq!(seen[2].method, "DELETE");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/webhooks/12345");
    }

    #[tokio::test(start_paused = true)]
    async fn token_webhook_routes_match_spec() {
        let c = client(vec![
            resp(200, SAMPLE_WEBHOOK),
            resp(200, SAMPLE_WEBHOOK),
            resp(204, ""),
        ]);

        let wh = c.get_webhook_with_token(sf(12345), "secret_tok").await.unwrap();
        assert_eq!(wh.id, sf(12345));

        let mod_req = ModifyWebhookWithToken::new().name("Token Hook");
        let updated = c.modify_webhook_with_token(sf(12345), "secret_tok", &mod_req).await.unwrap();
        assert_eq!(updated.id, sf(12345));

        c.delete_webhook_with_token(sf(12345), "secret_tok").await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/webhooks/12345/secret_tok");
        assert_eq!(seen[1].method, "PATCH");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/webhooks/12345/secret_tok");
        assert_eq!(seen[2].method, "DELETE");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/webhooks/12345/secret_tok");
    }

    #[tokio::test(start_paused = true)]
    async fn execute_webhook_and_third_party_routes() {
        let c = client(vec![
            resp(200, SAMPLE_MESSAGE),
            resp(204, ""),
            resp(204, ""),
            resp(204, ""),
        ]);

        let exec = ExecuteWebhook::new()
            .content("Hello World")
            .wait(true)
            .thread_id(sf(777));
        let msg = c.execute_webhook(sf(12345), "tok", &exec).await.unwrap();
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, sf(99999));

        let exec_no_wait = ExecuteWebhook::new().content("Fire and forget");
        let no_msg = c.execute_webhook(sf(12345), "tok", &exec_no_wait).await.unwrap();
        assert!(no_msg.is_none());

        let gh_payload = json!({"ref": "refs/heads/main"});
        c.execute_github_compatible_webhook(sf(12345), "tok", &gh_payload, Some(false), None).await.unwrap();

        let slack_payload = json!({"text": "slack format"});
        c.execute_slack_compatible_webhook(sf(12345), "tok", &slack_payload, None, Some(sf(777))).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/webhooks/12345/tok?wait=true&thread_id=777");
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/webhooks/12345/tok");
        assert_eq!(seen[2].method, "POST");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/webhooks/12345/tok/github?wait=false");
        assert_eq!(seen[3].method, "POST");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/webhooks/12345/tok/slack?thread_id=777");
    }

    #[tokio::test(start_paused = true)]
    async fn webhook_message_routes_match_spec() {
        let c = client(vec![
            resp(200, SAMPLE_MESSAGE),
            resp(200, SAMPLE_MESSAGE),
            resp(200, SAMPLE_MESSAGE),
            resp(204, ""),
        ]);

        let orig = c.get_original_webhook_message(sf(12345), "tok", Some(sf(555))).await.unwrap();
        assert_eq!(orig.id, sf(99999));

        let msg = c.get_webhook_message(sf(12345), "tok", sf(99999), None).await.unwrap();
        assert_eq!(msg.id, sf(99999));

        let edit = WebhookMessageEdit::new().content("Updated message");
        let edited = c.edit_webhook_message(sf(12345), "tok", sf(99999), &edit, Some(sf(555))).await.unwrap();
        assert_eq!(edited.id, sf(99999));

        c.delete_webhook_message(sf(12345), "tok", sf(99999), None).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(seen[0].url, "https://discord.com/api/v10/webhooks/12345/tok/messages/@original?thread_id=555");
        assert_eq!(seen[1].method, "GET");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/webhooks/12345/tok/messages/99999");
        assert_eq!(seen[2].method, "PATCH");
        assert_eq!(seen[2].url, "https://discord.com/api/v10/webhooks/12345/tok/messages/99999?thread_id=555");
        assert_eq!(seen[3].method, "DELETE");
        assert_eq!(seen[3].url, "https://discord.com/api/v10/webhooks/12345/tok/messages/99999");
    }

    #[tokio::test(start_paused = true)]
    async fn validation_prevents_network_calls() {
        let c = client(vec![]);

        // Empty execute request rejected
        let empty_exec = ExecuteWebhook::new();
        assert!(c.execute_webhook(sf(123), "tok", &empty_exec).await.is_err());

        // Webhook name too long (over 80)
        let long_name = "a".repeat(81);
        let bad_mod = ModifyWebhook::new().name(&long_name);
        assert!(c.modify_webhook(sf(123), &bad_mod, None).await.is_err());

        // Content too long (over 2000)
        let long_content = "a".repeat(2001);
        let bad_exec = ExecuteWebhook::new().content(&long_content);
        assert!(c.execute_webhook(sf(123), "tok", &bad_exec).await.is_err());

        // Ensure zero network requests were made
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }
}
