//! Sliding-window counter for invalid requests (401/403/429), in 10-second bins.

const BIN_MS: u64 = 10_000;
const WINDOW_BINS: u64 = 60; // 10 minutes
const SLOTS: usize = (WINDOW_BINS + 1) as usize;

#[derive(Clone, Debug)]
pub(crate) struct InvalidTracker {
    abs: [u64; SLOTS],
    cnt: [u32; SLOTS],
}

impl Default for InvalidTracker {
    fn default() -> Self {
        Self { abs: [0; SLOTS], cnt: [0; SLOTS] }
    }
}

impl InvalidTracker {
    pub(crate) fn record(&mut self, now_ms: u64) {
        let abs = now_ms / BIN_MS;
        let i = (abs % SLOTS as u64) as usize;
        if self.abs[i] != abs {
            self.abs[i] = abs;
            self.cnt[i] = 0;
        }
        self.cnt[i] = self.cnt[i].saturating_add(1);
    }

    fn live_bins(&self, now_ms: u64) -> Vec<(u64, u32)> {
        let now_abs = now_ms / BIN_MS;
        let mut v: Vec<(u64, u32)> = (0..SLOTS)
            .filter(|&i| self.cnt[i] > 0 && self.abs[i] <= now_abs && self.abs[i] + WINDOW_BINS >= now_abs)
            .map(|i| (self.abs[i], self.cnt[i]))
            .collect();
        v.sort_unstable();
        v
    }

    pub(crate) fn count(&self, now_ms: u64) -> u32 {
        self.live_bins(now_ms).iter().map(|&(_, c)| c).fold(0u32, u32::saturating_add)
    }

    /// If at least `threshold` invalid requests are in the window, how long
    /// until enough age out to drop below it.
    pub(crate) fn retry_after_if_at_least(&self, now_ms: u64, threshold: u32) -> Option<u64> {
        let bins = self.live_bins(now_ms);
        let mut total: u32 = bins.iter().map(|&(_, c)| c).fold(0u32, u32::saturating_add);
        if total < threshold {
            return None;
        }
        for (abs, c) in bins {
            total = total.saturating_sub(c);
            if total < threshold {
                let expires_at = (abs + WINDOW_BINS + 1) * BIN_MS;
                return Some(expires_at.saturating_sub(now_ms));
            }
        }
        Some(BIN_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_within_window_and_expires() {
        let mut t = InvalidTracker::default();
        for _ in 0..5 {
            t.record(0);
        }
        assert_eq!(t.count(0), 5);
        assert_eq!(t.count(599_999), 5);
        assert_eq!(t.count(610_000), 0);
    }

    #[test]
    fn retry_after_points_at_expiry() {
        let mut t = InvalidTracker::default();
        for _ in 0..5 {
            t.record(0);
        }
        assert_eq!(t.retry_after_if_at_least(1_000, 5), Some(609_000));
        assert_eq!(t.retry_after_if_at_least(1_000, 6), None);
    }
}
