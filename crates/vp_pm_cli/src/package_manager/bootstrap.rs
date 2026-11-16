//! Bootstrap setup's pinned pnpm through the shared authenticated downloader.

use serde::Deserialize;
use vp_error::Error;
use vt_path::AbsolutePathBuf;

use super::{
    PackageManagerType, download_package_manager_inner, install_cache_key,
    is_package_manager_install_complete, package_manager_install_dir,
};
use crate::{
    config::NpmConfig, env_forbidden::ensure_managed_package_manager_allowed, request::HttpClient,
};

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
    // Checked before the cache lookup: VP_ENV_FORBIDDEN also forbids cached installs.
    ensure_managed_package_manager_allowed(PackageManagerType::Pnpm)?;
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

#[cfg(test)]
mod tests {
    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    /// `VP_ENV_FORBIDDEN=1` rejects the pinned pnpm before the cache and the
    /// registry; other values still reuse the cached pin.
    #[tokio::test(flavor = "current_thread")]
    async fn env_forbidden_rejects_even_a_cached_pin() {
        use httpmock::prelude::*;

        let home = tempfile::tempdir().unwrap();
        let server = MockServer::start();
        let requests = server.mock(|when, then| {
            when.any_request();
            then.status(500);
        });
        let registry = server.base_url();
        // Any pin works: the cached install below is keyed by it, and no download runs.
        let integrity = format!("sha512.{}", "0".repeat(128));
        let vars = |forbidden: Option<&'static str>| {
            [
                (env_vars::VP_HOME, Some(home.path().as_os_str())),
                (env_vars::VP_ENV_FORBIDDEN, forbidden.map(std::ffi::OsStr::new)),
            ]
        };

        // A complete cached install under the pin's key, as
        // `is_package_manager_install_complete` checks it on every platform.
        let cached = EnvConfig::with_vars_async(vars(None), |_| async {
            let cache_key = install_cache_key(Some(&integrity), None);
            let install = package_manager_install_dir(
                PackageManagerType::Pnpm,
                "10.0.0",
                cache_key.as_deref(),
            );
            let bin = install.join("bin");
            std::fs::create_dir_all(&bin).unwrap();
            for name in ["pnpm", "pnpm.cmd", "pnpm.ps1", "pnpm.cjs"] {
                std::fs::write(bin.join(name), "").unwrap();
            }
            install
        })
        .await;

        for value in [None, Some("true")] {
            EnvConfig::with_vars_async(vars(value), |_| async {
                let install = bootstrap_pnpm("10.0.0", &integrity, Some(&registry)).await.unwrap();
                assert_eq!(install, cached, "{value:?}");
            })
            .await;
        }
        EnvConfig::with_vars_async(vars(Some("1")), |_| async {
            let error = bootstrap_pnpm("10.0.0", &integrity, Some(&registry)).await.unwrap_err();
            assert!(
                matches!(&error, Error::DisabledByEnvForbidden { tool } if tool == "pnpm"),
                "{error}"
            );
            assert!(!error.is_integrity_failure());
        })
        .await;
        requests.assert_hits(0);
    }
}
