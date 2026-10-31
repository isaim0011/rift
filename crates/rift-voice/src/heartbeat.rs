//! Voice Gateway v8 heartbeat scheduling, sequence tracking, and zombie-connection watchdog.

/// What the voice connection driver should do right now regarding heartbeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceHeartbeatAction {
    /// No heartbeat is due yet.
    Wait,
    /// Send opcode 3 (Heartbeat) with integer nonce `t` and `seq_ack`.
    Send {
        /// Nonce or timestamp sent in the `t` field.
        t: u64,
        /// Last acknowledged sequence number (`seq_ack`).
        seq_ack: Option<u64>,
    },
    /// A previous heartbeat was never acknowledged before the next heartbeat became due.
    /// The connection is a zombie; close and resume.
    Zombie,
}

/// Deterministic sans-I/O heartbeat state for one Discord Voice connection.
#[derive(Clone, Debug, Default)]
pub struct VoiceHeartbeat {
    interval_ms: u64,
    next_due_ms: u64,
    awaiting_ack: bool,
    started: bool,
    seq_ack: Option<u64>,
    last_nonce: u64,
    last_rtt_ms: Option<u64>,
    nonce_counter: u64,
}

impl VoiceHeartbeat {
    /// Create a new unstarted heartbeat tracker.
    pub const fn new() -> Self {
        Self {
            interval_ms: 0,
            next_due_ms: 0,
            awaiting_ack: false,
            started: false,
            seq_ack: None,
            last_nonce: 0,
            last_rtt_ms: None,
            nonce_counter: 0,
        }
    }

    /// Initialize heartbeat state upon receiving Opcode 8 Hello.
    ///
    /// The first heartbeat is scheduled with a jitter factor in `[0.0, 1.0]`.
    pub fn on_hello(&mut self, now_ms: u64, interval_ms: f64, jitter: f64) {
        let j = jitter.clamp(0.0, 1.0);
        let interval = interval_ms.max(1.0) as u64;
        self.interval_ms = interval;
        self.next_due_ms = now_ms + (interval as f64 * j) as u64;
        self.awaiting_ack = false;
        self.started = true;
    }

    /// Record an incoming frame's sequence number (`seq`).
    pub fn record_seq(&mut self, seq: u64) {
        self.seq_ack = Some(seq);
    }

    /// The latest sequence number received from the server.
    pub const fn seq_ack(&self) -> Option<u64> {
        self.seq_ack
    }

    /// When the heartbeat timer next expires, if running.
    pub const fn next_due_ms(&self) -> Option<u64> {
        if self.started {
            Some(self.next_due_ms)
        } else {
            None
        }
    }

    /// True if the heartbeat timer has been started via Hello.
    pub const fn is_started(&self) -> bool {
        self.started
    }

    /// True if an ACK is pending for a previously dispatched heartbeat.
    pub const fn awaiting_ack(&self) -> bool {
        self.awaiting_ack
    }

    /// Configured heartbeat interval in milliseconds.
    pub const fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Last observed round-trip latency in milliseconds.
    pub const fn last_rtt_ms(&self) -> Option<u64> {
        self.last_rtt_ms
    }

    /// Drive the heartbeat timer.
    pub fn poll(&mut self, now_ms: u64) -> VoiceHeartbeatAction {
        if !self.started || now_ms < self.next_due_ms {
            return VoiceHeartbeatAction::Wait;
        }

        if self.awaiting_ack {
            return VoiceHeartbeatAction::Zombie;
        }

        self.awaiting_ack = true;
        self.next_due_ms = now_ms + self.interval_ms;
        self.nonce_counter = self.nonce_counter.wrapping_add(1);
        self.last_nonce = now_ms ^ self.nonce_counter;

        VoiceHeartbeatAction::Send {
            t: self.last_nonce,
            seq_ack: self.seq_ack,
        }
    }

    /// Call upon receiving Opcode 6 Heartbeat ACK with the echoed nonce `t`.
    pub fn on_ack(&mut self, nonce: u64, now_ms: u64) {
        self.awaiting_ack = false;
        if nonce == self.last_nonce && now_ms >= (nonce ^ self.nonce_counter) {
            // Estimate RTT if nonce encodes timestamp
            let sent_ts = nonce ^ self.nonce_counter;
            if now_ms >= sent_ts {
                self.last_rtt_ms = Some(now_ms - sent_ts);
            }
        }
    }

    /// Call when the server sends Opcode 3 Heartbeat demanding an immediate heartbeat reply.
    pub fn on_server_request(&mut self, now_ms: u64) -> VoiceHeartbeatAction {
        self.awaiting_ack = true;
        self.next_due_ms = now_ms + self.interval_ms;
        self.nonce_counter = self.nonce_counter.wrapping_add(1);
        self.last_nonce = now_ms ^ self.nonce_counter;

        VoiceHeartbeatAction::Send {
            t: self.last_nonce,
            seq_ack: self.seq_ack,
        }
    }

    /// Reset heartbeat state on disconnection.
    pub fn reset(&mut self) {
        self.awaiting_ack = false;
        self.started = false;
        self.next_due_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_until_jittered_due() {
        let mut hb = VoiceHeartbeat::new();
        assert_eq!(hb.poll(1_000), VoiceHeartbeatAction::Wait);

        hb.on_hello(10_000, 20_000.0, 0.5);
        assert!(hb.is_started());
        assert_eq!(hb.interval_ms(), 20_000);
        assert_eq!(hb.next_due_ms(), Some(20_000));

        // Before due
        assert_eq!(hb.poll(10_000), VoiceHeartbeatAction::Wait);
        assert_eq!(hb.poll(19_999), VoiceHeartbeatAction::Wait);

        // When due
        let action = hb.poll(20_000);
        match action {
            VoiceHeartbeatAction::Send { t, seq_ack } => {
                assert_ne!(t, 0);
                assert_eq!(seq_ack, None);
            }
            other => panic!("expected Send, got {other:?}"),
        }
        assert!(hb.awaiting_ack());
    }

    #[test]
    fn zombie_detection_when_unacked() {
        let mut hb = VoiceHeartbeat::new();
        hb.on_hello(0, 10_000.0, 0.0);

        // First poll sends heartbeat
        let action = hb.poll(0);
        assert!(matches!(action, VoiceHeartbeatAction::Send { .. }));
        assert!(hb.awaiting_ack());

        // Next poll when due without ack -> Zombie
        assert_eq!(hb.poll(10_000), VoiceHeartbeatAction::Zombie);
    }

    #[test]
    fn ack_clears_awaiting_and_tracks_seq() {
        let mut hb = VoiceHeartbeat::new();
        hb.on_hello(0, 10_000.0, 0.0);
        hb.record_seq(105);
        assert_eq!(hb.seq_ack(), Some(105));

        let action = hb.poll(0);
        let sent_t = match action {
            VoiceHeartbeatAction::Send { t, seq_ack } => {
                assert_eq!(seq_ack, Some(105));
                t
            }
            _ => panic!("expected Send"),
        };

        hb.on_ack(sent_t, 25);
        assert!(!hb.awaiting_ack());
        assert_eq!(hb.last_rtt_ms(), Some(25));

        // Next beat is ready at 10_000
        hb.record_seq(106);
        let action2 = hb.poll(10_000);
        match action2 {
            VoiceHeartbeatAction::Send { seq_ack, .. } => {
                assert_eq!(seq_ack, Some(106));
            }
            _ => panic!("expected Send"),
        }
    }
}
