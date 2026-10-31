//! Deterministic sans-I/O Voice Gateway v8 session state machine.

use core::fmt;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
use rift_core::Snowflake;
use serde::{Deserialize, Serialize};

use crate::close::{classify_voice_close, IntoVoiceCloseCode, VoiceCloseAction};
use crate::heartbeat::{VoiceHeartbeat, VoiceHeartbeatAction};
use crate::opcode::VoiceOpcode;

/// Discord Voice transport encryption mode.
///
/// Prefer `aead_xchacha20_poly1305_rtpsize`, then `aead_aes256_gcm_rtpsize`.
/// Deprecated legacy `xsalsa20_poly1305*` modes removed by Discord are rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EncryptionMode {
    /// `aead_xchacha20_poly1305_rtpsize`
    #[serde(rename = "aead_xchacha20_poly1305_rtpsize")]
    AeadXChaCha20Poly1305RtpSize,
    /// `aead_aes256_gcm_rtpsize`
    #[serde(rename = "aead_aes256_gcm_rtpsize")]
    AeadAes256GcmRtpSize,
}

impl EncryptionMode {
    /// Identifier string for `aead_xchacha20_poly1305_rtpsize`.
    pub const AEAD_XCHACHA20_POLY1305_RTPSIZE: &'static str = "aead_xchacha20_poly1305_rtpsize";

    /// Identifier string for `aead_aes256_gcm_rtpsize`.
    pub const AEAD_AES256_GCM_RTPSIZE: &'static str = "aead_aes256_gcm_rtpsize";

    /// Returns the wire protocol name for this encryption mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AeadXChaCha20Poly1305RtpSize => Self::AEAD_XCHACHA20_POLY1305_RTPSIZE,
            Self::AeadAes256GcmRtpSize => Self::AEAD_AES256_GCM_RTPSIZE,
        }
    }

    /// Select the best supported encryption mode from an offered list.
    ///
    /// Prefers `aead_xchacha20_poly1305_rtpsize`, then `aead_aes256_gcm_rtpsize`.
    /// Legacy `xsalsa20_poly1305*` modes are strictly ignored.
    pub fn from_offered<S: AsRef<str>>(modes: &[S]) -> Option<Self> {
        let mut has_aes = false;

        for mode in modes {
            let m = mode.as_ref();
            if m == Self::AEAD_XCHACHA20_POLY1305_RTPSIZE {
                return Some(Self::AeadXChaCha20Poly1305RtpSize);
            }
            if m == Self::AEAD_AES256_GCM_RTPSIZE {
                has_aes = true;
            }
        }

        if has_aes {
            Some(Self::AeadAes256GcmRtpSize)
        } else {
            None
        }
    }
}

/// Bitflags representing speaking state in a Discord voice channel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpeakingFlags(u32);

impl SpeakingFlags {
    /// Normal voice transmission (microphone).
    pub const MICROPHONE: Self = Self(1 << 0);
    /// Soundshare transmission (game/screen audio).
    pub const SOUNDSHARE: Self = Self(1 << 1);
    /// Priority speaker (attenuates other users).
    pub const PRIORITY: Self = Self(1 << 2);
    /// No speaking flags set.
    pub const NONE: Self = Self(0);

    /// Create an empty speaking flags mask.
    pub const fn empty() -> Self {
        Self::NONE
    }

    /// Return raw bits.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Construct flags from raw bits.
    pub const fn from_bits_truncate(bits: u32) -> Self {
        Self(bits & 0x07)
    }

    /// Check if these flags contain all of `other`.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Check if these flags intersect with `other`.
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl BitOr for SpeakingFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for SpeakingFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for SpeakingFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for SpeakingFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// A generic Voice Gateway v8 JSON envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceGatewayPayload<T> {
    /// Voice Gateway opcode.
    pub op: VoiceOpcode,
    /// Inner payload data.
    pub d: T,
    /// Optional sequence number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}

impl<T> VoiceGatewayPayload<T> {
    /// Construct a payload without a sequence number.
    pub const fn new(op: VoiceOpcode, d: T) -> Self {
        Self { op, d, seq: None }
    }

