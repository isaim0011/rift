//! Identify budget guard.
//!
//! Discord allows 1000 Identifies per 24h across all shards. Hitting the limit
//! terminates every session and **resets the bot token**. This guard refuses to
//! spend the last `reserve` Identifies and prefers Resume.

use std::fmt;

const DAY_MS: u64 = 86_400_000;

/// Tracks remaining Identify calls, seeded from `session_start_limit`
/// (Get Gateway Bot).
#[derive(Clone, Debug)]
pub struct IdentifyBudget {
    total: u32,
    remaining: u32,
    reset_at_ms: u64,
    reserve: u32,
}

/// The budget refused an Identify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetError {
    /// Only the safety reserve is left.
    Exhausted {
        /// Milliseconds until the daily window resets.
        retry_after_ms: u64,
    },
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exhausted { retry_after_ms } => write!(
                f,
                "identify budget exhausted; refusing to risk a token reset, retry in {retry_after_ms} ms"
            ),
        }
    }
}

impl std::error::Error for BudgetError {}

impl IdentifyBudget {
    /// Seed from the Get Gateway Bot `session_start_limit` object.
    pub fn from_session_start_limit(
        total: u32,
        remaining: u32,
        reset_after_ms: u64,
        now_ms: u64,
        reserve: u32,
    ) -> Self {
        Self { total, remaining, reset_at_ms: now_ms + reset_after_ms, reserve }
    }

    /// Identifies left (including the reserve).
    pub fn remaining(&self) -> u32 {
        self.remaining
    }

    /// Spend one Identify, or refuse.
    pub fn try_acquire(&mut self, now_ms: u64) -> Result<(), BudgetError> {
        if now_ms >= self.reset_at_ms {
            self.remaining = self.total;
            self.reset_at_ms = now_ms + DAY_MS;
        }
        if self.remaining <= self.reserve {
            return Err(BudgetError::Exhausted { retry_after_ms: self.reset_at_ms - now_ms });
        }
        self.remaining -= 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_the_reserve() {
        let mut b = IdentifyBudget::from_session_start_limit(1000, 12, 10_000, 0, 10);
        assert!(b.try_acquire(0).is_ok());
        assert!(b.try_acquire(0).is_ok());
        assert_eq!(b.try_acquire(0), Err(BudgetError::Exhausted { retry_after_ms: 10_000 }));
        assert_eq!(b.remaining(), 10);
    }

    #[test]
    fn refills_after_reset() {
        let mut b = IdentifyBudget::from_session_start_limit(1000, 10, 10_000, 0, 10);
        assert!(b.try_acquire(5_000).is_err());
        assert!(b.try_acquire(10_000).is_ok());
        assert_eq!(b.remaining(), 999);
    }
}
