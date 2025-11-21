use napi::{Error, anyhow, bindgen_prelude::*};
use napi_derive::napi;
use vite_error::Error::{UnrecognizedPackageManager, UnsupportedPackageManager};
use vite_install::{PackageManagerType, get_package_manager_type_and_version};
use vite_path::AbsolutePathBuf;
use vite_workspace::{Error::PackageJsonNotFound, WorkspaceFile, find_workspace_root};

#[napi(object)]
#[derive(Debug)]
pub struct DownloadPackageManagerOptions {
    pub name: String,
    pub version: String,
    pub expected_hash: Option<String>,
}

#[napi(object)]
#[derive(Debug)]
pub struct DownloadPackageManagerResult {
    pub name: String,
    pub install_dir: String,
    pub bin_prefix: String,
    pub package_name: String,
    pub version: String,
}

/// Download a package manager
///
/// ## Parameters
///
/// - `options`: Configuration for the package manager to download, including:
///   - `name`: The name of the package manager
///   - `version`: The version of the package manager
///   - `expected_hash`: The expected hash of the package manager
///
/// ## Returns
///
/// Returns a `DownloadPackageManagerResult` containing:
/// - The name of the package manager
/// - The install directory of the package manager
/// - The binary prefix of the package manager
/// - The package name of the package manager
/// - The version of the package manager
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
        _ => {
            return Err(Error::from_reason(format!(
                "Invalid package manager name: {}",
                options.name
            )));
        }
    };

    let (install_dir, package_name, version) = vite_install::download_package_manager(
        package_manager_type,
        &options.version,
        options.expected_hash.as_deref(),
    )
    .await
    .map_err(anyhow::Error::from)?;

    Ok(DownloadPackageManagerResult {
        name: options.name,
        install_dir: install_dir.as_path().to_string_lossy().to_string(),
        bin_prefix: install_dir.join("bin").as_path().to_string_lossy().to_string(),
        package_name: package_name.to_string(),
        version: version.to_string(),
    })
}

#[napi(object)]
#[derive(Debug)]
pub struct DetectWorkspaceResult {
    pub package_manager_name: Option<String>,
    pub package_manager_version: Option<String>,
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
    let workspace_root = match find_workspace_root(&cwd) {
        Ok(workspace_root) => workspace_root,
        Err(PackageJsonNotFound(_)) => {
            return Ok(DetectWorkspaceResult {
                package_manager_name: None,
                package_manager_version: None,
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
        Ok((package_manager_type, version, _)) => Ok(DetectWorkspaceResult {
            package_manager_name: Some(package_manager_type.to_string()),
            package_manager_version: Some(version.to_string()),
            is_monorepo,
            root: Some(workspace_root_path),
        }),
        Err(UnsupportedPackageManager(_) | UnrecognizedPackageManager) => {
            Ok(DetectWorkspaceResult {
                package_manager_name: None,
                package_manager_version: None,
                is_monorepo,
                root: Some(workspace_root_path),
            })
        }
        Err(e) => {
            return Err(anyhow::Error::from(e).into());
        }
    }
}
