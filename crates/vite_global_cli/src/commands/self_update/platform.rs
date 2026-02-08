//! Platform detection for self-update.
//!
//! Detects the current platform and returns the npm package suffix
//! used to find the correct platform-specific binary package.

use crate::error::Error;

/// Detect the current platform suffix for npm package naming.
///
/// Returns strings like `darwin-arm64`, `linux-x64-gnu`, `linux-arm64-musl`, `win32-x64-msvc`.
pub fn detect_platform_suffix() -> Result<String, Error> {
    let os_name = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else {
        return Err(Error::SelfUpdate(
            format!("Unsupported operating system: {}", std::env::consts::OS).into(),
        ));
    };

    let arch_name = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        return Err(Error::SelfUpdate(
            format!("Unsupported architecture: {}", std::env::consts::ARCH).into(),
        ));
    };

    if os_name == "linux" {
        let libc = if cfg!(target_env = "musl") { "musl" } else { "gnu" };
        Ok(format!("{os_name}-{arch_name}-{libc}"))
    } else if os_name == "win32" {
        Ok(format!("{os_name}-{arch_name}-msvc"))
    } else {
        Ok(format!("{os_name}-{arch_name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform_suffix() {
        let suffix = detect_platform_suffix().unwrap();

        // Should be non-empty and contain a dash
        assert!(!suffix.is_empty());
        assert!(suffix.contains('-'));

        // Should match the current platform
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(suffix, "darwin-arm64");

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        assert_eq!(suffix, "darwin-x64");

        #[cfg(all(target_os = "linux", target_arch = "x86_64", not(target_env = "musl")))]
        assert_eq!(suffix, "linux-x64-gnu");

        #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
        assert_eq!(suffix, "linux-x64-musl");

        #[cfg(all(target_os = "linux", target_arch = "aarch64", not(target_env = "musl")))]
        assert_eq!(suffix, "linux-arm64-gnu");

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        assert_eq!(suffix, "win32-x64-msvc");
    }
}
