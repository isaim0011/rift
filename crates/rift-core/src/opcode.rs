//! Gateway opcodes (docs: Opcodes and Status Codes).

/// A gateway opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum Opcode {
    /// Receive: an event was dispatched.
    Dispatch = 0,
    /// Send/Receive: keep-alive.
    Heartbeat = 1,
    /// Send: start a new session.
    Identify = 2,
    /// Send: update presence.
    PresenceUpdate = 3,
    /// Send: join/leave/move voice channels.
    VoiceStateUpdate = 4,
    /// Send: resume a disconnected session.
    Resume = 6,
    /// Receive: reconnect and resume immediately.
    Reconnect = 7,
    /// Send: request offline guild members.
    RequestGuildMembers = 8,
    /// Receive: session invalidated.
    InvalidSession = 9,
    /// Receive: carries `heartbeat_interval`.
    Hello = 10,
    /// Receive: heartbeat acknowledged.
    HeartbeatAck = 11,
    /// Send: request soundboard sounds.
    RequestSoundboardSounds = 31,
    /// Send: request ephemeral channel info.
    RequestChannelInfo = 43,
}

impl Opcode {
    /// Parse a raw opcode.
    pub const fn from_u8(op: u8) -> Option<Self> {
        Some(match op {
            0 => Self::Dispatch,
            1 => Self::Heartbeat,
            2 => Self::Identify,
            3 => Self::PresenceUpdate,
            4 => Self::VoiceStateUpdate,
            6 => Self::Resume,
            7 => Self::Reconnect,
            8 => Self::RequestGuildMembers,
            9 => Self::InvalidSession,
            10 => Self::Hello,
            11 => Self::HeartbeatAck,
            31 => Self::RequestSoundboardSounds,
            43 => Self::RequestChannelInfo,
            _ => return None,
        })
    }

    /// Raw value.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for op in [0u8, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 31, 43] {
            assert_eq!(Opcode::from_u8(op).unwrap().as_u8(), op);
        }
        assert!(Opcode::from_u8(5).is_none());
    }
}
