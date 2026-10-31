//! # rift-core
//!
//! Pure, I/O-free Discord primitives. No dependencies, no unsafe, no allocation
//! on the hot paths. Everything here is derived from the official docs:
//! <https://docs.discord.com/developers/>

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod capabilities;
pub mod close;
pub mod intents;
pub mod message_flags;
pub mod opcode;
pub mod permissions;
pub mod shard;
pub mod snowflake;

pub use capabilities::Capabilities;
pub use close::{classify_close, CloseAction, CloseCode};
pub use intents::Intents;
pub use message_flags::MessageFlags;
pub use opcode::Opcode;
pub use permissions::{Overwrite, Permissions};
pub use snowflake::Snowflake;