    /// Construct a payload with a sequence number.
    pub const fn with_seq(op: VoiceOpcode, d: T, seq: u64) -> Self {
        Self {
            op,
            d,
            seq: Some(seq),
        }
    }
}

/// Inbound Opcode 8 Hello payload data.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VoiceHelloData {
    /// Gateway protocol version.
    pub v: u32,
    /// Heartbeat interval in milliseconds.
    pub heartbeat_interval: f64,
}

/// Outbound Opcode 0 Identify payload data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceIdentifyData {
    /// Voice server (guild) ID.
    pub server_id: String,
    /// Current bot/user ID.
    pub user_id: String,
    /// Voice session ID received from Gateway VOICE_STATE_UPDATE.
    pub session_id: String,
    /// Voice endpoint token received from Gateway VOICE_SERVER_UPDATE.
    pub token: String,
    /// Maximum DAVE MLS protocol version supported (e.g. 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_dave_protocol_version: Option<u8>,
}

/// Outbound Opcode 7 Resume payload data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceResumeData {
    /// Voice server (guild) ID.
    pub server_id: String,
    /// Voice session ID.
    pub session_id: String,
    /// Voice endpoint token.
    pub token: String,
    /// Last acknowledged sequence number (`seq_ack`).
    pub seq_ack: u64,
}

/// Inbound Opcode 2 Ready payload data.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VoiceReadyData {
    /// Assigned SSRC for sending audio.
    pub ssrc: u32,
    /// Voice server IP address.
    pub ip: String,
    /// Voice server UDP port.
    pub port: u16,
    /// Offered encryption modes.
    pub modes: Vec<String>,
    /// Heartbeat interval (optional, in case present in Ready).
    pub heartbeat_interval: Option<f64>,
}

/// Inner UDP connection info for Opcode 1 SelectProtocol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceProtocolDetails {
    /// Discovered external IP address.
    pub address: String,
    /// Discovered external UDP port.
    pub port: u16,
    /// Selected encryption mode.
    pub mode: String,
}

/// Outbound Opcode 1 SelectProtocol payload data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceSelectProtocolData {
    /// Protocol name ("udp").
    pub protocol: String,
    /// UDP address and encryption mode details.
    pub data: VoiceProtocolDetails,
}

/// Outbound/Inbound Opcode 5 Speaking payload data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceSpeakingData {
    /// Speaking bitmask.
    pub speaking: u32,
    /// Delay in milliseconds (usually 0).
    pub delay: u32,
    /// SSRC of the speaker.
    pub ssrc: u32,
}

/// Inbound Opcode 4 SessionDescription payload data.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct VoiceSessionDescriptionData {
    /// Audio codec in use (typically "opus").
    pub audio_codec: String,
    /// Unique media session ID.
    pub media_session_id: Option<String>,
    /// Confirmed encryption mode.
    pub mode: String,
    /// 32-byte secret key for voice packet encryption.
    pub secret_key: Vec<u8>,
    /// Video codec in use if video streaming.
    pub video_codec: Option<String>,
}

/// Outbound Opcode 3 Heartbeat payload data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VoiceHeartbeatData {
    /// Timestamp / nonce integer.
    pub t: u64,
    /// Last acknowledged sequence number.
    pub seq_ack: u64,
}

/// Outbound action generated when Hello arrives.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceHelloAction {
    /// Send Opcode 0 Identify.
    Identify(VoiceGatewayPayload<VoiceIdentifyData>),
    /// Send Opcode 7 Resume.
    Resume(VoiceGatewayPayload<VoiceResumeData>),
}

