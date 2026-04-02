/// Ensure a TLS crypto provider is installed.
pub fn ensure_tls_provider() {
    static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        // Err means a provider is already installed, which is fine
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}
