//! Package-manager provisioning for the local CLI (the `vite-plus` package).
//!
//! Downloads land in Vite+ directories, so the local CLI downloads only when a
//! global install has opted in. Without one, it uses the package managers on PATH.
//!
//! A system package manager is used as is, even when its version differs from
//! the requested one; the package manager enforces the project's own pin.

use vp_error::Error;
use vt_path::{AbsolutePath, AbsolutePathBuf};

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

/// Provide `kind` the way the global install's mode allows.
pub async fn provide_for_local_cli(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<PackageManager, Error> {
    match Source::resolve(cwd, kind).await? {
        Source::System { executable, provision } => {
            match PackageManager::from_system_executable(kind, &executable, cwd).await {
                Some(package_manager) => Ok(package_manager),
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

/// Like [`provide_for_local_cli`], but returns `None` for a system package manager
/// without running it to read its version.
pub async fn managed_for_local_cli(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<Option<PackageManager>, Error> {
    match Source::resolve(cwd, kind).await? {
        Source::System { .. } => Ok(None),
        Source::Download => download(cwd, kind, version_or_latest, expected_hash).await.map(Some),
    }
}

async fn download(
    cwd: &AbsolutePath,
    kind: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<PackageManager, Error> {
    let (install_dir, _, version) =
        download_package_manager_for_cwd(cwd, kind, version_or_latest, expected_hash).await?;
    Ok(PackageManager::from_install_dir(kind, version, install_dir))
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
}