/// State of the voice gateway session state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceSessionState {
    /// Initial unstarted state.
    Uninitialized,
    /// Waiting for Opcode 8 Hello from server.
    WaitingForHello,
    /// Identify sent, waiting for Opcode 2 Ready.
    Identifying,
    /// Resume sent, waiting for Opcode 9 Resumed.
    Resuming,
    /// Ready received; awaiting UDP IP discovery and SelectProtocol.
    Ready,
    /// SelectProtocol sent; waiting for Opcode 4 SessionDescription.
    SelectingProtocol,
    /// SessionDescription received; full voice media channel established.
    Connected,
    /// Session disconnected or closed.
    Disconnected,
}

/// Errors originating from voice session state transitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceSessionError {
    /// Operation is invalid in the current session state.
    InvalidState {
        /// Expected state description.
        expected: &'static str,
        /// Current state.
        actual: VoiceSessionState,
    },
    /// SSRC has not been assigned by the server yet.
    MissingSsrc,
    /// No compatible encryption mode could be selected from server's offered modes.
    NoCompatibleEncryptionMode {
        /// Modes offered by server.
        offered: Vec<String>,
    },
    /// Secret key provided in SessionDescription was invalid.
    InvalidSecretKeyLength(usize),
}

impl fmt::Display for VoiceSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidState { expected, actual } => {
                write!(f, "invalid voice session state: expected {expected}, was {actual:?}")
            }
            Self::MissingSsrc => write!(f, "missing SSRC: Ready has not been processed"),
            Self::NoCompatibleEncryptionMode { offered } => {
                write!(f, "no compatible encryption mode in offered list: {offered:?}")
            }
            Self::InvalidSecretKeyLength(len) => {
                write!(f, "expected 32-byte secret key, got {len} bytes")
            }
        }
    }
}

impl std::error::Error for VoiceSessionError {}

/// Deterministic sans-I/O Discord Voice Gateway v8 session state machine.
#[derive(Clone, Debug)]
pub struct VoiceSession {
    server_id: Snowflake,
    user_id: Snowflake,
    session_id: String,
    token: String,
    max_dave_protocol_version: Option<u8>,
    state: VoiceSessionState,
    heartbeat: VoiceHeartbeat,
    ssrc: Option<u32>,
    voice_ip: Option<String>,
    voice_port: Option<u16>,
    offered_modes: Vec<String>,
    selected_mode: Option<EncryptionMode>,
    secret_key: Option<[u8; 32]>,
    media_session_id: Option<String>,
    speaking: SpeakingFlags,
    resumable: bool,
}

impl VoiceSession {
    /// Create a new voice session state machine.
    pub fn new(
        server_id: Snowflake,
        user_id: Snowflake,
        session_id: impl Into<String>,
        token: impl Into<String>,
    ) -> Self {
        Self {
            server_id,
            user_id,
            session_id: session_id.into(),
            token: token.into(),
            max_dave_protocol_version: Some(1),
            state: VoiceSessionState::WaitingForHello,
            heartbeat: VoiceHeartbeat::new(),
            ssrc: None,
            voice_ip: None,
            voice_port: None,
            offered_modes: Vec::new(),
            selected_mode: None,
            secret_key: None,
            media_session_id: None,
            speaking: SpeakingFlags::empty(),
            resumable: false,
        }
    }

    /// Set the maximum supported DAVE protocol version.
    pub fn with_max_dave_protocol_version(mut self, version: u8) -> Self {
        self.max_dave_protocol_version = Some(version);
        self
    }

    /// Current state machine state.
    pub const fn state(&self) -> VoiceSessionState {
        self.state
    }

    /// True if voice media transmission is established (SessionDescription received).
    pub const fn is_connected(&self) -> bool {
        matches!(self.state, VoiceSessionState::Connected)
    }

    /// The client's assigned SSRC, if Ready has been received.
    pub const fn ssrc(&self) -> Option<u32> {
        self.ssrc
    }

    /// Voice server IP from Ready.
    pub fn voice_ip(&self) -> Option<&str> {
        self.voice_ip.as_deref()
    }

