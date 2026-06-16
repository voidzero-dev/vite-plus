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
//! provides. Two limitations follow from that model and are intentional:
//!
//! - **Expiry is checked against the signature's self-asserted creation time**
//!   (as `gpgv` does). An attacker who holds a leaked release private key also
//!   controls that timestamp, so the expiry check does not by itself stop a
//!   key-compromise attacker who backdates a forgery; it is defense-in-depth.
//!   The real protection against a compromised key is its revocation, which is
//!   enforced here, plus keeping the vendored keyring current.
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
    packet::{Signature, SignatureType},
    types::{KeyDetails, Timestamp},
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
/// A raw cryptographic match is not sufficient: the keyring is intentionally
/// historical and rPGP's low-level `verify` does not apply OpenPGP key policy.
/// So a match is only accepted when the signing key/subkey is not revoked, is a
/// signing-capable self-issued key/binding, and the signature was created before
/// that key/subkey expired. This mirrors `gpgv` against the release keyring.
///
/// Note the expiry check uses the signature's self-asserted creation time, so it
/// is defense-in-depth rather than a defense against a leaked key (see the module
/// docs); the trust boundary is the curated keyring plus revocation.
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

        // Primary-key signing path.
        if let Ok(signature) = message.verify(key)
            && let Some(signed_at) = signature_time(signature)
            && primary_signature_valid(key, signed_at)
        {
            return Ok(message.signed_text());
        }

        // Subkey signing path (some releasers sign with a signing subkey).
        for subkey in &key.public_subkeys {
            if let Ok(signature) = message.verify(subkey)
                && let Some(signed_at) = signature_time(signature)
                && subkey_signature_valid(key, subkey, signed_at)
            {
                return Ok(message.signed_text());
            }
        }
    }

    Err("signature does not match a valid, unexpired Node.js release key".to_string())
}

/// Unix-seconds creation time of a signature, if present. A signature with no
/// creation time cannot be checked against key validity and is rejected.
fn signature_time(signature: &Signature) -> Option<u64> {
    signature.created().map(|t| u64::from(t.as_secs()))
}

/// Whether the primary key carries a valid self-revocation certificate.
fn primary_key_revoked(key: &SignedPublicKey) -> bool {
    key.details
        .revocation_signatures
        .iter()
        .any(|revocation| revocation.verify_key(&key.primary_key).is_ok())
}

/// Whether a primary-key signature made at `signed_at` should be trusted. Only
/// the key's own self-issued certifications/direct signatures define its policy
/// (a third-party certification must not supply expiry or capability), the key
/// must not be marked certify-only, and the signature must predate the key's
/// expiry. The current (latest) self-signature defines the effective expiry, as
/// `gpgv` does: a key re-certified after a release (e.g. to extend its expiry)
/// must still verify that release's older signature.
fn primary_signature_valid(key: &SignedPublicKey, signed_at: u64) -> bool {
    let self_signatures = key
        .details
        .direct_signatures
        .iter()
        .chain(key.details.users.iter().flat_map(|user| user.signatures.iter()))
        .filter(|sig| is_self_issued(sig, key));

    let Some(self_sig) = latest_self_signature(self_signatures) else {
        // No self-signature at all: the key's policy can't be established.
        return false;
    };

    // Reject a primary whose own self-signature declares it certify-only (i.e.
    // explicitly capable of certifying but not of signing data).
    let flags = self_sig.key_flags();
    if flags.certify() && !flags.sign() {
        return false;
    }

    within_validity(signed_at, expiry_instant(key.created_at(), self_sig))
}

/// Whether `sig` was issued by `key`'s own primary key (a self-signature),
/// identified by issuer fingerprint or key id. The certifications come from the
/// trusted vendored keyring, so an identity match is sufficient to exclude
/// third-party certifications from defining the key's policy.
fn is_self_issued(sig: &Signature, key: &SignedPublicKey) -> bool {
    let fingerprint = key.fingerprint();
    let key_id = key.legacy_key_id();
    sig.issuer_fingerprint().iter().any(|f| **f == fingerprint)
        || sig.issuer_key_id().iter().any(|k| **k == key_id)
}

