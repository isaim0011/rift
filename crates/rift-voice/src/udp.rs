//! Discord Voice UDP framing: 74-byte IP Discovery, RTP header, and `rtpsize` nonce appendix.

use core::fmt;

/// Expected total length of an IP Discovery packet in bytes.
pub const IP_DISCOVERY_PACKET_LEN: usize = 74;

/// Packet type for IP Discovery request.
pub const IP_DISCOVERY_REQUEST_TYPE: u16 = 0x0001;

/// Packet type for IP Discovery response.
pub const IP_DISCOVERY_RESPONSE_TYPE: u16 = 0x0002;

/// Length field in the IP Discovery packet header (74 - 4 = 70).
pub const IP_DISCOVERY_PAYLOAD_LEN: u16 = 70;

/// Standard 12-byte RTP header length.
pub const RTP_HEADER_LEN: usize = 12;

/// Standard RTP Version 2 flags (`0x80`).
pub const RTP_VERSION_2: u8 = 0x80;

/// Discord standard payload type for Opus audio (`0x78` = 120).
pub const OPUS_PAYLOAD_TYPE: u8 = 0x78;

/// Length of the `rtpsize` 4-byte incremental nonce appendix in bytes.
pub const NONCE_APPENDIX_LEN: usize = 4;

/// Opus silence frame bytes (`[0xF8, 0xFF, 0xFE]`).
///
/// When stopping transmission, Discord requires clients to send 5 consecutive
/// frames of silence to avoid interpolation artifacts and cleanly reset the decoder.
pub const OPUS_SILENCE_FRAME: [u8; 3] = [0xF8, 0xFF, 0xFE];

/// Number of silence frames required by Discord after audio transmission stops.
pub const OPUS_SILENCE_FRAME_COUNT: usize = 5;

/// Errors that can occur when parsing UDP packets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UdpError {
    /// Packet buffer is too short.
    PacketTooShort {
        /// Minimum bytes expected.
        expected: usize,
        /// Actual bytes provided.
        actual: usize,
    },
    /// The message type field does not match the expected value.
    InvalidPacketType {
        /// Expected packet type.
        expected: u16,
        /// Actual packet type parsed.
        actual: u16,
    },
    /// The message length field does not match the expected value (70).
    InvalidPayloadLength {
        /// Expected payload length.
        expected: u16,
        /// Actual payload length parsed.
        actual: u16,
    },
    /// The IP address field in the response is not valid ASCII / UTF-8.
    InvalidIpString(String),
    /// The IP address string is too long to fit in the 64-byte field.
    IpTooLong {
        /// Maximum allowed bytes.
        max: usize,
        /// Actual bytes passed.
        actual: usize,
    },
    /// Invalid RTP version (expected 0x80).
    InvalidRtpVersion(u8),
    /// Invalid RTP payload type (expected 0x78 for Opus).
    InvalidPayloadType(u8),
}

impl fmt::Display for UdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PacketTooShort { expected, actual } => {
                write!(f, "packet too short: expected at least {expected} bytes, got {actual}")
            }
            Self::InvalidPacketType { expected, actual } => {
                write!(f, "invalid packet type: expected {expected:#06x}, got {actual:#06x}")
            }
            Self::InvalidPayloadLength { expected, actual } => {
                write!(f, "invalid payload length: expected {expected}, got {actual}")
            }
            Self::InvalidIpString(msg) => write!(f, "invalid IP string: {msg}"),
            Self::IpTooLong { max, actual } => {
                write!(f, "IP address exceeds maximum length of {max} bytes (got {actual})")
            }
            Self::InvalidRtpVersion(v) => write!(f, "invalid RTP version flags: {v:#04x}"),
            Self::InvalidPayloadType(pt) => write!(f, "invalid RTP payload type: {pt}"),
        }
    }
}

impl std::error::Error for UdpError {}

/// Parsed result of a Discord UDP IP Discovery response packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpDiscoveryResponse {
    /// SSRC echoed back by the server.
    pub ssrc: u32,
    /// Discovered external IP address (IPv4 or IPv6 string).
    pub ip: String,
    /// Discovered external UDP port.
    pub port: u16,
}

