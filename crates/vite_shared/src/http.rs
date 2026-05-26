//! Process-wide shared `reqwest::Client`.
//!
//! Built once, lazily, and reused for every HTTP call vp makes. The single
//! instance lets us configure proxy honoring and custom-CA injection in one
//! place so HTTPS-intercepting tools like Socket Firewall Free (sfw) and
//! corporate MITM proxies work without per-call setup.
//!
//! Configuration sources (all read at first call):
//! - `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` — honored automatically by
//!   reqwest; no explicit wiring needed.
//! - `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS` — each may point to a PEM bundle
//!   (one or more concatenated certs). Every cert is added as a trusted root.
//!   A read/parse failure logs a warning and is otherwise ignored so a
//!   malformed env var never blocks startup.
//! - `VP_INSECURE_TLS` — when set to any value, disables cert verification
//!   entirely. Diagnostic escape hatch only; emits a loud stderr warning.

use std::sync::OnceLock;

use crate::{env_vars, output};

/// Get the process-wide `reqwest::Client`.
///
/// The client is built on first call and reused thereafter. See module docs
/// for the env vars it honors.
#[must_use]
pub fn shared_http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(build_client)
}

fn build_client() -> reqwest::Client {
    crate::ensure_tls_provider();

    let mut builder = reqwest::Client::builder();

    for var in [env_vars::SSL_CERT_FILE, env_vars::NODE_EXTRA_CA_CERTS] {
        let Ok(path) = std::env::var(var) else { continue };
        if path.is_empty() {
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => match reqwest::Certificate::from_pem_bundle(&bytes) {
                Ok(certs) => {
                    for cert in certs {
                        builder = builder.add_root_certificate(cert);
                    }
                }
                Err(err) => {
                    tracing::warn!("failed to parse extra CA bundle from {var}={path}: {err}");
                }
            },
            Err(err) => {
                tracing::warn!("failed to read extra CA bundle from {var}={path}: {err}");
            }
        }
    }

    if std::env::var_os(env_vars::VP_INSECURE_TLS).is_some() {
        output::warn(
            "VP_INSECURE_TLS is set — TLS certificate verification is disabled. \
             Do not use this in production.",
        );
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder.build().expect("failed to build shared reqwest client")
}
