//! Bootstrap setup's pinned pnpm through the shared authenticated downloader.

use serde::Deserialize;
use vp_error::Error;
use vt_path::AbsolutePathBuf;

use super::{
    PackageManagerType, download_package_manager_inner, is_package_manager_install_complete,
    package_manager_install_dir,
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
    integrity: String,
}

/// Keep the normal cache, authentication, extraction, and installation lock.
/// Metadata supplies the registry's tarball URL and integrity for the exact pin.
pub async fn bootstrap_pnpm(
    version: &str,
    registry: Option<&str>,
) -> Result<AbsolutePathBuf, Error> {
    let parsed = semver::Version::parse(version)
        .map_err(|_| Error::InvalidArgument("pnpm bootstrap requires an exact version".into()))?;
    if parsed.major >= 12 {
        return Err(Error::InvalidArgument(
            "pnpm bootstrap requires a JavaScript pnpm release".into(),
        ));
    }
    let install = package_manager_install_dir(PackageManagerType::Pnpm, version)
        .expect("pnpm has an install directory");
    if is_package_manager_install_complete(&install, "pnpm")?
        && install.join("bin/pnpm.cjs").as_path().is_file()
    {
        return Ok(install);
    }

    let mut config = NpmConfig::load();
    if let Some(registry) = registry {
        config.values.insert("registry".to_string(), registry.to_string());
    }
    let metadata: Metadata = HttpClient::with_npm_config(3, 500, config.clone())
        .get_json(&config.package_version_url("pnpm", version))
        .await?;
    if metadata.name != "pnpm" || metadata.version != version || metadata.dist.integrity.is_empty()
    {
        return Err(Error::InvalidArgument("Registry returned an unexpected pnpm package".into()));
    }
    let (install, _, _) = download_package_manager_inner(
        None,
        PackageManagerType::Pnpm,
        version,
        Some(&metadata.dist.integrity),
        &config,
        Some(&metadata.dist.tarball),
    )
    .await?;
    Ok(install)
}
