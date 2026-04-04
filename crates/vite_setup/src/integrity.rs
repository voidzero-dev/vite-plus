//! Integrity verification for downloaded tarballs.
//!
//! Verifies SHA-512 integrity using the Subresource Integrity (SRI) format
//! that npm registries provide: `sha512-{base64}`.

use sha2::{Digest, Sha512};

use crate::error::Error;

/// Verify the integrity of data against an SRI hash.
///
/// Parses the SRI format `sha512-{base64}`, computes the SHA-512 hash
/// of the data, base64-encodes it, and compares.
pub fn verify_integrity(data: &[u8], expected_sri: &str) -> Result<(), Error> {
    let expected_b64 = expected_sri
        .strip_prefix("sha512-")
        .ok_or_else(|| Error::UnsupportedIntegrity(expected_sri.into()))?;

    let mut hasher = Sha512::new();
    hasher.update(data);
    let actual_b64 = base64_simd::STANDARD.encode_to_string(hasher.finalize());

    if actual_b64 != expected_b64 {
        return Err(Error::IntegrityMismatch {
            expected: expected_sri.into(),
            actual: format!("sha512-{actual_b64}").into(),
        });
    }

    tracing::debug!("Integrity verification successful");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_integrity_valid() {
        let data = b"Hello, World!";
        let mut hasher = Sha512::new();
        hasher.update(data);
        let hash = base64_simd::STANDARD.encode_to_string(hasher.finalize());
        let sri = format!("sha512-{hash}");

        assert!(verify_integrity(data, &sri).is_ok());
    }

    #[test]
    fn test_verify_integrity_mismatch() {
        let data = b"Hello, World!";
        let sri = "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        let err = verify_integrity(data, sri).unwrap_err();
        assert!(matches!(err, Error::IntegrityMismatch { .. }));
    }

    #[test]
    fn test_verify_integrity_unsupported_format() {
        let data = b"Hello, World!";
        let sri = "sha256-abc123";

        let err = verify_integrity(data, sri).unwrap_err();
        assert!(matches!(err, Error::UnsupportedIntegrity(_)));
    }

    #[test]
    fn test_verify_integrity_no_prefix() {
        let data = b"Hello, World!";
        let sri = "not-a-valid-sri";

        let err = verify_integrity(data, sri).unwrap_err();
        assert!(matches!(err, Error::UnsupportedIntegrity(_)));
    }
}
