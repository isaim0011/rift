//! Pure-Rust RFC 8439 ChaCha20-Poly1305 and XChaCha20-Poly1305 transport encryption.
//!
//! Provides deterministic AEAD packet encryption and decryption for Discord Voice
//! UDP framing, supporting the `aead_xchacha20_poly1305_rtpsize` encryption mode.

use core::fmt;

use crate::udp::{RtpHeader, UdpError, RTP_HEADER_LEN};

/// Errors that can occur during cryptographic encryption or decryption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CryptoError {
    /// Packet buffer is too short.
    PacketTooShort {
        /// Minimum bytes expected.
        expected: usize,
        /// Actual bytes provided.
        actual: usize,
    },
    /// RTP header parsing error.
    InvalidRtpHeader(UdpError),
    /// Authentication tag verification failed.
    AuthenticationFailed,
    /// Invalid packet structure.
    InvalidPacket(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PacketTooShort { expected, actual } => {
                write!(
                    f,
                    "packet too short: expected at least {expected} bytes, got {actual}"
                )
            }
            Self::InvalidRtpHeader(err) => write!(f, "invalid RTP header: {err}"),
            Self::AuthenticationFailed => write!(f, "Poly1305 MAC authentication failed"),
            Self::InvalidPacket(msg) => write!(f, "invalid packet: {msg}"),
        }
    }
}

impl std::error::Error for CryptoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidRtpHeader(err) => Some(err),
            _ => None,
        }
    }
}

impl From<UdpError> for CryptoError {
    fn from(err: UdpError) -> Self {
        Self::InvalidRtpHeader(err)
    }
}

/// Compare two byte slices in constant time.
#[inline]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// ChaCha20 quarter-round operation on 4 indices in a 16-word state array.
#[inline(always)]
fn quarter_round(a: usize, b: usize, c: usize, d: usize, state: &mut [u32; 16]) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);

    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}

/// RFC 8439 state initialization constants: "expand 32-byte k".
const CHACHA20_CONSTANTS: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

/// Execute 20 rounds of ChaCha (10 double rounds) on a 16-word state array.
#[inline(always)]
fn chacha20_rounds(state: &mut [u32; 16]) {
    for _ in 0..10 {
        // Column rounds
        quarter_round(0, 4, 8, 12, state);
        quarter_round(1, 5, 9, 13, state);
        quarter_round(2, 6, 10, 14, state);
        quarter_round(3, 7, 11, 15, state);

        // Diagonal rounds
        quarter_round(0, 5, 10, 15, state);
        quarter_round(1, 6, 11, 12, state);
        quarter_round(2, 7, 8, 13, state);
        quarter_round(3, 4, 9, 14, state);
    }
}

/// Generate a 64-byte keystream block using RFC 8439 ChaCha20.
pub fn chacha20_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut initial = [0u32; 16];
    initial[0..4].copy_from_slice(&CHACHA20_CONSTANTS);

    for i in 0..8 {
        initial[4 + i] = u32::from_le_bytes(key[i * 4..(i + 1) * 4].try_into().unwrap());
    }

    initial[12] = counter;

    for i in 0..3 {
        initial[13 + i] = u32::from_le_bytes(nonce[i * 4..(i + 1) * 4].try_into().unwrap());
    }

    let mut working = initial;
    chacha20_rounds(&mut working);

    let mut out = [0u8; 64];
    for i in 0..16 {
        let val = working[i].wrapping_add(initial[i]);
        out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_le_bytes());
    }
    out
}

/// Encrypt or decrypt data using RFC 8439 ChaCha20 stream cipher starting at `counter`.
pub fn chacha20_xor(key: &[u8; 32], counter: u32, nonce: &[u8; 12], data: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len());
    let mut current_counter = counter;

    for chunk in data.chunks(64) {
        let block = chacha20_block(key, current_counter, nonce);
        for (&d, &k) in chunk.iter().zip(block.iter()) {
            output.push(d ^ k);
        }
        current_counter = current_counter.wrapping_add(1);
    }

    output
}

