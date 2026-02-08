//! npm registry client for version resolution.
//!
//! Queries the npm registry to resolve versions and get tarball URLs
//! with integrity hashes for both the main package and platform-specific package.

use std::collections::HashMap;

use serde::Deserialize;
use vite_install::{config::NPM_REGISTRY, request::HttpClient};

use crate::error::Error;

/// npm package version metadata (subset of fields we need).
#[derive(Debug, Deserialize)]
pub struct PackageVersionMetadata {
    pub version: String,
    pub dist: DistInfo,
    #[serde(default, rename = "optionalDependencies")]
    pub optional_dependencies: HashMap<String, String>,
}

/// Distribution info from npm registry.
#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub integrity: String,
}

/// Resolved version info with URLs and integrity for both packages.
#[derive(Debug)]
pub struct ResolvedVersion {
    pub version: String,
    pub main_tarball_url: String,
    pub main_integrity: String,
    pub platform_tarball_url: String,
    pub platform_integrity: String,
}

const MAIN_PACKAGE_NAME: &str = "vite-plus-cli";
const PLATFORM_PACKAGE_SCOPE: &str = "@voidzero-dev";

/// Resolve a version from the npm registry.
///
/// Makes two HTTP calls:
/// 1. Main package metadata to get version, tarball URL, integrity, and optional deps
/// 2. Platform package metadata to get platform-specific tarball URL and integrity
pub async fn resolve_version(
    version_or_tag: &str,
    platform_suffix: &str,
    registry_override: Option<&str>,
) -> Result<ResolvedVersion, Error> {
    let registry = registry_override.unwrap_or_else(|| &NPM_REGISTRY);
    let client = HttpClient::new();

    // Step 1: Fetch main package metadata
    let main_url = format!("{registry}/{MAIN_PACKAGE_NAME}/{version_or_tag}");
    tracing::debug!("Fetching main package metadata: {}", main_url);

    let main_meta: PackageVersionMetadata = client.get_json(&main_url).await.map_err(|e| {
        Error::SelfUpdate(format!("Failed to fetch package metadata from {main_url}: {e}").into())
    })?;

    // Step 2: Determine platform package name from optionalDependencies
    let platform_package_name =
        format!("{PLATFORM_PACKAGE_SCOPE}/{MAIN_PACKAGE_NAME}-{platform_suffix}");

    if !main_meta.optional_dependencies.contains_key(&platform_package_name) {
        return Err(Error::SelfUpdate(
            format!(
                "Platform package '{platform_package_name}' not found in optionalDependencies of {MAIN_PACKAGE_NAME}@{}. \
                 Your platform ({platform_suffix}) may not be supported.",
                main_meta.version
            )
            .into(),
        ));
    }

    // Step 3: Fetch platform package metadata
    let platform_url = format!("{registry}/{platform_package_name}/{}", main_meta.version);
    tracing::debug!("Fetching platform package metadata: {}", platform_url);

    let platform_meta: PackageVersionMetadata =
        client.get_json(&platform_url).await.map_err(|e| {
            Error::SelfUpdate(
                format!("Failed to fetch platform package metadata from {platform_url}: {e}")
                    .into(),
            )
        })?;

    Ok(ResolvedVersion {
        version: main_meta.version,
        main_tarball_url: main_meta.dist.tarball,
        main_integrity: main_meta.dist.integrity,
        platform_tarball_url: platform_meta.dist.tarball,
        platform_integrity: platform_meta.dist.integrity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_construction() {
        let suffix = "darwin-arm64";
        let name = format!("{PLATFORM_PACKAGE_SCOPE}/{MAIN_PACKAGE_NAME}-{suffix}");
        assert_eq!(name, "@voidzero-dev/vite-plus-cli-darwin-arm64");
    }

    #[test]
    fn test_all_platform_suffixes_match_published_packages() {
        // These are the actual published optionalDependencies keys
        // (from packages/global/publish-native-addons.ts RUST_TARGETS keys)
        let published_suffixes = [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64-gnu",
            "linux-x64-gnu",
            "win32-arm64-msvc",
            "win32-x64-msvc",
        ];

        let published_deps: HashMap<String, String> = published_suffixes
            .iter()
            .map(|s| {
                (format!("{PLATFORM_PACKAGE_SCOPE}/{MAIN_PACKAGE_NAME}-{s}"), "0.1.0".to_string())
            })
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
            let package_name = format!("{PLATFORM_PACKAGE_SCOPE}/{MAIN_PACKAGE_NAME}-{suffix}");
            // musl variants are not published, so skip them
            if suffix.contains("musl") {
                continue;
            }
            assert!(
                published_deps.contains_key(&package_name),
                "Platform suffix '{suffix}' produces package name '{package_name}' \
                 which does not match any published package"
            );
        }
    }
}
