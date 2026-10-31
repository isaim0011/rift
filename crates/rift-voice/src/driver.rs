//! Async Tokio UDP voice driver and socket integration.
//!
//! Provides [`UdpVoiceSocket`] for performing 74-byte Discord UDP IP Discovery,
//! sending and receiving encrypted `rtpsize` RTP audio packets, and transmitting
//! required 5-frame silence bursts over real UDP sockets.

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::UdpSocket;

use crate::crypto::{CryptoError, VoicePacketCipher};
use crate::udp::{
    build_ip_discovery_request, parse_ip_discovery_response, IpDiscoveryResponse, RtpHeader,
    UdpError, IP_DISCOVERY_PACKET_LEN, OPUS_SILENCE_FRAME, OPUS_SILENCE_FRAME_COUNT,
};

/// Errors that can occur during async voice driver and UDP operations.
#[derive(Debug)]
pub enum VoiceDriverError {
    /// Underlying I/O error on the UDP socket.
    Io(std::io::Error),
    /// Error parsing UDP packet or IP discovery framing.
    Udp(UdpError),
    /// Cryptographic error during RTP encryption or decryption.
    Crypto(CryptoError),
    /// Packet cipher has not been configured on the socket.
    CipherNotConfigured,
    /// Remote socket address has not been configured.
    RemoteNotConfigured,
}

impl fmt::Display for VoiceDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "UDP I/O error: {err}"),
            Self::Udp(err) => write!(f, "UDP framing error: {err}"),
            Self::Crypto(err) => write!(f, "Voice crypto error: {err}"),
            Self::CipherNotConfigured => write!(f, "voice packet cipher is not configured"),
            Self::RemoteNotConfigured => write!(f, "remote voice address is not configured"),
        }
    }
}