/// HChaCha20 intermediate subkey derivation function (draft-irtf-cfrg-xchacha).
///
/// Takes a 32-byte key and 16-byte nonce, performs 20 rounds of ChaCha,
/// and returns the 32-byte subkey formed by words 0..4 and words 12..16
/// without adding the initial state.
pub fn hchacha20(key: &[u8; 32], nonce: &[u8; 16]) -> [u8; 32] {
    let mut state = [0u32; 16];
    state[0..4].copy_from_slice(&CHACHA20_CONSTANTS);

    for i in 0..8 {
        state[4 + i] = u32::from_le_bytes(key[i * 4..(i + 1) * 4].try_into().unwrap());
    }

    for i in 0..4 {
        state[12 + i] = u32::from_le_bytes(nonce[i * 4..(i + 1) * 4].try_into().unwrap());
    }

    chacha20_rounds(&mut state);

    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 4..(i + 1) * 4].copy_from_slice(&state[i].to_le_bytes());
    }
    for i in 0..4 {
        out[16 + i * 4..16 + (i + 1) * 4].copy_from_slice(&state[12 + i].to_le_bytes());
    }
    out
}

/// Constant-time Poly1305 MAC authenticator (RFC 8439).
#[derive(Clone, Debug)]
pub struct Poly1305 {
    r0: u64,
    r1: u64,
    s1: u64,
    h0: u64,
    h1: u64,
    h2: u64,
    key_s: [u8; 16],
    buf: [u8; 16],
    buf_len: usize,
}

impl Poly1305 {
    /// Initialize a Poly1305 instance with a 32-byte one-time key (r: 16 bytes, s: 16 bytes).
    pub fn new(key: &[u8; 32]) -> Self {
        let r0 = u64::from_le_bytes(key[0..8].try_into().unwrap()) & 0x0ffffffc0fffffff;
        let r1 = u64::from_le_bytes(key[8..16].try_into().unwrap()) & 0x0ffffffc0ffffffc;
        let s1 = r1 + (r1 >> 2);
        let mut key_s = [0u8; 16];
        key_s.copy_from_slice(&key[16..32]);

        Self {
            r0,
            r1,
            s1,
            h0: 0,
            h1: 0,
            h2: 0,
            key_s,
            buf: [0u8; 16],
            buf_len: 0,
        }
    }

    #[inline]
    fn process_block(&mut self, block: &[u8; 16], padbit: u64) {
        let m0 = u64::from_le_bytes(block[0..8].try_into().unwrap());
        let m1 = u64::from_le_bytes(block[8..16].try_into().unwrap());

        // h += m
        let d0_add = (self.h0 as u128) + (m0 as u128);
        self.h0 = d0_add as u64;
        let d1_add = (self.h1 as u128) + (m1 as u128) + (d0_add >> 64);
        self.h1 = d1_add as u64;
        self.h2 += ((d1_add >> 64) as u64) + padbit;

        // h *= r % p
        let d0 = (self.h0 as u128) * (self.r0 as u128) + (self.h1 as u128) * (self.s1 as u128);
        let mut d1 = (self.h0 as u128) * (self.r1 as u128)
            + (self.h1 as u128) * (self.r0 as u128)
            + (self.h2 as u128) * (self.s1 as u128);
        self.h2 *= self.r0;

        // last reduction step:
        // h2:h0 = h2<<128 + d1<<64 + d0
        self.h0 = d0 as u64;
        d1 += d0 >> 64;
        self.h1 = d1 as u64;
        self.h2 += (d1 >> 64) as u64;

        // (h2:h0 += (h2:h0 >> 130) * 5) %= 2^130
        let c = (self.h2 >> 2) + (self.h2 & !3);
        self.h2 &= 3;

        let (new_h0, carry0) = self.h0.overflowing_add(c);
        self.h0 = new_h0;
        let (new_h1, carry1) = self.h1.overflowing_add(carry0 as u64);
        self.h1 = new_h1;
        self.h2 += carry1 as u64;
    }

