use rift_core::CloseCode;
use rift_gateway::BudgetError;
use std::fmt;

/// Why a shard stopped.
#[derive(Debug)]
pub enum Error {
    /// Discord closed with a code that can never succeed on retry
    /// (bad token, bad intents, bad shard, ...).
    Fatal(CloseCode),
    /// The Identify budget guard refused to risk a token reset.
    Budget(BudgetError),
    /// Local setup failure (e.g. zstd context).
    Transport(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fatal(c) => write!(f, "fatal gateway close code {}: {:?}", c.as_u16(), c),
            Self::Budget(e) => write!(f, "{e}"),
            Self::Transport(e) => write!(f, "transport error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<BudgetError> for Error {
    fn from(e: BudgetError) -> Self {
        Self::Budget(e)
    }
}
