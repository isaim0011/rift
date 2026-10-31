//! Discord Voice Gateway close codes and close actions.

/// Voice Gateway close codes (4001..=4022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u16)]
pub enum VoiceCloseCode {
    /// 4001: Sent an invalid opcode.
    UnknownOpcode = 4001,
    /// 4002: Sent an invalid payload in identifying.
    FailedToDecode = 4002,
    /// 4003: Sent a payload before identifying with the gateway.
    NotAuthenticated = 4003,
    /// 4004: The token sent in your identify payload is incorrect.
    AuthenticationFailed = 4004,
    /// 4005: Sent more than one identify payload.
    AlreadyAuthenticated = 4005,
    /// 4006: Your session is no longer valid.
    SessionNoLongerValid = 4006,
    /// 4009: Your session has timed out.
    SessionTimeout = 4009,
    /// 4011: We can't find the server you're trying to connect to.
    ServerNotFound = 4011,
    /// 4012: We didn't recognize the selected protocol.
    UnknownProtocol = 4012,
    /// 4014: Channel was deleted, user kicked, voice server changed, or main gateway session dropped.
    Disconnected = 4014,
    /// 4015: The voice server crashed. Try resuming.
    VoiceServerCrashed = 4015,
    /// 4016: We didn't recognize your encryption mode.
    UnknownEncryptionMode = 4016,
    /// 4017: Bad request or DAVE handshake failure.
    BadRequest = 4017,
    /// 4020: DAVE MLS invalid commit or transition.
    DaveInvalidTransition = 4020,
    /// 4021: Client and server DAVE protocol versions mismatch.
    DaveProtocolVersionMismatch = 4021,
    /// 4022: Server requires DAVE protocol.
    DaveProtocolRequired = 4022,
}

impl VoiceCloseCode {
    /// Parse a raw close code into a recognized [`VoiceCloseCode`].
    pub const fn from_u16(code: u16) -> Option<Self> {
        Some(match code {
            4001 => Self::UnknownOpcode,
            4002 => Self::FailedToDecode,
            4003 => Self::NotAuthenticated,
            4004 => Self::AuthenticationFailed,
            4005 => Self::AlreadyAuthenticated,
            4006 => Self::SessionNoLongerValid,
            4009 => Self::SessionTimeout,
            4011 => Self::ServerNotFound,
            4012 => Self::UnknownProtocol,
            4014 => Self::Disconnected,
            4015 => Self::VoiceServerCrashed,
            4016 => Self::UnknownEncryptionMode,
            4017 => Self::BadRequest,
            4020 => Self::DaveInvalidTransition,
            4021 => Self::DaveProtocolVersionMismatch,
            4022 => Self::DaveProtocolRequired,
            _ => return None,
        })
    }

    /// Return the raw integer close code.
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    /// Whether this close code represents a fatal condition where retrying
    /// with the same credentials or configuration cannot succeed.
    pub const fn is_fatal(self) -> bool {
        matches!(
            self,
            Self::AuthenticationFailed
                | Self::SessionNoLongerValid
                | Self::ServerNotFound
                | Self::UnknownProtocol
                | Self::Disconnected
                | Self::UnknownEncryptionMode
                | Self::BadRequest
                | Self::DaveProtocolVersionMismatch
                | Self::DaveProtocolRequired
        )
    }

    /// Whether this close code indicates the connection can be resumed immediately.
    pub const fn can_resume(self) -> bool {
        matches!(self, Self::VoiceServerCrashed)
    }
}

/// The action to take when a Voice Gateway connection closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VoiceCloseAction {
    /// Reconnect and send Opcode 7 Resume to restore existing voice session.
    Resume,
    /// Reconnect and send Opcode 0 Identify to start a new voice session.
    Reconnect,
    /// Stop connecting. The failure is non-recoverable with current parameters.
    Fatal,
}

impl VoiceCloseAction {
    /// True if the action is [`VoiceCloseAction::Fatal`].
    pub const fn is_fatal(self) -> bool {
        matches!(self, Self::Fatal)
    }

    /// True if the action is [`VoiceCloseAction::Resume`].
    pub const fn is_resume(self) -> bool {
        matches!(self, Self::Resume)
    }

    /// True if the action is [`VoiceCloseAction::Reconnect`].
    pub const fn is_reconnect(self) -> bool {
        matches!(self, Self::Reconnect)
    }
}

