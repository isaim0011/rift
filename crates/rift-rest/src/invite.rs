//! Invite resource endpoints.
//!
//! Official Discord API reference:
//! <https://docs.discord.com/developers/resources/invite>
//!
//! Implements all 9 invite operations:
//! - Get Invite (`GET /invites/{code}`)
//! - Delete Invite (`DELETE /invites/{code}`)
//! - Get Invite Target Users (`GET /invites/{code}/target-users`)
//! - Update Invite Target Users (`PUT /invites/{code}/target-users`)
//! - Bulk Add Invite Target Users (`POST /invites/{code}/target-users/bulk-add`)
//! - Bulk Remove Invite Target Users (`POST /invites/{code}/target-users/bulk-delete`)
//! - Get Invite Target Users Job Status (`GET /invites/{code}/target-users/job-status`)
//! - Add Invite Target User (`PUT /invites/{code}/target-users/{user_id}`)
//! - Remove Invite Target User (`DELETE /invites/{code}/target-users/{user_id}`)

use crate::client::RestClient;
use crate::error::Error;
use crate::message::ser_snowflake;
use crate::transport::Transport;
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};
use std::fmt;

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

// ───────────────────────── Invite Models ─────────────────────────

/// Query parameters for `GET /invites/{code}`.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct GetInvite {
    /// Whether the invite should contain approximate member and presence counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_counts: Option<bool>,
    /// Whether the invite should contain the expiration date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_expiration: Option<bool>,
    /// The guild scheduled event to include with the invite.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub guild_scheduled_event_id: Option<Snowflake>,
    /// The target channel id for the invite.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub target_channel_id: Option<Snowflake>,
    /// The target message id for the invite.
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "crate::message::ser_opt_snowflake")]
    pub target_message_id: Option<Snowflake>,
}

/// Resolved invite response returned by `GET /invites/{code}` and `DELETE /invites/{code}`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ResolvedInvite {
    /// The invite code (unique ID).
    pub code: String,
    /// The type of invite (0 = Guild, 1 = Group DM, 2 = Friend).
    #[serde(rename = "type", default)]
    pub kind: Option<u8>,
    /// Partial guild object this invite is for.
    #[serde(default)]
    pub guild: Option<serde_json::Value>,
    /// Partial channel object this invite is for.
    #[serde(default)]
    pub channel: Option<serde_json::Value>,
    /// The user who created the invite.
    #[serde(default)]
    pub inviter: Option<crate::interaction::User>,
    /// The type of target for this voice channel invite.
    #[serde(default)]
    pub target_type: Option<u8>,
    /// The user whose stream to display for this voice channel stream invite.
    #[serde(default)]
    pub target_user: Option<crate::interaction::User>,
    /// The embedded application to open for this voice channel activity invite.
    #[serde(default)]
    pub target_application: Option<serde_json::Value>,
    /// Approximate count of online members, returned when `with_counts` is true.
    #[serde(default)]
    pub approximate_presence_count: Option<u64>,
    /// Approximate count of total members, returned when `with_counts` is true.
    #[serde(default)]
    pub approximate_member_count: Option<u64>,
    /// The expiration date of this invite, returned when `with_expiration` is true.
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Stage instance or scheduled event data, if applicable.
    #[serde(default)]
    pub guild_scheduled_event: Option<serde_json::Value>,
}

/// Status of the background job processing invite target users.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TargetUsersJobStatus {
    /// The status of the job processing the target users (e.g. "queued", "processing", "completed", "failed").
    pub status: String,
    /// Total number of users in the provided list.
    pub total_users: u32,
    /// Number of users processed so far.
    pub processed_users: u32,
    /// Timestamp when the job was created.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Timestamp when the job was successfully completed.
    #[serde(default)]
    pub completed_at: Option<String>,
    /// Error message if the job failed.
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Serialize)]
struct TargetUserIdsPayload<'a> {
    user_ids: &'a [SnowflakeStr],
}

#[derive(Serialize)]
struct SnowflakeStr(#[serde(serialize_with = "ser_snowflake")] Snowflake);

// ───────────────────────── Validation Errors ─────────────────────────

/// Validation errors for invite endpoint payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InviteError {
    /// Invite code is empty.
    EmptyCode,
    /// Target users count must be between 1 and 1000.
    BadTargetUserCount {
        /// Provided count.
        count: usize,
    },
}

impl fmt::Display for InviteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCode => write!(f, "invite code cannot be empty"),
            Self::BadTargetUserCount { count } => {
                write!(f, "target user count {count} is invalid; must be between 1 and 1000")
            }
        }
    }
}