    /// Voice server UDP port from Ready.
    pub const fn voice_port(&self) -> Option<u16> {
        self.voice_port
    }

    /// Selected transport encryption mode.
    pub const fn selected_mode(&self) -> Option<EncryptionMode> {
        self.selected_mode
    }

    /// 32-byte shared secret key from SessionDescription.
    pub fn secret_key(&self) -> Option<&[u8; 32]> {
        self.secret_key.as_ref()
    }

    /// Current speaking flags.
    pub const fn speaking(&self) -> SpeakingFlags {
        self.speaking
    }

    /// Access the heartbeat subsystem.
    pub const fn heartbeat(&self) -> &VoiceHeartbeat {
        &self.heartbeat
    }

    /// Mutable access to the heartbeat subsystem.
    pub fn heartbeat_mut(&mut self) -> &mut VoiceHeartbeat {
        &mut self.heartbeat
    }

    /// Record a sequence number seen from the voice gateway.
    pub fn record_seq(&mut self, seq: u64) {
        self.heartbeat.record_seq(seq);
    }

    /// Poll heartbeat timer.
    pub fn poll_heartbeat(&mut self, now_ms: u64) -> VoiceHeartbeatAction {
        self.heartbeat.poll(now_ms)
    }

    /// Call on Opcode 6 Heartbeat ACK.
    pub fn on_heartbeat_ack(&mut self, nonce: u64, now_ms: u64) {
        self.heartbeat.on_ack(nonce, now_ms);
    }

    /// Build an Opcode 3 Heartbeat message.
    pub fn build_heartbeat_payload(&self, t: u64) -> VoiceGatewayPayload<VoiceHeartbeatData> {
        VoiceGatewayPayload::new(
            VoiceOpcode::Heartbeat,
            VoiceHeartbeatData {
                t,
                seq_ack: self.heartbeat.seq_ack().unwrap_or(0),
            },
        )
    }

    /// Handle Opcode 8 Hello.
    ///
    /// If resuming an existing session with an acknowledged sequence, returns
    /// [`VoiceHelloAction::Resume`]. Otherwise, returns [`VoiceHelloAction::Identify`].
    pub fn on_hello(
        &mut self,
        now_ms: u64,
        heartbeat_interval: f64,
        jitter: f64,
    ) -> VoiceHelloAction {
        self.heartbeat.on_hello(now_ms, heartbeat_interval, jitter);

        if self.resumable && self.heartbeat.seq_ack().is_some() {
            self.state = VoiceSessionState::Resuming;
            VoiceHelloAction::Resume(self.build_resume_payload())
        } else {
            self.state = VoiceSessionState::Identifying;
            VoiceHelloAction::Identify(self.build_identify_payload())
        }
    }

    /// Build an Opcode 0 Identify payload.
    pub fn build_identify_payload(&self) -> VoiceGatewayPayload<VoiceIdentifyData> {
        VoiceGatewayPayload::new(
            VoiceOpcode::Identify,
            VoiceIdentifyData {
                server_id: self.server_id.to_string(),
                user_id: self.user_id.to_string(),
                session_id: self.session_id.clone(),
                token: self.token.clone(),
                max_dave_protocol_version: self.max_dave_protocol_version,
            },
        )
    }

    /// Build an Opcode 7 Resume payload.
    pub fn build_resume_payload(&self) -> VoiceGatewayPayload<VoiceResumeData> {
        VoiceGatewayPayload::new(
            VoiceOpcode::Resume,
            VoiceResumeData {
                server_id: self.server_id.to_string(),
                session_id: self.session_id.clone(),
                token: self.token.clone(),
                seq_ack: self.heartbeat.seq_ack().unwrap_or(0),
            },
        )
    }

