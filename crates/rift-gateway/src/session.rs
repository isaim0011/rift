//! Session bookkeeping and the resume-or-identify decision.

use crate::heartbeat::Heartbeat;
use rift_core::{classify_close, CloseAction, CloseCode};

/// Why the connection went away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disconnect {
    /// Socket closed; `None` if no close code arrived.
    Close(Option<u16>),
    /// Discord sent Reconnect (opcode 7).
    ReconnectRequested,
    /// Discord sent Invalid Session (opcode 9) with this `d` value.
    InvalidSession {
        /// `d == true` means the session may still be resumed.
        resumable: bool,
    },
    /// Our own heartbeat watchdog gave up.
    Zombie,
}

/// What the driver should do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReconnectPlan {
    /// Connect to `url` (the `resume_gateway_url`, same version/encoding), send Resume.
    Resume {
        /// `resume_gateway_url` from Ready.
        url: String,
        /// `session_id` from Ready.
        session_id: String,
        /// Last sequence number received.
        seq: u64,
    },
    /// Connect to the cached Get Gateway URL and Identify (spends budget).
    Identify,
    /// Do not reconnect.
    Fatal(CloseCode),
}

/// One shard's session state.
#[derive(Clone, Debug, Default)]
pub struct Session {
    /// Heartbeat timer and sequence number.
    pub heartbeat: Heartbeat,
    session_id: Option<String>,
    resume_url: Option<String>,
}

impl Session {
    /// Call on Ready: store what Resume needs.
    pub fn on_ready(&mut self, session_id: String, resume_gateway_url: String) {
        self.session_id = Some(session_id);
        self.resume_url = Some(resume_gateway_url);
    }

    /// True once a Ready has been processed.
    pub fn is_established(&self) -> bool {
        self.session_id.is_some()
    }

    /// Decide how to recover. Falls back to Identify if anything Resume needs
    /// is missing, and clears state whenever the session is dead.
    pub fn plan(&mut self, why: Disconnect) -> ReconnectPlan {
        let action = match why {
            Disconnect::Close(code) => classify_close(code),
            Disconnect::ReconnectRequested => CloseAction::Resume,
            Disconnect::InvalidSession { resumable: true } => CloseAction::Resume,
            Disconnect::InvalidSession { resumable: false } => CloseAction::Reidentify,
            Disconnect::Zombie => CloseAction::Resume,
        };
        match action {
            CloseAction::Fatal(code) => ReconnectPlan::Fatal(code),
            CloseAction::Resume => {
                match (&self.session_id, &self.resume_url, self.heartbeat.seq()) {
                    (Some(id), Some(url), Some(seq)) => ReconnectPlan::Resume {
                        url: url.clone(),
                        session_id: id.clone(),
                        seq,
                    },
                    _ => self.reset(),
                }
            }
            CloseAction::Reidentify => self.reset(),
        }
    }

    fn reset(&mut self) -> ReconnectPlan {
        self.session_id = None;
        self.resume_url = None;
        self.heartbeat = Heartbeat::default();
        ReconnectPlan::Identify
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_session() -> Session {
        let mut s = Session::default();
        s.on_ready("abc".into(), "wss://resume.example/".into());
        s.heartbeat.record_seq(1337);
        s
    }

    #[test]
    fn resumes_on_reconnect_opcode() {
        let mut s = ready_session();
        assert_eq!(
            s.plan(Disconnect::ReconnectRequested),
            ReconnectPlan::Resume {
                url: "wss://resume.example/".into(),
                session_id: "abc".into(),
                seq: 1337
            }
        );
    }

    #[test]
    fn resumes_with_no_close_code_and_on_zombie() {
        assert!(matches!(ready_session().plan(Disconnect::Close(None)), ReconnectPlan::Resume { .. }));
        assert!(matches!(ready_session().plan(Disconnect::Zombie), ReconnectPlan::Resume { .. }));
    }

    #[test]
    fn invalid_session_false_means_identify_and_clears_state() {
        let mut s = ready_session();
        assert_eq!(s.plan(Disconnect::InvalidSession { resumable: false }), ReconnectPlan::Identify);
        assert!(!s.is_established());
        // Nothing left to resume with.
        assert_eq!(s.plan(Disconnect::ReconnectRequested), ReconnectPlan::Identify);
    }

    #[test]
    fn bad_token_is_fatal() {
        assert_eq!(
            ready_session().plan(Disconnect::Close(Some(4004))),
            ReconnectPlan::Fatal(CloseCode::AuthenticationFailed)
        );
    }

    #[test]
    fn no_events_yet_means_identify() {
        let mut s = Session::default();
        s.on_ready("abc".into(), "wss://x/".into()); // no seq recorded
        assert_eq!(s.plan(Disconnect::Close(None)), ReconnectPlan::Identify);
    }

    #[test]
    fn session_timeout_forces_fresh_identify() {
        assert_eq!(ready_session().plan(Disconnect::Close(Some(4009))), ReconnectPlan::Identify);
    }
}