impl std::error::Error for InviteError {}

impl From<InviteError> for Error {
    fn from(e: InviteError) -> Self {
        Error::Transport(format!("invite validation: {e}"))
    }
}

// ───────────────────────── REST Methods ─────────────────────────

impl<T: Transport> RestClient<T> {
    /// `GET /invites/{code}` – Returns an invite object for the given code.
    pub async fn get_invite(&self, code: &str, query: &GetInvite) -> Result<ResolvedInvite, Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let qs = query_string(query);
        let path = format!("/invites/{code}{qs}");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `DELETE /invites/{code}` – Revokes an invite. Requires MANAGE_CHANNELS or MANAGE_GUILD.
    /// Supports `X-Audit-Log-Reason`.
    pub async fn delete_invite(&self, code: &str, reason: Option<&str>) -> Result<ResolvedInvite, Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let path = format!("/invites/{code}");
        let headers = reason
            .map(|r| vec![("X-Audit-Log-Reason".to_owned(), r.to_owned())])
            .unwrap_or_default();
        self.request_with_headers("DELETE", &path, None, headers)
            .await?
            .expect_success()?
            .json()
    }

    /// `GET /invites/{code}/target-users` – Returns a CSV file containing target users for the invite.
    pub async fn get_invite_target_users(&self, code: &str) -> Result<String, Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let path = format!("/invites/{code}/target-users");
        let resp = self.request("GET", &path, None).await?.expect_success()?;
        Ok(resp.text())
    }

    /// `PUT /invites/{code}/target-users` – Update the target users file for an existing invite.
    /// Sends the CSV bytes in a multipart/form-data boundary.
    pub async fn update_invite_target_users(&self, code: &str, csv_content: &[u8]) -> Result<(), Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let boundary = "----RiftTargetUsersBoundary7MA4YWxkTrZu0gW";
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"target_users_file\"; filename=\"target_users.csv\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: text/csv\r\n\r\n");
        body.extend_from_slice(csv_content);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let headers = vec![(
            "Content-Type".to_owned(),
            format!("multipart/form-data; boundary={boundary}"),
        )];
        let path = format!("/invites/{code}/target-users");
        self.request_with_headers("PUT", &path, Some(body), headers)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /invites/{code}/target-users/bulk-add` – Bulk adds target users to an invite (1-1000 users).
    pub async fn bulk_add_invite_target_users(&self, code: &str, user_ids: &[Snowflake]) -> Result<(), Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        if user_ids.is_empty() || user_ids.len() > 1000 {
            return Err(InviteError::BadTargetUserCount {
                count: user_ids.len(),
            }
            .into());
        }
        let wrapped: Vec<SnowflakeStr> = user_ids.iter().copied().map(SnowflakeStr).collect();
        let payload = TargetUserIdsPayload { user_ids: &wrapped };
        let b = serde_json::to_vec(&payload).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/invites/{code}/target-users/bulk-add");
        self.request("POST", &path, Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `POST /invites/{code}/target-users/bulk-delete` – Bulk removes target users from an invite (1-1000 users).
    pub async fn bulk_remove_invite_target_users(&self, code: &str, user_ids: &[Snowflake]) -> Result<(), Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        if user_ids.is_empty() || user_ids.len() > 1000 {
            return Err(InviteError::BadTargetUserCount {
                count: user_ids.len(),
            }
            .into());
        }
        let wrapped: Vec<SnowflakeStr> = user_ids.iter().copied().map(SnowflakeStr).collect();
        let payload = TargetUserIdsPayload { user_ids: &wrapped };
        let b = serde_json::to_vec(&payload).map_err(|e| Error::Decode(e.to_string()))?;
        let path = format!("/invites/{code}/target-users/bulk-delete");
        self.request("POST", &path, Some(b))
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `GET /invites/{code}/target-users/job-status` – Gets the processing job status of target users.
    pub async fn get_invite_target_users_job_status(&self, code: &str) -> Result<TargetUsersJobStatus, Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let path = format!("/invites/{code}/target-users/job-status");
        self.request("GET", &path, None)
            .await?
            .expect_success()?
            .json()
    }

    /// `PUT /invites/{code}/target-users/{user_id}` – Add a single target user to an invite.
    pub async fn add_invite_target_user(&self, code: &str, user_id: Snowflake) -> Result<(), Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let path = format!("/invites/{code}/target-users/{user_id}");
        self.request("PUT", &path, None)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `DELETE /invites/{code}/target-users/{user_id}` – Remove a single target user from an invite.
    pub async fn remove_invite_target_user(&self, code: &str, user_id: Snowflake) -> Result<(), Error> {
        if code.is_empty() {
            return Err(InviteError::EmptyCode.into());
        }
        let path = format!("/invites/{code}/target-users/{user_id}");
        self.request("DELETE", &path, None)
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
    fn invite_error_validation() {
        assert_eq!(
            InviteError::BadTargetUserCount { count: 0 }.to_string(),
            "target user count 0 is invalid; must be between 1 and 1000"
        );
        assert_eq!(InviteError::EmptyCode.to_string(), "invite code cannot be empty");
    }

    #[tokio::test(start_paused = true)]
    async fn get_and_delete_invite_routes() {
        let inv_json = json!({
            "code": "testcode",
            "type": 0,
            "approximate_member_count": 42
        })
        .to_string();

        let c = client(vec![resp(200, &inv_json), resp(200, &inv_json)]);

        let q = GetInvite {
            with_counts: Some(true),
            with_expiration: Some(true),
            guild_scheduled_event_id: Some(sf(777)),
            ..Default::default()
        };

        let inv = c.get_invite("testcode", &q).await.unwrap();
        assert_eq!(inv.code, "testcode");
        assert_eq!(inv.approximate_member_count, Some(42));

        let deleted = c.delete_invite("testcode", Some("audit reason")).await.unwrap();
        assert_eq!(deleted.code, "testcode");

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen[0].method, "GET");
        assert_eq!(
            seen[0].url,
            "https://discord.com/api/v10/invites/testcode?guild_scheduled_event_id=777&with_counts=true&with_expiration=true"
        );
        assert_eq!(seen[1].method, "DELETE");
        assert_eq!(seen[1].url, "https://discord.com/api/v10/invites/testcode");
        assert!(seen[1].headers.iter().any(|(k, v)| k == "X-Audit-Log-Reason" && v == "audit reason"));
    }

    #[tokio::test(start_paused = true)]
    async fn target_users_endpoints_routes_and_validation() {
        let job_json = json!({
            "status": "processing",
            "total_users": 100,
            "processed_users": 50,
            "created_at": "2026-01-01T00:00:00Z"
        })
        .to_string();

        let c = client(vec![
            resp(200, "user_id\n123\n456"),
            resp(204, ""),
            resp(204, ""),
            resp(204, ""),
            resp(200, &job_json),
            resp(204, ""),
            resp(204, ""),
        ]);

        let csv = c.get_invite_target_users("testcode").await.unwrap();
        assert!(csv.contains("user_id"));

        c.update_invite_target_users("testcode", b"user_id\n123").await.unwrap();

        c.bulk_add_invite_target_users("testcode", &[sf(123), sf(456)]).await.unwrap();
        c.bulk_remove_invite_target_users("testcode", &[sf(123), sf(456)]).await.unwrap();

        let job = c.get_invite_target_users_job_status("testcode").await.unwrap();
        assert_eq!(job.status, "processing");
        assert_eq!(job.processed_users, 50);

        c.add_invite_target_user("testcode", sf(999)).await.unwrap();
        c.remove_invite_target_user("testcode", sf(999)).await.unwrap();

        {
            let seen = c.transport().seen.lock().unwrap();
            assert_eq!(seen[0].url, "https://discord.com/api/v10/invites/testcode/target-users");
            assert_eq!(seen[0].method, "GET");

            assert_eq!(seen[1].url, "https://discord.com/api/v10/invites/testcode/target-users");
            assert_eq!(seen[1].method, "PUT");
            assert!(seen[1].headers.iter().any(|(k, v)| k == "Content-Type" && v.contains("multipart/form-data")));

            assert_eq!(seen[2].url, "https://discord.com/api/v10/invites/testcode/target-users/bulk-add");
            assert_eq!(seen[2].method, "POST");

            assert_eq!(seen[3].url, "https://discord.com/api/v10/invites/testcode/target-users/bulk-delete");
            assert_eq!(seen[3].method, "POST");

            assert_eq!(seen[4].url, "https://discord.com/api/v10/invites/testcode/target-users/job-status");
            assert_eq!(seen[4].method, "GET");

            assert_eq!(seen[5].url, "https://discord.com/api/v10/invites/testcode/target-users/999");
            assert_eq!(seen[5].method, "PUT");

            assert_eq!(seen[6].url, "https://discord.com/api/v10/invites/testcode/target-users/999");
            assert_eq!(seen[6].method, "DELETE");
        }

        // Validation: empty slice or over 1000
        assert!(c.bulk_add_invite_target_users("testcode", &[]).await.is_err());
        let thousand_one = vec![sf(1); 1001];
        assert!(c.bulk_add_invite_target_users("testcode", &thousand_one).await.is_err());
    }
}
