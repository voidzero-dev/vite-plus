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
//! - `VP_INSECURE_TLS` — when set to any value, disables cert verification
//!   entirely. Diagnostic escape hatch only; emits a loud stderr warning.

use std::{ffi::OsStr, path::Path, sync::OnceLock};

use crate::{env_vars, output};

const PEM_CERT_BEGIN: &[u8] = b"-----BEGIN CERTIFICATE-----";
const PEM_CERT_END: &[u8] = b"-----END CERTIFICATE-----";

/// Get the process-wide `reqwest::Client`.
///
/// The client is built on first call and reused thereafter. See module docs
/// for the env vars it honors.
///
/// If reqwest fails to build the client (e.g. malformed `HTTPS_PROXY`,
/// unusable TLS backend) the process exits with a clean error message rather
/// than panicking — the first HTTP call cannot proceed and there is nothing
/// useful to fall back to.
#[must_use]
pub fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(build_client)
}

fn build_client() -> reqwest::Client {
    crate::ensure_tls_provider();

    let mut builder = reqwest::Client::builder();

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

    if std::env::var_os(env_vars::VP_INSECURE_TLS).is_some() {
        output::warn(
            "VP_INSECURE_TLS is set — TLS certificate verification is disabled. \
             Do not use this in production.",
        );
        builder = builder.danger_accept_invalid_certs(true);
    }

    match builder.build() {
        Ok(client) => client,
        Err(err) => {
            output::error(&vite_str::format!("failed to initialize HTTP client: {err}"));
            std::process::exit(1);
        }
    }
}

fn os_str_is_blank(value: &OsStr) -> bool {
    value.to_str().is_some_and(|s| s.trim().is_empty())
}

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
        let Some(end_rel) =
            bundle[body_start..].windows(PEM_CERT_END.len()).position(|w| w == PEM_CERT_END)
        else {
            break;
        };
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
}
