use rift_gateway::heartbeat::HeartbeatAction;
use rift_gateway::session::{Disconnect, ReconnectPlan, Session};
use serde::{Deserialize, Serialize};

/// Heartbeat poll decision for the gateway transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatPollResult {
    /// "wait", "send", or "zombie".
    pub action: &'static str,
    /// Last received sequence number to include in opcode 1 (if action == "send").
    pub seq: Option<u64>,
}

/// Reconnect plan decision on connection drop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconnectPlanResult {
    /// "resume", "identify", or "fatal".
    pub plan: String,
    /// Resume gateway URL (if plan == "resume").
    pub url: Option<String>,
    /// Session ID (if plan == "resume").
    pub session_id: Option<String>,
    /// Last dispatch sequence number (if plan == "resume").
    pub seq: Option<u64>,
    /// Fatal Discord close code (if plan == "fatal").
    pub close_code: Option<u16>,
}

/// Sans-I/O Gateway v10 state machine for custom JS/Bun WebSocket transports.
pub struct NodeGatewaySession {
    inner: Session,
}

impl Default for NodeGatewaySession {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeGatewaySession {
    /// Creates a new sans-I/O gateway session tracker.
    pub fn new() -> Self {
        Self {
            inner: Session::default(),
        }
    }

    /// Handles Gateway Hello (opcode 10) to schedule first jittered heartbeat.
    pub fn on_hello(&mut self, now_ms: u64, interval_ms: u64, jitter: f64) {
        self.inner.heartbeat.on_hello(now_ms, interval_ms, jitter);
    }

    /// Handles Gateway Ready (opcode 0 READY) saving session resumption metadata.
    pub fn on_ready(&mut self, session_id: String, resume_gateway_url: String) {
        self.inner.on_ready(session_id, resume_gateway_url);
    }

    /// Records the sequence number `s` from an incoming Dispatch event.
    pub fn record_seq(&mut self, seq: u64) {
        self.inner.heartbeat.record_seq(seq);
    }

    /// Returns the last recorded sequence number, if any.
    pub fn seq(&self) -> Option<u64> {
        self.inner.heartbeat.seq()
    }

    /// Returns true if Ready has been received and session is active.
    pub fn is_established(&self) -> bool {
        self.inner.is_established()
    }

    /// Drives the heartbeat timer. Call this on WebSocket tick / timer.
    pub fn poll_heartbeat(&mut self, now_ms: u64) -> HeartbeatPollResult {
        match self.inner.heartbeat.poll(now_ms) {
            HeartbeatAction::Wait => HeartbeatPollResult {
                action: "wait",
                seq: None,
            },
            HeartbeatAction::Send { seq } => HeartbeatPollResult {
                action: "send",
                seq,
            },
            HeartbeatAction::Zombie => HeartbeatPollResult {
                action: "zombie",
                seq: None,
            },
        }
    }

    /// Decides whether to Resume, Identify, or abort based on the disconnect reason.
    pub fn plan_reconnect(
        &mut self,
        why: &str,
        code: Option<u16>,
        resumable: Option<bool>,
    ) -> ReconnectPlanResult {
        let disconnect = match why {
            "close" => Disconnect::Close(code),
            "reconnect_requested" => Disconnect::ReconnectRequested,
            "invalid_session" => Disconnect::InvalidSession {
                resumable: resumable.unwrap_or(false),
            },
            "zombie" => Disconnect::Zombie,
            _ => Disconnect::Close(code),
        };

        match self.inner.plan(disconnect) {
            ReconnectPlan::Resume {
                url,
                session_id,
                seq,
            } => ReconnectPlanResult {
                plan: "resume".to_string(),
                url: Some(url),
                session_id: Some(session_id),
                seq: Some(seq),
                close_code: None,
            },
            ReconnectPlan::Identify => ReconnectPlanResult {
                plan: "identify".to_string(),
                url: None,
                session_id: None,
                seq: None,
                close_code: None,
            },
            ReconnectPlan::Fatal(code) => ReconnectPlanResult {
                plan: "fatal".to_string(),
                url: None,
                session_id: None,
                seq: None,
                close_code: Some(code as u16),
            },
        }
    }

    /// Resets the session state.
    pub fn reset(&mut self) {
        self.inner = Session::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_session_lifecycle() {
        let mut session = NodeGatewaySession::new();
        assert!(!session.is_established());

        // Hello
        session.on_hello(1000, 41250, 0.5);
        assert_eq!(session.poll_heartbeat(1000).action, "wait");

        // Ready
        session.on_ready("sess123".to_string(), "wss://gateway.discord.gg".to_string());
        assert!(session.is_established());

        // Record seq
        session.record_seq(42);
        assert_eq!(session.seq(), Some(42));

        // Reconnect plan on normal close 4000 (reconnectable)
        let plan = session.plan_reconnect("close", Some(4000), None);
        assert_eq!(plan.plan, "resume");
        assert_eq!(plan.session_id.as_deref(), Some("sess123"));
        assert_eq!(plan.seq, Some(42));

        // Fatal close code 4004 (Authentication Failed)
        let fatal_plan = session.plan_reconnect("close", Some(4004), None);
        assert_eq!(fatal_plan.plan, "fatal");
        assert_eq!(fatal_plan.close_code, Some(4004));
    }
}
