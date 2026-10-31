//! Multipart/form-data file upload support for Discord REST endpoints.
//!
//! Discord allows attaching files to messages, webhook executions, and interaction
//! responses/followups. These requests must be sent as `multipart/form-data` with:
//! - a `payload_json` part containing the serialized JSON body
//! - binary file parts named `files[0]`, `files[1]`, etc.
//! - an `attachments` array referencing each uploaded file by its index `0`, `1`, etc.
//!
//! Limits verified from Discord developer documentation:
//! - Maximum 10 file attachments per message (`MAX_FILES`)
//! - Maximum 25 MiB total upload size for bots (`MAX_TOTAL_FILE_SIZE`)
//! - Filename: 1-1024 characters (`MAX_FILENAME_LEN`)
//! - Description / alt-text: max 1024 characters (`MAX_DESCRIPTION_LEN`)
//!
//! Unverified / server-tier dependent:
//! - Boosted servers can support 50 MiB or 100 MiB (Nitro users up to 500 MiB); the safe
//!   25 MiB bot baseline limit is enforced locally.

use crate::client::RestClient;
use crate::error::Error;
use crate::interaction::{InteractionMessage, InteractionResponse};
use crate::message::{
    validate_filename, CreateMessage, EditMessage, Message, ValidationError,
    MAX_DESCRIPTION_LEN, MAX_FILES, MAX_TOTAL_FILE_SIZE,
};
use crate::transport::{Multipart, Part, Transport};
use crate::webhook::{ExecuteWebhook, WebhookMessageEdit};
use rift_core::Snowflake;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

static BOUNDARY_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique MIME boundary string for multipart requests.
pub(crate) fn generate_boundary() -> String {
    let cnt = BOUNDARY_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("---------------------------rift{:016x}{:016x}", ts, cnt)
}

/// A file to be uploaded in a multipart request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadFile {
    /// The filename (e.g. `"screenshot.png"`).
    pub filename: String,
    /// Raw binary file data.
    pub data: Vec<u8>,
    /// Optional MIME content-type (e.g. `"image/png"`). Defaults to `"application/octet-stream"`.
    pub content_type: Option<String>,
    /// Optional description / alt-text (max 1024 characters).
    pub description: Option<String>,
}

impl UploadFile {
    /// Create a new upload file with a filename and raw bytes.
    pub fn new(filename: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            filename: filename.into(),
            data: data.into(),
            content_type: None,
            description: None,
        }
    }

    /// Set the MIME content-type (e.g. `"image/png"`, `"text/plain"`).
    pub fn content_type(mut self, ct: impl Into<String>) -> Self {
        self.content_type = Some(ct.into());
        self
    }

    /// Set an optional description / alt-text for the attachment (max 1024 characters).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Validate this file's filename and description locally.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_filename(&self.filename)?;
        if let Some(desc) = &self.description {
            let len = desc.chars().count();
            if len > MAX_DESCRIPTION_LEN {
                return Err(ValidationError::AttachmentDescriptionTooLong { len });
            }
        }
        Ok(())
    }
}

/// Validate a list of files locally against Discord's documented limits.
pub fn validate_files(files: &[UploadFile]) -> Result<(), ValidationError> {
    if files.len() > MAX_FILES {
        return Err(ValidationError::TooManyFiles { count: files.len() });
    }
    let mut total_size = 0usize;
    for file in files {
        file.validate()?;
        total_size += file.data.len();
    }
    if total_size > MAX_TOTAL_FILE_SIZE {
        return Err(ValidationError::FilesTooLarge {
            total: total_size,
            max: MAX_TOTAL_FILE_SIZE,
        });
    }
    Ok(())
}

/// Build a [`Multipart`] struct from a JSON payload and files with an auto-generated boundary.
pub fn build_multipart(payload_json: &[u8], files: Vec<UploadFile>) -> Multipart {
    build_multipart_with_boundary(payload_json, files, generate_boundary())
}

/// Build a [`Multipart`] struct from a JSON payload and files with a specific boundary.
pub fn build_multipart_with_boundary(
    payload_json: &[u8],
    files: Vec<UploadFile>,
    boundary: String,
) -> Multipart {
    let mut parts = Vec::with_capacity(1 + files.len());
    parts.push(
        Part::new("payload_json", payload_json.to_vec())
            .content_type("application/json"),
    );
    for (i, file) in files.into_iter().enumerate() {
        let ct = file
            .content_type
            .unwrap_or_else(|| "application/octet-stream".to_string());
        parts.push(
            Part::new(format!("files[{i}]"), file.data)
                .filename(file.filename)
                .content_type(ct),
        );
    }
    Multipart::new(boundary, parts)
}