impl std::error::Error for VoiceDriverError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Udp(err) => Some(err),
            Self::Crypto(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for VoiceDriverError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<UdpError> for VoiceDriverError {
    fn from(err: UdpError) -> Self {
        Self::Udp(err)
    }
}

impl From<CryptoError> for VoiceDriverError {
    fn from(err: CryptoError) -> Self {
        Self::Crypto(err)
    }
}

/// Async UDP socket for Discord voice connections.
///
/// Encapsulates the underlying Tokio [`UdpSocket`], manages RTP sequence numbers,
/// timestamps, incremental nonces, and coordinates encrypted Opus audio transmission.
#[derive(Debug)]
pub struct UdpVoiceSocket {
    socket: Arc<UdpSocket>,
    remote_addr: Option<SocketAddr>,
    cipher: Option<VoicePacketCipher>,
    ssrc: u32,
    sequence: u16,
    timestamp: u32,
    nonce_seq: u32,
}

impl UdpVoiceSocket {
    /// Bind a new UDP voice socket to a local address.
    pub async fn bind<A: tokio::net::ToSocketAddrs>(addr: A) -> Result<Self, VoiceDriverError> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self::from_socket(socket))
    }

    /// Construct a voice socket from an existing Tokio [`UdpSocket`].
    pub fn from_socket(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            remote_addr: None,
            cipher: None,
            ssrc: 0,
            sequence: 0,
            timestamp: 0,
            nonce_seq: 0,
        }
    }

    /// Connect the socket to a remote voice server address.
    pub async fn connect(&mut self, remote_addr: SocketAddr) -> Result<(), VoiceDriverError> {
        self.socket.connect(remote_addr).await?;
        self.remote_addr = Some(remote_addr);
        Ok(())
    }

    /// Set the remote address to send packets to without calling `connect` on the socket.
    pub fn set_remote_addr(&mut self, addr: SocketAddr) {
        self.remote_addr = Some(addr);
    }

    /// Get the configured remote socket address, if any.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Get the local address this socket is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, VoiceDriverError> {
        self.socket.local_addr().map_err(VoiceDriverError::Io)
    }

    /// Configure the AEAD cipher using the 32-byte secret key.
    pub fn set_cipher(&mut self, cipher: VoicePacketCipher) {
        self.cipher = Some(cipher);
    }

    /// Get a reference to the configured cipher, if any.
    pub fn cipher(&self) -> Option<&VoicePacketCipher> {
        self.cipher.as_ref()
    }

    /// Set the SSRC identifier for outbound RTP packets.
    pub fn set_ssrc(&mut self, ssrc: u32) {
        self.ssrc = ssrc;
    }

    /// Current SSRC identifier.
    pub fn ssrc(&self) -> u32 {
        self.ssrc
    }

    /// Current RTP sequence number.
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// Set the RTP sequence number.
    pub fn set_sequence(&mut self, sequence: u16) {
        self.sequence = sequence;
    }

    /// Current RTP timestamp.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Set the RTP timestamp.
    pub fn set_timestamp(&mut self, timestamp: u32) {
        self.timestamp = timestamp;
    }

    /// Current `rtpsize` nonce sequence counter.
    pub fn nonce_seq(&self) -> u32 {
        self.nonce_seq
    }

    /// Set the `rtpsize` nonce sequence counter.
    pub fn set_nonce_seq(&mut self, nonce_seq: u32) {
        self.nonce_seq = nonce_seq;
    }

    /// Reference to the underlying shared Tokio [`UdpSocket`].
    pub fn inner(&self) -> &Arc<UdpSocket> {
        &self.socket
    }

    /// Perform the 74-byte Discord UDP IP Discovery exchange.
    ///
    /// Sends a 74-byte IP Discovery request to `server_addr` containing the client's `ssrc`,
    /// awaits the 74-byte response, and parses the external IP and port.
    pub async fn discover_ip(
        &self,
        server_addr: SocketAddr,
        ssrc: u32,
    ) -> Result<IpDiscoveryResponse, VoiceDriverError> {
        let request = build_ip_discovery_request(ssrc);

        if let Err(_e) = self.socket.send_to(&request, server_addr).await {
            self.socket.send(&request).await?;
        }

        let mut buf = [0u8; IP_DISCOVERY_PACKET_LEN];
        let len = match self.socket.recv_from(&mut buf).await {
            Ok((n, _)) => n,
            Err(_) => self.socket.recv(&mut buf).await?,
        };

        let response = parse_ip_discovery_response(&buf[..len])?;
        Ok(response)
    }

    /// Encrypt and transmit an Opus audio frame over UDP using `aead_xchacha20_poly1305_rtpsize`.
    ///
    /// Constructs a standard 12-byte RTP header with current sequence, timestamp, and SSRC,
    /// encrypts the payload, appends the 4-byte nonce trailer, sends the packet to the remote
    /// address, and increments sequence by 1, timestamp by 960 (20ms @ 48kHz), and nonce by 1.
    pub async fn send_audio(&mut self, opus_payload: &[u8]) -> Result<usize, VoiceDriverError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(VoiceDriverError::CipherNotConfigured)?;

        let header = RtpHeader::new(self.sequence, self.timestamp, self.ssrc);
        let packet = cipher.encrypt_rtp(&header, opus_payload, self.nonce_seq);

        let sent = if let Some(remote) = self.remote_addr {
            match self.socket.send(&packet).await {
                Ok(n) => n,
                Err(_) => self.socket.send_to(&packet, remote).await?,
            }
        } else {
            self.socket.send(&packet).await?
        };

        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(960);
        self.nonce_seq = self.nonce_seq.wrapping_add(1);

        Ok(sent)
    }

    /// Transmit a burst of silence frames (`[0xF8, 0xFF, 0xFE]`) over UDP.
    ///
    /// Each frame increments sequence, timestamp, and nonce counter.
    pub async fn send_silence_frames(&mut self, count: usize) -> Result<(), VoiceDriverError> {
        for _ in 0..count {
            self.send_audio(&OPUS_SILENCE_FRAME).await?;
        }
        Ok(())
    }

    /// Transmit Discord's standard 5-frame silence burst (`OPUS_SILENCE_FRAME_COUNT`).
    pub async fn send_default_silence(&mut self) -> Result<(), VoiceDriverError> {
        self.send_silence_frames(OPUS_SILENCE_FRAME_COUNT).await
    }

    /// Receive and decrypt an inbound `rtpsize` encrypted RTP packet.
    pub async fn recv_audio(&self) -> Result<(RtpHeader, Vec<u8>), VoiceDriverError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(VoiceDriverError::CipherNotConfigured)?;

        let mut buf = [0u8; 4096];
        let len = match self.socket.recv(&mut buf).await {
            Ok(n) => n,
            Err(_) => self.socket.recv_from(&mut buf).await.map(|(n, _)| n)?,
        };

        let (header, payload) = cipher.decrypt_rtp(&buf[..len])?;
        Ok((header, payload))
    }

    /// Send a raw UDP packet to the configured remote address.
    pub async fn send_packet(&self, packet: &[u8]) -> Result<usize, VoiceDriverError> {
        if let Some(remote) = self.remote_addr {
            match self.socket.send(packet).await {
                Ok(n) => Ok(n),
                Err(_) => Ok(self.socket.send_to(packet, remote).await?),
            }
        } else {
            Ok(self.socket.send(packet).await?)
        }
    }

    /// Receive a raw UDP packet and return the number of bytes and source address.
    pub async fn recv_packet(
        &self,
        buf: &mut [u8],
    ) -> Result<(usize, SocketAddr), VoiceDriverError> {
        let res = self.socket.recv_from(buf).await?;
        Ok(res)
    }

    /// Split this socket into independent sending and receiving halves.
    pub fn split(self) -> (VoiceSender, VoiceReceiver) {
        let sender = VoiceSender {
            socket: Arc::clone(&self.socket),
            remote_addr: self.remote_addr,
            cipher: self.cipher.clone(),
            ssrc: self.ssrc,
            sequence: self.sequence,
            timestamp: self.timestamp,
            nonce_seq: self.nonce_seq,
        };

        let receiver = VoiceReceiver {
            socket: self.socket,
            cipher: self.cipher,
        };

        (sender, receiver)
    }
}

