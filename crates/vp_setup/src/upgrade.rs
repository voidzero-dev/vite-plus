//! Versioned handoff for preparing an upgrade before activation.

/// Request value for both the capability probe and the preparation entry point.
pub const PREPARE_PROTOCOL: &str = "upgrade-prepare-v1";

/// Exact response to the preparation capability probe.
pub const PREPARE_CAPABILITY: &str = "vite-plus-upgrade-prepare-v1\n";

/// Internal entry point that prepares dependencies without activating the release.
pub const PREPARE_ENV: &str = "VP_UPGRADE_PREPARE";

/// Preserve the original bootstrap response for existing shell and Windows installers.
pub fn capability_response(request: &std::ffi::OsStr) -> &'static str {
    if request == PREPARE_PROTOCOL { PREPARE_CAPABILITY } else { "vite-plus-self-setup-v1\n" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_probe_remains_compatible() {
        for request in ["1", "", "unknown"] {
            assert_eq!(capability_response(request.as_ref()), "vite-plus-self-setup-v1\n");
        }
        assert_eq!(capability_response(PREPARE_PROTOCOL.as_ref()), PREPARE_CAPABILITY);
    }
}
