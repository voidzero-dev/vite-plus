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
    user_config::load_config_if_present,
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
            match PackageManager::from_system_executable(kind, &executable).await {
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
