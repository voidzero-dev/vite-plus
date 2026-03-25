/// Ensure a TLS crypto provider is installed (no-op on Windows which uses native-tls).
#[cfg(not(target_os = "windows"))]
pub fn ensure_tls_provider() {
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        // Err means a provider is already installed, which is fine
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(target_os = "windows")]
pub fn ensure_tls_provider() {}
