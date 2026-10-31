use rift_http::verify;

/// Verifies Discord Ed25519 webhook signatures over `timestamp + body`.
/// Returns `true` if valid, `false` otherwise.
pub fn verify_interaction(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
) -> bool {
    verify::verify_interaction(public_key_hex, signature_hex, timestamp, body).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_interaction_invalid() {
        let pk = "a".repeat(64);
        let sig = "b".repeat(128);
        let ts = "1700000000";
        let body = b"{\"type\":1}";
        // Should safely fail with false, not panic or error
        assert!(!verify_interaction(&pk, &sig, ts, body));
    }

    #[test]
    fn test_verify_interaction_malformed_hex() {
        assert!(!verify_interaction("not-hex", "not-hex", "1700000000", b"{}"));
    }
}
