//! Process-wide shared `reqwest::Client`.
//!
//! Built once, lazily, and reused for every HTTP call vp makes. The single
//! instance lets us configure proxy honoring and custom-CA injection in one
//! place so HTTPS-intercepting tools like Socket Firewall Free (sfw) and
//! corporate MITM proxies work without per-call setup.
//!
//! Configuration sources (all read at first call):
//! - `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` — honored automatically by
//!   reqwest. With the `system-proxy` feature enabled, macOS System Settings
//!   proxies and Windows registry proxies are also picked up.
//! - `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS` — each may point to a PEM bundle.
//!   Every `-----BEGIN CERTIFICATE-----` block is parsed independently and
//!   added as an *additional* trusted root (system store is also kept —
//!   unlike OpenSSL's `SSL_CERT_FILE` which replaces it). Per-block parse
//!   failures emit a stderr warning and the remaining blocks are still added.
//! - `VP_INSECURE_TLS` — when set to a *truthy* value (`1`, `true`, `yes`,
//!   `on`, case-insensitive), disables cert verification entirely. Diagnostic
//!   escape hatch only; emits a loud stderr warning. Any other value
//!   (including `0`, `false`, `no`, `off`, empty string) leaves verification
//!   enabled.
//!
//! Note: env vars are read exactly once at the first HTTP call. In long-lived
//! processes (e.g. the NAPI binding embedded in Node), later
//! `process.env.SSL_CERT_FILE = ...` mutations do *not* re-configure the
//! client.

use std::{ffi::OsStr, path::Path, sync::OnceLock, time::Duration};

use crate::{env_vars, error::format_error_chain, output};

const PEM_CERT_BEGIN: &[u8] = b"-----BEGIN CERTIFICATE-----";
const PEM_CERT_END: &[u8] = b"-----END CERTIFICATE-----";

/// Per-request total timeout. Long enough for slow tarball downloads on
/// constrained CI runners, short enough that a single stuck stream doesn't
/// silently hang a build.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(2);

/// TCP connect timeout. Distinct from the request timeout above — without
/// this, a black-holed proxy can stall every HTTP call for kernel-level
/// retries (multiple minutes).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Get the process-wide `reqwest::Client`.
///
/// The client is built on first call and reused thereafter. See module docs
/// for the env vars it honors.
///
/// Panics on the *first* call if reqwest fails to build the client (malformed
/// `HTTPS_PROXY`, unusable TLS backend, etc.); subsequent calls in the same
/// process panic with the same message. Panic — not `process::exit` — so
/// destructors of in-flight work still run (lockfiles released, tempfiles
/// cleaned) and an embedding Node host (NAPI) keeps the process alive.
#[must_use]
pub fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    match CLIENT.get_or_init(build_client) {
        Ok(client) => client,
        Err(msg) => panic!("failed to initialize HTTP client: {msg}"),
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    crate::ensure_tls_provider();

    let mut builder =
        reqwest::Client::builder().timeout(REQUEST_TIMEOUT).connect_timeout(CONNECT_TIMEOUT);

    for var in [env_vars::SSL_CERT_FILE, env_vars::NODE_EXTRA_CA_CERTS] {
        let Some(value) = std::env::var_os(var) else { continue };
        if value.is_empty() || os_str_is_blank(&value) {
            continue;
        }
        let path = Path::new(&value);
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => {
                output::warn(&vite_str::format!(
                    "failed to read CA bundle from {var}={}: {err}",
                    path.display()
                ));
                continue;
            }
        };
        let blocks = extract_pem_cert_blocks(&bytes);
        if blocks.is_empty() {
            output::warn(&vite_str::format!(
                "no PEM certificate blocks found in {var}={}",
                path.display()
            ));
            continue;
        }
        let mut added = 0_usize;
        for (idx, block) in blocks.iter().enumerate() {
            match reqwest::Certificate::from_pem(block) {
                Ok(cert) => {
                    builder = builder.add_root_certificate(cert);
                    added += 1;
                }
                Err(err) => {
                    output::warn(&vite_str::format!(
                        "failed to parse certificate #{} from {var}={}: {err}",
                        idx + 1,
                        path.display()
                    ));
                }
            }
        }
        tracing::debug!("added {added} extra root certs from {var}");
    }

    if is_env_truthy(env_vars::VP_INSECURE_TLS) {
        output::warn(
            "VP_INSECURE_TLS is set — TLS certificate verification is disabled. \
             Do not use this in production.",
        );
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder.build().map_err(|err| format_error_chain(&err))
}

/// Returns `true` only for clearly affirmative env-var values
/// (`1`, `true`, `yes`, `on`, case-insensitive).
///
/// Avoids the footgun where `VP_INSECURE_TLS=0` or `VP_INSECURE_TLS=false`
/// is interpreted as "the variable is set, so feature on" — users naturally
/// expect those values to *disable* the flag.
fn is_env_truthy(var: &str) -> bool {
    let Some(value) = std::env::var_os(var) else { return false };
    let Some(s) = value.to_str() else { return false };
    let trimmed = s.trim();
    ["1", "true", "yes", "on"].iter().any(|v| trimmed.eq_ignore_ascii_case(v))
}

fn os_str_is_blank(value: &OsStr) -> bool {
    value.to_str().is_some_and(|s| s.trim().is_empty())
}