/// Build a 74-byte Discord UDP IP Discovery request packet.
///
/// Layout:
/// - `0x0001` (2 bytes, big-endian message type)
/// - `70` (`0x0046`, 2 bytes, big-endian payload length)
/// - `ssrc` (4 bytes, big-endian)
/// - 64 bytes of zero padding
/// - 2 bytes of zero padding
pub fn build_ip_discovery_request(ssrc: u32) -> [u8; IP_DISCOVERY_PACKET_LEN] {
    let mut packet = [0u8; IP_DISCOVERY_PACKET_LEN];
    packet[0..2].copy_from_slice(&IP_DISCOVERY_REQUEST_TYPE.to_be_bytes());
    packet[2..4].copy_from_slice(&IP_DISCOVERY_PAYLOAD_LEN.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    packet
}

/// Parse a 74-byte Discord UDP IP Discovery request packet.
pub fn parse_ip_discovery_request(packet: &[u8]) -> Result<u32, UdpError> {
    if packet.len() < IP_DISCOVERY_PACKET_LEN {
        return Err(UdpError::PacketTooShort {
            expected: IP_DISCOVERY_PACKET_LEN,
            actual: packet.len(),
        });
    }

    let msg_type = u16::from_be_bytes([packet[0], packet[1]]);
    if msg_type != IP_DISCOVERY_REQUEST_TYPE {
        return Err(UdpError::InvalidPacketType {
            expected: IP_DISCOVERY_REQUEST_TYPE,
            actual: msg_type,
        });
    }

    let payload_len = u16::from_be_bytes([packet[2], packet[3]]);
    if payload_len != IP_DISCOVERY_PAYLOAD_LEN {
        return Err(UdpError::InvalidPayloadLength {
            expected: IP_DISCOVERY_PAYLOAD_LEN,
            actual: payload_len,
        });
    }

    let ssrc = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
    Ok(ssrc)
}

/// Build a 74-byte Discord UDP IP Discovery response packet (primarily for testing and mock servers).
pub fn build_ip_discovery_response(
    ssrc: u32,
    ip: &str,
    port: u16,
) -> Result<[u8; IP_DISCOVERY_PACKET_LEN], UdpError> {
    let ip_bytes = ip.as_bytes();
    if ip_bytes.len() > 63 {
        return Err(UdpError::IpTooLong {
            max: 63,
            actual: ip_bytes.len(),
        });
    }

    let mut packet = [0u8; IP_DISCOVERY_PACKET_LEN];
    packet[0..2].copy_from_slice(&IP_DISCOVERY_RESPONSE_TYPE.to_be_bytes());
    packet[2..4].copy_from_slice(&IP_DISCOVERY_PAYLOAD_LEN.to_be_bytes());
    packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
    packet[8..8 + ip_bytes.len()].copy_from_slice(ip_bytes);
    packet[72..74].copy_from_slice(&port.to_be_bytes());

    Ok(packet)
}

/// Parse a 74-byte Discord UDP IP Discovery response packet.
pub fn parse_ip_discovery_response(packet: &[u8]) -> Result<IpDiscoveryResponse, UdpError> {
    if packet.len() < IP_DISCOVERY_PACKET_LEN {
        return Err(UdpError::PacketTooShort {
            expected: IP_DISCOVERY_PACKET_LEN,
            actual: packet.len(),
        });
    }

    let msg_type = u16::from_be_bytes([packet[0], packet[1]]);
    if msg_type != IP_DISCOVERY_RESPONSE_TYPE {
        return Err(UdpError::InvalidPacketType {
            expected: IP_DISCOVERY_RESPONSE_TYPE,
            actual: msg_type,
        });
    }

    let payload_len = u16::from_be_bytes([packet[2], packet[3]]);
    if payload_len != IP_DISCOVERY_PAYLOAD_LEN {
        return Err(UdpError::InvalidPayloadLength {
            expected: IP_DISCOVERY_PAYLOAD_LEN,
            actual: payload_len,
        });
    }

    let ssrc = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);

    // Read null-terminated ASCII IP from bytes 8..72 (64 bytes)
    let ip_slice = &packet[8..72];
    let null_pos = ip_slice.iter().position(|&b| b == 0).unwrap_or(ip_slice.len());
    let ip_str = std::str::from_utf8(&ip_slice[..null_pos])
        .map_err(|e| UdpError::InvalidIpString(e.to_string()))?;

    if ip_str.is_empty() {
        return Err(UdpError::InvalidIpString("empty IP address".into()));
    }

    let port = u16::from_be_bytes([packet[72], packet[73]]);

    Ok(IpDiscoveryResponse {
        ssrc,
        ip: ip_str.to_string(),
        port,
    })
}

/// Standard 12-byte RTP header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtpHeader {
    /// RTP version flags (`0x80` = Version 2, no padding, no extension, no CSRC).
    pub version: u8,
    /// Payload type (`0x78` = 120 for Opus audio).
    pub payload_type: u8,
    /// 16-bit sequence number (increments per packet).
    pub sequence: u16,
    /// 32-bit timestamp (increments by sample rate / frame size, e.g. 960 per 20ms).
    pub timestamp: u32,
    /// 32-bit Synchronization Source identifier.
    pub ssrc: u32,
}

