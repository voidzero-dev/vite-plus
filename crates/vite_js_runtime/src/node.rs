use std::env;

use vite_str::Str;

use crate::{Error, Platform};

/// Default Node.js distribution base URL
const DEFAULT_NODE_DIST_URL: &str = "https://nodejs.org/dist";

/// Environment variable to override the Node.js distribution URL
const NODE_DIST_MIRROR_ENV: &str = "VITE_NODE_DIST_MIRROR";

/// Get the Node.js distribution base URL
///
/// Returns the value of `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise returns the default `https://nodejs.org/dist`.
fn get_dist_url() -> Str {
    env::var(NODE_DIST_MIRROR_ENV)
        .map_or_else(|_| DEFAULT_NODE_DIST_URL.into(), |url| url.trim_end_matches('/').into())
}

/// Get the archive filename for a Node.js version on a specific platform
///
/// # Arguments
/// * `version` - The Node.js version (e.g., "22.13.1")
/// * `platform` - The target platform
///
/// # Returns
/// The archive filename (e.g., "node-v22.13.1-linux-x64.tar.gz")
pub fn get_archive_filename(version: &str, platform: Platform) -> Str {
    let platform_str = platform.node_platform_string();
    let ext = platform.archive_extension();
    vite_str::format!("node-v{version}-{platform_str}.{ext}")
}

/// Get the download URL for a Node.js archive
///
/// Uses `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise defaults to `https://nodejs.org/dist`.
///
/// # Arguments
/// * `version` - The Node.js version (e.g., "22.13.1")
/// * `platform` - The target platform
///
/// # Returns
/// The full download URL
pub fn get_download_url(version: &str, platform: Platform) -> Str {
    let base_url = get_dist_url();
    let filename = get_archive_filename(version, platform);
    vite_str::format!("{base_url}/v{version}/{filename}")
}

/// Get the URL for SHASUMS256.txt for a Node.js version
///
/// Uses `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise defaults to `https://nodejs.org/dist`.
///
/// # Arguments
/// * `version` - The Node.js version (e.g., "22.13.1")
///
/// # Returns
/// The SHASUMS256.txt URL
pub fn get_shasums_url(version: &str) -> Str {
    let base_url = get_dist_url();
    vite_str::format!("{base_url}/v{version}/SHASUMS256.txt")
}

/// Parse SHASUMS256.txt content and extract the hash for a specific filename
///
/// # Arguments
/// * `shasums_content` - The content of SHASUMS256.txt
/// * `filename` - The filename to find the hash for
///
/// # Returns
/// The SHA256 hash for the filename
///
/// # Format
/// Each line in SHASUMS256.txt is: `<hash>  <filename>`
pub fn parse_shasums(shasums_content: &str, filename: &str) -> Result<Str, Error> {
    for line in shasums_content.lines() {
        // Format: "<hash>  <filename>" (two spaces between hash and filename)
        let parts: Vec<&str> = line.splitn(2, "  ").collect();
        if parts.len() == 2 {
            let hash = parts[0].trim();
            let file = parts[1].trim();
            if file == filename {
                return Ok(hash.into());
            }
        }
    }

    Err(Error::HashNotFound { filename: filename.into() })
}

/// Get the directory name inside the archive after extraction
///
/// For Node.js, the archive contains a directory named like:
/// - Linux/macOS: `node-v22.13.1-linux-x64/`
/// - Windows: `node-v22.13.1-win-x64/`
pub fn get_extracted_dir_name(version: &str, platform: Platform) -> Str {
    let platform_str = platform.node_platform_string();
    vite_str::format!("node-v{version}-{platform_str}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Os};

    #[test]
    fn test_get_archive_filename() {
        let cases = [
            (
                "22.13.1",
                Platform { os: Os::Linux, arch: Arch::X64 },
                "node-v22.13.1-linux-x64.tar.gz",
            ),
            (
                "22.13.1",
                Platform { os: Os::Darwin, arch: Arch::Arm64 },
                "node-v22.13.1-darwin-arm64.tar.gz",
            ),
            ("22.13.1", Platform { os: Os::Windows, arch: Arch::X64 }, "node-v22.13.1-win-x64.zip"),
        ];

        for (version, platform, expected) in cases {
            assert_eq!(get_archive_filename(version, platform), expected);
        }
    }

    #[test]
    fn test_get_download_url() {
        let platform = Platform { os: Os::Linux, arch: Arch::X64 };
        let url = get_download_url("22.13.1", platform);
        assert_eq!(url, "https://nodejs.org/dist/v22.13.1/node-v22.13.1-linux-x64.tar.gz");
    }

    #[test]
    fn test_get_shasums_url() {
        let url = get_shasums_url("22.13.1");
        assert_eq!(url, "https://nodejs.org/dist/v22.13.1/SHASUMS256.txt");
    }

    #[test]
    fn test_parse_shasums() {
        let content = r"abc123def456  node-v22.13.1-linux-x64.tar.gz
789xyz000111  node-v22.13.1-darwin-arm64.tar.gz
fedcba987654  node-v22.13.1-win-x64.zip";

        assert_eq!(
            parse_shasums(content, "node-v22.13.1-linux-x64.tar.gz").unwrap(),
            "abc123def456"
        );
        assert_eq!(
            parse_shasums(content, "node-v22.13.1-darwin-arm64.tar.gz").unwrap(),
            "789xyz000111"
        );
        assert_eq!(parse_shasums(content, "node-v22.13.1-win-x64.zip").unwrap(), "fedcba987654");

        // Test missing filename
        let result = parse_shasums(content, "nonexistent.tar.gz");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_extracted_dir_name() {
        let platform = Platform { os: Os::Linux, arch: Arch::X64 };
        assert_eq!(get_extracted_dir_name("22.13.1", platform), "node-v22.13.1-linux-x64");
    }

    #[test]
    fn test_get_dist_url_default() {
        // When env var is not set, should return default URL
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
    }

    #[test]
    fn test_get_dist_url_with_mirror() {
        unsafe { env::set_var(NODE_DIST_MIRROR_ENV, "https://nodejs.org/dist") };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
    }

    #[test]
    fn test_get_dist_url_trims_trailing_slash() {
        // Should trim trailing slash from mirror URL
        unsafe { env::set_var(NODE_DIST_MIRROR_ENV, "https://nodejs.org/dist/") };
        assert_eq!(get_dist_url(), "https://nodejs.org/dist");
        unsafe { env::remove_var(NODE_DIST_MIRROR_ENV) };
    }
}