fn make_attachments_json(files: &[UploadFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut obj = serde_json::json!({
                "id": i,
                "filename": f.filename,
            });
            if let Some(desc) = &f.description {
                obj["description"] = serde_json::Value::String(desc.clone());
            }
            obj
        })
        .collect()
}

impl<T: Transport> RestClient<T> {
    /// `POST /channels/{channel_id}/messages`: create a message with uploaded files.
    pub async fn create_message_with_files(
        &self,
        channel_id: Snowflake,
        msg: &CreateMessage,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;

        let mut val = serde_json::to_value(msg).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            let atts = val
                .get_mut("attachments")
                .and_then(|a| a.as_array_mut());
            if let Some(existing) = atts {
                if existing.is_empty() {
                    *existing = make_attachments_json(&files);
                }
            } else {
                val["attachments"] = serde_json::Value::Array(make_attachments_json(&files));
            }
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/channels/{channel_id}/messages");
        self.request_multipart("POST", &path, multipart)
            .await?
            .expect_success()?
            .json()
    }

    /// `PATCH /channels/{channel_id}/messages/{message_id}`: edit a message with file attachments.
    pub async fn edit_message_with_files(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        edit: &EditMessage,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;
        edit.validate()?;

        let mut val = serde_json::to_value(edit).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            let new_atts = make_attachments_json(&files);
            if let Some(existing) = val.get_mut("attachments").and_then(|a| a.as_array_mut()) {
                existing.extend(new_atts);
            } else {
                val["attachments"] = serde_json::Value::Array(new_atts);
            }
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/channels/{channel_id}/messages/{message_id}");
        self.request_multipart("PATCH", &path, multipart)
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /webhooks/{webhook_id}/{token}`: execute a webhook with uploaded files.
    pub async fn execute_webhook_with_files(
        &self,
        webhook_id: Snowflake,
        token: &str,
        exec: &ExecuteWebhook,
        files: Vec<UploadFile>,
    ) -> Result<Option<Message>, Error> {
        validate_files(&files)?;
        exec.validate()?;

        let mut val = serde_json::to_value(exec).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            val["attachments"] = serde_json::Value::Array(make_attachments_json(&files));
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);

        let mut path = format!("/webhooks/{webhook_id}/{token}");
        let mut qs = Vec::new();
        if let Some(w) = exec.wait {
            qs.push(format!("wait={w}"));
        }
        if let Some(t) = exec.thread_id {
            qs.push(format!("thread_id={t}"));
        }
        if let Some(c) = exec.with_components {
            qs.push(format!("with_components={c}"));
        }
        if !qs.is_empty() {
            path.push('?');
            path.push_str(&qs.join("&"));
        }

        let resp = self.request_multipart("POST", &path, multipart).await?;
        if resp.status == 204 {
            Ok(None)
        } else {
            Ok(Some(resp.expect_success()?.json()?))
        }
    }

    /// `PATCH /webhooks/{webhook_id}/{token}/messages/{message_id}`: edit a webhook message with file attachments.
    pub async fn edit_webhook_message_with_files(
        &self,
        webhook_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        edit: &WebhookMessageEdit,
        thread_id: Option<Snowflake>,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;
        edit.validate()?;

        let mut val = serde_json::to_value(edit).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            let new_atts = make_attachments_json(&files);
            if let Some(existing) = val.get_mut("attachments").and_then(|a| a.as_array_mut()) {
                existing.extend(new_atts);
            } else {
                val["attachments"] = serde_json::Value::Array(new_atts);
            }
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);

        let mut path = format!("/webhooks/{webhook_id}/{token}/messages/{message_id}");
        if let Some(t) = thread_id {
            path.push_str(&format!("?thread_id={t}"));
        }

        self.request_multipart("PATCH", &path, multipart)
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /interactions/{interaction_id}/{token}/callback`: respond to an interaction with files.
    ///
    /// The Discord documentation only allows file uploads for interaction responses with
    /// message data: `CHANNEL_MESSAGE_WITH_SOURCE` (type 4) and `UPDATE_MESSAGE` (type 7).
    /// Other response types return a local validation error.
    pub async fn create_interaction_response_with_files(
        &self,
        interaction_id: Snowflake,
        token: &str,
        response: &InteractionResponse,
        files: Vec<UploadFile>,
    ) -> Result<(), Error> {
        validate_files(&files)?;
        response.validate()?;

        let mut val = serde_json::to_value(response).map_err(|e| Error::Decode(e.to_string()))?;
        let kind = val.get("type").and_then(|t| t.as_u64());
        if !files.is_empty() && kind != Some(4) && kind != Some(7) {
            return Err(Error::Transport(
                "Discord only allows file attachments on interaction responses of type 4 (ChannelMessageWithSource) or 7 (UpdateMessage)".into()
            ));
        }

        if let Some(data) = val.get_mut("data").and_then(|d| d.as_object_mut()) {
            if !data.contains_key("attachments") && !files.is_empty() {
                data.insert("attachments".to_string(), serde_json::Value::Array(make_attachments_json(&files)));
            }
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/interactions/{interaction_id}/{token}/callback");
        self.request_multipart("POST", &path, multipart)
            .await?
            .expect_success()?;
        Ok(())
    }

    /// `PATCH /webhooks/{application_id}/{token}/messages/@original`: edit the initial interaction response with files.
    pub async fn edit_original_response_with_files(
        &self,
        application_id: Snowflake,
        token: &str,
        msg: &InteractionMessage,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;
        msg.validate()?;

        let mut val = serde_json::to_value(msg).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            val["attachments"] = serde_json::Value::Array(make_attachments_json(&files));
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/webhooks/{application_id}/{token}/messages/@original");
        self.request_multipart("PATCH", &path, multipart)
            .await?
            .expect_success()?
            .json()
    }

    /// `POST /webhooks/{application_id}/{token}`: create an interaction followup message with files.
    pub async fn create_followup_with_files(
        &self,
        application_id: Snowflake,
        token: &str,
        msg: &InteractionMessage,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;
        msg.validate_new()?;

        let mut val = serde_json::to_value(msg).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            val["attachments"] = serde_json::Value::Array(make_attachments_json(&files));
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/webhooks/{application_id}/{token}");
        self.request_multipart("POST", &path, multipart)
            .await?
            .expect_success()?
            .json()
    }

    /// `PATCH /webhooks/{application_id}/{token}/messages/{message_id}`: edit an interaction followup message with files.
    pub async fn edit_followup_with_files(
        &self,
        application_id: Snowflake,
        token: &str,
        message_id: Snowflake,
        msg: &InteractionMessage,
        files: Vec<UploadFile>,
    ) -> Result<Message, Error> {
        validate_files(&files)?;
        msg.validate()?;

        let mut val = serde_json::to_value(msg).map_err(|e| Error::Decode(e.to_string()))?;
        if !files.is_empty() {
            val["attachments"] = serde_json::Value::Array(make_attachments_json(&files));
        }

        let payload_json = serde_json::to_vec(&val).map_err(|e| Error::Decode(e.to_string()))?;
        let multipart = build_multipart(&payload_json, files);
        let path = format!("/webhooks/{application_id}/{token}/messages/{message_id}");
        self.request_multipart("PATCH", &path, multipart)
            .await?
            .expect_success()?
            .json()
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

    #[test]
    fn multipart_body_and_boundary_shape() {
        let boundary = "test_boundary_12345";
        let payload = br#"{"content":"hello world"}"#;
        let files = vec![
            UploadFile::new("file1.txt", b"content of file 1".to_vec()).content_type("text/plain"),
            UploadFile::new("cat.png", b"\x89PNG\r\n\x1a\n".to_vec()).content_type("image/png"),
        ];

        let mp = build_multipart_with_boundary(payload, files, boundary.to_string());
        assert_eq!(mp.boundary, boundary);
        assert_eq!(mp.parts.len(), 3);

        let encoded = mp.encode();
        let body_str = String::from_utf8_lossy(&encoded);

        // Verify boundary delimiters and part order
        let expected_payload = "--test_boundary_12345\r\n\
            Content-Disposition: form-data; name=\"payload_json\"\r\n\
            Content-Type: application/json\r\n\r\n\
            {\"content\":\"hello world\"}\r\n";
        assert!(body_str.contains(expected_payload));

        let expected_file1 = "--test_boundary_12345\r\n\
            Content-Disposition: form-data; name=\"files[0]\"; filename=\"file1.txt\"\r\n\
            Content-Type: text/plain\r\n\r\n\
            content of file 1\r\n";
        assert!(body_str.contains(expected_file1));

        let expected_file2_header = "--test_boundary_12345\r\n\
            Content-Disposition: form-data; name=\"files[1]\"; filename=\"cat.png\"\r\n\
            Content-Type: image/png\r\n\r\n";
        assert!(body_str.contains(expected_file2_header));

        // Verify closing boundary
        assert!(body_str.ends_with("--test_boundary_12345--\r\n"));
    }

    #[tokio::test(start_paused = true)]
    async fn create_message_with_files_sends_multipart_and_references_attachments() {
        let msg_json = json!({
            "id": "100",
            "channel_id": "200",
            "content": "see attached"
        }).to_string();
        let c = client(vec![resp(200, &msg_json)]);

        let msg = CreateMessage::new().content("see attached");
        let files = vec![
            UploadFile::new("diagram.png", b"fake png bytes".to_vec())
                .content_type("image/png")
                .description("architecture diagram"),
        ];

        let m = c.create_message_with_files(sf(200), &msg, files).await.unwrap();
        assert_eq!(m.id, sf(100));

        let seen = c.transport().seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        let req = &seen[0];
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://discord.com/api/v10/channels/200/messages");

        // Header check
        let ct_header = req.headers.iter().find(|(k, _)| k == "Content-Type").unwrap();
        assert!(ct_header.1.starts_with("multipart/form-data; boundary="));

        // Body checks
        let body = req.body.as_ref().unwrap();
        let body_str = String::from_utf8_lossy(body);
        assert!(body_str.contains("Content-Disposition: form-data; name=\"payload_json\""));
        assert!(body_str.contains("\"content\":\"see attached\""));
        assert!(body_str.contains("\"attachments\":[{\"description\":\"architecture diagram\",\"filename\":\"diagram.png\",\"id\":0}]"));
        assert!(body_str.contains("Content-Disposition: form-data; name=\"files[0]\"; filename=\"diagram.png\""));
        assert!(body_str.contains("Content-Type: image/png"));
        assert!(body_str.contains("fake png bytes"));
    }

    #[tokio::test(start_paused = true)]
    async fn local_validation_enforces_file_limits() {
        let c = client(vec![]);
        let msg = CreateMessage::new().content("hello");

        // More than 10 files
        let eleven_files: Vec<_> = (0..11)
            .map(|i| UploadFile::new(format!("f{i}.txt"), b"data".to_vec()))
            .collect();
        let err = c.create_message_with_files(sf(1), &msg, eleven_files).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::TooManyFiles { count: 11 }))));