/// Helper trait to allow passing `u16`, `Option<u16>`, or `VoiceCloseCode` to [`classify_voice_close`].
pub trait IntoVoiceCloseCode {
    /// Convert to an optional raw close code.
    fn into_raw_code(self) -> Option<u16>;
}

impl IntoVoiceCloseCode for u16 {
    fn into_raw_code(self) -> Option<u16> {
        Some(self)
    }
}

impl IntoVoiceCloseCode for Option<u16> {
    fn into_raw_code(self) -> Option<u16> {
        self
    }
}

impl IntoVoiceCloseCode for VoiceCloseCode {
    fn into_raw_code(self) -> Option<u16> {
        Some(self.as_u16())
    }
}

// Option<VoiceCloseCode> omitted to avoid inference ambiguity on None

/// Classify a voice close code into a [`VoiceCloseAction`].
///
/// Handles all Voice Close Codes (4001..=4022):
/// - `Fatal`: 4004, 4006, 4011, 4012, 4014, 4016, 4017, 4021, 4022.
/// - `Resume`: 4015 (server crashed), `None` (transient drop), or unexpected network close.
/// - `Reconnect`: 1000/1001 (clean exit), 4001, 4002, 4003, 4005, 4009, 4020, and all other 4001..=4022 codes.
pub fn classify_voice_close(code: impl IntoVoiceCloseCode) -> VoiceCloseAction {
    match code.into_raw_code() {
        None => VoiceCloseAction::Resume,
        Some(1000) | Some(1001) => VoiceCloseAction::Reconnect,
        Some(4015) => VoiceCloseAction::Resume,
        Some(4004 | 4006 | 4011 | 4012 | 4014 | 4016 | 4017 | 4021 | 4022) => {
            VoiceCloseAction::Fatal
        }
        Some(4001..=4022) => VoiceCloseAction::Reconnect,
        Some(_) => VoiceCloseAction::Resume,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatal_codes_classified_correctly() {
        let fatal_codes = [4004, 4006, 4011, 4012, 4014, 4016, 4017, 4021, 4022];
        for code in fatal_codes {
            let parsed = VoiceCloseCode::from_u16(code)
                .unwrap_or_else(|| panic!("expected {code} to be recognized"));
            assert!(parsed.is_fatal(), "code {code} must be fatal");
            assert_eq!(classify_voice_close(code), VoiceCloseAction::Fatal);
            assert_eq!(classify_voice_close(Some(code)), VoiceCloseAction::Fatal);
            assert_eq!(classify_voice_close(parsed), VoiceCloseAction::Fatal);
        }
    }

    #[test]
    fn resume_codes_classified_correctly() {
        assert_eq!(classify_voice_close(None), VoiceCloseAction::Resume);
        assert_eq!(classify_voice_close(4015), VoiceCloseAction::Resume);
        assert_eq!(
            classify_voice_close(VoiceCloseCode::VoiceServerCrashed),
            VoiceCloseAction::Resume
        );
        assert_eq!(classify_voice_close(1006), VoiceCloseAction::Resume);
    }

    #[test]
    fn reconnect_codes_classified_correctly() {
        let reconnect_codes = [4001, 4002, 4003, 4005, 4009, 4020];
        for code in reconnect_codes {
            let parsed = VoiceCloseCode::from_u16(code)
                .unwrap_or_else(|| panic!("expected {code} to be recognized"));
            assert!(!parsed.is_fatal(), "code {code} must not be fatal");
            assert_eq!(classify_voice_close(code), VoiceCloseAction::Reconnect);
            assert_eq!(classify_voice_close(parsed), VoiceCloseAction::Reconnect);
        }
        assert_eq!(classify_voice_close(1000), VoiceCloseAction::Reconnect);
        assert_eq!(classify_voice_close(1001), VoiceCloseAction::Reconnect);
    }

    #[test]
    fn all_range_4001_to_4022_covered() {
        for code in 4001..=4022 {
            let action = classify_voice_close(code);
            match code {
                4004 | 4006 | 4011 | 4012 | 4014 | 4016 | 4017 | 4021 | 4022 => {
                    assert_eq!(action, VoiceCloseAction::Fatal, "{code}");
                }
                4015 => {
                    assert_eq!(action, VoiceCloseAction::Resume, "{code}");
                }
                _ => {
                    assert_eq!(action, VoiceCloseAction::Reconnect, "{code}");
                }
            }
        }
    }
}
