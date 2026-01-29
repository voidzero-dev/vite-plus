//! JavaScript runtime provider trait and supporting types.
//!
//! This module defines the trait that all runtime providers (Node, Bun, Deno)
//! must implement, along with types for describing download information.

use async_trait::async_trait;
use vite_str::Str;

use crate::{Error, Platform};

/// Archive format for runtime distributions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// Gzip-compressed tar archive (.tar.gz)
    TarGz,
    /// ZIP archive (.zip)
    Zip,
}

impl ArchiveFormat {
    /// Get the file extension for this archive format
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

/// How to verify the integrity of a downloaded archive
#[derive(Debug, Clone)]
pub enum HashVerification {
    /// Download a SHASUMS file and parse it to find the hash
    /// Used by Node.js (SHASUMS256.txt format)
    ShasumsFile {
        /// URL to the SHASUMS file
        url: Str,
    },
    /// No hash verification (not recommended, but some runtimes may not provide checksums)
    None,
}

/// Information needed to download a runtime
#[derive(Debug, Clone)]
pub struct DownloadInfo {
    /// URL to download the archive from
    pub archive_url: Str,
    /// Filename of the archive
    pub archive_filename: Str,
    /// Format of the archive
    pub archive_format: ArchiveFormat,
    /// How to verify the download integrity
    pub hash_verification: HashVerification,
    /// Name of the directory inside the archive after extraction
    pub extracted_dir_name: Str,
}

/// Trait for JavaScript runtime providers
///
/// Each runtime (Node.js, Bun, Deno) implements this trait to provide
/// runtime-specific logic for downloading and installing.
#[async_trait]
pub trait JsRuntimeProvider: Send + Sync {
    /// Get the name of this runtime (e.g., "node", "bun", "deno")
    fn name(&self) -> &'static str;

    /// Get the platform string used in download URLs for this runtime
    /// e.g., "linux-x64", "darwin-arm64", "win-x64"
    fn platform_string(&self, platform: Platform) -> Str;

    /// Get download information for a specific version and platform
    fn get_download_info(&self, version: &str, platform: Platform) -> DownloadInfo;

    /// Get the relative path to the runtime binary from the install directory
    /// e.g., "bin/node" on Unix, "node.exe" on Windows
    fn binary_relative_path(&self, platform: Platform) -> Str;

    /// Get the relative path to the bin directory from the install directory
    /// e.g., "bin" on Unix, "" (empty) on Windows
    fn bin_dir_relative_path(&self, platform: Platform) -> Str;

    /// Parse a SHASUMS file to extract the hash for a specific filename
    /// Different runtimes may have different SHASUMS formats
    ///
    /// # Errors
    ///
    /// Returns an error if the filename is not found in the SHASUMS content.
    fn parse_shasums(&self, shasums_content: &str, filename: &str) -> Result<Str, Error>;
}
