use std::collections::HashMap;

use napi::{Error, anyhow, bindgen_prelude::*};
use napi_derive::napi;
use vp_error::Error::{UnrecognizedPackageManager, UnsupportedPackageManager};
use vp_pm_cli::{PackageManagerType, get_package_manager_type_and_version};
use vt_path::AbsolutePathBuf;
use vt_workspace::{Error::PackageJsonNotFound, WorkspaceFile, find_workspace_root};

#[napi(object)]
#[derive(Debug)]
pub struct DownloadPackageManagerOptions {
    pub name: String,
    pub version: String,
    pub expected_hash: Option<String>,
    pub cwd: Option<String>,
    /// Use the npm that ships with Node.js when the requested manager is unavailable.
    /// Set it only when the caller chose the manager itself, as `vp install` does.
    pub fallback_to_npm: Option<bool>,
}

#[napi(object)]
#[derive(Debug)]
pub struct DownloadPackageManagerResult {
    pub name: String,
    pub install_dir: String,
    pub bin_prefix: String,
    pub package_name: String,
    pub version: String,
    /// Executables in `bin_prefix` by name. Launch these, not a bare name that a
    /// Vite+ shim earlier on PATH would resolve to another version.
    pub bin_paths: HashMap<String, String>,
}

#[napi(object)]
pub struct PackageManagerOverride {
    pub name: String,
    pub version: String,
    pub expected_hash: Option<String>,
}

/// Read the same `VP_PACKAGE_MANAGER` override used by `vp install`.
#[napi]
pub fn get_package_manager_override() -> Result<Option<PackageManagerOverride>> {
    Ok(vp_pm_cli::package_manager_override().map_err(anyhow::Error::from)?.map(
        |(name, version, expected_hash)| PackageManagerOverride {
            name: name.to_string(),
            version,
            expected_hash,
        },
    ))
}

/// Download a package manager, or use the one on PATH when no global install manages it.
///
/// See `vp_pm_cli::provide_for_local_cli` for when a download is allowed. The
/// result describes the system installation when one is used.
///
/// ## Parameters
///
/// - `options`: Configuration for the package manager to download, including:
///   - `name`: The name of the package manager
///   - `version`: The version of the package manager
///   - `expected_hash`: The expected hash of the package manager
///   - `cwd`: An absolute directory used to load workspace npm configuration and resolve relative PATH entries
///   - `fallback_to_npm`: Whether to use npm when the requested package manager is unavailable
///
/// ## Returns
///
/// Returns a `DownloadPackageManagerResult` containing:
/// - The name of the package manager, `npm` after a fallback
/// - The install directory of the package manager
/// - The binary prefix of the package manager
/// - The package name of the package manager
/// - The version of the package manager
/// - The executables of the package manager by name
///
/// ## Example
///
/// ```javascript
/// const result = await downloadPackageManager({
///   name: "pnpm",
///   version: "latest",
/// });
/// console.log(`Package manager name: ${result.name}`);
/// console.log(`Package manager install directory: ${result.installDir}`);
/// console.log(`Package manager binary prefix: ${result.binPrefix}`);
/// console.log(`Package manager package name: ${result.packageName}`);
/// console.log(`Package manager version: ${result.version}`);
/// ```
#[napi]
pub async fn download_package_manager(
    options: DownloadPackageManagerOptions,
) -> Result<DownloadPackageManagerResult> {
    let package_manager_type = match options.name.as_str() {
        "pnpm" => PackageManagerType::Pnpm,
        "yarn" => PackageManagerType::Yarn,
        "npm" => PackageManagerType::Npm,
        "bun" => PackageManagerType::Bun,
        _ => {
            return Err(Error::from_reason(format!(
                "Invalid package manager name: {}",
                options.name
            )));
        }
    };

    let cwd = match options.cwd {
        Some(cwd) => AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?,
        None => vt_path::current_dir().map_err(anyhow::Error::from)?,
    };
    let (manager, package_name) = match vp_pm_cli::provide_for_local_cli(
        &cwd,
        package_manager_type,
        &options.version,
        options.expected_hash.as_deref(),
    )
    .await
    {
        Err(
            vp_error::Error::PackageManagerNotInstalled(_)
            | vp_error::Error::PackageManagerNotRunnable { .. },
        ) if options.fallback_to_npm == Some(true) => {
            vp_pm_cli::provide_for_local_cli(&cwd, PackageManagerType::Npm, "latest", None).await
        }
        result => result,
    }
    .map_err(anyhow::Error::from)?;
    let bin_prefix = manager.get_bin_prefix();
    let install_dir =
        bin_prefix.parent().map_or_else(|| bin_prefix.clone(), |dir| dir.to_absolute_path_buf());

    Ok(DownloadPackageManagerResult {
        name: manager.get_client().to_string(),
        install_dir: install_dir.as_path().to_string_lossy().to_string(),
        bin_prefix: bin_prefix.as_path().to_string_lossy().to_string(),
        package_name: package_name.to_string(),
        version: manager.get_version().to_owned(),
        bin_paths: manager
            .bin_paths()
            .into_iter()
            .map(|(name, path)| (name.to_owned(), path.as_path().to_string_lossy().to_string()))
            .collect(),
    })
}

