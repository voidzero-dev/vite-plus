//! Shared helpers used by every PM handler.

use vt_path::AbsolutePath;

use crate::{PackageManager, PackageManagerType, error::Error};

/// Build a `PackageManager`, falling back to a default npm instance when no
/// package.json is found. Uses `build()` instead of `build_with_default()`
/// to skip the interactive package manager selection prompt on the fallback path.
///
/// Callers should ensure npm is on PATH before invoking commands that hit
/// this fallback (the global CLI does this via its managed Node runtime;
/// the local CLI relies on the system Node).
pub async fn build_package_manager_or_npm_default(
    cwd: &AbsolutePath,
) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build().await {
        Ok(pm) => Ok(pm),
        Err(vp_error::Error::WorkspaceError(vt_workspace::Error::PackageJsonNotFound(_)))
        | Err(vp_error::Error::UnrecognizedPackageManager) => Ok(default_npm_package_manager(cwd)),
        Err(e) => Err(Error::Install(e)),
    }
}

pub(crate) fn default_npm_package_manager(cwd: &AbsolutePath) -> PackageManager {
    PackageManager {
        client: PackageManagerType::Npm,
        version: "latest".into(),
        bin_prefix: cwd.join("bin"),
    }
}
