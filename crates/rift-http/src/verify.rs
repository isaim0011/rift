//! Ed25519 signature verification for Discord serverless HTTP interactions.
//!
//! Discord delivers interactions to registered webhook endpoints via HTTP POST.
//! Each request carries `X-Signature-Ed25519` and `X-Signature-Timestamp` headers.
//! Endpoints must verify that the signature is valid over `timestamp || body`
//! using the application's Ed25519 public key before handling the payload.
//!
//! # Example
//!
//! ```
//! use rift_http::verify::{verify_interaction, VerifyError};
//!
//! # fn demo() -> Result<(), VerifyError> {
//! let public_key = "a".repeat(64); // 32-byte hex public key
//! let signature = "b".repeat(128); // 64-byte hex signature
//! let timestamp = "1700000000";
//! let body = br#"{"type":1}"#;
//!
//! // Returns Err(VerifyError::InvalidSignature) for invalid keys/signatures:
//! let _ = verify_interaction(&public_key, &signature, timestamp, body);
//! # Ok(()) }
//! ```

use std::fmt;

/// Standard Discord header carrying the hex-encoded Ed25519 signature.
pub const HEADER_SIGNATURE: &str = "X-Signature-Ed25519";

/// Standard Discord header carrying the Unix timestamp string.
pub const HEADER_TIMESTAMP: &str = "X-Signature-Timestamp";

