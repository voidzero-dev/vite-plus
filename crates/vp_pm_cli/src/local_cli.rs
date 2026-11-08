//! Package-manager provisioning for the local CLI (the `vite-plus` package).
//!
//! Downloads land in Vite+ directories, so the local CLI downloads only when a
//! global install has opted in. Without one, it uses the package managers on PATH.
//!
//! A system package manager is used as is, even when its version differs from
//! the requested one; the package manager enforces the project's own pin.

use vp_error::Error;
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;

use crate::{
    PackageManager, PackageManagerType, ShimMode, download_package_manager_for_cwd,
    ensure_managed_package_manager_allowed, user_config::load_config_if_present,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provision {
    /// The global install manages this family.
    Managed,
    /// The global install prefers the system family and falls back to a download.
    SystemFirst,
    /// No global install has written a config, so nothing may be downloaded.
    SystemOnly,
}

impl Provision {
    async fn for_family(kind: PackageManagerType) -> Result<Self, Error> {
        let Some(config) = load_config_if_present::<Error>().await? else {
            return Ok(Self::SystemOnly);
        };
        Ok(match config.package_manager_shim_mode_for(kind) {
            ShimMode::Managed => Self::Managed,
            ShimMode::SystemFirst => Self::SystemFirst,
        })
    }
}

enum Source {
    System { executable: AbsolutePathBuf, provision: Provision },
    Download,
}

impl Source {
    async fn resolve(cwd: &AbsolutePath, kind: PackageManagerType) -> Result<Self, Error> {
        let provision = Provision::for_family(kind).await?;
        if provision != Provision::Managed
            && let Some(executable) = find_system_executable(cwd, kind)
        {
            // A parent process may have put a managed install on PATH.
            if vp_command::is_managed_install(executable.as_path()) {
                ensure_managed_package_manager_allowed(kind)?;
            }
            return Ok(Self::System { executable, provision });
        }
        if provision == Provision::SystemOnly {
            return Err(Error::PackageManagerNotInstalled(kind.to_string().into()));
        }
        Ok(Self::Download)
    }
}

/// Provide `kind` the way the global install's mode allows, with the registry
/// package that supplied a download (such as `@yarnpkg/cli-dist`).
pub async fn provide_for_local_cli(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<(PackageManager, Str), Error> {
    match Source::resolve(cwd, kind).await? {
        Source::System { executable, provision } => {
            match PackageManager::from_system_executable(kind, &executable, cwd).await {
                Some(package_manager) => Ok((package_manager, kind.to_string().into())),
                // Like the global CLI, system-first falls back to a download when the system one fails.
                None if provision == Provision::SystemFirst => {
                    download(cwd, kind, version_or_latest, expected_hash).await
                }
                None => Err(Error::PackageManagerNotRunnable {
                    name: kind.to_string().into(),
                    executable,
                }),
            }
        }
        Source::Download => download(cwd, kind, version_or_latest, expected_hash).await,
    }
}

/// Like [`provide_for_local_cli`], but returns `None` for a usable system package
/// manager. Only system-first mode runs it to check.
pub async fn managed_for_local_cli(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<Option<PackageManager>, Error> {
    if let Source::System { executable, provision } = Source::resolve(cwd, kind).await? {
        // Only system-first can fall back, so only it pays for running the system one.
        if provision != Provision::SystemFirst
            || PackageManager::from_system_executable(kind, &executable, cwd).await.is_some()
        {
            return Ok(None);
        }
    }
    download(cwd, kind, version_or_latest, expected_hash).await.map(|(pm, _)| Some(pm))
}

/// Whether [`provide_for_local_cli`] can provide `kind`: it is on PATH, or the global install allows a download.
pub async fn is_available_for_local_cli(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
) -> Result<bool, Error> {
    match Source::resolve(cwd, kind).await {
        Ok(_) => Ok(true),
        Err(Error::PackageManagerNotInstalled(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

async fn download(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<(PackageManager, Str), Error> {
    let (install_dir, package_name, version) =
        download_package_manager_for_cwd(cwd, kind, version_or_latest, expected_hash).await?;
    Ok((PackageManager::from_install_dir(kind, version, install_dir), package_name))
}

fn find_system_executable(cwd: &AbsolutePath, kind: PackageManagerType) -> Option<AbsolutePathBuf> {
    let path = std::env::var_os("PATH")?;
    // Skip Vite+ shims so the result is the system installation itself.
    let query = vp_command::ExternalToolQuery { path: &path, bypass: &[], self_exe: None };
    vp_command::find_external_tool(&kind.to_string(), &query, cwd)
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsStr;

    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    /// Under `VP_ENV_FORBIDDEN=1`, a managed pnpm that a parent put on PATH is
    /// rejected; otherwise it is reused like a system one.
    #[tokio::test(flavor = "current_thread")]
    async fn managed_install_on_path_is_rejected_when_env_forbidden() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        let bin = home.path().join("package_manager/pnpm/10.0.0/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let pnpm = bin.join("pnpm");
        std::fs::write(&pnpm, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&pnpm, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cwd = AbsolutePathBuf::new(home.path().to_path_buf()).unwrap();
        let resolve = |forbidden: Option<&'static str>| {
            let vars = [
                (env_vars::VP_HOME, Some(home.path().as_os_str())),
                ("PATH", Some(bin.as_os_str())),
                (env_vars::VP_ENV_FORBIDDEN, forbidden.map(OsStr::new)),
            ];
            EnvConfig::with_vars_async(vars, |_| Source::resolve(&cwd, PackageManagerType::Pnpm))
        };

        let error = resolve(Some("1")).await.err().expect("the managed pnpm is rejected");
        assert!(
            matches!(&error, Error::DisabledByEnvForbidden { tool } if tool == "pnpm"),
            "{error:?}"
        );
        let source = resolve(None).await.expect("the managed pnpm is reused");
        assert!(
            matches!(source, Source::System { executable, .. } if executable.as_path() == pnpm)
        );
    }

    /// In system-first mode, a system pnpm that fails to run is replaced by a download.
    #[tokio::test(flavor = "current_thread")]
    async fn system_first_downloads_when_the_system_one_fails() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join("config.json"),
            r#"{"packageManagerShimModes":{"pnpm":"system_first"}}"#,
        )
        .unwrap();
        let cwd = AbsolutePathBuf::new(home.path().to_path_buf()).unwrap();
        // The registry has nothing, so a download attempt shows up as an error.
        // Keep the server so the pool cannot hand it to a parallel test.
        let server = httpmock::MockServer::start_async().await;
        let registry = server.base_url();
        let managed = |script: &'static str| {
            let bin = home.path().join(if script.contains("exit") { "broken" } else { "working" });
            std::fs::create_dir_all(&bin).unwrap();
            let pnpm = bin.join("pnpm");
            std::fs::write(&pnpm, script).unwrap();
            std::fs::set_permissions(&pnpm, std::fs::Permissions::from_mode(0o755)).unwrap();
            let vars = [
                (env_vars::VP_HOME, Some(home.path().as_os_str().to_owned())),
                ("PATH", Some(bin.into_os_string())),
                ("NPM_CONFIG_REGISTRY", Some(registry.clone().into())),
            ];
            EnvConfig::with_vars_async(vars, |_| {
                managed_for_local_cli(&cwd, PackageManagerType::Pnpm, "10.18.0", None)
            })
        };

        let working = managed("#!/bin/sh\necho 10.18.0\n").await;
        assert!(matches!(working, Ok(None)), "the working system pnpm is used: {working:?}");
        let broken = managed("#!/bin/sh\nexit 1\n").await;
        assert!(broken.is_err(), "the broken system pnpm is replaced by a download: {broken:?}");
    }
}
