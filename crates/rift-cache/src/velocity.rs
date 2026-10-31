//! High-frequency sliding-window velocity tracker for rate-limiting, anti-raid, and event-flow governance.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// A single-key sliding-window counter tracking events over a rolling duration.
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    window: Duration,
    timestamps: VecDeque<Instant>,
}

impl SlidingWindow {
    /// Creates a new sliding window over the specified `window` duration.
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            timestamps: VecDeque::with_capacity(32),
        }
    }

    /// Records an event occurrence at the current time (`Instant::now()`),
    /// evicts expired timestamps, and returns the current event count in the window.
    pub fn record(&mut self) -> u32 {
        self.record_at(Instant::now())
    }

    /// Records an event occurrence at the given timestamp.
    pub fn record_at(&mut self, now: Instant) -> u32 {
        self.evict_expired(now);
        self.timestamps.push_back(now);
        self.timestamps.len() as u32
    }

    /// Returns the active number of events within the window as of `now`.
    pub fn count_at(&mut self, now: Instant) -> u32 {
        self.evict_expired(now);
        self.timestamps.len() as u32
    }

    /// Returns the active number of events within the window right now.
    pub fn count(&mut self) -> u32 {
        self.count_at(Instant::now())
    }

    /// Returns `true` if the event count in the window meets or exceeds `threshold`.
    pub fn is_exceeded(&mut self, threshold: u32) -> bool {
        self.count() >= threshold
    }

    /// Clears all recorded timestamps in the window.
    pub fn clear(&mut self) {
        self.timestamps.clear();
    }

    fn evict_expired(&mut self, now: Instant) {
        while let Some(&oldest) = self.timestamps.front() {
            if now.duration_since(oldest) > self.window {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }
    }
}

/// A multi-key thread-safe sliding-window tracker.
///
/// Ideal for tracking per-user, per-guild, or per-channel velocity (e.g. joins/sec, messages/sec, bans/min).
#[derive(Debug)]
pub struct VelocityTracker<K: Eq + Hash> {
    window: Duration,
    entries: Mutex<HashMap<K, SlidingWindow>>,
}

impl<K: Eq + Hash + Clone> VelocityTracker<K> {
    /// Creates a new multi-key velocity tracker with the given rolling duration.
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Records an event for the given key and returns the updated count within the window.
    pub fn record(&self, key: K) -> u32 {
        self.record_at(key, Instant::now())
    }

    /// Records an event for the given key at an explicit timestamp.
    pub fn record_at(&self, key: K, now: Instant) -> u32 {
        let mut map = self.entries.lock().unwrap();
        map.entry(key)
            .or_insert_with(|| SlidingWindow::new(self.window))
            .record_at(now)
    }

    /// Returns the current event count for `key` within the window.
    pub fn count(&self, key: &K) -> u32 {
        let mut map = self.entries.lock().unwrap();
        if let Some(w) = map.get_mut(key) {
            w.count()
        } else {
            0
        }
    }

    /// Checks whether the event count for `key` meets or exceeds `threshold`.
    pub fn is_exceeded(&self, key: &K, threshold: u32) -> bool {
        self.count(key) >= threshold
    }

    /// Resets the recorded history for a specific key.
    pub fn reset(&self, key: &K) {
        let mut map = self.entries.lock().unwrap();
        map.remove(key);
    }

    /// Cleans up keys that have zero active events in their window to reclaim memory.
    pub fn cleanup(&self) {
        let mut map = self.entries.lock().unwrap();
        let now = Instant::now();
        map.retain(|_, w| w.count_at(now) > 0);
    }

    /// Returns the total number of currently tracked keys.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// Returns `true` if no keys are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_window_eviction() {
        let mut window = SlidingWindow::new(Duration::from_millis(50));
        let start = Instant::now();

        assert_eq!(window.record_at(start), 1);
        assert_eq!(window.record_at(start + Duration::from_millis(10)), 2);
        assert_eq!(window.record_at(start + Duration::from_millis(20)), 3);

        // Within window
        assert_eq!(window.count_at(start + Duration::from_millis(40)), 3);

        // After 55ms, only the 0ms event is evicted; 10ms and 20ms remain
        assert_eq!(window.count_at(start + Duration::from_millis(55)), 2);

        // After 65ms, both 0ms and 10ms are evicted; only 20ms remains
        assert_eq!(window.count_at(start + Duration::from_millis(65)), 1);

        // After 75ms, all initial events evicted
        assert_eq!(window.count_at(start + Duration::from_millis(75)), 0);
    }

    #[test]
    fn velocity_tracker_multi_key() {
        let tracker = VelocityTracker::new(Duration::from_millis(100));
        let user_a = 12345u64;
        let user_b = 67890u64;

        assert_eq!(tracker.record(user_a), 1);
        assert_eq!(tracker.record(user_a), 2);
        assert_eq!(tracker.record(user_b), 1);

        assert!(tracker.is_exceeded(&user_a, 2));
        assert!(!tracker.is_exceeded(&user_b, 2));

        tracker.reset(&user_a);
        assert_eq!(tracker.count(&user_a), 0);
        assert_eq!(tracker.count(&user_b), 1);
    }
}