        // Combined file size > 25 MiB
        let large_file = UploadFile::new("large.bin", vec![0u8; 26 * 1024 * 1024]);
        let err = c.create_message_with_files(sf(1), &msg, vec![large_file]).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::FilesTooLarge { .. }))));

        // Empty filename
        let empty_name = UploadFile::new("", b"data".to_vec());
        let err = c.create_message_with_files(sf(1), &msg, vec![empty_name]).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::EmptyFilename))));

        // Filename too long (> 1024 chars)
        let long_name = UploadFile::new("a".repeat(1025), b"data".to_vec());
        let err = c.create_message_with_files(sf(1), &msg, vec![long_name]).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::FilenameTooLong { len: 1025 }))));

        // Invalid filename control characters
        let bad_name = UploadFile::new("file\nname.txt", b"data".to_vec());
        let err = c.create_message_with_files(sf(1), &msg, vec![bad_name]).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::InvalidFilename { .. }))));

        // Description too long (> 1024 chars)
        let long_desc = UploadFile::new("file.txt", b"data".to_vec()).description("d".repeat(1025));
        let err = c.create_message_with_files(sf(1), &msg, vec![long_desc]).await;
        assert!(matches!(err, Err(Error::Validation(ValidationError::AttachmentDescriptionTooLong { len: 1025 }))));

        // None of these invalid requests reached the network!
        assert!(c.transport().seen.lock().unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn webhook_and_interaction_tokens_stay_out_of_error_text() {
        let secret_webhook_token = "secret_webhook_token_xyz_999";
        let secret_interaction_token = "secret_interaction_token_abc_777";

        // Simulated network error
        let c = client(vec![Err("connection reset by peer".into()), Err("timeout".into())]);

        let exec = ExecuteWebhook::new().content("alert");
        let file = UploadFile::new("log.txt", b"error log".to_vec());
        let err = c.execute_webhook_with_files(sf(12345), secret_webhook_token, &exec, vec![file.clone()]).await;
        let err_text = format!("{err:?}");
        assert!(!err_text.contains(secret_webhook_token));

        let resp_msg = InteractionResponse::message(InteractionMessage::new().content("done"));
        let err2 = c.create_interaction_response_with_files(sf(67890), secret_interaction_token, &resp_msg, vec![file]).await;
        let err2_text = format!("{err2:?}");
        assert!(!err2_text.contains(secret_interaction_token));
    }

    #[tokio::test(start_paused = true)]
    async fn interaction_response_only_allows_files_on_message_types() {
        let c = client(vec![]);
        let file = UploadFile::new("test.txt", b"data".to_vec());

        // Pong does not allow files
        let pong = InteractionResponse::pong();
        let err = c.create_interaction_response_with_files(sf(1), "tok", &pong, vec![file.clone()]).await;
        assert!(err.is_err());

        // Defer does not allow files
        let defer = InteractionResponse::defer(false);
        let err = c.create_interaction_response_with_files(sf(1), "tok", &defer, vec![file]).await;
        assert!(err.is_err());
    }
}
