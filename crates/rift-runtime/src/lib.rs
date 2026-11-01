//! # rift-runtime
//!
//! Async driver for [`rift_gateway`]. It owns the socket and the clock; all
//! protocol decisions (heartbeat, resume, budgets, limits) come from the
//! sans-IO state machine.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod command;
mod config;
mod error;
mod frame;
mod shard;
mod transport;
mod zstd_stream;

pub use command::{
    Activity, ActivityType, CommandError, RequestGuildMembers, RequestSoundboardSounds, Status,
    UpdatePresence, UpdateVoiceState,
};
pub use config::{EventBufferPolicy, ShardConfig};
pub use error::Error;
pub use shard::{
    event_channel, now_ms, run_shard, Command, Event, EventReceiver, EventSendError, EventSender, SharedBudget,
};

/// Internals exposed only so the benchmark crate measures the real code.
/// Not a stable API; do not depend on it.
#[doc(hidden)]
pub mod bench_support {
    pub use crate::shard::envelope_probe;
    pub use crate::zstd_stream::StreamDecoder;
}
