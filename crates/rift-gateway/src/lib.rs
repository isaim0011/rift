//! # rift-gateway
//!
//! A **sans-IO** gateway state machine. It never opens a socket and never reads
//! the clock: callers pass `now_ms`, so every behavior is deterministic and
//! unit-testable. The async runtime layer (next crate) just feeds it events.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod budget;
pub mod heartbeat;
pub mod limiter;
pub mod session;

pub use budget::{BudgetError, IdentifyBudget};
pub use heartbeat::{Heartbeat, HeartbeatAction};
pub use limiter::{SendError, SendLimiter};
pub use session::{Disconnect, ReconnectPlan, Session};
