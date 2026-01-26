//! Node.js runtime provider implementation.

use std::env;

use async_trait::async_trait;
use vite_str::Str;

use crate::{
    Error, Platform,
    platform::Os,
    provider::{ArchiveFormat, DownloadInfo, HashVerification, JsRuntimeProvider},
};

/// Default Node.js distribution base URL
const DEFAULT_NODE_DIST_URL: &str = "https://nodejs.org/dist";

/// Environment variable to override the Node.js distribution URL
const NODE_DIST_MIRROR_ENV: &str = "VITE_NODE_DIST_MIRROR";

/// Node.js runtime provider
#[derive(Debug, Default)]
pub struct NodeProvider;

impl NodeProvider {
    /// Create a new `NodeProvider`
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Get the archive format for a platform
    const fn archive_format(platform: Platform) -> ArchiveFormat {
        match platform.os {
            Os::Windows => ArchiveFormat::Zip,
            Os::Linux | Os::Darwin => ArchiveFormat::TarGz,
        }
    }
}

/// Get the Node.js distribution base URL
///
/// Returns the value of `VITE_NODE_DIST_MIRROR` environment variable if set,
/// otherwise returns the default `https://nodejs.org/dist`.
fn get_dist_url() -> Str {
    env::var(NODE_DIST_MIRROR_ENV)
        .map_or_else(|_| DEFAULT_NODE_DIST_URL.into(), |url| url.trim_end_matches('/').into())
}

#[async_trait]
impl JsRuntimeProvider for NodeProvider {
    fn name(&self) -> &'static str {
        "node"
    }

    fn platform_string(&self, platform: Platform) -> Str {
        let os = match platform.os {
            Os::Linux => "linux",
            Os::Darwin => "darwin",
            Os::Windows => "win",
        };
        let arch = match platform.arch {
            crate::platform::Arch::X64 => "x64",
            crate::platform::Arch::Arm64 => "arm64",
        };
        vite_str::format!("{os}-{arch}")
    }

    fn get_download_info(&self, version: &str, platform: Platform) -> DownloadInfo {
        let base_url = get_dist_url();
        let platform_str = self.platform_string(platform);
        let format = Self::archive_format(platform);
        let ext = format.extension();

        let archive_filename: Str = vite_str::format!("node-v{version}-{platform_str}.{ext}");
        let archive_url = vite_str::format!("{base_url}/v{version}/{archive_filename}");
        let shasums_url = vite_str::format!("{base_url}/v{version}/SHASUMS256.txt");
        let extracted_dir_name = vite_str::format!("node-v{version}-{platform_str}");

        DownloadInfo {
            archive_url,
            archive_filename,
            archive_format: format,
            hash_verification: HashVerification::ShasumsFile { url: shasums_url },
            extracted_dir_name,
        }
    }

    fn binary_relative_path(&self, platform: Platform) -> Str {
        match platform.os {
            Os::Windows => "node.exe".into(),
            Os::Linux | Os::Darwin => "bin/node".into(),
        }
    }

    fn bin_dir_relative_path(&self, platform: Platform) -> Str {
        match platform.os {
            Os::Windows => "".into(),
            Os::Linux | Os::Darwin => "bin".into(),
        }
    }

    fn parse_shasums(&self, shasums_content: &str, filename: &str) -> Result<Str, Error> {
        // Node.js SHASUMS256.txt format: "<hash>  <filename>" (two spaces between)
        for line in shasums_content.lines() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Os};

    #[test]
    fn test_platform_string() {
        let provider = NodeProvider::new();

        let cases = [
            (Platform { os: Os::Linux, arch: Arch::X64 }, "linux-x64"),
            (Platform { os: Os::Linux, arch: Arch::Arm64 }, "linux-arm64"),
            (Platform { os: Os::Darwin, arch: Arch::X64 }, "darwin-x64"),
            (Platform { os: Os::Darwin, arch: Arch::Arm64 }, "darwin-arm64"),
            (Platform { os: Os::Windows, arch: Arch::X64 }, "win-x64"),
            (Platform { os: Os::Windows, arch: Arch::Arm64 }, "win-arm64"),
        ];

        for (platform, expected) in cases {
            assert_eq!(provider.platform_string(platform), expected);
        }
    }

    #[test]
    fn test_get_download_info() {
        let provider = NodeProvider::new();
        let platform = Platform { os: Os::Linux, arch: Arch::X64 };

        let info = provider.get_download_info("22.13.1", platform);

        assert_eq!(info.archive_filename, "node-v22.13.1-linux-x64.tar.gz");
        assert_eq!(
            info.archive_url,
            "https://nodejs.org/dist/v22.13.1/node-v22.13.1-linux-x64.tar.gz"
        );
        assert_eq!(info.archive_format, ArchiveFormat::TarGz);
        assert_eq!(info.extracted_dir_name, "node-v22.13.1-linux-x64");

        if let HashVerification::ShasumsFile { url } = &info.hash_verification {
            assert_eq!(url, "https://nodejs.org/dist/v22.13.1/SHASUMS256.txt");
        } else {
            panic!("Expected ShasumsFile verification");
        }
    }

    #[test]
    fn test_get_download_info_windows() {
        let provider = NodeProvider::new();
        let platform = Platform { os: Os::Windows, arch: Arch::X64 };

        let info = provider.get_download_info("22.13.1", platform);

        assert_eq!(info.archive_filename, "node-v22.13.1-win-x64.zip");
        assert_eq!(info.archive_format, ArchiveFormat::Zip);
    }

    #[test]
    fn test_binary_relative_path() {
        let provider = NodeProvider::new();

        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Linux, arch: Arch::X64 }),
            "bin/node"
        );
        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Darwin, arch: Arch::Arm64 }),
            "bin/node"
        );
        assert_eq!(
            provider.binary_relative_path(Platform { os: Os::Windows, arch: Arch::X64 }),
            "node.exe"
        );
    }

    #[test]
    fn test_bin_dir_relative_path() {
        let provider = NodeProvider::new();

        assert_eq!(
            provider.bin_dir_relative_path(Platform { os: Os::Linux, arch: Arch::X64 }),
            "bin"
        );
        assert_eq!(
            provider.bin_dir_relative_path(Platform { os: Os::Windows, arch: Arch::X64 }),
            ""
        );
    }

    #[test]
    fn test_parse_shasums() {
        let provider = NodeProvider::new();

        let content = r"abc123def456  node-v22.13.1-linux-x64.tar.gz
789xyz000111  node-v22.13.1-darwin-arm64.tar.gz
fedcba987654  node-v22.13.1-win-x64.zip";

        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-linux-x64.tar.gz").unwrap(),
            "abc123def456"
        );
        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-darwin-arm64.tar.gz").unwrap(),
            "789xyz000111"
        );
        assert_eq!(
            provider.parse_shasums(content, "node-v22.13.1-win-x64.zip").unwrap(),
            "fedcba987654"
        );

        // Test missing filename
        let result = provider.parse_shasums(content, "nonexistent.tar.gz");
        assert!(result.is_err());
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