    /// Update the MAC with arbitrary length data.
    pub fn update(&mut self, mut data: &[u8]) {
        if self.buf_len > 0 {
            let to_copy = (16 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + to_copy].copy_from_slice(&data[..to_copy]);
            self.buf_len += to_copy;
            data = &data[to_copy..];

            if self.buf_len == 16 {
                let block = self.buf;
                self.process_block(&block, 1);
                self.buf_len = 0;
            }
        }

        let mut chunks = data.chunks_exact(16);
        for chunk in &mut chunks {
            let block: &[u8; 16] = chunk.try_into().unwrap();
            self.process_block(block, 1);
        }

        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buf[..remainder.len()].copy_from_slice(remainder);
            self.buf_len = remainder.len();
        }
    }

    /// Finalize the MAC in constant time and produce the 16-byte authentication tag.
    pub fn finalize(mut self) -> [u8; 16] {
        if self.buf_len > 0 {
            self.buf[self.buf_len] = 0x01;
            self.buf[self.buf_len + 1..].fill(0);
            let block = self.buf;
            self.process_block(&block, 0);
            self.buf_len = 0;
        }

        // compare to modulus by computing h + 5
        let t0 = (self.h0 as u128) + 5;
        let g0 = t0 as u64;
        let t1 = (self.h1 as u128) + (t0 >> 64);
        let g1 = t1 as u64;
        let g2 = self.h2 + ((t1 >> 64) as u64);

        // if there was carry into 131st bit, mask is all 1s
        let mask = 0u64.wrapping_sub(g2 >> 2);
        let h0_final = (self.h0 & !mask) | (g0 & mask);
        let h1_final = (self.h1 & !mask) | (g1 & mask);

        // compute h + s (nonce/s is self.key_s)
        let s0 = u64::from_le_bytes(self.key_s[0..8].try_into().unwrap());
        let s1 = u64::from_le_bytes(self.key_s[8..16].try_into().unwrap());

        let t0 = (h0_final as u128) + (s0 as u128);
        let out0 = t0 as u64;
        let t1 = (h1_final as u128) + (s1 as u128) + (t0 >> 64);
        let out1 = t1 as u64;

        let mut mac = [0u8; 16];
        mac[0..8].copy_from_slice(&out0.to_le_bytes());
        mac[8..16].copy_from_slice(&out1.to_le_bytes());
        mac
    }
}

/// Compute the AEAD Poly1305 tag over AAD and ciphertext (RFC 8439 Section 2.8).
fn compute_aead_tag(poly_key: &[u8; 32], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let mut poly = Poly1305::new(poly_key);

    poly.update(aad);
    if aad.len() % 16 != 0 {
        let pad_len = 16 - (aad.len() % 16);
        poly.update(&[0u8; 16][..pad_len]);
    }

    poly.update(ciphertext);
    if ciphertext.len() % 16 != 0 {
        let pad_len = 16 - (ciphertext.len() % 16);
        poly.update(&[0u8; 16][..pad_len]);
    }

    let mut len_block = [0u8; 16];
    len_block[0..8].copy_from_slice(&(aad.len() as u64).to_le_bytes());
    len_block[8..16].copy_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    poly.update(&len_block);

    poly.finalize()
}

/// Encrypt data using RFC 8439 ChaCha20-Poly1305 AEAD.
pub fn chacha20_poly1305_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    plaintext: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    let poly_block = chacha20_block(key, 0, nonce);
    let poly_key: &[u8; 32] = poly_block[0..32].try_into().unwrap();

    let ciphertext = chacha20_xor(key, 1, nonce, plaintext);
    let tag = compute_aead_tag(poly_key, aad, &ciphertext);

    (ciphertext, tag)
}

/// Decrypt data using RFC 8439 ChaCha20-Poly1305 AEAD.
pub fn chacha20_poly1305_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    ciphertext: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, CryptoError> {
    let poly_block = chacha20_block(key, 0, nonce);
    let poly_key: &[u8; 32] = poly_block[0..32].try_into().unwrap();

    let expected_tag = compute_aead_tag(poly_key, aad, ciphertext);
    if !constant_time_eq(tag, &expected_tag) {
        return Err(CryptoError::AuthenticationFailed);
    }

    let plaintext = chacha20_xor(key, 1, nonce, ciphertext);
    Ok(plaintext)
}

/// Encrypt data using XChaCha20-Poly1305 AEAD with 24-byte nonce.
pub fn xchacha20_poly1305_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    aad: &[u8],
    plaintext: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    let subkey = hchacha20(key, nonce[0..16].try_into().unwrap());
    let mut subnonce = [0u8; 12];
    subnonce[4..12].copy_from_slice(&nonce[16..24]);

    chacha20_poly1305_encrypt(&subkey, &subnonce, aad, plaintext)
}

