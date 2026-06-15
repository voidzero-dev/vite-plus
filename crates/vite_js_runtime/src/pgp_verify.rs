//! PGP signature verification for runtime checksum files.
//!
//! Node.js signs its `SHASUMS256.txt` with the PGP key of the releaser who cut
//! the release (see <https://github.com/nodejs/node#verifying-binaries>). This
//! module verifies the clearsigned `SHASUMS256.txt.asc` against an embedded copy
//! of the Node.js release signing keys before any checksum from it is trusted,
//! so a tampered or attacker-controlled SHASUMS file cannot pass off a malicious
//! archive whose hash it also controls.
//!
//! The trusted keys are vendored from the [`nodejs/release-keys`] repository.
//! They currently only cover Node.js; when another runtime gains signature
//! support, the embedded keyring and [`verify_signed_shasums`] should be
//! generalized to take the relevant keys.
//!
//! [`nodejs/release-keys`]: https://github.com/nodejs/release-keys

use std::sync::LazyLock;

use pgp::composed::{CleartextSignedMessage, Deserializable, SignedPublicKey};
use vite_str::Str;

use crate::Error;

/// ASCII-armored Node.js release signing keys (current and historical),
/// concatenated from <https://github.com/nodejs/release-keys/tree/main/keys>.
const NODE_RELEASE_KEYS_ARMOR: &str = include_str!("assets/node-release-keys.asc");

/// Verify a clearsigned `SHASUMS256.txt.asc` against the Node.js release keys.
///
/// On success returns the verified plaintext (the `SHASUMS256.txt` content that
/// was actually signed), which the caller then parses for the archive hash.
///
/// Runs on a blocking thread because parsing the keyring on first use and
/// verifying the signature are CPU-bound.
///
/// # Errors
///
/// Returns [`Error::SignatureVerificationFailed`] if the message cannot be
/// parsed or no embedded release key produced a valid signature.
pub async fn verify_signed_shasums(signed_armor: String, filename: &str) -> Result<String, Error> {
    let filename: Str = filename.into();
    tokio::task::spawn_blocking(move || {
        verify_clearsigned(&signed_armor, node_release_keys()).map_err(|reason| {
            Error::SignatureVerificationFailed { file: filename, reason: reason.into() }
        })
    })
    .await?
}

/// Verify a clearsigned message against a set of trusted public keys.
///
/// Returns the verified, normalized plaintext on success. Each key is tried
/// against its primary key and every subkey, because Node.js releasers may sign
/// with a signing subkey rather than the primary key.
fn verify_clearsigned(
    signed_armor: &str,
    trusted_keys: &[SignedPublicKey],
) -> Result<String, String> {
    let (message, _headers) = CleartextSignedMessage::from_string(signed_armor)
        .map_err(|e| format!("failed to parse clearsigned message: {e}"))?;

    for key in trusted_keys {
        if message.verify(key).is_ok() {
            return Ok(message.signed_text());
        }
        for subkey in &key.public_subkeys {
            if message.verify(subkey).is_ok() {
                return Ok(message.signed_text());
            }
        }
    }

    Err("signature does not match any trusted Node.js release key".to_string())
}

/// Lazily parsed embedded Node.js release keys.
fn node_release_keys() -> &'static [SignedPublicKey] {
    static KEYS: LazyLock<Vec<SignedPublicKey>> =
        LazyLock::new(|| parse_public_keys(NODE_RELEASE_KEYS_ARMOR));
    &KEYS
}

/// Parse every ASCII-armored public key block from a concatenated keyring.
///
/// Keys that fail to parse (e.g. unsupported legacy algorithms) are skipped so a
/// single bad block cannot disable verification for the remaining keys.
fn parse_public_keys(armored: &str) -> Vec<SignedPublicKey> {
    let mut keys = Vec::new();
    for block in split_armored_blocks(armored) {
        match SignedPublicKey::from_string(&block) {
            Ok((key, _)) => keys.push(key),
            Err(e) => tracing::debug!("skipping unparsable release key: {e}"),
        }
    }
    keys
}

/// Split a string holding multiple concatenated ASCII-armored public key blocks
/// into the individual `-----BEGIN/END PGP PUBLIC KEY BLOCK-----` sections.
fn split_armored_blocks(input: &str) -> Vec<String> {
    const BEGIN: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----";
    const END: &str = "-----END PGP PUBLIC KEY BLOCK-----";

    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in input.lines() {
        if line.starts_with(BEGIN) {
            current = Some(String::new());
        }
        if let Some(buf) = current.as_mut() {
            buf.push_str(line);
            buf.push('\n');
            if line.starts_with(END) {
                blocks.push(current.take().unwrap());
            }
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real, untampered `SHASUMS256.txt.asc` from Node.js v22.13.1.
    const FIXTURE_SIGNED: &str = include_str!("assets/test/SHASUMS256-v22.13.1.txt.asc");

    #[test]
    fn embedded_keys_parse() {
        let keys = node_release_keys();
        // The keyring should parse cleanly enough to cover the current releasers.
        assert!(keys.len() >= 8, "expected at least 8 parsed release keys, got {}", keys.len());
    }

    #[test]
    fn split_armored_blocks_finds_every_key() {
        let blocks = split_armored_blocks(NODE_RELEASE_KEYS_ARMOR);
        assert_eq!(blocks.len(), 28, "expected 28 vendored release key blocks");
        assert!(blocks.iter().all(|b| b.contains("-----END PGP PUBLIC KEY BLOCK-----")));
    }

    #[test]
    fn verifies_genuine_signed_shasums() {
        let content =
            verify_clearsigned(FIXTURE_SIGNED, node_release_keys()).expect("should verify");
        // The verified content is the SHASUMS256.txt with the real checksums.
        assert!(content.contains("node-v22.13.1-linux-x64.tar.gz"));
        assert!(content.contains(
            "666148b9fe0c7e1301cc1b029e33a45e9e4a893f68d2d2bb1cc88a931a88a004  \
             node-v22.13.1-linux-x64.tar.gz"
        ));
    }

    #[test]
    fn rejects_tampered_content() {
        // Flip one hex digit in a checksum: the body no longer matches the signature.
        let tampered = FIXTURE_SIGNED.replacen(
            "666148b9fe0c7e1301cc1b029e33a45e9e4a893f68d2d2bb1cc88a931a88a004",
            "766148b9fe0c7e1301cc1b029e33a45e9e4a893f68d2d2bb1cc88a931a88a004",
            1,
        );
        assert_ne!(tampered, FIXTURE_SIGNED, "fixture should contain the target checksum");
        assert!(verify_clearsigned(&tampered, node_release_keys()).is_err());
    }

    #[test]
    fn rejects_signature_from_untrusted_key() {
        // With an empty trusted keyring, even a genuine signature must be rejected.
        assert!(verify_clearsigned(FIXTURE_SIGNED, &[]).is_err());
    }

    #[test]
    fn rejects_non_clearsigned_input() {
        assert!(verify_clearsigned("not a pgp message", node_release_keys()).is_err());
    }

    #[test]
    fn every_vendored_key_parses() {
        // All vendored release keys must parse; a key that silently fails to
        // parse would create a coverage gap for versions it signed.
        assert_eq!(
            node_release_keys().len(),
            split_armored_blocks(NODE_RELEASE_KEYS_ARMOR).len(),
            "every vendored release key block should parse"
        );
    }
}
