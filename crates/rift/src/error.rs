use std::fmt;

/// Errors from the top-level client.
#[derive(Debug)]
pub enum Error {
    /// A REST call failed (including `GET /gateway/bot` at startup).
    Rest(rift_rest::Error),
    /// A shard stopped for good (bad token, bad intents, identify budget, ...).
    Shard(rift_runtime::Error),
    /// The session-start limit can't cover every shard without eating the safety reserve.
    NotEnoughIdentifies {
        /// Shards that must identify.
        needed: u32,
        /// Identifies available after the reserve.
        usable: u32,
        /// Identifies Discord says remain in the current window.
        remaining: u32,
    },
    /// No such shard.
    UnknownShard(u32),
    /// The shard has stopped and can't take commands.
    ShardGone(u32),
    /// A gateway command payload failed validation or serialization.
    Command(rift_runtime::CommandError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rest(e) => write!(f, "REST error: {e}"),
            Self::Shard(e) => write!(f, "shard error: {e}"),
            Self::NotEnoughIdentifies { needed, usable, remaining } => write!(
                f,
                "need {needed} identifies but only {usable} are safe to use ({remaining} remain); \
                 starting would risk a token reset"
            ),
            Self::UnknownShard(id) => write!(f, "no shard {id}"),
            Self::ShardGone(id) => write!(f, "shard {id} has stopped"),
            Self::Command(e) => write!(f, "command error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rift_rest::Error> for Error {
    fn from(e: rift_rest::Error) -> Self {
        Self::Rest(e)
    }
}

impl From<rift_runtime::Error> for Error {
    fn from(e: rift_runtime::Error) -> Self {
        Self::Shard(e)
    }
}

impl From<rift_runtime::CommandError> for Error {
    fn from(e: rift_runtime::CommandError) -> Self {
        Self::Command(e)
    }
}