/// Decrypt data using XChaCha20-Poly1305 AEAD with 24-byte nonce.
pub fn xchacha20_poly1305_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    aad: &[u8],
    ciphertext: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, CryptoError> {
    let subkey = hchacha20(key, nonce[0..16].try_into().unwrap());
    let mut subnonce = [0u8; 12];
    subnonce[4..12].copy_from_slice(&nonce[16..24]);

    chacha20_poly1305_decrypt(&subkey, &subnonce, aad, ciphertext, tag)
}

/// Cipher for Discord Voice RTP packets using `aead_xchacha20_poly1305_rtpsize`.
#[derive(Clone, Debug)]
pub struct VoicePacketCipher {
    key: [u8; 32],
}

impl VoicePacketCipher {
    /// Create a new cipher from the 32-byte secret key provided in `SessionDescription`.
    pub const fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Access the underlying 32-byte secret key.
    pub const fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Encrypt an Opus audio payload into an `rtpsize` RTP packet.
    ///
    /// Constructs the 12-byte RTP header as AAD, pads the 4-byte big-endian
    /// `nonce_seq` to a 24-byte XChaCha20 nonce (`nonce[0..4] = nonce_seq.to_be_bytes()`, `nonce[4..24] = 0`),
    /// encrypts `opus_payload` with XChaCha20-Poly1305 (appending the 16-byte tag),
    /// and appends the 4-byte `nonce_seq` trailer as required by Discord's `rtpsize` format.
    pub fn encrypt_rtp(&self, header: &RtpHeader, opus_payload: &[u8], nonce_seq: u32) -> Vec<u8> {
        let header_bytes = header.to_bytes();
        let mut nonce = [0u8; 24];
        nonce[0..4].copy_from_slice(&nonce_seq.to_be_bytes());

        let (ciphertext, tag) =
            xchacha20_poly1305_encrypt(&self.key, &nonce, &header_bytes, opus_payload);

        let mut packet =
            Vec::with_capacity(header_bytes.len() + ciphertext.len() + tag.len() + 4);
        packet.extend_from_slice(&header_bytes);
        packet.extend_from_slice(&ciphertext);
        packet.extend_from_slice(&tag);
        packet.extend_from_slice(&nonce_seq.to_be_bytes());
        packet
    }