/// List the package managers `downloadPackageManager` can provide.
///
/// Without a global install, only the package managers on PATH are available.
/// Pass the `cwd` that `downloadPackageManager` will receive, so relative PATH
/// entries resolve against the same directory.
#[napi]
pub async fn available_package_managers(cwd: Option<String>) -> Result<Vec<String>> {
    let cwd = match cwd {
        Some(cwd) => AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?,
        None => vt_path::current_dir().map_err(anyhow::Error::from)?,
    };
    let mut available = Vec::new();
    for kind in [
        PackageManagerType::Pnpm,
        PackageManagerType::Yarn,
        PackageManagerType::Npm,
        PackageManagerType::Bun,
    ] {
        if vp_pm_cli::is_available_for_local_cli(&cwd, kind).await.map_err(anyhow::Error::from)? {
            available.push(kind.to_string());
        }
    }
    Ok(available)
}

#[napi(object)]
#[derive(Debug)]
pub struct DetectWorkspaceResult {
    pub package_manager_name: Option<String>,
    pub package_manager_version: Option<String>,
    pub package_manager_hash: Option<String>,
    pub is_monorepo: bool,
    pub root: Option<String>,
}

/// Detect the workspace root and package manager type and version
///
/// ## Parameters
///
/// - `cwd`: The current working directory to detect the workspace root
///
/// ## Returns
///
/// Returns a `DetectWorkspaceResult` containing:
/// - The name of the package manager
/// - The version of the package manager
/// - The integrity hash from the package manager pin, if present
/// - Whether the workspace is a monorepo
/// - The workspace root, where the package.json file is located.
///
/// ## Example
///
/// ```javascript
/// const result = await detectWorkspace("/path/to/workspace");
/// console.log(`Package manager name: ${result.packageManagerName}`);
/// console.log(`Package manager version: ${result.packageManagerVersion}`);
/// console.log(`Is monorepo: ${result.isMonorepo}`);
/// console.log(`Workspace root: ${result.root}`);
/// ```
#[napi]
pub async fn detect_workspace(cwd: String) -> Result<DetectWorkspaceResult> {
    let cwd = AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?;
    let (workspace_root, _relative_path) = match find_workspace_root(&cwd) {
        Ok(result) => result,
        Err(PackageJsonNotFound(_)) => {
            return Ok(DetectWorkspaceResult {
                package_manager_name: None,
                package_manager_version: None,
                package_manager_hash: None,
                is_monorepo: false,
                root: None,
            });
        }
        Err(e) => {
            return Err(anyhow::Error::from(e).into());
        }
    };

    let is_monorepo = matches!(
        workspace_root.workspace_file,
        WorkspaceFile::PnpmWorkspaceYaml(_) | WorkspaceFile::NpmWorkspaceJson(_)
    );
    let workspace_root_path = workspace_root.path.as_path().to_string_lossy().to_string();

    match get_package_manager_type_and_version(&workspace_root, None) {
        Ok((package_manager_type, version, hash, _)) => Ok(DetectWorkspaceResult {
            package_manager_name: Some(package_manager_type.to_string()),
            package_manager_version: Some(version.to_string()),
            package_manager_hash: hash.map(|hash| hash.to_string()),
            is_monorepo,
            root: Some(workspace_root_path),
        }),
        Err(UnsupportedPackageManager(_) | UnrecognizedPackageManager) => {
            Ok(DetectWorkspaceResult {
                package_manager_name: None,
                package_manager_version: None,
                package_manager_hash: None,
                is_monorepo,
                root: Some(workspace_root_path),
            })
        }
        Err(e) => {
            return Err(anyhow::Error::from(e).into());
        }
    }
}