impl RtpHeader {
    /// Create a new RTP header with standard Discord Opus defaults (`version = 0x80`, `payload_type = 0x78`).
    pub const fn new(sequence: u16, timestamp: u32, ssrc: u32) -> Self {
        Self {
            version: RTP_VERSION_2,
            payload_type: OPUS_PAYLOAD_TYPE,
            sequence,
            timestamp,
            ssrc,
        }
    }

    /// Serialize header to 12 bytes in network byte order.
    pub fn to_bytes(&self) -> [u8; RTP_HEADER_LEN] {
        let mut bytes = [0u8; RTP_HEADER_LEN];
        bytes[0] = self.version;
        bytes[1] = self.payload_type;
        bytes[2..4].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.ssrc.to_be_bytes());
        bytes
    }

    /// Parse an RTP header from a byte slice.
    pub fn parse(buf: &[u8]) -> Result<Self, UdpError> {
        if buf.len() < RTP_HEADER_LEN {
            return Err(UdpError::PacketTooShort {
                expected: RTP_HEADER_LEN,
                actual: buf.len(),
            });
        }

        let version = buf[0];
        if (version >> 6) != 2 {
            return Err(UdpError::InvalidRtpVersion(version));
        }

        let payload_type = buf[1] & 0x7F;
        let sequence = u16::from_be_bytes([buf[2], buf[3]]);
        let timestamp = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let ssrc = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

        Ok(Self {
            version,
            payload_type,
            sequence,
            timestamp,
            ssrc,
        })
    }
}

/// 4-byte incremental nonce appendix used in Discord's `rtpsize` encryption modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RtpNonceAppendix {
    /// Incremental 32-bit counter.
    pub counter: u32,
}

impl RtpNonceAppendix {
    /// Create a new nonce appendix with a specific 32-bit counter.
    pub const fn new(counter: u32) -> Self {
        Self { counter }
    }

    /// Serialize to 4 big-endian bytes.
    pub fn to_bytes(&self) -> [u8; NONCE_APPENDIX_LEN] {
        self.counter.to_be_bytes()
    }

    /// Parse from a 4-byte slice.
    pub fn parse(buf: &[u8]) -> Result<Self, UdpError> {
        if buf.len() < NONCE_APPENDIX_LEN {
            return Err(UdpError::PacketTooShort {
                expected: NONCE_APPENDIX_LEN,
                actual: buf.len(),
            });
        }
        let counter = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        Ok(Self { counter })
    }

    /// Parse the 4-byte nonce appendix from the end of a complete UDP packet.
    pub fn parse_from_packet_end(packet: &[u8]) -> Result<Self, UdpError> {
        if packet.len() < RTP_HEADER_LEN + NONCE_APPENDIX_LEN {
            return Err(UdpError::PacketTooShort {
                expected: RTP_HEADER_LEN + NONCE_APPENDIX_LEN,
                actual: packet.len(),
            });
        }
        let start = packet.len() - NONCE_APPENDIX_LEN;
        Self::parse(&packet[start..])
    }
}

/// Split an `rtpsize` encrypted UDP packet into its 12-byte RTP header,
/// the encrypted payload slice (ciphertext + 16-byte tag), and the 4-byte nonce counter.
pub fn split_rtpsize_packet(packet: &[u8]) -> Result<(RtpHeader, &[u8], u32), UdpError> {
    if packet.len() < RTP_HEADER_LEN + NONCE_APPENDIX_LEN {
        return Err(UdpError::PacketTooShort {
            expected: RTP_HEADER_LEN + NONCE_APPENDIX_LEN,
            actual: packet.len(),
        });
    }

    let header = RtpHeader::parse(&packet[..RTP_HEADER_LEN])?;
    let payload_end = packet.len() - NONCE_APPENDIX_LEN;
    let payload = &packet[RTP_HEADER_LEN..payload_end];
    let appendix = RtpNonceAppendix::parse(&packet[payload_end..])?;

    Ok((header, payload, appendix.counter))
}

