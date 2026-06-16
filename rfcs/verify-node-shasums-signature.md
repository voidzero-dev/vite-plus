# RFC: Verify Node.js `SHASUMS256.txt` PGP Signature

- Issue: [#1807](https://github.com/voidzero-dev/vite-plus/issues/1807)
- Status: Implemented in [#1848](https://github.com/voidzero-dev/vite-plus/pull/1848)

## Summary

When Vite+ downloads a managed Node.js runtime, it verifies the archive's
SHA-256 against `SHASUMS256.txt` (see [`js-runtime.md`](./js-runtime.md)). That
proves the download was not corrupted, but it proves nothing about
**authenticity**: an attacker who can tamper with the `SHASUMS256.txt` response
can also supply a matching malicious archive, and the checksum still passes.

Node.js signs `SHASUMS256.txt` with the PGP key of the releaser who cut the
release (see [Verifying binaries](https://github.com/nodejs/node#verifying-binaries)).
This RFC adds verification of that signature: Vite+ downloads the clearsigned
`SHASUMS256.txt.asc` and verifies it against an embedded copy of the Node.js
release signing keys before trusting any checksum from it. This is the same
guarantee `gpgv` provides against the official release keyring, done in pure
Rust with no external `gpg` dependency.

## Background

The runtime download flow resolves an expected hash once, then verifies the
archive against it (`crates/vite_js_runtime/src/runtime.rs`):

```rust
let expected_hash = match &download_info.hash_verification {
    HashVerification::ShasumsFile { url } => {
        let shasums_content = download_text(url).await?;       // plain SHASUMS256.txt
        Some(provider.parse_shasums(&shasums_content, &archive_filename)?)
    }
    HashVerification::None => None,
};
```

The `SHASUMS256.txt` was fetched over HTTPS and trusted as-is. HTTPS protects
the transport, but the threat model includes a compromised or coerced mirror, a
misconfigured proxy, or any party able to influence the bytes returned for the
SHASUMS request. Without signature verification, such a party can serve a
SHASUMS file whose hash matches an archive they control.

## Prior art in Node.js version managers

Download verification across the popular Node.js version managers ranges from
nothing to full signature checking. Two levels matter: **integrity** (SHA-256
against `SHASUMS256.txt`) and **authenticity** (the PGP signature on
`SHASUMS256.txt.asc`, which proves the SHASUMS file itself came from a Node.js
releaser). Only authenticity defends against a tampered SHASUMS; HTTPS transport
is table stakes for all of them.

| Manager             | SHA-256 |         PGP signature          | Mechanism                                               |
| ------------------- | :-----: | :----------------------------: | ------------------------------------------------------- |
| Vite+ (`vp`)        |   yes   | yes (default, official source) | built-in, pure Rust; bundled keyring; no external `gpg` |
| [asdf-nodejs]       |   yes   |     yes (default `strict`)     | external `gpg`; user imports the release keyring        |
| [mise]              |   yes   |       yes (configurable)       | external `gpg`                                          |
| [nvm]               |   yes   |               no               | GPG [declined][nvm-pr] to stay POSIX / dependency-free  |
| [Volta]             |   yes   |               no               | `sha2` crate (project unmaintained)                     |
| nodenv / node-build |   yes   |               no               | checksum embedded in version definitions                |
| [fnm]               |   no    |               no               | HTTPS only, no verification                             |

Most managers stop at checksums, which only bind the archive to a SHASUMS file
fetched over the same channel, and `fnm` does nothing beyond HTTPS. Only
asdf-nodejs and mise also verify the signature, and both shell out to system
`gpg` and require the release keyring to be imported, which is fragile in
practice (mise has a recurring stream of GPG key-import/encoding failures).

Vite+ is the only one that verifies the signature in-process with no external
`gpg`. That is what justifies bundling rPGP and the keyring (~1.2 MiB added to
the `vp` binary): it brings the strongest guarantee available while keeping the
zero-dependency, cross-platform install that `vp` requires.

[asdf-nodejs]: https://github.com/asdf-vm/asdf-nodejs
[mise]: https://mise.jdx.dev/
[nvm]: https://github.com/nvm-sh/nvm
[nvm-pr]: https://github.com/nvm-sh/nvm/pull/736
[Volta]: https://github.com/volta-cli/volta
[fnm]: https://github.com/Schniz/fnm

## Goals

1. Verify the PGP signature of `SHASUMS256.txt` before trusting any checksum.
2. No external tooling: do not shell out to `gpg`/`gpgv` (it is frequently
   absent and requires the release keys to be imported into a keyring).
3. Cross-platform, including musl and Windows cross-builds: pure Rust, no new
   C/system dependencies.
4. Do not regress existing installs: every Node.js version Vite+ can install
   today must continue to verify, including old LTS lines and custom mirrors.

## Non-Goals

- A runtime keyring update/sync mechanism (the keyring is a vendored snapshot,
  see [Limitations](#trust-model-and-limitations)).
- Verifying signatures for runtimes other than Node.js (Bun/Deno do not publish
  a comparable clearsigned checksum file today).
- Enforcing OpenPGP key expiry as a hard failure (see
  [Why expiry is not enforced](#why-expiry-is-not-enforced)).

## Design

### Download the clearsigned SHASUMS

`SHASUMS256.txt.asc` is a [cleartext-signed](https://www.rfc-editor.org/rfc/rfc9580.html#name-cleartext-signature-framework)
message: it embeds both the `SHASUMS256.txt` body and the signature. Vite+
downloads the `.asc`, verifies the signature, and parses the **verified**
plaintext for the archive hash, so the checksum that is trusted is exactly the
one that was signed.

### Verification library

Verification uses [`pgp`](https://crates.io/crates/pgp) (rPGP), a pure-Rust
OpenPGP implementation. It is added with `default-features = false`: the default
feature pulls in `bzip2`, and clearsigned verification needs no decompression.
The crypto is RustCrypto-based (RSA, ECDSA, EdDSA, SHA-2), so no new C or system
dependency is introduced and musl/Windows cross-builds are unaffected.

### Embedded release keyring

The trusted keys are vendored from the
[`nodejs/release-keys`](https://github.com/nodejs/release-keys) repository
(all current and historical release keys) into
`crates/vite_js_runtime/src/assets/node-release-keys.asc` and embedded with
`include_str!`. Historical keys are required: for example `node-v18.20.5` is
signed by a key that is not in Node's current "primary keys" list, so a
current-keys-only set would break Node 18 LTS verification.

### Verification policy (matches `gpgv`)

A clearsigned SHASUMS is accepted when:

1. Its signature **cryptographically verifies** against a key (primary or
   subkey) in the embedded keyring, and
2. That primary key is **not revoked**, and
3. For a **subkey** signature, the subkey is a validly-bound, signing-capable
   subkey of the primary (signing key-flag set, valid binding signature, valid
   embedded primary-key back-signature), so a leaked encryption-only subkey
   cannot be used to sign.

This mirrors what `gpgv --keyring <node-release-keyring> SHASUMS256.txt.asc`
accepts. Node releasers currently sign with their primary keys; the subkey path
is defensive for future releasers who sign with a signing subkey.

### Verification source: required vs best-effort

`HashVerification::ShasumsFile` carries an optional signature descriptor:

```rust
pub enum HashVerification {
    ShasumsFile {
        url: Str,                          // plain SHASUMS256.txt (fallback)
        signature: Option<ShasumsSignature>,
    },
    None,
}

pub struct ShasumsSignature {
    pub url: Str,       // SHASUMS256.txt.asc
    pub required: bool, // hard-required for the official host
}
```

- **Official `nodejs.org`** (default, or a `VP_NODE_DIST_MIRROR` pointed back at
  `nodejs.org`): `required = true`. A missing or invalid signature is a hard
  error. `required` is decided by the resolved host, not merely whether the
  mirror env var is set.
- **Custom mirror** (`VP_NODE_DIST_MIRROR`, e.g. an internal Artifactory that
  publishes only the archives and `SHASUMS256.txt`): `required = false`. If the
  `.asc` is unavailable, fall back to the plain `SHASUMS256.txt` with a warning.
  A downloaded-but-invalid signature still fails everywhere.
- **musl unofficial builds** (`unofficial-builds.nodejs.org`): no `.asc` is
  published, so `signature` is `None` and the plain `SHASUMS256.txt` is used.

The signature fetch/verify stays outside the content-integrity retry loop, like
the existing SHASUMS fetch/parse: signature failures are permanent, and
`download_text` already retries the network layer.

## Trust model and limitations

The trust boundary is the curated set of Node.js release keys plus honoring key
revocation, which is exactly what `gpgv` against the release keyring provides.
Two properties follow and are intentional.

### Why expiry is not enforced

`gpgv` treats key expiry as advisory for signature verification: it still
reports an expired key's signature as good. Verified empirically, `gpgv` exits
`0` for `node-v16.20.0`, whose `SHASUMS256.txt.asc` was signed a few days
**after** its signing key's expiry, and for `node-v20.18.0`, whose signing key
was re-certified after the release was signed.

Enforcing expiry would therefore reject legitimate Node releases. It would also
not add protection against a leaked release key, because the attacker controls
the signature's self-asserted creation time and can backdate a forgery to before
any expiry. The real protections against a compromised key are **revocation**
(enforced here) and keeping the vendored keyring current. So expiry is not
enforced, matching `gpgv`.

### Vendored keyring currency

The keyring is a vendored snapshot while Node version resolution is live. A
release signed by a releaser key added after the snapshot was built has no
matching trusted key and fails closed on the official source until the keyring
(and Vite+) is updated. All current releasers' keys are included, so this only
affects a brand-new releaser before a refresh.

The keyring is kept current automatically. A scheduled workflow
(`.github/workflows/update-node-release-keys.yml`, weekly) regenerates the
embedded keyring from `nodejs/release-keys` via
`.github/scripts/update-node-release-keys.sh` and opens a pull request when it
changes. The PR is **not** auto-merged: a human reviews which keys changed
before the trust anchor is updated, and PR CI (the `vite_js_runtime` tests)
confirms every vendored key still parses. The same script can be run locally to
refresh the keyring on demand.

### `rsa` advisory

Pulling in `pgp` transitively adds the `rsa` crate, which carries
[RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) (the Marvin
timing side-channel). That advisory affects RSA **private-key** operations
observable over a network; Vite+ only performs **public-key signature
verification**, so it does not apply. There is no patched `rsa` release and no
pure-Rust OpenPGP library can verify RSA-signed releases without it.

## Implementation

- `crates/vite_js_runtime/src/pgp_verify.rs` (new): keyring parsing,
  `verify_clearsigned`, revocation and subkey-policy checks, and the async
  `verify_signed_shasums` wrapper (runs on a blocking thread; key parsing and
  verification are CPU-bound).
- `crates/vite_js_runtime/src/assets/node-release-keys.asc` (new): vendored
  release keyring.
- `crates/vite_js_runtime/src/provider.rs`: `ShasumsSignature`, and the
  `signature` field on `HashVerification::ShasumsFile`.
- `crates/vite_js_runtime/src/providers/node.rs`: build the `.asc` URL and set
  `required` from the resolved host.
- `crates/vite_js_runtime/src/runtime.rs`: fetch/verify the `.asc`, with the
  best-effort fallback for non-official mirrors.
- `crates/vite_js_runtime/src/error.rs`: `SignatureVerificationFailed`.

## Testing

Unit tests use vendored real fixtures under
`crates/vite_js_runtime/src/assets/test/`:

- A genuine release verifies and yields the expected checksum.
- A tampered SHASUMS body is rejected.
- A signature against an empty/untrusted keyring is rejected.
- Non-clearsigned input is rejected.
- Regression fixtures for the cases that previously broke: a release from a
  now-expired key (`v18.14.0`), a release whose key was re-certified afterward
  (`v20.18.0`), and a release signed shortly after its key's expiry (`v16.20.0`).

A broad spread of real releases (Node 16.x through 24.x) was cross-checked to
verify through this code exactly when `gpgv` accepts them, and the end-to-end
`vp env exec` download path was exercised against a fresh install.

## Alternatives considered

- **Shell out to `gpg`/`gpgv`**: rejected. `gpg` is often not installed, and it
  would require importing the release keyring into the user's GnuPG home.
- **`sequoia-openpgp`**: its pure-Rust backend also depends on the `rsa` crate
  (same advisory), and its other backends (nettle/OpenSSL/CNG) need C/system
  libraries that break static musl and Windows cross-builds.
- **Restrict to current "primary" keys only**: rejected. It breaks verification
  of older LTS releases signed by now-retired keys (e.g. Node 18.x).
- **Enforce expiry / require current-time validity**: rejected. It diverges from
  `gpgv`, rejects legitimate releases, and does not stop a backdating attacker.

## References

- Issue [#1807](https://github.com/voidzero-dev/vite-plus/issues/1807)
- [Node.js: Verifying binaries](https://github.com/nodejs/node#verifying-binaries)
- [`nodejs/release-keys`](https://github.com/nodejs/release-keys)
- [`pgp` (rPGP)](https://crates.io/crates/pgp)
- [`js-runtime.md`](./js-runtime.md)