/// Outbound sending half of a split [`UdpVoiceSocket`].
#[derive(Debug)]
pub struct VoiceSender {
    socket: Arc<UdpSocket>,
    remote_addr: Option<SocketAddr>,
    cipher: Option<VoicePacketCipher>,
    ssrc: u32,
    sequence: u16,
    timestamp: u32,
    nonce_seq: u32,
}

impl VoiceSender {
    /// Configure the AEAD cipher using the 32-byte secret key.
    pub fn set_cipher(&mut self, cipher: VoicePacketCipher) {
        self.cipher = Some(cipher);
    }

    /// Set the SSRC identifier.
    pub fn set_ssrc(&mut self, ssrc: u32) {
        self.ssrc = ssrc;
    }

    /// Current sequence number.
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// Current timestamp.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Current nonce counter.
    pub fn nonce_seq(&self) -> u32 {
        self.nonce_seq
    }

    /// Encrypt and transmit an Opus audio frame over UDP.
    pub async fn send_audio(&mut self, opus_payload: &[u8]) -> Result<usize, VoiceDriverError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(VoiceDriverError::CipherNotConfigured)?;

        let header = RtpHeader::new(self.sequence, self.timestamp, self.ssrc);
        let packet = cipher.encrypt_rtp(&header, opus_payload, self.nonce_seq);

        let sent = if let Some(remote) = self.remote_addr {
            match self.socket.send(&packet).await {
                Ok(n) => n,
                Err(_) => self.socket.send_to(&packet, remote).await?,
            }
        } else {
            self.socket.send(&packet).await?
        };

        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(960);
        self.nonce_seq = self.nonce_seq.wrapping_add(1);

        Ok(sent)
    }

    /// Transmit silence frames over UDP.
    pub async fn send_silence_frames(&mut self, count: usize) -> Result<(), VoiceDriverError> {
        for _ in 0..count {
            self.send_audio(&OPUS_SILENCE_FRAME).await?;
        }
        Ok(())
    }

    /// Transmit the standard 5-frame silence burst.
    pub async fn send_default_silence(&mut self) -> Result<(), VoiceDriverError> {
        self.send_silence_frames(OPUS_SILENCE_FRAME_COUNT).await
    }
}

/// Inbound receiving half of a split [`UdpVoiceSocket`].
#[derive(Debug)]
pub struct VoiceReceiver {
    socket: Arc<UdpSocket>,
    cipher: Option<VoicePacketCipher>,
}

impl VoiceReceiver {
    /// Configure the AEAD cipher using the 32-byte secret key.
    pub fn set_cipher(&mut self, cipher: VoicePacketCipher) {
        self.cipher = Some(cipher);
    }

