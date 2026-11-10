//! Bootstrap setup's pinned pnpm through the shared authenticated downloader.

use serde::Deserialize;
use vp_error::Error;
use vt_path::AbsolutePathBuf;

use super::{
    PackageManagerType, download_package_manager_inner, install_cache_key,
    is_package_manager_install_complete, package_manager_install_dir,
};
use crate::{config::NpmConfig, request::HttpClient};

#[derive(Deserialize)]
struct Metadata {
    name: String,
    version: String,
    dist: Distribution,
}

#[derive(Deserialize)]
struct Distribution {
    tarball: String,
}

/// Keep the normal cache, authentication, extraction, and installation lock.
/// Metadata supplies the registry's tarball URL for the exact pin.
///
/// `expected_integrity` is the `packageManager`-format hash (`sha512.<hex>`)
/// that the caller pins in the binary. The download is verified against it, not
/// against any value the registry reports. A project `.npmrc` may point the
/// registry at any host, so the registry's own integrity cannot be trusted. A
/// genuine registry or mirror serves identical bytes and still matches the pin.
pub async fn bootstrap_pnpm(
    version: &str,
    expected_integrity: &str,
    registry: Option<&str>,
) -> Result<AbsolutePathBuf, Error> {
    let parsed = semver::Version::parse(version)
        .map_err(|_| Error::InvalidArgument("pnpm bootstrap requires an exact version".into()))?;
    if parsed.major >= 12 {
        return Err(Error::InvalidArgument(
            "pnpm bootstrap requires a JavaScript pnpm release".into(),
        ));
    }
    // A JavaScript pnpm release is a tarball manager, so its pinned install is
    // key-addressed by the pin alone (no platform origin). A complete keyed
    // install holds content verified against this pin; reuse it. An install a
    // different pin produced lives under a different key and is not reused, so it
    // triggers a fresh, verified download.
    let cache_key = install_cache_key(Some(expected_integrity), None);
    let install =
        package_manager_install_dir(PackageManagerType::Pnpm, version, cache_key.as_deref());
    if is_package_manager_install_complete(&install, "pnpm")? {
        return Ok(install);
    }

    let mut config = NpmConfig::load();
    if let Some(registry) = registry {
        config.set_registry(registry);
    }
    let metadata: Metadata = HttpClient::with_npm_config(3, 500, config.clone())
        .get_json(&config.package_version_url("pnpm", version))
        .await?;
    if metadata.name != "pnpm" || metadata.version != version {
        return Err(Error::InvalidArgument("Registry returned an unexpected pnpm package".into()));
    }
    let (install, _, _) = download_package_manager_inner(
        None,
        PackageManagerType::Pnpm,
        version,
        Some(expected_integrity),
        &config,
        Some(&metadata.dist.tarball),
    )
    .await?;
    Ok(install)
}
