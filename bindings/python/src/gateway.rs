use rift_gateway::heartbeat::HeartbeatAction;
use rift_gateway::session::{Disconnect, ReconnectPlan, Session};

/// Sans-I/O Gateway state machine for Python.
pub struct PyGatewaySession {
    inner: Session,
}

impl Default for PyGatewaySession {
    fn default() -> Self {
        Self::new()
    }
}

impl PyGatewaySession {
    pub fn new() -> Self {
        Self {
            inner: Session::default(),
        }
    }

    pub fn on_hello(&mut self, now_ms: u64, interval_ms: u64, jitter: f64) {
        self.inner.heartbeat.on_hello(now_ms, interval_ms, jitter);
    }

    pub fn on_ready(&mut self, session_id: String, resume_gateway_url: String) {
        self.inner.on_ready(session_id, resume_gateway_url);
    }

    pub fn record_seq(&mut self, seq: u64) {
        self.inner.heartbeat.record_seq(seq);
    }

    pub fn seq(&self) -> Option<u64> {
        self.inner.heartbeat.seq()
    }

    pub fn is_established(&self) -> bool {
        self.inner.is_established()
    }

    /// Returns (action: "wait"|"send"|"zombie", seq: Option<u64>)
    pub fn poll_heartbeat(&mut self, now_ms: u64) -> (String, Option<u64>) {
        match self.inner.heartbeat.poll(now_ms) {
            HeartbeatAction::Wait => ("wait".to_string(), None),
            HeartbeatAction::Send { seq } => ("send".to_string(), seq),
            HeartbeatAction::Zombie => ("zombie".to_string(), None),
        }
    }

    /// Returns (plan: "resume"|"identify"|"fatal", url, session_id, seq, close_code)
    pub fn plan_reconnect(
        &mut self,
        why: &str,
        code: Option<u16>,
        resumable: Option<bool>,
    ) -> (String, Option<String>, Option<String>, Option<u64>, Option<u16>) {
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
            } => (
                "resume".to_string(),
                Some(url),
                Some(session_id),
                Some(seq),
                None,
            ),
            ReconnectPlan::Identify => ("identify".to_string(), None, None, None, None),
            ReconnectPlan::Fatal(code) => {
                ("fatal".to_string(), None, None, None, Some(code as u16))
            }
        }
    }

    pub fn reset(&mut self) {
        self.inner = Session::default();
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "GatewaySession")]
    pub struct PyGatewaySessionClass {
        inner: PyGatewaySession,
    }

    #[pymethods]
    impl PyGatewaySessionClass {
        #[new]
        pub fn new() -> Self {
            Self {
                inner: PyGatewaySession::new(),
            }
        }

        #[pyo3(signature = (now_ms, interval_ms, jitter=0.5))]
        pub fn on_hello(&mut self, now_ms: u64, interval_ms: u64, jitter: f64) {
            self.inner.on_hello(now_ms, interval_ms, jitter);
        }

        pub fn on_ready(&mut self, session_id: String, resume_gateway_url: String) {
            self.inner.on_ready(session_id, resume_gateway_url);
        }

        pub fn record_seq(&mut self, seq: u64) {
            self.inner.record_seq(seq);
        }

        pub fn seq(&self) -> Option<u64> {
            self.inner.seq()
        }

        pub fn is_established(&self) -> bool {
            self.inner.is_established()
        }

        pub fn poll_heartbeat(&mut self, now_ms: u64) -> (String, Option<u64>) {
            self.inner.poll_heartbeat(now_ms)
        }

        #[pyo3(signature = (why, code=None, resumable=None))]
        pub fn plan_reconnect(
            &mut self,
            why: &str,
            code: Option<u16>,
            resumable: Option<bool>,
        ) -> (String, Option<String>, Option<String>, Option<u64>, Option<u16>) {
            self.inner.plan_reconnect(why, code, resumable)
        }

        pub fn reset(&mut self) {
            self.inner.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_gateway_session() {
        let mut session = PyGatewaySession::new();
        session.on_hello(1000, 41250, 0.5);
        session.on_ready("sess".to_string(), "wss://resume.discord.gg".to_string());
        session.record_seq(12);

        let (plan, url, sess_id, seq, _) = session.plan_reconnect("close", Some(4000), None);
        assert_eq!(plan, "resume");
        assert_eq!(url.as_deref(), Some("wss://resume.discord.gg"));
        assert_eq!(sess_id.as_deref(), Some("sess"));
        assert_eq!(seq, Some(12));
    }
}