/// Whether a subkey signature made at `signed_at` should be trusted: the subkey
/// must not be revoked and must carry a signing-capable binding signature (with
/// a valid embedded primary-key back-signature) that has not expired. The
/// current (latest) binding signature governs capability and expiry. rPGP's
/// `verify` applies none of this subkey policy itself, so a non-signing or
/// revoked subkey would otherwise be accepted.
fn subkey_signature_valid(
    key: &SignedPublicKey,
    subkey: &SignedPublicSubKey,
    signed_at: u64,
) -> bool {
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

    // The current (latest) binding signature governs capability and expiry.
    let bindings =
        subkey.signatures.iter().filter(|s| s.typ() == Some(SignatureType::SubkeyBinding));
    let Some(binding) = latest_self_signature(bindings) else {
        return false;
    };

    binding.key_flags().sign()
        && binding.verify_subkey_binding(primary, &subkey.key).is_ok()
        && binding
            .embedded_signature()
            .is_some_and(|back| back.verify_primary_key_binding(&subkey.key, primary).is_ok())
        && within_validity(signed_at, expiry_instant(subkey.created_at(), binding))
}

/// The current self/binding signature: the most recently created one. This is
/// the signature that defines the key's effective policy (capability/expiry),
/// matching `gpgv`. It is intentionally not filtered by the message signature's
/// timestamp, because a key may be re-certified after it signs a release.
fn latest_self_signature<'a>(
    signatures: impl Iterator<Item = &'a Signature>,
) -> Option<&'a Signature> {
    signatures.max_by_key(|s| s.created().map_or(0, |t| t.as_secs()))
}

/// Expiration instant (unix seconds) declared by a self/binding signature,
/// relative to the key's creation time. A `KeyExpirationTime` of 0 means the
/// key does not expire.
fn expiry_instant(key_created_at: Timestamp, self_sig: &Signature) -> Option<u64> {
    self_sig
        .key_expiration_time()
        .map(|duration| u64::from(duration.as_secs()))
        .filter(|secs| *secs != 0)
        .map(|secs| u64::from(key_created_at.as_secs()) + secs)
}

/// Pure validity check: a signature made at `signed_at` (unix seconds) is valid
/// when the key has no expiration, or the signature predates the expiry.
const fn within_validity(signed_at: u64, expires_at: Option<u64>) -> bool {
    match expires_at {
        Some(expiry) => signed_at <= expiry,
        None => true,
    }
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

    /// A real `SHASUMS256.txt.asc` from Node.js v18.14.0, signed in Feb 2023 by
    /// a release key that has since expired (2023-03-26). The genuine signature
    /// predates the key's expiry, so it must still verify.
    const FIXTURE_EXPIRED_SIGNER: &str = include_str!("assets/test/SHASUMS256-v18.14.0.txt.asc");

    /// A real `SHASUMS256.txt.asc` from Node.js v20.18.0, signed Oct 2024 by a
    /// key whose self-certification was later refreshed (re-certified Dec 2025).
    /// The current self-signature postdates this release signature, so policy
    /// must use the latest self-signature for expiry, not one predating the
    /// release signature.
    const FIXTURE_RECERTIFIED_SIGNER: &str =
        include_str!("assets/test/SHASUMS256-v20.18.0.txt.asc");

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

    #[test]
    fn validity_window_rejects_signatures_made_after_expiry() {
        let created = 1_000;
        let expires_at = Some(created + 2_000); // key valid until 3_000

        // A signature made while the key was valid is accepted...
        assert!(within_validity(created, expires_at));
        assert!(within_validity(3_000, expires_at));
        // ...but a fresh signature made after the key expired is rejected,
        // which is exactly the compromised-expired-key attack.
        assert!(!within_validity(3_001, expires_at));
        assert!(!within_validity(9_999_999, expires_at));
    }

    #[test]
    fn validity_window_allows_keys_without_expiry() {
        assert!(within_validity(0, None));
        assert!(within_validity(9_999_999, None));
    }

    #[test]
    fn genuine_fixture_passes_key_policy() {
        // The real v22.13.1 signature was made before its signing key expires,
        // so the added revocation/expiry policy must not reject it.
        assert!(verify_clearsigned(FIXTURE_SIGNED, node_release_keys()).is_ok());
    }

    #[test]
    fn accepts_genuine_old_signature_from_now_expired_key() {
        // The signing key has expired since, but the signature was made while it
        // was valid, so it must still verify (validity is checked against the
        // signature's creation time, not "now").
        let verified =
            verify_clearsigned(FIXTURE_EXPIRED_SIGNER, node_release_keys()).expect("should verify");
        assert!(verified.contains("node-v18.14.0-linux-x64.tar.gz"));
    }

    #[test]
    fn accepts_release_signed_before_key_was_recertified() {
        // The signing key's current self-signature was created after this
        // release was signed; policy must use the latest self-signature for
        // expiry rather than rejecting because none predates the release.
        let verified = verify_clearsigned(FIXTURE_RECERTIFIED_SIGNER, node_release_keys())
            .expect("should verify");
        assert!(verified.contains("node-v20.18.0-linux-x64.tar.gz"));
    }
}
