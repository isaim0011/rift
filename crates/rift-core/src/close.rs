//! Gateway close codes and what to do about them.

/// Discord's application-defined gateway close codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u16)]
pub enum CloseCode {
    /// 4000
    UnknownError = 4000,
    /// 4001
    UnknownOpcode = 4001,
    /// 4002
    DecodeError = 4002,
    /// 4003
    NotAuthenticated = 4003,
    /// 4004 (bad token)
    AuthenticationFailed = 4004,
    /// 4005
    AlreadyAuthenticated = 4005,
    /// 4007
    InvalidSeq = 4007,
    /// 4008
    RateLimited = 4008,
    /// 4009
    SessionTimedOut = 4009,
    /// 4010
    InvalidShard = 4010,
    /// 4011
    ShardingRequired = 4011,
    /// 4012
    InvalidApiVersion = 4012,
    /// 4013
    InvalidIntents = 4013,
    /// 4014
    DisallowedIntents = 4014,
}

impl CloseCode {
    /// Parse a raw close code.
    pub const fn from_u16(code: u16) -> Option<Self> {
        Some(match code {
            4000 => Self::UnknownError,
            4001 => Self::UnknownOpcode,
            4002 => Self::DecodeError,
            4003 => Self::NotAuthenticated,
            4004 => Self::AuthenticationFailed,
            4005 => Self::AlreadyAuthenticated,
            4007 => Self::InvalidSeq,
            4008 => Self::RateLimited,
            4009 => Self::SessionTimedOut,
            4010 => Self::InvalidShard,
            4011 => Self::ShardingRequired,
            4012 => Self::InvalidApiVersion,
            4013 => Self::InvalidIntents,
            4014 => Self::DisallowedIntents,
            _ => return None,
        })
    }

    /// Raw value.
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    /// The docs' "Reconnect" column: `false` means reconnecting with the same
    /// configuration can never succeed, so stop (avoids broken reconnect loops).
    pub const fn can_reconnect(self) -> bool {
        matches!(
            self,
            Self::UnknownError
                | Self::UnknownOpcode
                | Self::DecodeError
                | Self::NotAuthenticated
                | Self::AlreadyAuthenticated
                | Self::InvalidSeq
                | Self::RateLimited
                | Self::SessionTimedOut
        )
    }

    /// Whether the existing session can still be resumed. The docs say to
    /// "start a new session" for 4007 and 4009, and 4003 says the session may
    /// have been invalidated, so those three force a fresh Identify.
    pub const fn can_resume(self) -> bool {
        matches!(
            self,
            Self::UnknownError
                | Self::UnknownOpcode
                | Self::DecodeError
                | Self::AlreadyAuthenticated
                | Self::RateLimited
        )
    }
}

/// What to do after a disconnect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseAction {
    /// Reconnect to `resume_gateway_url` and send Resume.
    Resume,
    /// Open a fresh connection and Identify (costs one of 1000 daily Identifies).
    Reidentify,
    /// Stop. Retrying with the same config will fail.
    Fatal(CloseCode),
}

/// Decide what to do for a close code (`None` = connection dropped with no code).
///
/// Per the docs: no code means resume; closing with 1000/1001 invalidates the
/// session; non-Discord codes (1006 etc.) are treated as transient.
pub fn classify_close(code: Option<u16>) -> CloseAction {
    match code {
        None => CloseAction::Resume,
        Some(1000) | Some(1001) => CloseAction::Reidentify,
        Some(c) => match CloseCode::from_u16(c) {
            Some(cc) if !cc.can_reconnect() => CloseAction::Fatal(cc),
            Some(cc) if cc.can_resume() => CloseAction::Resume,
            Some(_) => CloseAction::Reidentify,
            None => CloseAction::Resume,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatal_codes_match_docs() {
        for c in [4004u16, 4010, 4011, 4012, 4013, 4014] {
            let cc = CloseCode::from_u16(c).unwrap();
            assert!(!cc.can_reconnect(), "{c}");
            assert_eq!(classify_close(Some(c)), CloseAction::Fatal(cc));
        }
    }

    #[test]
    fn reconnectable_codes_match_docs() {
        for c in [4000u16, 4001, 4002, 4003, 4005, 4007, 4008, 4009] {
            assert!(CloseCode::from_u16(c).unwrap().can_reconnect(), "{c}");
        }
    }

    #[test]
    fn resume_vs_reidentify() {
        assert_eq!(classify_close(None), CloseAction::Resume);
        assert_eq!(classify_close(Some(4000)), CloseAction::Resume);
        assert_eq!(classify_close(Some(4007)), CloseAction::Reidentify);
        assert_eq!(classify_close(Some(4009)), CloseAction::Reidentify);
        assert_eq!(classify_close(Some(1000)), CloseAction::Reidentify);
        assert_eq!(classify_close(Some(1006)), CloseAction::Resume);
    }
}
