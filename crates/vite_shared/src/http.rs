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
//!   Parsed via `reqwest::Certificate::from_pem_bundle` and merged into the
//!   trust store as *additional* roots. Note: the system store is **kept**
//!   (matches Node's `NODE_EXTRA_CA_CERTS` semantics, *not* OpenSSL/curl/git
//!   which use `SSL_CERT_FILE` as the sole bundle). Users who need strict
//!   isolation should enforce it at the network layer.
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
        match reqwest::Certificate::from_pem_bundle(&bytes) {
            Ok(certs) if certs.is_empty() => {
                output::warn(&vite_str::format!(
                    "no PEM certificate blocks found in {var}={}",
                    path.display()
                ));
            }
            Ok(certs) => {
                let n = certs.len();
                builder = builder.tls_certs_merge(certs);
                tracing::debug!("added {n} extra root certs from {var}");
            }
            Err(err) => {
                output::warn(&vite_str::format!(
                    "failed to parse CA bundle from {var}={}: {err}",
                    path.display()
                ));
            }
        }
    }

    if is_env_truthy(env_vars::VP_INSECURE_TLS) {
        output::warn(
            "VP_INSECURE_TLS is set — TLS certificate verification is disabled. \
             Do not use this in production.",
        );
        builder = builder.tls_danger_accept_invalid_certs(true);
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn os_str_is_blank_matches_whitespace_only() {
        assert!(os_str_is_blank(&OsString::from("")));
        assert!(os_str_is_blank(&OsString::from("   ")));
        assert!(os_str_is_blank(&OsString::from("\t\n")));
        assert!(!os_str_is_blank(&OsString::from("/etc/ssl/cert.pem")));
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
