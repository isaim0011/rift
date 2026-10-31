//! Sans-I/O Discord Voice Gateway v8 and UDP framing state machine.
//!
//! Provides deterministic protocol framing, state machines, heartbeat watchdogs,
//! UDP IP discovery, and RTP transport utilities without coupling to any specific
//! async runtime or network socket.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod close;
pub mod crypto;
pub mod driver;
pub mod heartbeat;
pub mod opcode;
pub mod session;
pub mod udp;

pub use close::{classify_voice_close, IntoVoiceCloseCode, VoiceCloseAction, VoiceCloseCode};
pub use crypto::{
    chacha20_block, chacha20_poly1305_decrypt, chacha20_poly1305_encrypt, chacha20_xor,
    constant_time_eq, hchacha20, xchacha20_poly1305_decrypt, xchacha20_poly1305_encrypt,
    CryptoError, Poly1305, VoicePacketCipher,
};
pub use driver::{
    UdpVoiceSocket, VoiceDriverError, VoiceReceiver, VoiceSender,
};
pub use heartbeat::{VoiceHeartbeat, VoiceHeartbeatAction};
pub use opcode::VoiceOpcode;
pub use session::{
    EncryptionMode, SpeakingFlags, VoiceGatewayPayload, VoiceHelloAction, VoiceHelloData,
    VoiceIdentifyData, VoiceProtocolDetails, VoiceReadyData, VoiceResumeData,
    VoiceSelectProtocolData, VoiceSession, VoiceSessionDescriptionData, VoiceSessionError,
    VoiceSessionState, VoiceSpeakingData,
};
pub use udp::{
    build_ip_discovery_request, build_ip_discovery_response, build_rtpsize_packet,
    parse_ip_discovery_request, parse_ip_discovery_response, split_rtpsize_packet,
    IpDiscoveryResponse, RtpHeader, RtpNonceAppendix, UdpError, IP_DISCOVERY_PACKET_LEN,
    IP_DISCOVERY_PAYLOAD_LEN, IP_DISCOVERY_REQUEST_TYPE, IP_DISCOVERY_RESPONSE_TYPE,
    NONCE_APPENDIX_LEN, OPUS_PAYLOAD_TYPE, OPUS_SILENCE_FRAME, OPUS_SILENCE_FRAME_COUNT,
    RTP_HEADER_LEN, RTP_VERSION_2,
};
