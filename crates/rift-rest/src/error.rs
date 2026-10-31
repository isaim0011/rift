use crate::command::CommandError;
use crate::message::ValidationError;
use rift_http::BlockReason;
use std::fmt;

/// REST client errors.
#[derive(Debug)]
pub enum Error {
    /// The request never completed (DNS, TLS, connection, ...).
    Transport(String),
    /// The limiter refused to send (invalid-request circuit open, or dead webhook).
    Blocked(BlockReason),
    /// Still rate limited after every retry.
    RateLimited,
    /// A non-success status where success was required.
    Status {
        /// HTTP status.
        status: u16,
        /// Response body (lossy UTF-8).
        body: String,
    },
    /// The body wasn't the JSON we expected.
    Decode(String),
    /// Discord rejected the request and explained why (`{"code", "message"}` body).
    Api {
        /// HTTP status.
        status: u16,
        /// Discord's JSON error code, if present.
        code: Option<u64>,
        /// Discord's message.
        message: String,
    },
    /// The request was invalid and was never sent.
    Validation(ValidationError),
    /// A command definition was invalid and was never sent.
    InvalidCommand(CommandError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "transport error: {e}"),
            Self::Blocked(BlockReason::CircuitOpen { retry_after_ms }) => write!(
                f,
                "too many invalid requests; refusing to send for {retry_after_ms} ms to avoid a ban"
            ),
            Self::Blocked(BlockReason::DeadWebhook) => write!(f, "webhook no longer exists (404)"),
            Self::RateLimited => write!(f, "still rate limited after all retries"),
            Self::Status { status, body } => write!(f, "unexpected status {status}: {body}"),
            Self::Decode(e) => write!(f, "could not decode response: {e}"),
            Self::Api { status, code: Some(code), message } => {
                write!(f, "Discord API error {status} (code {code}): {message}")
            }
            Self::Api { status, code: None, message } => {
                write!(f, "Discord API error {status}: {message}")
            }
            Self::Validation(e) => write!(f, "invalid request (not sent): {e}"),
            Self::InvalidCommand(e) => write!(f, "invalid command (not sent): {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ValidationError> for Error {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<CommandError> for Error {
    fn from(e: CommandError) -> Self {
        Self::InvalidCommand(e)
    }
}