    /// Receive and decrypt an inbound audio packet.
    pub async fn recv_audio(&self) -> Result<(RtpHeader, Vec<u8>), VoiceDriverError> {
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(VoiceDriverError::CipherNotConfigured)?;

        let mut buf = [0u8; 4096];
        let (len, _) = self.socket.recv_from(&mut buf).await?;
        let (header, payload) = cipher.decrypt_rtp(&buf[..len])?;
        Ok((header, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::udp::build_ip_discovery_response;

    #[tokio::test]
    async fn ip_discovery_over_loopback_mock_server() {
        // Start mock voice UDP server on loopback
        let server_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();

        let ssrc = 0xabcdef12;
        let mock_ip = "198.51.100.42";
        let mock_port = 54321;

        // Spawn server task to handle IP discovery request
        let server_handle = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let (len, client_addr) = server_socket.recv_from(&mut buf).await.unwrap();
            assert_eq!(len, IP_DISCOVERY_PACKET_LEN);

            // Respond with discovered external IP and port
            let resp = build_ip_discovery_response(ssrc, mock_ip, mock_port).unwrap();
            server_socket.send_to(&resp, client_addr).await.unwrap();
        });

        // Client performs IP discovery
        let client_socket = UdpVoiceSocket::bind("127.0.0.1:0").await.unwrap();
        let discovery = client_socket
            .discover_ip(server_addr, ssrc)
            .await
            .expect("IP discovery failed");

        assert_eq!(discovery.ssrc, ssrc);
        assert_eq!(discovery.ip, mock_ip);
        assert_eq!(discovery.port, mock_port);

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn send_and_recv_encrypted_audio_and_silence_over_loopback() {
        let secret_key = [0x99u8; 32];
        let cipher = VoicePacketCipher::new(secret_key);

        // Server socket
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();

        // Client voice socket
        let mut client = UdpVoiceSocket::bind("127.0.0.1:0").await.unwrap();
        client.set_remote_addr(server_addr);
        client.set_cipher(cipher.clone());
        let ssrc = 0x55667788;
        client.set_ssrc(ssrc);

        // 1. Send single audio frame
        let opus_data = vec![0x78, 0x01, 0x02, 0x03, 0x04];
        let sent = client.send_audio(&opus_data).await.unwrap();
        assert!(sent > opus_data.len());

        let mut buf = [0u8; 1024];
        let (n, client_addr) = server.recv_from(&mut buf).await.unwrap();
        let (header, decrypted) = cipher.decrypt_rtp(&buf[..n]).unwrap();
        assert_eq!(header.sequence, 0);
        assert_eq!(header.timestamp, 0);
        assert_eq!(header.ssrc, ssrc);
        assert_eq!(decrypted, opus_data);

        // Client sequence and timestamp should have incremented
        assert_eq!(client.sequence(), 1);
        assert_eq!(client.timestamp(), 960);
        assert_eq!(client.nonce_seq(), 1);

        // 2. Send 5-frame silence burst
        client.send_default_silence().await.unwrap();
        assert_eq!(client.sequence(), 6);
        assert_eq!(client.timestamp(), 960 * 6);
        assert_eq!(client.nonce_seq(), 6);

        // Verify each of the 5 silence frames received on server
        for i in 1..=5 {
            let (n, _) = server.recv_from(&mut buf).await.unwrap();
            let (h, payload) = cipher.decrypt_rtp(&buf[..n]).unwrap();
            assert_eq!(h.sequence, i as u16);
            assert_eq!(h.timestamp, (i * 960) as u32);
            assert_eq!(h.ssrc, ssrc);
            assert_eq!(payload, OPUS_SILENCE_FRAME);
        }

        // 3. Test server sending audio to client and client receiving via recv_audio
        let inbound_header = RtpHeader::new(100, 48000, 0x11112222);
        let inbound_payload = vec![0xAA, 0xBB, 0xCC];
        let inbound_packet = cipher.encrypt_rtp(&inbound_header, &inbound_payload, 500);

        server.send_to(&inbound_packet, client_addr).await.unwrap();

        let (recvd_header, recvd_payload) = client.recv_audio().await.unwrap();
        assert_eq!(recvd_header.sequence, 100);
        assert_eq!(recvd_header.timestamp, 48000);
        assert_eq!(recvd_header.ssrc, 0x11112222);
        assert_eq!(recvd_payload, inbound_payload);
    }

    #[tokio::test]
    async fn split_sender_receiver_loopback() {
        let key = [0x77u8; 32];
        let cipher = VoicePacketCipher::new(key);

        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server.local_addr().unwrap();

        let mut client = UdpVoiceSocket::bind("127.0.0.1:0").await.unwrap();
        client.set_remote_addr(server_addr);
        client.set_cipher(cipher.clone());
        client.set_ssrc(0x1234);

        let (mut sender, receiver) = client.split();

        // Sender sends audio
        sender.send_audio(b"split test").await.unwrap();

        let mut buf = [0u8; 512];
        let (n, client_addr) = server.recv_from(&mut buf).await.unwrap();
        let (_, dec) = cipher.decrypt_rtp(&buf[..n]).unwrap();
        assert_eq!(dec, b"split test");

        // Server replies, receiver receives
        let reply_header = RtpHeader::new(20, 1920, 0x9999);
        let reply_packet = cipher.encrypt_rtp(&reply_header, b"reply back", 77);
        server.send_to(&reply_packet, client_addr).await.unwrap();

        let (rec_hdr, rec_pay) = receiver.recv_audio().await.unwrap();
        assert_eq!(rec_hdr.sequence, 20);
        assert_eq!(rec_pay, b"reply back");
    }
}