    /// Process Opcode 2 Ready.
    ///
    /// Stores SSRC, server IP, port, and offered encryption modes. Automatically selects
    /// the preferred encryption mode (`aead_xchacha20_poly1305_rtpsize` then `aead_aes256_gcm_rtpsize`).
    pub fn on_ready(&mut self, ready: VoiceReadyData) -> Result<EncryptionMode, VoiceSessionError> {
        let mode = EncryptionMode::from_offered(&ready.modes).ok_or_else(|| {
            VoiceSessionError::NoCompatibleEncryptionMode {
                offered: ready.modes.clone(),
            }
        })?;

        self.ssrc = Some(ready.ssrc);
        self.voice_ip = Some(ready.ip);
        self.voice_port = Some(ready.port);
        self.offered_modes = ready.modes;
        self.selected_mode = Some(mode);
        self.resumable = true;
        self.state = VoiceSessionState::Ready;

        Ok(mode)
    }

    /// Process Opcode 9 Resumed.
    pub fn on_resumed(&mut self) -> Result<(), VoiceSessionError> {
        self.state = VoiceSessionState::Connected;
        Ok(())
    }

    /// Build Opcode 1 SelectProtocol with external IP and port discovered via UDP IP Discovery.
    pub fn build_select_protocol(
        &mut self,
        external_ip: &str,
        external_port: u16,
    ) -> Result<VoiceGatewayPayload<VoiceSelectProtocolData>, VoiceSessionError> {
        let mode = self
            .selected_mode
            .ok_or(VoiceSessionError::InvalidState {
                expected: "Ready with selected encryption mode",
                actual: self.state,
            })?;

        self.state = VoiceSessionState::SelectingProtocol;

        Ok(VoiceGatewayPayload::new(
            VoiceOpcode::SelectProtocol,
            VoiceSelectProtocolData {
                protocol: "udp".to_string(),
                data: VoiceProtocolDetails {
                    address: external_ip.to_string(),
                    port: external_port,
                    mode: mode.as_str().to_string(),
                },
            },
        ))
    }

