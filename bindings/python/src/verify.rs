use rift_http::verify;

/// Verifies Discord interaction webhook Ed25519 signature.
pub fn verify_interaction(
    public_key_hex: &str,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
) -> bool {
    verify::verify_interaction(public_key_hex, signature_hex, timestamp, body).is_ok()
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyfunction(name = "verify_interaction")]
    pub fn py_verify_interaction(
        public_key_hex: &str,
        signature_hex: &str,
        timestamp: &str,
        body: &[u8],
    ) -> bool {
        verify_interaction(public_key_hex, signature_hex, timestamp, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_verify_interaction() {
        assert!(!verify_interaction("bad", "bad", "123", b"test"));
    }
}