/// Find the monorepo root containing `cwd` with the same rules as `vp run` and `vp install`.
///
/// Returns `null` when `cwd` is not inside a pnpm or `package.json` workspace.
#[napi]
pub fn find_monorepo_root(cwd: String) -> Result<Option<String>> {
    let cwd = AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?;
    match find_workspace_root(&cwd) {
        Ok((workspace_root, _)) => match workspace_root.workspace_file {
            WorkspaceFile::PnpmWorkspaceYaml(_) | WorkspaceFile::NpmWorkspaceJson(_) => {
                Ok(Some(workspace_root.path.as_path().to_string_lossy().to_string()))
            }
            WorkspaceFile::NonWorkspacePackage(_) => Ok(None),
        },
        Err(PackageJsonNotFound(_)) => Ok(None),
        Err(e) => Err(anyhow::Error::from(e).into()),
    }
}

/// Resolve the npm registry for packages in `scope` (without `@`) from the npm config at `cwd`.
#[napi]
pub fn resolve_npm_registry(cwd: String, scope: Option<String>) -> Result<String> {
    let cwd = AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?;
    Ok(vp_pm_cli::npm_registry_for_scope(&cwd, scope.as_deref()))
}

/// Resolve the `Authorization` header value npm would send to `url` from the npm config at `cwd`.
#[napi]
pub fn resolve_npm_authorization(cwd: String, url: String) -> Result<Option<String>> {
    let cwd = AbsolutePathBuf::new(cwd.into()).ok_or(Error::from_reason("invalid cwd"))?;
    Ok(vp_pm_cli::npm_authorization_for_url(&cwd, &url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn download_package_manager_rejects_relative_cwd() {
        let error = download_package_manager(DownloadPackageManagerOptions {
            name: "pnpm".to_string(),
            version: "latest".to_string(),
            expected_hash: None,
            cwd: Some("relative/project".to_string()),
            fallback_to_npm: None,
        })
        .await
        .unwrap_err();
        assert_eq!(error.reason, "invalid cwd");
    }

    /// Without a global install, a missing pnpm falls back to the npm on PATH only when asked.
    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn download_package_manager_falls_back_to_npm_on_request() {
        use std::os::unix::fs::PermissionsExt;

        let home = tempfile::tempdir().unwrap();
        let bin = home.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let npm = bin.join("npm");
        std::fs::write(&npm, "#!/bin/sh\necho 10.9.2\n").unwrap();
        std::fs::set_permissions(&npm, std::fs::Permissions::from_mode(0o755)).unwrap();
        let download = |fallback_to_npm| {
            download_package_manager(DownloadPackageManagerOptions {
                name: "pnpm".to_string(),
                version: "latest".to_string(),
                expected_hash: None,
                cwd: Some(home.path().to_string_lossy().to_string()),
                fallback_to_npm: Some(fallback_to_npm),
            })
        };

        vp_shared::EnvConfig::with_vars_async(
            [
                (vp_shared::env_vars::VP_HOME, Some(home.path().join("vp-home").as_os_str())),
                ("PATH", Some(bin.as_os_str())),
            ],
            |_| async {
                let error = download(false).await.unwrap_err();
                assert!(error.reason.contains("No usable pnpm"), "{}", error.reason);

                let result = download(true).await.unwrap();
                assert_eq!((result.name.as_str(), result.version.as_str()), ("npm", "10.9.2"));
                assert_eq!(result.bin_paths["npm"], npm.to_string_lossy());
            },
        )
        .await;
    }
}