    /// Process Opcode 4 SessionDescription.
    pub fn on_session_description(
        &mut self,
        desc: VoiceSessionDescriptionData,
    ) -> Result<(), VoiceSessionError> {
        if desc.secret_key.len() != 32 {
            return Err(VoiceSessionError::InvalidSecretKeyLength(
                desc.secret_key.len(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&desc.secret_key);
        self.secret_key = Some(key);
        self.media_session_id = desc.media_session_id;
        self.state = VoiceSessionState::Connected;

        Ok(())
    }

    /// Build Opcode 5 Speaking payload.
    pub fn build_speaking(
        &mut self,
        flags: SpeakingFlags,
        delay: u32,
    ) -> Result<VoiceGatewayPayload<VoiceSpeakingData>, VoiceSessionError> {
        let ssrc = self.ssrc.ok_or(VoiceSessionError::MissingSsrc)?;
        self.speaking = flags;

        Ok(VoiceGatewayPayload::new(
            VoiceOpcode::Speaking,
            VoiceSpeakingData {
                speaking: flags.bits(),
                delay,
                ssrc,
            },
        ))
    }

    /// Handle connection close or disconnect.
    pub fn on_disconnect(&mut self, code: impl IntoVoiceCloseCode) -> VoiceCloseAction {
        let action = classify_voice_close(code);
        match action {
            VoiceCloseAction::Fatal => {
                self.state = VoiceSessionState::Disconnected;
                self.resumable = false;
                self.heartbeat.reset();
            }
            VoiceCloseAction::Reconnect => {
                self.state = VoiceSessionState::WaitingForHello;
                self.resumable = false;
                self.heartbeat.reset();
            }
            VoiceCloseAction::Resume => {
                self.state = VoiceSessionState::WaitingForHello;
                self.heartbeat.reset();
                // Retain seq_ack and resumable flag
            }
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_mode_selection_priority() {
        // Prefers xchacha over aes-gcm
        let modes = vec![
            "aead_aes256_gcm_rtpsize".to_string(),
            "aead_xchacha20_poly1305_rtpsize".to_string(),
        ];
        assert_eq!(
            EncryptionMode::from_offered(&modes),
            Some(EncryptionMode::AeadXChaCha20Poly1305RtpSize)
        );

        // Fallback to aes-gcm
        let modes_aes = vec!["aead_aes256_gcm_rtpsize".to_string()];
        assert_eq!(
            EncryptionMode::from_offered(&modes_aes),
            Some(EncryptionMode::AeadAes256GcmRtpSize)
        );

        // Rejects legacy xsalsa20 modes
        let legacy_modes = vec![
            "xsalsa20_poly1305".to_string(),
            "xsalsa20_poly1305_suffix".to_string(),
            "xsalsa20_poly1305_lite".to_string(),
        ];
        assert_eq!(EncryptionMode::from_offered(&legacy_modes), None);
    }

    #[test]
    fn speaking_flags_operations() {
        let mut flags = SpeakingFlags::MICROPHONE;
        assert!(flags.contains(SpeakingFlags::MICROPHONE));
        assert!(!flags.contains(SpeakingFlags::PRIORITY));

        flags |= SpeakingFlags::PRIORITY;
        assert!(flags.contains(SpeakingFlags::MICROPHONE));
        assert!(flags.contains(SpeakingFlags::PRIORITY));
        assert_eq!(flags.bits(), 1 | 4);
    }

    #[test]
    fn voice_session_full_lifecycle() {
        let server_id = Snowflake::new(123456789);
        let user_id = Snowflake::new(987654321);
        let session_id = "test_session_id";
        let token = "test_voice_token";

        let mut session = VoiceSession::new(server_id, user_id, session_id, token)
            .with_max_dave_protocol_version(1);

        assert_eq!(session.state(), VoiceSessionState::WaitingForHello);

        // 1. Hello arrives
        let hello_action = session.on_hello(0, 20_000.0, 0.5);
        assert_eq!(session.state(), VoiceSessionState::Identifying);

        match hello_action {
            VoiceHelloAction::Identify(id_msg) => {
                assert_eq!(id_msg.op, VoiceOpcode::Identify);
                assert_eq!(id_msg.d.server_id, "123456789");
                assert_eq!(id_msg.d.user_id, "987654321");
                assert_eq!(id_msg.d.session_id, "test_session_id");
                assert_eq!(id_msg.d.token, "test_voice_token");
                assert_eq!(id_msg.d.max_dave_protocol_version, Some(1));
            }
            _ => panic!("expected identify"),
        }

        // 2. Ready arrives
        let ready = VoiceReadyData {
            ssrc: 42,
            ip: "198.51.100.10".into(),
            port: 50000,
            modes: vec![
                "xsalsa20_poly1305".into(),
                "aead_aes256_gcm_rtpsize".into(),
                "aead_xchacha20_poly1305_rtpsize".into(),
            ],
            heartbeat_interval: None,
        };

        let selected = session.on_ready(ready).unwrap();
        assert_eq!(selected, EncryptionMode::AeadXChaCha20Poly1305RtpSize);
        assert_eq!(session.state(), VoiceSessionState::Ready);
        assert_eq!(session.ssrc(), Some(42));

        // 3. SelectProtocol
        let select = session.build_select_protocol("203.0.113.1", 54321).unwrap();
        assert_eq!(select.op, VoiceOpcode::SelectProtocol);
        assert_eq!(select.d.protocol, "udp");
        assert_eq!(select.d.data.address, "203.0.113.1");
        assert_eq!(select.d.data.port, 54321);
        assert_eq!(
            select.d.data.mode,
            "aead_xchacha20_poly1305_rtpsize"
        );
        assert_eq!(session.state(), VoiceSessionState::SelectingProtocol);

        // 4. SessionDescription arrives
        let desc = VoiceSessionDescriptionData {
            audio_codec: "opus".into(),
            media_session_id: Some("media_sess_1".into()),
            mode: "aead_xchacha20_poly1305_rtpsize".into(),
            secret_key: vec![7u8; 32],
            video_codec: None,
        };
        session.on_session_description(desc).unwrap();
        assert!(session.is_connected());
        assert_eq!(session.secret_key().unwrap(), &[7u8; 32]);

        // 5. Speaking update
        let spk = session.build_speaking(SpeakingFlags::MICROPHONE | SpeakingFlags::PRIORITY, 0).unwrap();
        assert_eq!(spk.op, VoiceOpcode::Speaking);
        assert_eq!(spk.d.ssrc, 42);
        assert_eq!(spk.d.speaking, 5);

        // 6. Record seq and heartbeat
        session.record_seq(99);
        let hb_action = session.poll_heartbeat(10_000);
        let sent_t = match hb_action {
            VoiceHeartbeatAction::Send { t, seq_ack } => {
                assert_eq!(seq_ack, Some(99));
                t
            }
            _ => panic!("expected heartbeat send"),
        };
        session.on_heartbeat_ack(sent_t, 10_015);

        // 7. Disconnect and resume
        let close_action = session.on_disconnect(4015); // VoiceServerCrashed -> Resume
        assert_eq!(close_action, VoiceCloseAction::Resume);

        // Reconnect and send Hello
        let hello_action2 = session.on_hello(20_000, 20_000.0, 0.0);
        match hello_action2 {
            VoiceHelloAction::Resume(res_msg) => {
                assert_eq!(res_msg.op, VoiceOpcode::Resume);
                assert_eq!(res_msg.d.seq_ack, 99);
                assert_eq!(res_msg.d.session_id, "test_session_id");
            }
            _ => panic!("expected resume"),
        }
        assert_eq!(session.state(), VoiceSessionState::Resuming);

        // Acknowledge Resumed
        session.on_resumed().unwrap();
        assert!(session.is_connected());
    }

    #[test]
    fn session_rejects_empty_or_incompatible_encryption_modes() {
        let server_id = Snowflake::new(123);
        let user_id = Snowflake::new(456);
        let mut session = VoiceSession::new(server_id, user_id, "s", "t");
        let _ = session.on_hello(0, 10_000.0, 0.0);

        let incompatible = VoiceReadyData {
            ssrc: 1,
            ip: "127.0.0.1".into(),
            port: 1234,
            modes: vec!["xsalsa20_poly1305".into()],
            heartbeat_interval: None,
        };

        let err = session.on_ready(incompatible).unwrap_err();
        assert!(matches!(err, VoiceSessionError::NoCompatibleEncryptionMode { .. }));
    }

    #[test]
    fn session_speaking_requires_ssrc() {
        let server_id = Snowflake::new(123);
        let user_id = Snowflake::new(456);
        let mut session = VoiceSession::new(server_id, user_id, "s", "t");

        let err = session.build_speaking(SpeakingFlags::MICROPHONE, 0).unwrap_err();
        assert_eq!(err, VoiceSessionError::MissingSsrc);
    }

    #[test]
    fn session_fatal_disconnect_clears_resumable() {
        let server_id = Snowflake::new(123);
        let user_id = Snowflake::new(456);
        let mut session = VoiceSession::new(server_id, user_id, "s", "t");
        let _ = session.on_hello(0, 10_000.0, 0.0);

        let ready = VoiceReadyData {
            ssrc: 1,
            ip: "127.0.0.1".into(),
            port: 1234,
            modes: vec!["aead_aes256_gcm_rtpsize".into()],
            heartbeat_interval: None,
        };
        session.on_ready(ready).unwrap();
        session.record_seq(5);

        // Disconnect with 4004 (AuthenticationFailed) -> Fatal
        let action = session.on_disconnect(4004);
        assert_eq!(action, VoiceCloseAction::Fatal);
        assert_eq!(session.state(), VoiceSessionState::Disconnected);

        // Next Hello produces Identify, not Resume
        let next_hello = session.on_hello(20_000, 10_000.0, 0.0);
        assert!(matches!(next_hello, VoiceHelloAction::Identify(_)));
    }
}
