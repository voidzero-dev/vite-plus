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
//! # Trust model and limitations
//!
//! The trust boundary is the curated set of Node.js release keys plus honoring
//! key/subkey revocation, which is what `gpgv` against the release keyring
//! provides. Three properties follow from that model and are intentional:
//!
//! - **Key expiry is not enforced.** `gpgv` treats an expired key as advisory
//!   and still reports its signatures as good (verified: it exits 0 for a real
//!   release signed shortly after its key's expiry, e.g. `node-v16.20.0`).
//!   Enforcing expiry would reject such legitimate releases. It would also not
//!   add protection against a leaked key, because the attacker controls the
//!   signature's self-asserted creation time and can backdate a forgery. The
//!   real protection against a compromised key is its revocation (enforced
//!   here) plus keeping the vendored keyring current.
//! - **The keyring is a vendored snapshot.** Node version resolution is live, so
//!   a release signed by a releaser key added after this snapshot was built has
//!   no matching trusted key and fails closed on the official source until the
//!   keyring (and `vite-plus`) is updated. The current releasers' keys are
//!   included, so this only affects brand-new releasers; the keyring must be
//!   refreshed from [`nodejs/release-keys`] as the releaser set changes.
//!
//! [`nodejs/release-keys`]: https://github.com/nodejs/release-keys

use std::sync::LazyLock;

use pgp::{
    composed::{CleartextSignedMessage, Deserializable, SignedPublicKey, SignedPublicSubKey},
    packet::SignatureType,
};
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
///
/// This mirrors `gpgv` against the Node release keyring: a cryptographically
/// valid signature from a trusted, non-revoked key is accepted. A subkey
/// signature additionally requires the subkey to be a validly-bound, signing-
/// capable subkey of the primary, so a leaked encryption-only subkey cannot be
/// used to sign. Key expiry is intentionally not enforced (see the module docs):
/// `gpgv` treats it as advisory and still reports such signatures good, so
/// enforcing it would reject legitimate releases signed near a key's expiry
/// while not stopping a backdating attacker.
fn verify_clearsigned(
    signed_armor: &str,
    trusted_keys: &[SignedPublicKey],
) -> Result<String, String> {
    let (message, _headers) = CleartextSignedMessage::from_string(signed_armor)
        .map_err(|e| format!("failed to parse clearsigned message: {e}"))?;

    for key in trusted_keys {
        // A revoked primary key (and, with it, all its subkeys) is never trusted.
        if primary_key_revoked(key) {
            continue;
        }

        // Primary-key signing path: a valid signature from a trusted, non-revoked
        // primary key is accepted, as gpgv trusts keys present in the keyring.
        if message.verify(key).is_ok() {
            return Ok(message.signed_text());
        }

        // Subkey signing path (some releasers sign with a signing subkey).
        for subkey in &key.public_subkeys {
            if message.verify(subkey).is_ok() && subkey_is_valid_signer(key, subkey) {
                return Ok(message.signed_text());
            }
        }
    }

    Err("signature does not match a trusted Node.js release key".to_string())
}

/// Whether the primary key carries a valid self-revocation certificate.
fn primary_key_revoked(key: &SignedPublicKey) -> bool {
    key.details
        .revocation_signatures
        .iter()
        .any(|revocation| revocation.verify_key(&key.primary_key).is_ok())
}