    /// Decrypt an `rtpsize` RTP packet and return the parsed header and Opus audio frame.
    ///
    /// Parses the RTP header (including CSRC list if `cc > 0` and header extensions
    /// if the extension bit is set, which `rtpsize` includes in the AAD),
    /// extracts the trailing 4-byte nonce and 16-byte tag, verifies the Poly1305 MAC
    /// in constant time, and returns the decrypted Opus frame.
    pub fn decrypt_rtp(&self, packet: &[u8]) -> Result<(RtpHeader, Vec<u8>), CryptoError> {
        if packet.len() < RTP_HEADER_LEN + 16 + 4 {
            return Err(CryptoError::PacketTooShort {
                expected: RTP_HEADER_LEN + 16 + 4,
                actual: packet.len(),
            });
        }

        let header = RtpHeader::parse(&packet[..RTP_HEADER_LEN])?;

        let cc = (packet[0] & 0x0F) as usize;
        let mut header_len = RTP_HEADER_LEN + cc * 4;

        if packet.len() < header_len + 16 + 4 {
            return Err(CryptoError::PacketTooShort {
                expected: header_len + 16 + 4,
                actual: packet.len(),
            });
        }

        let has_extension = (packet[0] & 0x10) != 0;
        if has_extension {
            if packet.len() < header_len + 4 + 16 + 4 {
                return Err(CryptoError::PacketTooShort {
                    expected: header_len + 4 + 16 + 4,
                    actual: packet.len(),
                });
            }
            let ext_len_words =
                u16::from_be_bytes([packet[header_len + 2], packet[header_len + 3]]) as usize;
            let ext_bytes = 4 + ext_len_words * 4;
            header_len += ext_bytes;
            if packet.len() < header_len + 16 + 4 {
                return Err(CryptoError::PacketTooShort {
                    expected: header_len + 16 + 4,
                    actual: packet.len(),
                });
            }
        }

        let aad = &packet[..header_len];

        let nonce_start = packet.len() - 4;
        let nonce_seq = u32::from_be_bytes(packet[nonce_start..].try_into().unwrap());

        let mut nonce = [0u8; 24];
        nonce[0..4].copy_from_slice(&nonce_seq.to_be_bytes());

        let tag_start = nonce_start - 16;
        let tag: [u8; 16] = packet[tag_start..nonce_start].try_into().unwrap();

        let ciphertext = &packet[header_len..tag_start];

        let plaintext = xchacha20_poly1305_decrypt(&self.key, &nonce, aad, ciphertext, &tag)?;

        Ok((header, plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc8439_poly1305_test_vector() {
        let key = [
            0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52, 0xfe, 0x42, 0xd5,
            0x06, 0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d, 0xb2, 0xfd, 0x4a, 0xbf, 0xf6, 0xaf,
            0x41, 0x49, 0xf5, 0x1b,
        ];
        let msg = b"Cryptographic Forum Research Group";
        let expected_tag = [
            0xa8, 0x06, 0x1d, 0xc1, 0x30, 0x51, 0x36, 0xc6, 0xc2, 0x2b, 0x8b, 0xaf, 0x0c, 0x01,
            0x27, 0xa9,
        ];

        let mut poly = Poly1305::new(&key);
        poly.update(msg);
        let tag = poly.finalize();
        assert_eq!(tag, expected_tag);
    }

    #[test]
    fn hchacha20_test_vector() {
        // draft-irtf-cfrg-xchacha Section 2.2.1
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let nonce = [
            0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00, 0x31, 0x41,
            0x59, 0x27,
        ];
        let expected_out = [
            0x82, 0x41, 0x3b, 0x42, 0x27, 0xb2, 0x7b, 0xfe, 0xd3, 0x0e, 0x42, 0x50, 0x8a, 0x87,
            0x7d, 0x73, 0xa0, 0xf9, 0xe4, 0xd5, 0x8a, 0x74, 0xa8, 0x53, 0xc1, 0x2e, 0xc4, 0x13,
            0x26, 0xd3, 0xec, 0xdc,
        ];

        let out = hchacha20(&key, &nonce);
        assert_eq!(out, expected_out);
    }

    #[test]
    fn rfc8439_chacha20_poly1305_test_vector() {
        let key = [
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d,
            0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
            0x9c, 0x9d, 0x9e, 0x9f,
        ];
        let nonce = [
            0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47,
        ];
        let aad = [
            0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
        ];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let (ciphertext, tag) = chacha20_poly1305_encrypt(&key, &nonce, &aad, plaintext);
        let expected_tag = [
            0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a, 0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60,
            0x06, 0x91,
        ];
        assert_eq!(tag, expected_tag);

        let decrypted = chacha20_poly1305_decrypt(&key, &nonce, &aad, &ciphertext, &tag).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn xchacha20_poly1305_aead_test_vector() {
        // draft-irtf-cfrg-xchacha Appendix A.3.1
        let key = [
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d,
            0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b,
            0x9c, 0x9d, 0x9e, 0x9f,
        ];
        let nonce = [
            0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d,
            0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57,
        ];
        let aad = [
            0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
        ];
        let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";

        let expected_tag = [
            0xc0, 0x87, 0x59, 0x24, 0xc1, 0xc7, 0x98, 0x79, 0x47, 0xde, 0xaf, 0xd8, 0x78, 0x0a,
            0xcf, 0x49,
        ];

        let (ciphertext, tag) = xchacha20_poly1305_encrypt(&key, &nonce, &aad, plaintext);
        assert_eq!(tag, expected_tag);

        let decrypted = xchacha20_poly1305_decrypt(&key, &nonce, &aad, &ciphertext, &tag).unwrap();
        assert_eq!(&decrypted, plaintext);
    }

    #[test]
    fn voice_packet_cipher_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let cipher = VoicePacketCipher::new(key);
        let header = RtpHeader::new(1, 960, 0x11223344);
        let opus_payload = vec![0xF8, 0xFF, 0xFE, 0x01, 0x02, 0x03];
        let nonce_seq = 1001;

        let packet = cipher.encrypt_rtp(&header, &opus_payload, nonce_seq);
        assert_eq!(packet.len(), RTP_HEADER_LEN + opus_payload.len() + 16 + 4);

        let (dec_header, dec_payload) = cipher.decrypt_rtp(&packet).unwrap();
        assert_eq!(dec_header, header);
        assert_eq!(dec_payload, opus_payload);
    }

    #[test]
    fn voice_packet_cipher_tampering_fails() {
        let key = [0x55u8; 32];
        let cipher = VoicePacketCipher::new(key);
        let header = RtpHeader::new(5, 4800, 0xdeadbeef);
        let opus_payload = b"test audio frame";
        let nonce_seq = 42;

        let mut packet = cipher.encrypt_rtp(&header, opus_payload, nonce_seq);

        // Tamper with ciphertext
        packet[15] ^= 0x01;
        let res = cipher.decrypt_rtp(&packet);
        assert_eq!(res, Err(CryptoError::AuthenticationFailed));

        // Restore and tamper with header (AAD)
        packet[15] ^= 0x01;
        packet[2] ^= 0x01;
        let res = cipher.decrypt_rtp(&packet);
        assert_eq!(res, Err(CryptoError::AuthenticationFailed));

        // Restore and tamper with tag
        packet[2] ^= 0x01;
        let tag_idx = packet.len() - 10;
        packet[tag_idx] ^= 0x01;
        let res = cipher.decrypt_rtp(&packet);
        assert_eq!(res, Err(CryptoError::AuthenticationFailed));

        // Restore and tamper with trailing nonce
        packet[tag_idx] ^= 0x01;
        let nonce_idx = packet.len() - 2;
        packet[nonce_idx] ^= 0x01;
        let res = cipher.decrypt_rtp(&packet);
        assert_eq!(res, Err(CryptoError::AuthenticationFailed));
    }

    #[test]
    fn voice_packet_cipher_extended_header_csrc_and_extension() {
        let key = [0x77u8; 32];
        let cipher = VoicePacketCipher::new(key);

        // Header with extension bit set (0x90 instead of 0x80) and 1 CSRC (CC=1 -> 0x91)
        let ssrc: u32 = 0x99887766;
        let csrc: u32 = 0x11223344;
        let mut raw_header = Vec::new();
        raw_header.push(0x91); // V=2, X=1, CC=1
        raw_header.push(0x78); // Payload type 120
        raw_header.extend_from_slice(&10u16.to_be_bytes()); // Sequence 10
        raw_header.extend_from_slice(&9600u32.to_be_bytes()); // Timestamp 9600
        raw_header.extend_from_slice(&ssrc.to_be_bytes()); // SSRC
        raw_header.extend_from_slice(&csrc.to_be_bytes()); // 1 CSRC (4 bytes)

        // RTP header extension (RFC 3550): profile (2 bytes), length in words (2 bytes), then words
        let profile: u16 = 0xBEDE; // One-byte header extension profile
        let ext_words: u16 = 1; // 1 32-bit word = 4 bytes
        raw_header.extend_from_slice(&profile.to_be_bytes());
        raw_header.extend_from_slice(&ext_words.to_be_bytes());
        raw_header.extend_from_slice(&[0x10, 0x01, 0x00, 0x00]); // 4 bytes of extension data

        let opus_payload = b"extended rtp opus audio payload";
        let nonce_seq: u32 = 2024;
        let mut nonce = [0u8; 24];
        nonce[0..4].copy_from_slice(&nonce_seq.to_be_bytes());

        // Encrypt with raw extended header as AAD
        let (ciphertext, tag) =
            xchacha20_poly1305_encrypt(&cipher.key, &nonce, &raw_header, opus_payload);

        let mut packet = raw_header;
        packet.extend_from_slice(&ciphertext);
        packet.extend_from_slice(&tag);
        packet.extend_from_slice(&nonce_seq.to_be_bytes());

        let (header, decrypted) = cipher.decrypt_rtp(&packet).unwrap();
        assert_eq!(header.version, 0x91);
        assert_eq!(header.sequence, 10);
        assert_eq!(header.timestamp, 9600);
        assert_eq!(header.ssrc, ssrc);
        assert_eq!(decrypted, opus_payload);
    }
}
