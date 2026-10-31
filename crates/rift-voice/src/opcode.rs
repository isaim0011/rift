//! Voice Gateway v8 opcodes.

use core::fmt;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Voice Gateway v8 opcodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum VoiceOpcode {
    /// Send: Begin a voice websocket connection.
    Identify = 0,
    /// Send: Select the voice protocol and encryption mode.
    SelectProtocol = 1,
    /// Receive: Complete the websocket handshake.
    Ready = 2,
    /// Send: Keep the websocket connection alive.
    Heartbeat = 3,
    /// Receive: Describe the session encryption and codecs.
    SessionDescription = 4,
    /// Send/Receive: Indicate speaking state.
    Speaking = 5,
    /// Receive: Acknowledge a heartbeat.
    HeartbeatAck = 6,
    /// Send: Resume a dropped connection.
    Resume = 7,
    /// Receive: Time to wait between heartbeats.
    Hello = 8,
    /// Receive: Acknowledge a successful session resume.
    Resumed = 9,
    /// Receive: One or more clients connected to the voice channel.
    ClientsConnect = 11,
    /// Receive: A client disconnected from the voice channel.
    ClientDisconnect = 13,
    /// Receive: Prepare for DAVE epoch / protocol transition.
    DavePrepareTransition = 21,
    /// Receive: Execute DAVE epoch / protocol transition.
    DaveExecuteTransition = 22,
    /// Send: Acknowledge transition ready.
    DaveTransitionReady = 23,
    /// Receive: Prepare DAVE epoch.
    DavePrepareEpoch = 24,
    /// Receive: MLS external sender package.
    DaveMlsExternalSender = 25,
    /// Send/Receive: MLS key package.
    DaveMlsKeyPackage = 26,
    /// Send/Receive: MLS proposals.
    DaveMlsProposals = 27,
    /// Receive: MLS commit and welcome.
    DaveMlsCommitWelcome = 28,
    /// Receive: Announce commit transition.
    DaveMlsAnnounceCommitTransition = 29,
    /// Send/Receive: MLS welcome message.
    DaveMlsWelcome = 30,
    /// Send/Receive: Report invalid commit or welcome.
    DaveMlsInvalidCommitWelcome = 31,
}

impl VoiceOpcode {
    /// Parse a raw opcode into a known [`VoiceOpcode`].
    pub const fn from_u8(op: u8) -> Option<Self> {
        Some(match op {
            0 => Self::Identify,
            1 => Self::SelectProtocol,
            2 => Self::Ready,
            3 => Self::Heartbeat,
            4 => Self::SessionDescription,
            5 => Self::Speaking,
            6 => Self::HeartbeatAck,
            7 => Self::Resume,
            8 => Self::Hello,
            9 => Self::Resumed,
            11 => Self::ClientsConnect,
            13 => Self::ClientDisconnect,
            21 => Self::DavePrepareTransition,
            22 => Self::DaveExecuteTransition,
            23 => Self::DaveTransitionReady,
            24 => Self::DavePrepareEpoch,
            25 => Self::DaveMlsExternalSender,
            26 => Self::DaveMlsKeyPackage,
            27 => Self::DaveMlsProposals,
            28 => Self::DaveMlsCommitWelcome,
            29 => Self::DaveMlsAnnounceCommitTransition,
            30 => Self::DaveMlsWelcome,
            31 => Self::DaveMlsInvalidCommitWelcome,
            _ => return None,
        })
    }

    /// Return the raw `u8` value of this opcode.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Serialize for VoiceOpcode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.as_u8())
    }
}

impl<'de> Deserialize<'de> for VoiceOpcode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = VoiceOpcode;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid Discord Voice Gateway v8 opcode integer")
            }

            fn visit_u64<E>(self, v: u64) -> Result<VoiceOpcode, E>
            where
                E: de::Error,
            {
                if v <= u8::MAX as u64 {
                    VoiceOpcode::from_u8(v as u8)
                        .ok_or_else(|| de::Error::custom(format!("unknown voice opcode: {v}")))
                } else {
                    Err(de::Error::custom(format!("voice opcode out of range: {v}")))
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<VoiceOpcode, E>
            where
                E: de::Error,
            {
                if v >= 0 && v <= u8::MAX as i64 {
                    self.visit_u64(v as u64)
                } else {
                    Err(de::Error::custom(format!("voice opcode out of range: {v}")))
                }
            }
        }

        deserializer.deserialize_u8(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes_round_trip_all_u8() {
        let expected = [
            (0, VoiceOpcode::Identify),
            (1, VoiceOpcode::SelectProtocol),
            (2, VoiceOpcode::Ready),
            (3, VoiceOpcode::Heartbeat),
            (4, VoiceOpcode::SessionDescription),
            (5, VoiceOpcode::Speaking),
            (6, VoiceOpcode::HeartbeatAck),
            (7, VoiceOpcode::Resume),
            (8, VoiceOpcode::Hello),
            (9, VoiceOpcode::Resumed),
            (11, VoiceOpcode::ClientsConnect),
            (13, VoiceOpcode::ClientDisconnect),
            (21, VoiceOpcode::DavePrepareTransition),
            (22, VoiceOpcode::DaveExecuteTransition),
            (23, VoiceOpcode::DaveTransitionReady),
            (24, VoiceOpcode::DavePrepareEpoch),
            (25, VoiceOpcode::DaveMlsExternalSender),
            (26, VoiceOpcode::DaveMlsKeyPackage),
            (27, VoiceOpcode::DaveMlsProposals),
            (28, VoiceOpcode::DaveMlsCommitWelcome),
            (29, VoiceOpcode::DaveMlsAnnounceCommitTransition),
            (30, VoiceOpcode::DaveMlsWelcome),
            (31, VoiceOpcode::DaveMlsInvalidCommitWelcome),
        ];

        for (code, op) in expected {
            assert_eq!(VoiceOpcode::from_u8(code), Some(op));
            assert_eq!(op.as_u8(), code);
        }

        // Test non-existent opcodes
        for invalid in [10, 12, 14, 20, 32, 255] {
            assert_eq!(VoiceOpcode::from_u8(invalid), None);
        }
    }

    #[test]
    fn serde_json_round_trip() {
        let op = VoiceOpcode::DavePrepareTransition;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, "21");
        let parsed: VoiceOpcode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, op);

        assert!(serde_json::from_str::<VoiceOpcode>("999").is_err());
        assert!(serde_json::from_str::<VoiceOpcode>("\"invalid\"").is_err());
    }
}
