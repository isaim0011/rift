//! Stage Instance resource endpoints.
//!
//! Official Discord API reference:
//! <https://docs.discord.com/developers/resources/stage-instance>
//!
//! Implements all 4 stage instance operations:
//! - Create Stage Instance (`POST /stage-instances`)
//! - Get Stage Instance (`GET /stage-instances/{channel_id}`)
//! - Modify Stage Instance (`PATCH /stage-instances/{channel_id}`)
//! - Delete Stage Instance (`DELETE /stage-instances/{channel_id}`)

use crate::client::RestClient;
use crate::error::Error;
use crate::message::{de_opt_snowflake, de_snowflake, ser_snowflake};
use crate::transport::Transport;
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};
use std::fmt;

// ───────────────────────── Stage Models ─────────────────────────

/// Privacy level of the Stage instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum StageInstancePrivacyLevel {
    /// The Stage instance is visible publicly (deprecated by Discord).
    Public = 1,
    /// The Stage instance is visible to only guild members.
    GuildOnly = 2,
}

impl StageInstancePrivacyLevel {
    /// Parse from raw integer.
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Public),
            2 => Some(Self::GuildOnly),
            _ => None,
        }
    }
    /// As integer.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Serialize for StageInstancePrivacyLevel {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for StageInstancePrivacyLevel {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = u8::deserialize(deserializer)?;
        Self::from_u8(v).ok_or_else(|| serde::de::Error::custom(format!("invalid privacy level: {v}")))
    }
}

/// A Stage instance holds information about a live stage in a Stage channel.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct StageInstance {
    /// The id of this Stage instance.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub id: Snowflake,
    /// The guild id of the associated Stage channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub guild_id: Snowflake,
    /// The id of the associated Stage channel.
    #[serde(deserialize_with = "de_snowflake", serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,
    /// The topic of the Stage instance (1-120 characters).
    pub topic: String,
    /// The privacy level of the Stage instance.
    pub privacy_level: StageInstancePrivacyLevel,
    /// Whether or not Stage Discovery is disabled (deprecated).
    #[serde(default)]
    pub discoverable_disabled: Option<bool>,
    /// The id of the scheduled event for this Stage instance.
    #[serde(default, deserialize_with = "de_opt_snowflake", serialize_with = "crate::message::ser_opt_snowflake")]
    pub guild_scheduled_event_id: Option<Snowflake>,
}

/// Request payload for `POST /stage-instances`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CreateStageInstance {
    /// The id of the Stage channel.
    #[serde(serialize_with = "ser_snowflake")]
    pub channel_id: Snowflake,
    /// The topic of the Stage instance (1-120 characters).
    pub topic: String,
    /// The privacy level of the Stage instance (default GuildOnly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<StageInstancePrivacyLevel>,
    /// Notify @everyone that a Stage instance has started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_start_notification: Option<bool>,
    /// The guild scheduled event id associated with this Stage instance.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub guild_scheduled_event_id: Option<Snowflake>,
}

/// Request payload for `PATCH /stage-instances/{channel_id}`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ModifyStageInstance {
    /// The topic of the Stage instance (1-120 characters).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    /// The privacy level of the Stage instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<StageInstancePrivacyLevel>,
}

// ───────────────────────── Validation Errors ─────────────────────────

/// Validation error for Stage instance operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageError {
    /// Stage topic length is invalid (must be 1-120 characters).
    BadTopic {
        /// Provided topic length.
        len: usize,
    },
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadTopic { len } => {
                write!(f, "stage topic length {len} is invalid; must be between 1 and 120 characters")
            }
        }
    }
}

impl std::error::Error for StageError {}

impl From<StageError> for Error {
    fn from(e: StageError) -> Self {
        Error::Transport(format!("stage instance validation: {e}"))
    }
}