/// Extract `-----BEGIN CERTIFICATE-----`…`-----END CERTIFICATE-----` blocks
/// from a PEM bundle, byte-window-based.
///
/// Handles a malformed bundle where a `BEGIN` is not followed by a matching
/// `END` before the next `BEGIN` — that orphan is skipped (logged at debug)
/// rather than greedily consuming the next certificate's body.
fn extract_pem_cert_blocks(bundle: &[u8]) -> Vec<&[u8]> {
    let mut blocks = Vec::new();
    let mut cursor = 0_usize;
    while cursor < bundle.len() {
        let Some(start_rel) =
            bundle[cursor..].windows(PEM_CERT_BEGIN.len()).position(|w| w == PEM_CERT_BEGIN)
        else {
            break;
        };
        let start = cursor + start_rel;
        let body_start = start + PEM_CERT_BEGIN.len();
        let search_slice = &bundle[body_start..];
        let next_end = search_slice.windows(PEM_CERT_END.len()).position(|w| w == PEM_CERT_END);
        let Some(end_rel) = next_end else {
            // No END marker at all: orphan BEGIN — stop scanning, nothing
            // valid can follow.
            tracing::debug!("PEM bundle: unterminated BEGIN CERTIFICATE at byte {start}");
            break;
        };
        // If a *new* BEGIN appears before this END, the current BEGIN is
        // orphaned. Skip past just this orphan and resume scanning at the
        // intervening BEGIN — without this, both certs are lost.
        let next_begin =
            search_slice.windows(PEM_CERT_BEGIN.len()).position(|w| w == PEM_CERT_BEGIN);
        if let Some(next_begin_rel) = next_begin
            && next_begin_rel < end_rel
        {
            tracing::debug!(
                "PEM bundle: orphan BEGIN CERTIFICATE at byte {start} (no END before next BEGIN); skipping"
            );
            cursor = body_start + next_begin_rel;
            continue;
        }
        let end = body_start + end_rel + PEM_CERT_END.len();
        blocks.push(&bundle[start..end]);
        cursor = end;
    }
    blocks
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    const SAMPLE_CERT: &[u8] =
        b"-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIJAKHHIglt\n-----END CERTIFICATE-----";

    #[test]
    fn os_str_is_blank_matches_whitespace_only() {
        assert!(os_str_is_blank(&OsString::from("")));
        assert!(os_str_is_blank(&OsString::from("   ")));
        assert!(os_str_is_blank(&OsString::from("\t\n")));
        assert!(!os_str_is_blank(&OsString::from("/etc/ssl/cert.pem")));
    }

    #[test]
    fn extract_blocks_finds_single_cert() {
        let blocks = extract_pem_cert_blocks(SAMPLE_CERT);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], SAMPLE_CERT);
    }

    #[test]
    fn extract_blocks_skips_non_cert_pem() {
        let bundle = b"\
-----BEGIN PRIVATE KEY-----\n\
ignored\n\
-----END PRIVATE KEY-----\n\
-----BEGIN CERTIFICATE-----\n\
keepme\n\
-----END CERTIFICATE-----\n";
        let blocks = extract_pem_cert_blocks(bundle);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].starts_with(b"-----BEGIN CERTIFICATE-----"));
        assert!(blocks[0].ends_with(b"-----END CERTIFICATE-----"));
    }

    #[test]
    fn extract_blocks_finds_multiple_certs() {
        let bundle = b"\
-----BEGIN CERTIFICATE-----\n\
one\n\
-----END CERTIFICATE-----\n\
junk in between\n\
-----BEGIN CERTIFICATE-----\n\
two\n\
-----END CERTIFICATE-----\n";
        let blocks = extract_pem_cert_blocks(bundle);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn extract_blocks_drops_unterminated_block() {
        let bundle = b"-----BEGIN CERTIFICATE-----\nno end marker\n";
        assert!(extract_pem_cert_blocks(bundle).is_empty());
    }

    #[test]
    fn extract_blocks_recovers_after_orphan_begin() {
        // Hand-concatenated bundle missing a newline + END marker between
        // two certs: the orphan first BEGIN must not swallow the second
        // cert's body. The valid second cert is recovered.
        let bundle = b"\
-----BEGIN CERTIFICATE-----\n\
truncated, no END\n\
-----BEGIN CERTIFICATE-----\n\
valid\n\
-----END CERTIFICATE-----\n";
        let blocks = extract_pem_cert_blocks(bundle);
        assert_eq!(blocks.len(), 1, "expected to recover the second cert");
        let recovered = std::str::from_utf8(blocks[0]).unwrap();
        assert!(recovered.contains("valid"));
        assert!(recovered.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(recovered.ends_with("-----END CERTIFICATE-----"));
    }

    #[test]
    #[serial_test::serial(env)]
    fn is_env_truthy_accepts_only_affirmative_values() {
        // Use unique var names per case to avoid test-ordering interference
        // when std::env is process-global.
        for affirmative in ["1", "true", "TRUE", "True", "yes", "Yes", "on", "ON", " 1 "] {
            // SAFETY: tests are run serially within this module for env vars.
            unsafe {
                std::env::set_var("VP_TEST_TRUTHY_VALUE", affirmative);
            }
            assert!(is_env_truthy("VP_TEST_TRUTHY_VALUE"), "should be truthy: {affirmative:?}");
        }
        for negative in ["0", "false", "FALSE", "no", "off", "", "  "] {
            unsafe {
                std::env::set_var("VP_TEST_TRUTHY_VALUE", negative);
            }
            assert!(!is_env_truthy("VP_TEST_TRUTHY_VALUE"), "should be falsy: {negative:?}");
        }
        unsafe {
            std::env::remove_var("VP_TEST_TRUTHY_VALUE");
        }
        assert!(!is_env_truthy("VP_TEST_TRUTHY_VALUE"));
    }
}
