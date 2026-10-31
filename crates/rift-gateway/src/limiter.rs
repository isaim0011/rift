//! Outgoing gateway limits: 120 events per 60s per connection, 4096-byte payloads.

use std::collections::VecDeque;
use std::fmt;

/// Max serialized size of a payload we send (exceeding it closes with 4002).
pub const MAX_PAYLOAD_BYTES: usize = 4096;
/// Events allowed per window.
pub const LIMIT: usize = 120;
/// Window length.
pub const WINDOW_MS: u64 = 60_000;
/// Slots kept free so heartbeats/resumes are never starved by app traffic.
pub const PRIORITY_RESERVE: usize = 10;

/// Why a send was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendError {
    /// Payload exceeds 4096 bytes.
    TooLarge {
        /// Offending length.
        len: usize,
    },
    /// Window is full; retry later.
    RateLimited {
        /// Milliseconds until a slot frees up.
        retry_after_ms: u64,
    },
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len } => write!(f, "payload of {len} bytes exceeds {MAX_PAYLOAD_BYTES}"),
            Self::RateLimited { retry_after_ms } => {
                write!(f, "gateway send limit reached, retry in {retry_after_ms} ms")
            }
        }
    }
}

impl std::error::Error for SendError {}

/// Sliding-window limiter for one connection.
#[derive(Clone, Debug, Default)]
pub struct SendLimiter {
    sent: VecDeque<u64>,
}

impl SendLimiter {
    /// Try to send. `priority` sends (heartbeat, identify, resume) may use the
    /// reserved slots; everything else stops `PRIORITY_RESERVE` short.
    pub fn try_send(&mut self, now_ms: u64, payload_len: usize, priority: bool) -> Result<(), SendError> {
        if payload_len > MAX_PAYLOAD_BYTES {
            return Err(SendError::TooLarge { len: payload_len });
        }
        while let Some(&t) = self.sent.front() {
            if now_ms.saturating_sub(t) >= WINDOW_MS {
                self.sent.pop_front();
            } else {
                break;
            }
        }
        let cap = if priority { LIMIT } else { LIMIT - PRIORITY_RESERVE };
        if self.sent.len() >= cap {
            let oldest = self.sent.front().copied().unwrap_or(now_ms);
            return Err(SendError::RateLimited {
                retry_after_ms: (oldest + WINDOW_MS).saturating_sub(now_ms),
            });
        }
        self.sent.push_back(now_ms);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_payloads() {
        let mut l = SendLimiter::default();
        assert_eq!(l.try_send(0, 4097, false), Err(SendError::TooLarge { len: 4097 }));
        assert!(l.try_send(0, 4096, false).is_ok());
    }

    #[test]
    fn app_traffic_stops_at_the_reserve() {
        let mut l = SendLimiter::default();
        for _ in 0..(LIMIT - PRIORITY_RESERVE) {
            assert!(l.try_send(0, 10, false).is_ok());
        }
        assert!(matches!(l.try_send(0, 10, false), Err(SendError::RateLimited { .. })));
        // Heartbeats can still go out.
        assert!(l.try_send(0, 10, true).is_ok());
    }

    #[test]
    fn window_slides() {
        let mut l = SendLimiter::default();
        for _ in 0..(LIMIT - PRIORITY_RESERVE) {
            l.try_send(1_000, 10, false).unwrap();
        }
        assert_eq!(
            l.try_send(1_001, 10, false),
            Err(SendError::RateLimited { retry_after_ms: 59_999 })
        );
        assert!(l.try_send(61_000, 10, false).is_ok());
    }
}