fn check_topic(topic: &str) -> Result<(), StageError> {
    let count = topic.chars().count();
    if count == 0 || count > 120 {
        Err(StageError::BadTopic { len: count })
    } else {
        Ok(())
    }
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    /// `POST /stage-instances` – Creates a new Stage instance associated with a Stage channel.
    /// Supports `X-Audit-Log-Reason`.
    pub async fn create_stage_instance(
        &self,
        body: &CreateStageInstance,
        reason: Option<&str>,
    ) -> Result<StageInstance, Error> {
        check_topic(&body.topic)?;
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        self.request_with_headers("POST", "/stage-instances", Some(b), headers)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /stage-instances/{channel_id}` – Gets the stage instance associated with the Stage channel, if one exists.
    pub async fn get_stage_instance(&self, channel_id: Snowflake) -> Result<StageInstance, Error> {
        let path = format!("/stage-instances/{channel_id}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `PATCH /stage-instances/{channel_id}` – Updates fields of an existing Stage instance.
    /// Supports `X-Audit-Log-Reason`.
    pub async fn modify_stage_instance(
        &self,
        channel_id: Snowflake,
        body: &ModifyStageInstance,
        reason: Option<&str>,
    ) -> Result<StageInstance, Error> {
        if let Some(t) = &body.topic {
            check_topic(t)?;
        }
        let b = serde_json::to_vec(body).map_err(|e| Error::Decode(e.to_string()))?;
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        let path = format!("/stage-instances/{channel_id}");
        self.request_with_headers("PATCH", &path, Some(b), headers)
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /stage-instances/{channel_id}` – Deletes the Stage instance.
    /// Supports `X-Audit-Log-Reason`.
    pub async fn delete_stage_instance(&self, channel_id: Snowflake, reason: Option<&str>) -> Result<(), Error> {
        let path = format!("/stage-instances/{channel_id}");
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        self.request_with_headers("DELETE", &path, None, headers)
            .await?
            .expect_success()?;
        Ok(())
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
    fn topic_validation() {
        assert!(check_topic("").is_err());
        assert!(check_topic(&"a".repeat(121)).is_err());
        assert!(check_topic("Valid Stage Topic").is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn stage_instance_lifecycle_and_routes() {
        let stage_json = json!({
            "id": "1001",
            "guild_id": "2002",
            "channel_id": "3003",
            "topic": "Rust Q&A",
            "privacy_level": 2
        })
        .to_string();

        let c = client(vec![
            resp(200, &stage_json),
            resp(200, &stage_json),
            resp(200, &stage_json),
            resp(204, ""),
        ]);

        let create = CreateStageInstance {
            channel_id: sf(3003),
            topic: "Rust Q&A".into(),
            privacy_level: Some(StageInstancePrivacyLevel::GuildOnly),
            send_start_notification: Some(true),
            guild_scheduled_event_id: None,
        };
        let created = c.create_stage_instance(&create, Some("started stage")).await.unwrap();
        assert_eq!(created.topic, "Rust Q&A");
        assert_eq!(created.privacy_level, StageInstancePrivacyLevel::GuildOnly);

        let get = c.get_stage_instance(sf(3003)).await.unwrap();
        assert_eq!(get.id, sf(1001));

        let modify = ModifyStageInstance {
            topic: Some("Rust Q&A (Part 2)".into()),
            privacy_level: None,
        };
        let updated = c.modify_stage_instance(sf(3003), &modify, Some("updated topic")).await.unwrap();
        assert_eq!(updated.id, sf(1001));

        c.delete_stage_instance(sf(3003), Some("closed stage")).await.unwrap();

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].url, "https://discord.com/api/v10/stage-instances");
        assert_eq!(seen[0].method, "POST");
        assert!(seen[0].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "started stage"));

        assert_eq!(seen[1].url, "https://discord.com/api/v10/stage-instances/3003");
        assert_eq!(seen[1].method, "GET");

        assert_eq!(seen[2].url, "https://discord.com/api/v10/stage-instances/3003");
        assert_eq!(seen[2].method, "PATCH");
        assert!(seen[2].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "updated topic"));

        assert_eq!(seen[3].url, "https://discord.com/api/v10/stage-instances/3003");
        assert_eq!(seen[3].method, "DELETE");
        assert!(seen[3].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "closed stage"));
    }

    #[tokio::test(start_paused = true)]
    async fn modify_validates_topic_length() {
        let c = client(vec![]);
        let bad = ModifyStageInstance {
            topic: Some("".into()),
            privacy_level: None,
        };
        assert!(c.modify_stage_instance(sf(3003), &bad, None).await.is_err());
    }
}