/// Errors that can occur when verifying an HTTP interaction request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// Missing required signature header (`X-Signature-Ed25519`).
    MissingSignature,
    /// Missing required timestamp header (`X-Signature-Timestamp`).
    MissingTimestamp,
    /// Public key hex string is invalid or not 64 hex characters (32 bytes).
    InvalidPublicKey,
    /// Signature hex string is invalid or not 128 hex characters (64 bytes).
    InvalidSignatureHex,
    /// Timestamp cannot be parsed as a valid numeric Unix timestamp.
    InvalidTimestamp,
    /// Timestamp is outside the allowed freshness window.
    TimestampExpired {
        /// Timestamp provided in header (seconds).
        timestamp: u64,
        /// Current Unix time (seconds).
        now: u64,
        /// Maximum allowed difference (seconds).
        max_age: u64,
    },
    /// Cryptographic signature verification failed.
    InvalidSignature,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSignature => write!(f, "missing {HEADER_SIGNATURE} header"),
            Self::MissingTimestamp => write!(f, "missing {HEADER_TIMESTAMP} header"),
            Self::InvalidPublicKey => write!(f, "invalid ed25519 public key (expected 64 hex characters)"),
            Self::InvalidSignatureHex => write!(f, "invalid ed25519 signature (expected 128 hex characters)"),
            Self::InvalidTimestamp => write!(f, "invalid timestamp format"),
            Self::TimestampExpired { timestamp, now, max_age } => write!(
                f,
                "timestamp {timestamp} is expired (now: {now}, max skew: {max_age}s)"
            ),
            Self::InvalidSignature => write!(f, "cryptographic signature verification failed"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Pluggable cryptographic signature verifier for Ed25519.
pub trait Verifier {
    /// Verifies that `signature` is valid for `message` under `public_key`.
    fn verify(&self, public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool;
}

/// Default cryptographic verifier backed by `ed25519-dalek`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ed25519Verifier;

impl Verifier for Ed25519Verifier {
    fn verify(&self, public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
        use ed25519_dalek::{Signature, Verifier as DalekVerifier, VerifyingKey};
        let Ok(verifying_key) = VerifyingKey::from_bytes(public_key) else {
            return false;
        };
        let sig = Signature::from_bytes(signature);
        verifying_key.verify(message, &sig).is_ok()
    }
}

fn from_hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Decodes a 64-character hex string into a 32-byte array.
pub fn hex_decode_32(s: &str) -> Result<[u8; 32], VerifyError> {
    let s = s.trim();
    if s.len() != 64 {
        return Err(VerifyError::InvalidPublicKey);
    }
    let mut out = [0u8; 32];
    let bytes = s.as_bytes();
    for i in 0..32 {
        let hi = from_hex_digit(bytes[i * 2]).ok_or(VerifyError::InvalidPublicKey)?;
        let lo = from_hex_digit(bytes[i * 2 + 1]).ok_or(VerifyError::InvalidPublicKey)?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

/// Decodes a 128-character hex string into a 64-byte array.
pub fn hex_decode_64(s: &str) -> Result<[u8; 64], VerifyError> {
    let s = s.trim();
    if s.len() != 128 {
        return Err(VerifyError::InvalidSignatureHex);
    }
    let mut out = [0u8; 64];
    let bytes = s.as_bytes();
    for i in 0..64 {
        let hi = from_hex_digit(bytes[i * 2]).ok_or(VerifyError::InvalidSignatureHex)?;
        let lo = from_hex_digit(bytes[i * 2 + 1]).ok_or(VerifyError::InvalidSignatureHex)?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

/// Validates that a timestamp string is numeric and within `max_age_secs` of `now_unix_secs`.
pub fn validate_timestamp(timestamp_str: &str, now_unix_secs: u64, max_age_secs: u64) -> Result<u64, VerifyError> {
    let ts = timestamp_str.trim().parse::<u64>().map_err(|_| VerifyError::InvalidTimestamp)?;
    let diff = ts.abs_diff(now_unix_secs);
    if diff > max_age_secs {
        return Err(VerifyError::TimestampExpired {
            timestamp: ts,
            now: now_unix_secs,
            max_age: max_age_secs,
        });
    }
    Ok(ts)
}

/// Verifies a Discord serverless interaction HTTP request.
///
/// Concatenates `timestamp` and `body` (`timestamp || body`) and verifies the
/// Ed25519 signature against `public_key`.
pub fn verify_interaction(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), VerifyError> {
    verify_interaction_custom(&Ed25519Verifier, public_key_hex, signature_hex, timestamp, body)
}

/// Verifies a Discord serverless interaction HTTP request, including timestamp freshness.
pub fn verify_interaction_with_timestamp(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
    now_unix_secs: u64,
    max_age_secs: u64,
) -> Result<(), VerifyError> {
    validate_timestamp(timestamp, now_unix_secs, max_age_secs)?;
    verify_interaction(public_key_hex, signature_hex, timestamp, body)
}

/// Verifies a Discord serverless interaction using a custom pluggable verifier.
pub fn verify_interaction_custom<V: Verifier>(
    verifier: &V,
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), VerifyError> {
    let pk_bytes = hex_decode_32(public_key_hex)?;
    let sig_bytes = hex_decode_64(signature_hex)?;

    let mut msg = Vec::with_capacity(timestamp.len() + body.len());
    msg.extend_from_slice(timestamp.as_bytes());
    msg.extend_from_slice(body);

    if verifier.verify(&pk_bytes, &msg, &sig_bytes) {
        Ok(())
    } else {
        Err(VerifyError::InvalidSignature)
    }
}

/// Verifies a signature over raw 32-byte public key and 64-byte signature.
pub fn verify_interaction_raw(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    timestamp: &[u8],
    body: &[u8],
) -> Result<(), VerifyError> {
    let mut msg = Vec::with_capacity(timestamp.len() + body.len());
    msg.extend_from_slice(timestamp);
    msg.extend_from_slice(body);
    if Ed25519Verifier.verify(public_key, &msg, signature) {
        Ok(())
    } else {
        Err(VerifyError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    #[test]
    fn validates_valid_ed25519_signature() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();
        let pk_hex = hex::encode(verifying_key.as_bytes());

        let timestamp = "1700000000";
        let body = br#"{"type":1}"#;

        let mut msg = Vec::new();
        msg.extend_from_slice(timestamp.as_bytes());
        msg.extend_from_slice(body);

        let sig = signing_key.sign(&msg);
        let sig_hex = hex::encode(&sig.to_bytes());

        assert!(verify_interaction(&pk_hex, &sig_hex, timestamp, body).is_ok());

        // Tampered body should fail
        assert_eq!(
            verify_interaction(&pk_hex, &sig_hex, timestamp, b"tampered body"),
            Err(VerifyError::InvalidSignature)
        );

        // Tampered timestamp should fail
        assert_eq!(
            verify_interaction(&pk_hex, &sig_hex, "1700000001", body),
            Err(VerifyError::InvalidSignature)
        );
    }

    #[test]
    fn validates_timestamp_freshness() {
        let now = 1_700_000_100;
        let valid_ts = "1700000090"; // 10s ago
        assert_eq!(validate_timestamp(valid_ts, now, 60).unwrap(), 1_700_000_090);

        let expired_ts = "1700000000"; // 100s ago, max 60s
        assert!(matches!(
            validate_timestamp(expired_ts, now, 60),
            Err(VerifyError::TimestampExpired { .. })
        ));

        let bad_ts = "not-a-number";
        assert_eq!(validate_timestamp(bad_ts, now, 60), Err(VerifyError::InvalidTimestamp));
    }

    #[test]
    fn rejects_invalid_hex() {
        let short_pk = "abcdef";
        let sig = "00".repeat(64);
        assert_eq!(
            verify_interaction(short_pk, &sig, "1700000000", b"body"),
            Err(VerifyError::InvalidPublicKey)
        );

        let pk = "00".repeat(32);
        let short_sig = "abcdef";
        assert_eq!(
            verify_interaction(&pk, short_sig, "1700000000", b"body"),
            Err(VerifyError::InvalidSignatureHex)
        );
    }

    // Simple hex encoder for unit tests
    mod hex {
        pub fn encode(data: &[u8]) -> String {
            const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
            let mut s = String::with_capacity(data.len() * 2);
            for &b in data {
                s.push(HEX_CHARS[(b >> 4) as usize] as char);
                s.push(HEX_CHARS[(b & 0x0f) as usize] as char);
            }
            s
        }
    }
}
