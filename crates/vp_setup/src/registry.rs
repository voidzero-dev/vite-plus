//! npm registry client for version resolution.
//!
//! Queries the npm registry to resolve versions and get tarball URLs
//! with integrity hashes for both the main package and platform-specific package.

use serde::Deserialize;
use vp_pm_cli::{HttpClient, npm_registry};

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
    #[serde(default)]
    pub attestations: Option<NpmAttestations>,
}

/// npm attestations attached to a package version.
#[derive(Debug, Deserialize)]
pub struct NpmAttestations {
    #[serde(default)]
    pub provenance: Option<NpmProvenance>,
}

/// npm provenance metadata used to identify the attestation predicate.
#[derive(Debug, Deserialize)]
pub struct NpmProvenance {
    #[serde(rename = "predicateType", default)]
    pub predicate_type: Option<String>,
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
const SUPPORTED_PROVENANCE_PREDICATE_TYPES: [&str; 2] =
    ["https://slsa.dev/provenance/v1", "https://slsa.dev/provenance/v0.2"];

fn validate_platform_package_provenance(
    package_name: &str,
    version: &str,
    dist: &DistInfo,
) -> Result<(), Error> {
    let predicate_type = dist
        .attestations
        .as_ref()
        .and_then(|attestations| attestations.provenance.as_ref())
        .and_then(|provenance| provenance.predicate_type.as_deref())
        .filter(|predicate_type| !predicate_type.is_empty());

    if predicate_type.is_some_and(|predicate_type| {
        SUPPORTED_PROVENANCE_PREDICATE_TYPES.contains(&predicate_type)
    }) {
        return Ok(());
    }

    Err(Error::UnsupportedPlatformPackageProvenance {
        package: package_name.into(),
        version: version.into(),
    })
}

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
        Error::Setup(format!("Failed to fetch package metadata from {main_url}: {e}").into())
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
        Error::Setup(
            format!(
                "Failed to fetch CLI package metadata from {cli_url}: {e}. \
                     Your platform ({platform_suffix}) may not be supported."
            )
            .into(),
        )
    })?;

    // npm registry signatures only prove that registry metadata was signed. The
    // provenance object separately binds the package to its supported build
    // attestation, so reject before exposing the tarball URL to any caller.
    validate_platform_package_provenance(&cli_package_name, version, &cli_meta.dist)?;

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
    use httpmock::prelude::*;

    use super::*;

    const TEST_PACKAGE_NAME: &str = "@voidzero-dev/vite-plus-cli-darwin-arm64";
    const TEST_VERSION: &str = "1.2.3";

    fn parse_metadata(dist: serde_json::Value) -> PackageVersionMetadata {
        serde_json::from_value(serde_json::json!({
            "version": TEST_VERSION,
            "dist": dist,
        }))
        .unwrap()
    }

    fn dist_with_provenance(predicate_type: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "tarball": "https://registry.example.test/platform.tgz",
            "integrity": "sha512-test",
            "signatures": [{ "keyid": "registry-signature-is-not-provenance" }],
            "attestations": {
                "provenance": {
                    "predicateType": predicate_type,
                }
            }
        })
    }

    #[test]
    fn test_cli_package_name_construction() {
        let suffix = "darwin-arm64";
        let name = format!("{PLATFORM_PACKAGE_SCOPE}/{CLI_PACKAGE_NAME_PREFIX}-{suffix}");
        assert_eq!(name, "@voidzero-dev/vite-plus-cli-darwin-arm64");
    }

    #[test]
    fn test_platform_package_accepts_supported_provenance_predicates() {
        for predicate_type in SUPPORTED_PROVENANCE_PREDICATE_TYPES {
            let metadata = parse_metadata(dist_with_provenance(predicate_type.into()));
            assert!(
                validate_platform_package_provenance(
                    TEST_PACKAGE_NAME,
                    TEST_VERSION,
                    &metadata.dist,
                )
                .is_ok(),
                "expected {predicate_type} to be accepted"
            );
        }
    }

    #[test]
    fn test_platform_package_rejects_missing_or_unsupported_provenance() {
        let cases = [
            serde_json::json!({
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test",
            }),
            serde_json::json!({
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test",
                "attestations": {},
            }),
            serde_json::json!({
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test",
                "attestations": { "provenance": {} },
            }),
            dist_with_provenance("".into()),
            dist_with_provenance(" https://slsa.dev/provenance/v1 ".into()),
            dist_with_provenance("https://example.test/unknown-provenance/v1".into()),
            serde_json::json!({
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test",
                "signatures": [{ "keyid": "signature-only" }],
            }),
        ];

        for dist in cases {
            let metadata = parse_metadata(dist);
            let error = validate_platform_package_provenance(
                TEST_PACKAGE_NAME,
                TEST_VERSION,
                &metadata.dist,
            )
            .unwrap_err();

            match error {
                Error::UnsupportedPlatformPackageProvenance { package, version } => {
                    assert_eq!(package.as_str(), TEST_PACKAGE_NAME);
                    assert_eq!(version.as_str(), TEST_VERSION);
                }
                other => panic!("unexpected error: {other:?}"),
            }
        }
    }

    #[test]
    fn test_platform_package_ignores_top_level_attestations() {
        let metadata: PackageVersionMetadata = serde_json::from_value(serde_json::json!({
            "version": TEST_VERSION,
            "attestations": {
                "provenance": {
                    "predicateType": "https://slsa.dev/provenance/v1"
                }
            },
            "dist": {
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test"
            }
        }))
        .unwrap();

        assert!(matches!(
            validate_platform_package_provenance(TEST_PACKAGE_NAME, TEST_VERSION, &metadata.dist,),
            Err(Error::UnsupportedPlatformPackageProvenance { .. })
        ));
    }

    #[test]
    fn test_platform_package_metadata_rejects_malformed_provenance_shape() {
        let result = serde_json::from_value::<PackageVersionMetadata>(serde_json::json!({
            "version": TEST_VERSION,
            "dist": {
                "tarball": "https://registry.example.test/platform.tgz",
                "integrity": "sha512-test",
                "attestations": { "provenance": "not-an-object" }
            }
        }));

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_platform_package_returns_verified_distribution() {
        let server = MockServer::start();
        let metadata_mock = server.mock(|when, then| {
            when.method(GET).path("/@voidzero-dev/vite-plus-cli-darwin-arm64/1.2.3");
            then.status(200).json_body(serde_json::json!({
                "version": TEST_VERSION,
                "dist": dist_with_provenance("https://slsa.dev/provenance/v1".into()),
            }));
        });

        let resolved =
            resolve_platform_package(TEST_VERSION, "darwin-arm64", Some(&server.base_url()))
                .await
                .unwrap();

        metadata_mock.assert();
        assert_eq!(resolved.version, TEST_VERSION);
        assert_eq!(resolved.platform_tarball_url, "https://registry.example.test/platform.tgz");
        assert_eq!(resolved.platform_integrity, "sha512-test");
    }

    #[tokio::test]
    async fn test_resolve_platform_package_rejects_before_returning_distribution() {
        let server = MockServer::start();
        let metadata_mock = server.mock(|when, then| {
            when.method(GET).path("/@voidzero-dev/vite-plus-cli-darwin-arm64/1.2.3");
            then.status(200).json_body(serde_json::json!({
                "version": TEST_VERSION,
                "dist": {
                    "tarball": format!("{}/platform.tgz", server.base_url()),
                    "integrity": "sha512-test",
                }
            }));
        });
        let tarball_mock = server.mock(|when, then| {
            when.method(GET).path("/platform.tgz");
            then.status(200).body("must not be downloaded");
        });

        let error =
            resolve_platform_package(TEST_VERSION, "darwin-arm64", Some(&server.base_url()))
                .await
                .unwrap_err();

        metadata_mock.assert();
        assert_eq!(tarball_mock.hits(), 0);
        assert!(matches!(error, Error::UnsupportedPlatformPackageProvenance { .. }));
        assert!(error.to_string().contains(TEST_PACKAGE_NAME));
        assert!(error.to_string().contains(TEST_VERSION));
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
