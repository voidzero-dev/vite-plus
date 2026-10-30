//! npm registry client for version resolution.
//!
//! Queries the npm registry to resolve versions and get tarball URLs
//! with integrity hashes for both the main package and platform-specific package.

use serde::{Deserialize, Deserializer, de};
use vp_pm_cli::{HttpClient, npm_registry};

use crate::{error::Error, is_commit_preview_version};

/// npm package version metadata (subset of fields we need).
#[derive(Debug, Deserialize)]
pub struct PackageVersionMetadata {
    pub version: String,
    pub dist: DistInfo,
    #[serde(default, rename = "vpInstallLockfile")]
    pub has_install_lockfile: bool,
    #[serde(default, rename = "vpNodeVersion", deserialize_with = "deserialize_node_version")]
    pub node_version: Option<String>,
}

fn deserialize_node_version<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    let version = String::deserialize(deserializer)?;
    let is_exact = js_semver::Version::parse(&version).is_ok_and(|parsed| {
        parsed.pre_release.is_empty() && parsed.build.is_empty() && parsed.to_string() == version
    });
    if !is_exact {
        return Err(de::Error::custom("vpNodeVersion must be an exact Node.js version (x.y.z)"));
    }
    Ok(Some(version))
}

/// Distribution info from npm registry.
#[derive(Debug, Deserialize)]
pub struct DistInfo {
    pub tarball: String,
    pub integrity: String,
    pub attestations: Option<NpmAttestations>,
}

/// npm attestations attached to a package version.
#[derive(Debug, Deserialize)]
pub struct NpmAttestations {
    pub provenance: Option<NpmProvenance>,
}

/// npm provenance metadata used to identify the attestation predicate.
#[derive(Debug, Deserialize)]
pub struct NpmProvenance {
    #[serde(rename = "predicateType")]
    pub predicate_type: Option<String>,
}

/// Resolved version info with URLs and integrity for the platform package.
#[derive(Debug)]
pub struct ResolvedVersion {
    pub version: String,
    pub platform_tarball_url: String,
    pub platform_integrity: String,
    pub has_install_lockfile: bool,
    pub node_version: Option<String>,
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
    // Commit preview builds do not carry npm provenance, regardless of registry.
    if is_commit_preview_version(version) {
        return Ok(());
    }

    let predicate_type = dist
        .attestations
        .as_ref()
        .and_then(|attestations| attestations.provenance.as_ref())
        .and_then(|provenance| provenance.predicate_type.as_deref());

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

    // Check release provenance metadata before returning the tarball URL.
    validate_platform_package_provenance(&cli_package_name, version, &cli_meta.dist)?;

    Ok(ResolvedVersion {
        version: version.to_owned(),
        platform_tarball_url: cli_meta.dist.tarball,
        platform_integrity: cli_meta.dist.integrity,
        has_install_lockfile: cli_meta.has_install_lockfile,
        node_version: cli_meta.node_version,
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

    fn dist_with_provenance(predicate_type: &str) -> serde_json::Value {
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
    fn test_platform_package_accepts_supported_provenance_predicates() {
        for predicate_type in SUPPORTED_PROVENANCE_PREDICATE_TYPES {
            let metadata = parse_metadata(dist_with_provenance(predicate_type));
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
            dist_with_provenance(""),
            dist_with_provenance(" https://slsa.dev/provenance/v1 "),
            dist_with_provenance("https://example.test/unknown-provenance/v1"),
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

    #[test]
    fn test_platform_node_version_rejects_non_exact_versions() {
        for node_version in serde_json::json!([
            null,
            24,
            false,
            {},
            "",
            "lts",
            "latest",
            "24",
            "24.22",
            "^24.22.1",
            "v24.22.1",
            " 24.22.1",
            "24.22.1\n",
            "24.22.1-rc.1",
            "24.22.1+build",
            "024.22.1"
        ])
        .as_array()
        .unwrap()
        {
            let metadata = serde_json::json!({
                "version": TEST_VERSION,
                "dist": dist_with_provenance("https://slsa.dev/provenance/v1"),
                "vpNodeVersion": node_version,
            });
            assert!(
                serde_json::from_value::<PackageVersionMetadata>(metadata).is_err(),
                "accepted invalid vpNodeVersion: {node_version}",
            );
        }
    }

    #[tokio::test]
    async fn test_resolve_node_version_from_exact_platform_metadata() {
        for node_version in [Some("22.23.1"), None] {
            let server = MockServer::start();
            let main_mock = server.mock(|when, then| {
                when.method(GET).path("/vite-plus/latest");
                then.status(200).json_body(serde_json::json!({
                    "version": TEST_VERSION,
                    "dist": dist_with_provenance("https://slsa.dev/provenance/v1"),
                    "vpNodeVersion": "24.22.1",
                }));
            });
            let mut platform_metadata = serde_json::json!({
                "version": TEST_VERSION,
                "dist": dist_with_provenance("https://slsa.dev/provenance/v1"),
            });
            if let Some(node_version) = node_version {
                platform_metadata["vpNodeVersion"] = node_version.into();
            }
            let platform_mock = server.mock(|when, then| {
                when.method(GET).path(format!("/{TEST_PACKAGE_NAME}/{TEST_VERSION}"));
                then.status(200).json_body(platform_metadata);
            });

            let resolved =
                resolve_version("latest", "darwin-arm64", Some(&server.base_url())).await.unwrap();

            main_mock.assert_calls(1);
            platform_mock.assert_calls(1);
            assert_eq!(resolved.version, TEST_VERSION);
            assert_eq!(resolved.node_version.as_deref(), node_version);
        }
    }

    #[tokio::test]
    async fn test_resolve_platform_package_returns_verified_distribution() {
        let server = MockServer::start();
        let metadata_mock = server.mock(|when, then| {
            when.method(GET).path("/@voidzero-dev/vite-plus-cli-darwin-arm64/1.2.3");
            then.status(200).json_body(serde_json::json!({
                "version": TEST_VERSION,
                "dist": dist_with_provenance("https://slsa.dev/provenance/v1"),
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
        assert!(resolved.node_version.is_none());
    }

    #[tokio::test]
    async fn test_resolve_preview_without_provenance_from_custom_registry() {
        let version = "0.0.0-commit.0123456789abcdef0123456789abcdef01234567";
        let server = MockServer::start();
        let metadata = serde_json::json!({
            "version": version,
            "vpNodeVersion": "24.22.1",
            "dist": {
                "tarball": format!("{}/platform.tgz", server.base_url()),
                "integrity": "sha512-preview",
            },
        });
        let main_mock = server.mock(|when, then| {
            when.method(GET).path("/vite-plus/preview");
            then.status(200).json_body(metadata.clone());
        });
        let platform_mock = server.mock(|when, then| {
            when.method(GET).path(format!("/{TEST_PACKAGE_NAME}/{version}"));
            then.status(200).json_body(metadata);
        });

        let resolved =
            resolve_version("preview", "darwin-arm64", Some(&server.base_url())).await.unwrap();

        main_mock.assert();
        platform_mock.assert();
        assert_eq!(resolved.version, version);
        assert_eq!(resolved.platform_tarball_url, format!("{}/platform.tgz", server.base_url()));
        assert_eq!(resolved.platform_integrity, "sha512-preview");
        assert_eq!(resolved.node_version.as_deref(), Some("24.22.1"));
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
}