/// Assemble an `rtpsize` UDP packet from an RTP header, encrypted payload, and 4-byte nonce counter.
pub fn build_rtpsize_packet(
    header: &RtpHeader,
    ciphertext_and_tag: &[u8],
    nonce_counter: u32,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(RTP_HEADER_LEN + ciphertext_and_tag.len() + NONCE_APPENDIX_LEN);
    packet.extend_from_slice(&header.to_bytes());
    packet.extend_from_slice(ciphertext_and_tag);
    packet.extend_from_slice(&nonce_counter.to_be_bytes());
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_discovery_request_round_trip() {
        let ssrc = 0x12345678;
        let packet = build_ip_discovery_request(ssrc);
        assert_eq!(packet.len(), 74);
        assert_eq!(&packet[0..2], &[0x00, 0x01]);
        assert_eq!(&packet[2..4], &[0x00, 0x46]);
        assert_eq!(&packet[4..8], &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(&packet[8..], &[0u8; 66]);

        let parsed_ssrc = parse_ip_discovery_request(&packet).unwrap();
        assert_eq!(parsed_ssrc, ssrc);
    }

    #[test]
    fn ip_discovery_response_round_trip() {
        let ssrc = 0x99887766;
        let ip = "203.0.113.195";
        let port = 54321;

        let packet = build_ip_discovery_response(ssrc, ip, port).unwrap();
        assert_eq!(packet.len(), 74);
        assert_eq!(&packet[0..2], &[0x00, 0x02]);
        assert_eq!(&packet[2..4], &[0x00, 0x46]);

        let response = parse_ip_discovery_response(&packet).unwrap();
        assert_eq!(response.ssrc, ssrc);
        assert_eq!(response.ip, ip);
        assert_eq!(response.port, port);
    }

    #[test]
    fn ip_discovery_invalid_packets() {
        let short = [0u8; 70];
        assert!(matches!(
            parse_ip_discovery_request(&short),
            Err(UdpError::PacketTooShort { .. })
        ));
        assert!(matches!(
            parse_ip_discovery_response(&short),
            Err(UdpError::PacketTooShort { .. })
        ));

        let mut invalid_type = build_ip_discovery_request(10);
        invalid_type[0..2].copy_from_slice(&0x0005u16.to_be_bytes());
        assert!(matches!(
            parse_ip_discovery_request(&invalid_type),
            Err(UdpError::InvalidPacketType { .. })
        ));

        let long_ip = "a".repeat(64);
        assert!(matches!(
            build_ip_discovery_response(1, &long_ip, 1234),
            Err(UdpError::IpTooLong { .. })
        ));
    }

    #[test]
    fn rtp_header_round_trip() {
        let header = RtpHeader::new(1234, 48000, 0xAABBCCDD);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 12);
        assert_eq!(bytes[0], 0x80);
        assert_eq!(bytes[1], 0x78);

        let parsed = RtpHeader::parse(&bytes).unwrap();
        assert_eq!(parsed, header);
        assert_eq!(parsed.sequence, 1234);
        assert_eq!(parsed.timestamp, 48000);
        assert_eq!(parsed.ssrc, 0xAABBCCDD);
    }

    #[test]
    fn rtpsize_packet_split_and_build() {
        let header = RtpHeader::new(10, 960, 42);
        let ciphertext = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
        let nonce = 777;

        let packet = build_rtpsize_packet(&header, &ciphertext, nonce);
        assert_eq!(packet.len(), 12 + 6 + 4);

        let (parsed_header, parsed_payload, parsed_nonce) =
            split_rtpsize_packet(&packet).unwrap();
        assert_eq!(parsed_header, header);
        assert_eq!(parsed_payload, &ciphertext);
        assert_eq!(parsed_nonce, nonce);
    }

    #[test]
    fn opus_silence_frame_constants() {
        assert_eq!(OPUS_SILENCE_FRAME, [0xF8, 0xFF, 0xFE]);
        assert_eq!(OPUS_SILENCE_FRAME_COUNT, 5);
    }

    #[test]
    fn rtp_header_invalid_version_and_short_buffer() {
        let short = [0x80, 0x78, 0x00];
        assert!(matches!(RtpHeader::parse(&short), Err(UdpError::PacketTooShort { .. })));

        let mut invalid_version = RtpHeader::new(1, 100, 10).to_bytes();
        invalid_version[0] = 0x00; // version 0 instead of 2 (0x80)
        assert!(matches!(RtpHeader::parse(&invalid_version), Err(UdpError::InvalidRtpVersion(0x00))));
    }

    #[test]
    fn rtp_nonce_appendix_parse_and_split_errors() {
        let appendix = RtpNonceAppendix::new(0x01020304);
        assert_eq!(appendix.to_bytes(), [0x01, 0x02, 0x03, 0x04]);

        let parsed = RtpNonceAppendix::parse(&[0x01, 0x02, 0x03, 0x04]).unwrap();
        assert_eq!(parsed.counter, 0x01020304);

        let packet = [0u8; 15]; // shorter than 12 + 4 = 16
        assert!(matches!(
            RtpNonceAppendix::parse_from_packet_end(&packet),
            Err(UdpError::PacketTooShort { .. })
        ));
        assert!(matches!(
            split_rtpsize_packet(&packet),
            Err(UdpError::PacketTooShort { .. })
        ));
    }
}
