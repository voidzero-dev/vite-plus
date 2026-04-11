//! npm registry client for version resolution.
//!
//! Queries the npm registry to resolve versions and get tarball URLs
//! with integrity hashes for both the main package and platform-specific package.

use serde::Deserialize;
use vite_install::{config::npm_registry, request::HttpClient};

use crate::error::Error;

/// npm package version metadata (subset of fields we need).
#[derive(Debug, Deserialize)]
pub struct PackageVersionMetadata {
    pub version: String,
    pub dist: DistInfo,
}

/// Distribution info from npm registry.
#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub integrity: String,
}

/// Resolved version info with URLs and integrity for the platform package.
#[derive(Debug)]
pub struct ResolvedVersion {
    pub version: String,
    pub platform_tarball_url: String,
    pub platform_integrity: String,
}

const MAIN_PACKAGE_NAME: &str = "vite-plus";
const PLATFORM_PACKAGE_SCOPE: &str = "@voidzero-dev";
const CLI_PACKAGE_NAME_PREFIX: &str = "vite-plus-cli";

/// Resolve a version string from the npm registry.
///
/// Single HTTP call to resolve a version or tag (e.g., "latest" → "1.2.3").
/// Does NOT verify the platform-specific package exists.
pub async fn resolve_version_string(
    version_or_tag: &str,
    registry_override: Option<&str>,
) -> Result<String, Error> {
    let default_registry = npm_registry();
    let registry_raw = registry_override.unwrap_or(&default_registry);
    let registry = registry_raw.trim_end_matches('/');
    let client = HttpClient::new();

    let main_url = format!("{registry}/{MAIN_PACKAGE_NAME}/{version_or_tag}");
    tracing::debug!("Fetching main package metadata: {}", main_url);

    let main_meta: PackageVersionMetadata = client.get_json(&main_url).await.map_err(|e| {
        Error::Upgrade(format!("Failed to fetch package metadata from {main_url}: {e}").into())
    })?;

    Ok(main_meta.version)
}

/// Resolve the platform-specific package metadata for a given version.
///
/// Single HTTP call to fetch the tarball URL and integrity hash for the
/// platform-specific CLI binary package.
pub async fn resolve_platform_package(
    version: &str,
    platform_suffix: &str,
    registry_override: Option<&str>,
) -> Result<ResolvedVersion, Error> {
    let default_registry = npm_registry();
    let registry_raw = registry_override.unwrap_or(&default_registry);
    let registry = registry_raw.trim_end_matches('/');
    let client = HttpClient::new();

    let cli_package_name =
        format!("{PLATFORM_PACKAGE_SCOPE}/{CLI_PACKAGE_NAME_PREFIX}-{platform_suffix}");
    let cli_url = format!("{registry}/{cli_package_name}/{version}");
    tracing::debug!("Fetching CLI package metadata: {}", cli_url);

    let cli_meta: PackageVersionMetadata = client.get_json(&cli_url).await.map_err(|e| {
        Error::Upgrade(
            format!(
                "Failed to fetch CLI package metadata from {cli_url}: {e}. \
                     Your platform ({platform_suffix}) may not be supported."
            )
            .into(),
        )
    })?;

    Ok(ResolvedVersion {
        version: version.to_owned(),
        platform_tarball_url: cli_meta.dist.tarball,
        platform_integrity: cli_meta.dist.integrity,
    })
}

/// Resolve a version from the npm registry with platform package verification.
///
/// Makes two HTTP calls:
/// 1. Main package metadata to resolve version tags (e.g., "latest" → "1.2.3")
/// 2. CLI platform package metadata to get tarball URL and integrity
pub async fn resolve_version(
    version_or_tag: &str,
    platform_suffix: &str,
    registry_override: Option<&str>,
) -> Result<ResolvedVersion, Error> {
    let version = resolve_version_string(version_or_tag, registry_override).await?;
    resolve_platform_package(&version, platform_suffix, registry_override).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_package_name_construction() {
        let suffix = "darwin-arm64";
        let name = format!("{PLATFORM_PACKAGE_SCOPE}/{CLI_PACKAGE_NAME_PREFIX}-{suffix}");
        assert_eq!(name, "@voidzero-dev/vite-plus-cli-darwin-arm64");
    }

    #[test]
    fn test_all_platform_suffixes_match_published_cli_packages() {
        // These are the actual published CLI package suffixes
        // (from packages/cli/publish-native-addons.ts RUST_TARGETS keys)
        let published_suffixes = [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64-gnu",
            "linux-arm64-musl",
            "linux-x64-gnu",
            "linux-x64-musl",
            "win32-arm64-msvc",
            "win32-x64-msvc",
        ];

        let published_packages: Vec<String> = published_suffixes
            .iter()
            .map(|s| format!("{PLATFORM_PACKAGE_SCOPE}/{CLI_PACKAGE_NAME_PREFIX}-{s}"))
            .collect();

        // All known platform suffixes that detect_platform_suffix() can return
        let detection_suffixes = [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64-gnu",
            "linux-x64-gnu",
            "linux-arm64-musl",
            "linux-x64-musl",
            "win32-arm64-msvc",
            "win32-x64-msvc",
        ];

        for suffix in &detection_suffixes {
            let package_name =
                format!("{PLATFORM_PACKAGE_SCOPE}/{CLI_PACKAGE_NAME_PREFIX}-{suffix}");
            assert!(
                published_packages.contains(&package_name),
                "Platform suffix '{suffix}' produces CLI package name '{package_name}' \
                 which does not match any published CLI package"
            );
        }
    }
}
