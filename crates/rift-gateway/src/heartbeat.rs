//! Heartbeat scheduling and zombie-connection detection.

/// What the connection driver should do right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartbeatAction {
    /// Nothing due yet.
    Wait,
    /// Send opcode 1 with this sequence number as `d` (`None` serializes to `null`).
    Send {
        /// Last dispatch sequence number seen.
        seq: Option<u64>,
    },
    /// The previous heartbeat was never ACKed: the connection is a zombie.
    /// Close it with a code other than 1000/1001 and resume.
    Zombie,
}

/// Heartbeat state for one connection.
#[derive(Clone, Debug, Default)]
pub struct Heartbeat {
    interval_ms: u64,
    next_due_ms: u64,
    awaiting_ack: bool,
    started: bool,
    seq: Option<u64>,
}

impl Heartbeat {
    /// Call on Hello (opcode 10). The first heartbeat is delayed by
    /// `interval * jitter` with `jitter` in `[0, 1]`, so a fleet of clients
    /// doesn't reconnect in lockstep.
    pub fn on_hello(&mut self, now_ms: u64, interval_ms: u64, jitter: f64) {
        let j = jitter.clamp(0.0, 1.0);
        self.interval_ms = interval_ms;
        self.next_due_ms = now_ms + (interval_ms as f64 * j) as u64;
        self.awaiting_ack = false;
        self.started = true;
    }

    /// Record the `s` field of a Dispatch event.
    pub fn record_seq(&mut self, seq: u64) {
        self.seq = Some(seq);
    }

    /// Last recorded sequence number (needed for Resume).
    pub fn seq(&self) -> Option<u64> {
        self.seq
    }

    /// When `poll` next needs calling, if the heartbeat is running.
    pub fn next_due_ms(&self) -> Option<u64> {
        self.started.then_some(self.next_due_ms)
    }

    /// Drive the timer.
    pub fn poll(&mut self, now_ms: u64) -> HeartbeatAction {
        if !self.started || now_ms < self.next_due_ms {
            return HeartbeatAction::Wait;
        }
        if self.awaiting_ack {
            return HeartbeatAction::Zombie;
        }
        self.awaiting_ack = true;
        self.next_due_ms = now_ms + self.interval_ms;
        HeartbeatAction::Send { seq: self.seq }
    }

    /// Call on Heartbeat ACK (opcode 11).
    pub fn on_ack(&mut self) {
        self.awaiting_ack = false;
    }

    /// Call when Discord sends opcode 1: answer immediately.
    pub fn on_server_request(&mut self, now_ms: u64) -> HeartbeatAction {
        self.awaiting_ack = true;
        self.next_due_ms = now_ms + self.interval_ms;
        HeartbeatAction::Send { seq: self.seq }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_until_jittered_first_beat() {
        let mut h = Heartbeat::default();
        h.on_hello(1_000, 40_000, 0.5);
        assert_eq!(h.poll(1_000), HeartbeatAction::Wait);
        assert_eq!(h.poll(20_999), HeartbeatAction::Wait);
        assert_eq!(h.poll(21_000), HeartbeatAction::Send { seq: None });
    }

    #[test]
    fn carries_latest_seq() {
        let mut h = Heartbeat::default();
        h.on_hello(0, 1_000, 0.0);
        h.record_seq(41);
        h.record_seq(42);
        assert_eq!(h.poll(0), HeartbeatAction::Send { seq: Some(42) });
    }

    #[test]
    fn missing_ack_is_a_zombie() {
        let mut h = Heartbeat::default();
        h.on_hello(0, 1_000, 0.0);
        assert!(matches!(h.poll(0), HeartbeatAction::Send { .. }));
        assert_eq!(h.poll(1_000), HeartbeatAction::Zombie);
    }

    #[test]
    fn acked_heartbeat_keeps_going() {
        let mut h = Heartbeat::default();
        h.on_hello(0, 1_000, 0.0);
        assert!(matches!(h.poll(0), HeartbeatAction::Send { .. }));
        h.on_ack();
        assert!(matches!(h.poll(1_000), HeartbeatAction::Send { .. }));
    }

    #[test]
    fn server_request_answers_immediately() {
        let mut h = Heartbeat::default();
        h.on_hello(0, 45_000, 1.0);
        assert!(matches!(h.on_server_request(5), HeartbeatAction::Send { .. }));
        assert_eq!(h.poll(6), HeartbeatAction::Wait);
    }

    #[test]
    fn idle_before_hello() {
        let mut h = Heartbeat::default();
        assert_eq!(h.poll(u64::MAX), HeartbeatAction::Wait);
        assert_eq!(h.next_due_ms(), None);
    }
}