/// Whether `subkey` may legitimately sign on the primary's behalf: it must not
/// be revoked and must carry a signing-capable binding signature with a valid
/// embedded primary-key back-signature. rPGP's `verify` applies no subkey policy
/// itself, so without this a leaked encryption-only or revoked subkey would be
/// accepted as a signer.
fn subkey_is_valid_signer(key: &SignedPublicKey, subkey: &SignedPublicSubKey) -> bool {
    let primary = &key.primary_key;

    // Reject if a valid subkey revocation exists.
    let revoked = subkey
        .signatures
        .iter()
        .filter(|s| s.typ() == Some(SignatureType::SubkeyRevocation))
        .any(|s| s.verify_subkey_binding(primary, &subkey.key).is_ok());
    if revoked {
        return false;
    }

    // Require a signing-capable binding signature with a valid embedded
    // primary-key back-signature.
    subkey.signatures.iter().filter(|s| s.typ() == Some(SignatureType::SubkeyBinding)).any(
        |binding| {
            binding.key_flags().sign()
                && binding.verify_subkey_binding(primary, &subkey.key).is_ok()
                && binding.embedded_signature().is_some_and(|back| {
                    back.verify_primary_key_binding(&subkey.key, primary).is_ok()
                })
        },
    )
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

    /// A real `SHASUMS256.txt.asc` from Node.js v18.14.0, signed by a release
    /// key that has since expired. `gpgv` still verifies it, so we must too.
    const FIXTURE_EXPIRED_SIGNER: &str = include_str!("assets/test/SHASUMS256-v18.14.0.txt.asc");

    /// A real `SHASUMS256.txt.asc` from Node.js v20.18.0, whose signing key was
    /// re-certified after this release was signed. `gpgv` still verifies it.
    const FIXTURE_RECERTIFIED_SIGNER: &str =
        include_str!("assets/test/SHASUMS256-v20.18.0.txt.asc");

    /// A real `SHASUMS256.txt.asc` from Node.js v16.20.0, signed a few days
    /// *after* its signing key's expiry. `gpgv` reports this good (exit 0), so
    /// enforcing expiry would wrongly reject a legitimate release.
    const FIXTURE_EXPIRED_AT_SIGNING: &str =
        include_str!("assets/test/SHASUMS256-v16.20.0.txt.asc");

    #[test]
    fn embedded_keys_parse() {
        let keys = node_release_keys();
        // The keyring should parse cleanly enough to cover the current releasers.
        assert!(keys.len() >= 8, "expected at least 8 parsed release keys, got {}", keys.len());
    }

    #[test]
    fn split_armored_blocks_finds_every_key() {
        // Self-consistent against the vendored file rather than a fixed count, so
        // adding keys upstream needs no test edit. The floor stays at the current
        // 28 keys: nodejs/release-keys retains historical keys, so the set should
        // only grow; dropping below 28 means a regression worth catching.
        let begin_markers =
            NODE_RELEASE_KEYS_ARMOR.matches("-----BEGIN PGP PUBLIC KEY BLOCK-----").count();
        let blocks = split_armored_blocks(NODE_RELEASE_KEYS_ARMOR);
        assert_eq!(blocks.len(), begin_markers, "every BEGIN block should be captured");
        assert!(blocks.len() >= 28, "keyring unexpectedly small: {}", blocks.len());
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

    #[test]
    fn accepts_release_from_now_expired_key() {
        // The signing key has since expired, but gpgv still verifies the
        // signature, so we must too.
        let verified =
            verify_clearsigned(FIXTURE_EXPIRED_SIGNER, node_release_keys()).expect("should verify");
        assert!(verified.contains("node-v18.14.0-linux-x64.tar.gz"));
    }

    #[test]
    fn accepts_release_signed_before_key_was_recertified() {
        // The signing key's self-certification was refreshed after this release
        // was signed; it must still verify.
        let verified = verify_clearsigned(FIXTURE_RECERTIFIED_SIGNER, node_release_keys())
            .expect("should verify");
        assert!(verified.contains("node-v20.18.0-linux-x64.tar.gz"));
    }

    #[test]
    fn accepts_release_signed_after_key_expiry() {
        // Signed days after the key's expiry; gpgv reports it good, so enforcing
        // expiry would wrongly reject this legitimate release.
        let verified = verify_clearsigned(FIXTURE_EXPIRED_AT_SIGNING, node_release_keys())
            .expect("should verify");
        assert!(verified.contains("node-v16.20.0-linux-x64.tar.gz"));
    }
}
