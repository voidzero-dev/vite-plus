use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tokio::fs::remove_dir_all;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf, RelativePathBuf};
use vite_str::Str;

use crate::{
    config::{get_cache_dir, get_npm_package_tgz_url, get_npm_package_version_url},
    request::{HttpClient, download_and_extract_tgz},
    shim,
};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct PackageJson {
    #[serde(default)]
    pub version: Str,
    #[serde(default)]
    pub package_manager: Str,
}

/// The package manager type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManagerType {
    Pnpm,
    Yarn,
    Npm,
}

impl fmt::Display for PackageManagerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pnpm => write!(f, "pnpm"),
            Self::Yarn => write!(f, "yarn"),
            Self::Npm => write!(f, "npm"),
        }
    }
}

// TODO(@fengmk2): should move ResolveCommandResult to vite-common crate
#[derive(Debug)]
pub struct ResolveCommandResult {
    pub bin_path: String,
    pub envs: HashMap<String, String>,
}

/// The package manager.
/// Use `PackageManager::builder()` to create a package manager.
/// Then use `PackageManager::resolve_command()` to resolve the command result.
#[derive(Debug)]
pub struct PackageManager {
    pub package_manager_type: PackageManagerType,
    pub package_name: Str,
    pub version: Str,
    pub bin_name: Str,
    pub workspace_root: AbsolutePathBuf,
    pub install_dir: AbsolutePathBuf,
}

#[derive(Debug)]
pub struct PackageManagerBuilder {
    package_manager_type: Option<PackageManagerType>,
    cwd: AbsolutePathBuf,
}

impl PackageManagerBuilder {
    pub fn new(cwd: impl AsRef<AbsolutePath>) -> Self {
        Self { package_manager_type: None, cwd: cwd.as_ref().to_absolute_path_buf() }
    }

    #[must_use]
    pub const fn package_manager_type(mut self, package_manager_type: PackageManagerType) -> Self {
        self.package_manager_type = Some(package_manager_type);
        self
    }

    /// Build the package manager.
    /// Detect the package manager from the current working directory.
    pub async fn build(self) -> Result<PackageManager, Error> {
        let workspace_root = find_workspace_root(&self.cwd)?;
        let (package_manager_type, mut version) =
            get_package_manager_type_and_version(&workspace_root, self.package_manager_type)?;

        let mut package_name = package_manager_type.to_string();
        let mut should_update_package_manager_field = false;

        if version == "latest" {
            version = get_latest_version(package_manager_type).await?;
            should_update_package_manager_field = true;
        }

        // handle yarn >= 2.0.0 to use `@yarnpkg/cli-dist` as package name
        // @see https://github.com/nodejs/corepack/blob/main/config.json#L135
        if matches!(package_manager_type, PackageManagerType::Yarn) {
            let version_req = VersionReq::parse(">=2.0.0")?;
            if version_req.matches(&Version::parse(&version)?) {
                package_name = "@yarnpkg/cli-dist".to_string();
            }
        }

        // only download the package manager if it's not already downloaded
        let install_dir =
            download_package_manager(package_manager_type, &package_name, &version).await?;

        if should_update_package_manager_field {
            // auto set `packageManager` field in package.json
            let package_json_path = workspace_root.path.join("package.json");
            set_package_manager_field(&package_json_path, package_manager_type, &version).await?;
        }

        Ok(PackageManager {
            package_manager_type,
            package_name: package_name.into(),
            version,
            bin_name: package_manager_type.to_string().into(),
            workspace_root: workspace_root.path.to_absolute_path_buf(),
            install_dir,
        })
    }
}

impl PackageManager {
    pub fn builder(workspace_root: impl AsRef<AbsolutePath>) -> PackageManagerBuilder {
        PackageManagerBuilder::new(workspace_root)
    }

    #[must_use]
    pub fn get_bin_prefix(&self) -> AbsolutePathBuf {
        self.install_dir.join("bin")
    }

    #[must_use]
    pub fn resolve_command(&self) -> ResolveCommandResult {
        ResolveCommandResult {
            bin_path: self.bin_name.to_string(),
            envs: HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]),
        }
    }
}

/// The package root directory and its package.json file.
#[derive(Debug)]
pub struct PackageRoot<'a> {
    pub path: &'a AbsolutePath,
    pub cwd: RelativePathBuf,
    pub package_json: File,
}

/// Find the package root directory from the current working directory. `original_cwd` must be absolute.
///
/// If the package.json file is not found, will return `PackageJsonNotFound` error.
pub fn find_package_root(original_cwd: &AbsolutePath) -> Result<PackageRoot<'_>, Error> {
    let mut cwd = original_cwd;
    loop {
        // Check for package.json
        if let Some(file) = open_exists_file(cwd.join("package.json"))? {
            return Ok(PackageRoot {
                path: cwd,
                cwd: original_cwd.strip_prefix(cwd)?.expect("cwd must be within the package root"),
                package_json: file,
            });
        }

        if let Some(parent) = cwd.parent() {
            // Move up one directory
            cwd = parent;
        } else {
            // We've reached the root, return PackageJsonNotFound error.
            return Err(Error::PackageJsonNotFound(original_cwd.to_absolute_path_buf()));
        }
    }
}

/// The workspace file.
///
/// - `PnpmWorkspaceYaml` is the pnpm workspace file.
/// - `NpmWorkspaceJson` is the package.json file of a yarn/npm workspace.
/// - `NonWorkspacePackage` is the package.json file of a non-workspace package.
#[derive(Debug)]
pub enum WorkspaceFile {
    /// The pnpm-workspace.yaml file of a pnpm workspace.
    PnpmWorkspaceYaml(File),
    /// The package.json file of a yarn/npm workspace.
    NpmWorkspaceJson(File),
    /// The package.json file of a non-workspace package.
    NonWorkspacePackage(File),
}

/// The workspace root directory and its workspace file.
///
/// If the workspace file is not found, but a package is found, `workspace_file` will be `NonWorkspacePackage` with the `package.json` File.
#[derive(Debug)]
pub struct WorkspaceRoot<'a> {
    pub path: &'a AbsolutePath,
    pub cwd: RelativePathBuf,
    pub workspace_file: WorkspaceFile,
}

/// Find the workspace root directory from the current working directory. `original_cwd` must be absolute.
///
/// If the workspace file is not found, but a package is found, `workspace_file` will be `NonWorkspacePackage` with the `package.json` File.
///
/// If neither workspace nor package is found, will return `PackageJsonNotFound` error.
pub fn find_workspace_root(original_cwd: &AbsolutePath) -> Result<WorkspaceRoot<'_>, Error> {
    let mut cwd = original_cwd;

    loop {
        // Check for pnpm-workspace.yaml for pnpm workspace
        if let Some(file) = open_exists_file(cwd.join("pnpm-workspace.yaml"))? {
            return Ok(WorkspaceRoot {
                path: cwd,
                cwd: original_cwd
                    .strip_prefix(cwd)?
                    .expect("cwd must be within the pnpm workspace"),
                workspace_file: WorkspaceFile::PnpmWorkspaceYaml(file),
            });
        }

        // Check for package.json with workspaces field for npm/yarn workspace
        let package_json_path = cwd.join("package.json");
        if let Some(mut file) = open_exists_file(&package_json_path)? {
            let package_json: serde_json::Value = serde_json::from_reader(BufReader::new(&file))?;
            if package_json.get("workspaces").is_some() {
                // Reset the file cursor since we consumed it reading
                file.seek(SeekFrom::Start(0))?;
                return Ok(WorkspaceRoot {
                    path: cwd,
                    cwd: original_cwd.strip_prefix(cwd)?.expect("cwd must be within the workspace"),
                    workspace_file: WorkspaceFile::NpmWorkspaceJson(file),
                });
            }
        }

        // TODO(@fengmk2): other package manager support

        // Move up one directory
        if let Some(parent) = cwd.parent() {
            cwd = parent;
        } else {
            // We've reached the root, try to find the package root and return the non-workspace package.
            let package_root = find_package_root(original_cwd)?;
            let workspace_file = WorkspaceFile::NonWorkspacePackage(package_root.package_json);
            return Ok(WorkspaceRoot {
                path: package_root.path,
                cwd: package_root.cwd,
                workspace_file,
            });
        }
    }
}

/// Get the package manager name and version from the workspace root.
fn get_package_manager_type_and_version(
    workspace_root: &WorkspaceRoot,
    default: Option<PackageManagerType>,
) -> Result<(PackageManagerType, Str), Error> {
    // check packageManager field in package.json
    let package_json_path = workspace_root.path.join("package.json");
    if let Some(file) = open_exists_file(&package_json_path)? {
        let package_json: PackageJson = serde_json::from_reader(BufReader::new(&file))?;
        if !package_json.package_manager.is_empty()
            && let Some((name, version)) = package_json.package_manager.split_once('@')
        {
            // check if the version is a valid semver
            semver::Version::parse(version).map_err(|_| Error::PackageManagerVersionInvalid {
                name: name.into(),
                version: version.into(),
                package_json_path: package_json_path.to_absolute_path_buf(),
            })?;
            match name {
                "pnpm" => return Ok((PackageManagerType::Pnpm, version.into())),
                "yarn" => return Ok((PackageManagerType::Yarn, version.into())),
                "npm" => return Ok((PackageManagerType::Npm, version.into())),
                _ => return Err(Error::UnsupportedPackageManager(name.into())),
            }
        }
    }

    // TODO(@fengmk2): check devEngines.packageManager field in package.json

    let version = Str::from("latest");
    // if pnpm-workspace.yaml exists, use pnpm@latest
    if matches!(workspace_root.workspace_file, WorkspaceFile::PnpmWorkspaceYaml(_)) {
        return Ok((PackageManagerType::Pnpm, version));
    }

    // if pnpm-lock.yaml exists, use pnpm@latest
    let pnpm_lock_yaml_path = workspace_root.path.join("pnpm-lock.yaml");
    if is_exists_file(&pnpm_lock_yaml_path)? {
        return Ok((PackageManagerType::Pnpm, version));
    }

    // if yarn.lock or .yarnrc.yml exists, use yarn@latest
    let yarn_lock_path = workspace_root.path.join("yarn.lock");
    let yarnrc_yml_path = workspace_root.path.join(".yarnrc.yml");
    if is_exists_file(&yarn_lock_path)? || is_exists_file(&yarnrc_yml_path)? {
        return Ok((PackageManagerType::Yarn, version));
    }

    // if package-lock.json exists, use npm@latest
    let package_lock_json_path = workspace_root.path.join("package-lock.json");
    if is_exists_file(&package_lock_json_path)? {
        return Ok((PackageManagerType::Npm, version));
    }

    // if pnpmfile.cjs exists, use pnpm@latest
    let pnpmfile_cjs_path = workspace_root.path.join("pnpmfile.cjs");
    if is_exists_file(&pnpmfile_cjs_path)? {
        return Ok((PackageManagerType::Pnpm, version));
    }

    // if yarn.config.cjs exists, use yarn@latest (yarn 2.0+)
    let yarn_config_cjs_path = workspace_root.path.join("yarn.config.cjs");
    if is_exists_file(&yarn_config_cjs_path)? {
        return Ok((PackageManagerType::Yarn, version));
    }

    // if default is specified, use it
    if let Some(default) = default {
        return Ok((default, version));
    }

    // unrecognized package manager, let user specify the package manager
    Err(Error::UnrecognizedPackageManager)
}

/// Open the file if it exists, otherwise return None.
fn open_exists_file(path: impl AsRef<Path>) -> Result<Option<File>, Error> {
    match File::open(path) {
        Ok(file) => Ok(Some(file)),
        // if the file does not exist, return None
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Check if the file exists.
fn is_exists_file(path: impl AsRef<Path>) -> Result<bool, Error> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn get_latest_version(package_manager_type: PackageManagerType) -> Result<Str, Error> {
    let package_name = if matches!(package_manager_type, PackageManagerType::Yarn) {
        // yarn latest version should use `@yarnpkg/cli-dist` as package name
        "@yarnpkg/cli-dist".to_string()
    } else {
        package_manager_type.to_string()
    };
    let url = get_npm_package_version_url(&package_name, "latest");
    let package_json: PackageJson = HttpClient::new().get_json(&url).await?;
    Ok(package_json.version)
}

/// Download the package manager and extract it to the cache directory.
/// Return the install directory, e.g. $`CACHE_DIR/vite/package_manager/pnpm/10.0.0/pnpm`
async fn download_package_manager(
    package_manager_type: PackageManagerType,
    package_name: &str,
    version: &str,
) -> Result<AbsolutePathBuf, Error> {
    let tgz_url = get_npm_package_tgz_url(package_name, version);
    let cache_dir = get_cache_dir()?;
    let bin_name = package_manager_type.to_string();
    // $CACHE_DIR/vite/package_manager/pnpm/10.0.0
    let target_dir = cache_dir.join(format!("package_manager/{bin_name}/{version}"));
    let install_dir = target_dir.join(&bin_name);

    // If all shims are already exists, return the target directory
    // $CACHE_DIR/vite/package_manager/pnpm/10.0.0/pnpm/bin/(pnpm|pnpm.cmd|pnpm.ps1)
    let bin_prefix = install_dir.join("bin");
    let bin_file = bin_prefix.join(&bin_name);
    if is_exists_file(&bin_file)?
        && is_exists_file(bin_file.with_extension("cmd"))?
        && is_exists_file(bin_file.with_extension("ps1"))?
    {
        return Ok(install_dir);
    }

    // $CACHE_DIR/vite/package_manager/pnpm/{tmp_name}
    // Use tempfile::TempDir for robust temporary directory creation
    let parent_dir = target_dir.parent().unwrap();
    tokio::fs::create_dir_all(parent_dir).await?;
    let target_dir_tmp = tempfile::tempdir_in(parent_dir)?.path().to_path_buf();

    download_and_extract_tgz(&tgz_url, &target_dir_tmp).await.map_err(|err| {
        // status 404 means the version is not found, convert to PackageManagerVersionNotFound error
        if let Error::ReqwestError(e) = &err
            && let Some(status) = e.status()
            && status == reqwest::StatusCode::NOT_FOUND
        {
            Error::PackageManagerVersionNotFound {
                name: package_manager_type.to_string().into(),
                version: version.into(),
                url: tgz_url.into(),
            }
        } else {
            err
        }
    })?;

    // rename $target_dir_tmp/package to $target_dir_tmp/{bin_name}
    tracing::debug!("Rename package dir to {}", bin_name);
    tokio::fs::rename(&target_dir_tmp.join("package"), &target_dir_tmp.join(&bin_name)).await?;

    // check bin_file again, for the concurrent download cases
    if is_exists_file(&bin_file)? {
        tracing::debug!("bin_file already exists, skip rename");
        return Ok(install_dir);
    }

    // rename $target_dir_tmp to $target_dir
    tracing::debug!("Rename {:?} to {:?}", target_dir_tmp, target_dir);
    remove_dir_all_force(&target_dir).await?;
    tokio::fs::rename(&target_dir_tmp, &target_dir).await?;

    // create shim file
    tracing::debug!("Create shim files for {}", bin_name);
    create_shim_files(package_manager_type, &bin_prefix).await?;

    Ok(install_dir)
}

/// Remove the directory and all its contents.
/// Ignore the error if the directory is not found.
async fn remove_dir_all_force(path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    match remove_dir_all(path).await {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

/// Create shim files for the package manager.
///
/// Will automatically create `{cli_name}.cjs`, `{cli_name}.cmd`, `{cli_name}.ps1` files for the package manager.
/// Example:
/// - $`bin_prefix/pnpm` -> $`bin_prefix/pnpm.cjs`
/// - $`bin_prefix/pnpm.cmd` -> $`bin_prefix/pnpm.cjs`
/// - $`bin_prefix/pnpm.ps1` -> $`bin_prefix/pnpm.cjs`
/// - $`bin_prefix/pnpx` -> $`bin_prefix/pnpx.cjs`
/// - $`bin_prefix/pnpx.cmd` -> $`bin_prefix/pnpx.cjs`
/// - $`bin_prefix/pnpx.ps1` -> $`bin_prefix/pnpx.cjs`
async fn create_shim_files(
    package_manager_type: PackageManagerType,
    bin_prefix: impl AsRef<AbsolutePath>,
) -> Result<(), Error> {
    let mut bin_names: Vec<(&str, &str)> = Vec::new();

    match package_manager_type {
        PackageManagerType::Pnpm => {
            bin_names.push(("pnpm", "pnpm"));
            bin_names.push(("pnpx", "pnpx"));
        }
        PackageManagerType::Yarn => {
            // yarn don't have the `npx` like cli, so we don't need to create shim files for it
            bin_names.push(("yarn", "yarn"));
            // but it has alias `yarnpkg`
            bin_names.push(("yarnpkg", "yarn"));
        }
        PackageManagerType::Npm => {
            // npm has two cli: bin/npm-cli.js and bin/npx-cli.js
            bin_names.push(("npm", "npm-cli"));
            bin_names.push(("npx", "npx-cli"));
        }
    }

    let bin_prefix = bin_prefix.as_ref();
    for (bin_name, js_bin_basename) in bin_names {
        // try .cjs first
        let mut js_bin_name = format!("{js_bin_basename}.cjs");
        if !is_exists_file(bin_prefix.join(&js_bin_name))? {
            // fallback to .js
            js_bin_name = format!("{js_bin_basename}.js");
            if !is_exists_file(bin_prefix.join(&js_bin_name))? {
                continue;
            }
        }

        let source_file = bin_prefix.join(js_bin_name);
        let to_bin = bin_prefix.join(bin_name);
        shim::write_shims(&source_file, &to_bin).await?;
    }
    Ok(())
}

async fn set_package_manager_field(
    package_json_path: impl AsRef<AbsolutePath>,
    package_manager_type: PackageManagerType,
    version: &str,
) -> Result<(), Error> {
    let package_json_path = package_json_path.as_ref();
    let package_manager_value = format!("{package_manager_type}@{version}");
    let mut package_json = if is_exists_file(package_json_path)? {
        let content = tokio::fs::read(&package_json_path).await?;
        serde_json::from_slice(&content)?
    } else {
        serde_json::json!({})
    };
    // use IndexMap to preserve the order of the fields
    if let Some(package_json) = package_json.as_object_mut() {
        package_json.insert("packageManager".into(), serde_json::json!(package_manager_value));
    }
    let json_string = serde_json::to_string_pretty(&package_json)?;
    tokio::fs::write(&package_json_path, json_string).await?;
    tracing::debug!(
        "set_package_manager_field: {:?} to {:?}",
        package_json_path,
        package_manager_value
    );
    Ok(())
}

fn format_path_env(bin_prefix: impl AsRef<Path>) -> String {
    let mut paths = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    paths.insert(0, bin_prefix.as_ref().to_path_buf());
    env::join_paths(paths).unwrap().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::{TempDir, tempdir};

    use super::*;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_package_json(dir: &AbsolutePath, content: &str) {
        fs::write(dir.join("package.json"), content).expect("Failed to write package.json");
    }

    fn create_pnpm_workspace_yaml(dir: &AbsolutePath, content: &str) {
        fs::write(dir.join("pnpm-workspace.yaml"), content)
            .expect("Failed to write pnpm-workspace.yaml");
    }

    #[test]
    fn test_find_package_root() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let nested_dir = temp_dir_path.join("a").join("b").join("c");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json in a/b
        let package_dir = temp_dir_path.join("a").join("b");
        File::create(package_dir.join("package.json")).unwrap();

        // Should find package.json in parent directory
        let found = find_package_root(&nested_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);

        // Should return the same directory if package.json is there
        let found = find_package_root(&package_dir);
        let package_root = found.unwrap();
        assert_eq!(package_root.path, package_dir);

        // Should return PackageJsonNotFound error if no package.json found
        let root_dir = temp_dir_path.join("x").join("y");
        fs::create_dir_all(&root_dir).unwrap();
        let found = find_package_root(&root_dir);
        let err = found.unwrap_err();
        assert!(matches!(err, Error::PackageJsonNotFound(_)));
    }

    #[test]
    fn test_find_workspace_root_with_pnpm() {
        let temp_dir = create_temp_dir();

        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let nested_dir = temp_dir_path.join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create pnpm-workspace.yaml at root
        File::create(temp_dir_path.join("pnpm-workspace.yaml")).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::PnpmWorkspaceYaml(_)));
    }

    #[test]
    fn test_find_workspace_root_with_npm_workspaces() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let nested_dir = temp_dir_path.join("packages").join("app");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json with workspaces field
        let package_json = r#"{"workspaces": ["packages/*"]}"#;
        fs::write(temp_dir_path.join("package.json"), package_json).unwrap();

        // Should find workspace root
        let found = find_workspace_root(&temp_dir_path).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::NpmWorkspaceJson(_)));
    }

    #[test]
    fn test_find_workspace_root_fallback_to_package_root() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let nested_dir = temp_dir_path.join("src");
        fs::create_dir_all(&nested_dir).unwrap();

        // Create package.json without workspaces field
        let package_json = r#"{"name": "test"}"#;
        fs::write(temp_dir_path.join("package.json"), package_json).unwrap();

        // Should fallback to package root
        let found = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(found.path, temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::NonWorkspacePackage(_)));
        let package_root = find_package_root(&temp_dir_path).unwrap();
        // equal to workspace root
        assert_eq!(package_root.path, found.path);
    }

    #[test]
    fn test_find_workspace_root_with_package_json_not_found() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let nested_dir = temp_dir_path.join("src");
        fs::create_dir_all(&nested_dir).unwrap();

        // Should return PackageJsonNotFound error if no package.json found
        let found = find_workspace_root(&nested_dir);
        let err = found.unwrap_err();
        assert!(matches!(err, Error::PackageJsonNotFound(_)));
    }

    #[test]
    fn test_find_package_root_with_package_json_in_current_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = find_package_root(&temp_dir_path).unwrap();
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_package_root_with_package_json_in_parent_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let result = find_package_root(&sub_dir).unwrap();
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_package_root_with_package_json_in_grandparent_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let sub_dir = temp_dir_path.join("subdir").join("nested");
        fs::create_dir_all(&sub_dir).expect("Failed to create nested directories");

        let result = find_package_root(&sub_dir).unwrap();
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_pnpm_workspace_yaml() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let result = find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_pnpm_workspace_yaml_in_parent_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let result = find_workspace_root(&sub_dir).expect("Should find workspace root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_package_json_workspaces() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-workspace", "workspaces": ["packages/*"]}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = find_workspace_root(&temp_dir_path).unwrap();
        assert_eq!(result.path, temp_dir_path);
        assert!(matches!(result.workspace_file, WorkspaceFile::NpmWorkspaceJson(_)));
    }

    #[test]
    fn test_find_workspace_root_with_package_json_workspaces_in_parent_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-workspace", "workspaces": ["packages/*"]}"#;
        create_package_json(&temp_dir_path, package_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let result = find_workspace_root(&sub_dir).unwrap();
        assert_eq!(result.path, temp_dir_path);
        assert!(matches!(result.workspace_file, WorkspaceFile::NpmWorkspaceJson(_)));
    }

    #[test]
    fn test_find_workspace_root_prioritizes_pnpm_workspace_over_package_json() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create package.json with workspaces first
        let package_content = r#"{"name": "test-workspace", "workspaces": ["packages/*"]}"#;
        create_package_json(&temp_dir_path, package_content);

        // Then create pnpm-workspace.yaml (should take precedence)
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let result = find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_falls_back_to_package_root_when_no_workspace_found() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let result = find_workspace_root(&sub_dir).expect("Should fall back to package root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_nested_structure() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let nested_dir = temp_dir_path.join("packages").join("app").join("src");
        fs::create_dir_all(&nested_dir).expect("Failed to create nested directories");

        let result = find_workspace_root(&nested_dir).expect("Should find workspace root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_without_workspace_files_returns_package_root() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = find_workspace_root(&temp_dir_path).expect("Should return package root");
        assert_eq!(result.path, temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_invalid_package_json_handles_error() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let invalid_package_content = "{ invalid json content";
        create_package_json(&temp_dir_path, invalid_package_content);

        let result = find_workspace_root(&temp_dir_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_workspace_root_with_mixed_structure() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // Create a package.json without workspaces
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create a subdirectory with its own package.json
        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");
        let sub_package_content = r#"{"name": "sub-package"}"#;
        create_package_json(&sub_dir, sub_package_content);

        // Should find the subdirectory package.json since find_package_root searches upward from original_cwd
        let workspace_root =
            find_workspace_root(&sub_dir).expect("Should find subdirectory package");
        assert_eq!(workspace_root.path, sub_dir);
        assert!(matches!(workspace_root.workspace_file, WorkspaceFile::NonWorkspacePackage(_)));
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_pnpm_workspace_yaml() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let result =
            PackageManager::builder(temp_dir_path).build().await.expect("Should detect pnpm");
        assert_eq!(result.bin_name, "pnpm");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_pnpm_lock_yaml() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "version": "1.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create pnpm-lock.yaml
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'")
            .expect("Failed to write pnpm-lock.yaml");

        let result =
            PackageManager::builder(temp_dir_path).build().await.expect("Should detect pnpm");
        assert_eq!(result.bin_name, "pnpm");

        // check if the package.json file has the `packageManager` field
        let package_json_path = temp_dir.path().join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        println!("package_json: {:?}", package_json);
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("pnpm@"));
        // keep other fields
        assert_eq!(package_json["version"].as_str().unwrap(), "1.0.0");
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_yarn_lock() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create yarn.lock
        fs::write(temp_dir_path.join("yarn.lock"), "# yarn lockfile v1")
            .expect("Failed to write yarn.lock");

        let result = PackageManager::builder(temp_dir_path.to_absolute_path_buf())
            .build()
            .await
            .expect("Should detect yarn");
        assert_eq!(result.bin_name, "yarn");
        assert_eq!(result.workspace_root, temp_dir_path);
        assert!(
            result.get_bin_prefix().ends_with("yarn/bin"),
            "bin_prefix should end with yarn/bin, but got {:?}",
            result.get_bin_prefix()
        );
        // package.json should have the `packageManager` field
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        println!("package_json: {:?}", package_json);
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("yarn@"));
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    #[cfg(not(windows))] // FIXME
    async fn test_detect_package_manager_with_package_lock_json() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create package-lock.json
        fs::write(temp_dir_path.join("package-lock.json"), r#"{"lockfileVersion": 2}"#)
            .expect("Failed to write package-lock.json");

        let result =
            PackageManager::builder(temp_dir_path).build().await.expect("Should detect npm");
        assert_eq!(result.bin_name, "npm");

        // check shim files
        let bin_prefix = result.get_bin_prefix();
        assert!(is_exists_file(&bin_prefix.join("npm")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("npm.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("npm.ps1")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("npx")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("npx.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("npx.ps1")).unwrap());

        // run npm --version
        let mut paths =
            env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
        paths.insert(0, bin_prefix.into_path_buf());
        let output = Command::new("npm")
            .arg("--version")
            .env("PATH", env::join_paths(&paths).unwrap())
            .output()
            .expect("Failed to run npm");
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
        // println!("npm --version: {:?}", String::from_utf8_lossy(&output.stdout));

        // run npx --version
        let output = Command::new("npx")
            .arg("--version")
            .env("PATH", env::join_paths(&paths).unwrap())
            .output()
            .expect("Failed to run npx");
        assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    #[tokio::test]
    #[cfg(not(windows))] // FIXME
    async fn test_detect_package_manager_with_package_manager_field() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "pnpm@8.15.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect pnpm with version");
        assert_eq!(result.bin_name, "pnpm");

        // check shim files
        let bin_prefix = result.get_bin_prefix();
        assert!(is_exists_file(&bin_prefix.join("pnpm.cjs")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("pnpm.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("pnpm.ps1")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("pnpx.cjs")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("pnpx.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("pnpx.ps1")).unwrap());

        // run pnpm --version
        let mut paths =
            env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
        paths.insert(0, bin_prefix.into_path_buf());
        let output = Command::new("pnpm")
            .arg("--version")
            .env("PATH", env::join_paths(paths).unwrap())
            .output()
            .expect("Failed to run pnpm");
        // println!("pnpm --version: {:?}", output);
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "8.15.0");
    }

    #[tokio::test]
    #[cfg(not(windows))] // FIXME
    async fn test_detect_package_manager_with_yarn_package_manager_field() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@4.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path.clone())
            .build()
            .await
            .expect("Should detect yarn with version");
        assert_eq!(result.bin_name, "yarn");

        assert_eq!(result.version, "4.0.0");
        assert_eq!(result.workspace_root, temp_dir_path);
        assert!(
            result.get_bin_prefix().ends_with("yarn/bin"),
            "bin_prefix should end with yarn/bin, but got {:?}",
            result.get_bin_prefix()
        );

        // check shim files
        let bin_prefix = result.get_bin_prefix();
        assert!(is_exists_file(&bin_prefix.join("yarn.js")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarn")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarn.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarn.ps1")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarnpkg")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarnpkg.cmd")).unwrap());
        assert!(is_exists_file(&bin_prefix.join("yarnpkg.ps1")).unwrap());

        // run yarn --version
        let output = Command::new(bin_prefix.join("yarn").into_path_buf())
            .arg("--version")
            .output()
            .expect("Failed to run yarn");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "4.0.0");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_npm_package_manager_field() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "npm@10.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect npm with version");
        assert_eq!(result.bin_name, "npm");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_invalid_package_manager_field() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "invalid@1.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path).build().await;
        assert!(result.is_err());
        // Check if it's the expected error type
        if let Err(Error::UnsupportedPackageManager(name)) = result {
            assert_eq!(name, "invalid");
        } else {
            panic!("Expected UnsupportedPackageManager error");
        }
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_not_exists_version_in_package_manager_field() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content =
            r#"{"name": "test-package", "packageManager": "yarn@10000000000.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path).build().await;
        assert!(result.is_err());
        println!("result: {:?}", result);
        // Check if it's the expected error type
        if let Err(Error::PackageManagerVersionNotFound { name, version, .. }) = result {
            assert_eq!(name, "yarn");
            assert_eq!(version, "10000000000.0.0");
        } else {
            panic!("Expected PackageManagerVersionNotFound error, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_invalid_semver() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content =
            r#"{"name": "test-package", "packageManager": "pnpm@invalid-version"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path).build().await;
        println!("result: {:?}", result);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_default_fallback() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path.clone())
            .package_manager_type(PackageManagerType::Yarn)
            .build()
            .await
            .expect("Should use default");
        assert_eq!(result.bin_name, "yarn");
        // package.json should have the `packageManager` field
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        // println!("package_json: {:?}", package_json);
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("yarn@"));
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    async fn test_detect_package_manager_without_any_indicators() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path).build().await;
        assert!(result.is_err());
        // Check if it's the expected error type
        if let Err(Error::UnrecognizedPackageManager) = result {
            // Expected error
        } else {
            panic!("Expected UnrecognizedPackageManager error");
        }
    }

    #[tokio::test]
    async fn test_detect_package_manager_prioritizes_package_manager_field_over_lock_files() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@4.0.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create pnpm-lock.yaml (should be ignored due to packageManager field)
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'")
            .expect("Failed to write pnpm-lock.yaml");

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect yarn from packageManager field");
        assert_eq!(result.bin_name, "yarn");
    }

    #[tokio::test]
    async fn test_detect_package_manager_prioritizes_pnpm_workspace_over_lock_files() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create yarn.lock (should be ignored due to pnpm-workspace.yaml)
        fs::write(temp_dir_path.join("yarn.lock"), "# yarn lockfile v1")
            .expect("Failed to write yarn.lock");

        // Create pnpm-workspace.yaml (should take precedence)
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect pnpm from workspace file");
        assert_eq!(result.bin_name, "pnpm");
    }

    #[tokio::test]
    async fn test_detect_package_manager_from_subdirectory() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let sub_dir = temp_dir_path.join("packages").join("app");
        fs::create_dir_all(&sub_dir).expect("Failed to create subdirectory");

        let result = PackageManager::builder(sub_dir)
            .build()
            .await
            .expect("Should detect pnpm from parent workspace");
        assert_eq!(result.bin_name, "pnpm");
        assert!(result.get_bin_prefix().ends_with("pnpm/bin"));
    }

    #[tokio::test]
    async fn test_download_package_manager() {
        let result =
            download_package_manager(PackageManagerType::Yarn, "@yarnpkg/cli-dist", "4.9.2").await;
        assert!(result.is_ok());
        let target_dir = result.unwrap();
        println!("result: {:?}", target_dir);
        assert!(is_exists_file(&target_dir.join("bin/yarn")).unwrap());
        assert!(is_exists_file(&target_dir.join("bin/yarn.cmd")).unwrap());

        // again should skip download
        let result =
            download_package_manager(PackageManagerType::Yarn, "@yarnpkg/cli-dist", "4.9.2").await;
        assert!(result.is_ok());
        let target_dir = result.unwrap();
        assert!(is_exists_file(&target_dir.join("bin/yarn")).unwrap());
        assert!(is_exists_file(&target_dir.join("bin/yarn.cmd")).unwrap());

        remove_dir_all_force(target_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_latest_version() {
        let result = get_latest_version(PackageManagerType::Yarn).await;
        assert!(result.is_ok());
        let version = result.unwrap();
        // println!("version: {:?}", version);
        assert!(!version.is_empty());
        // check version should >= 4.0.0
        let version_req = VersionReq::parse(">=4.0.0");
        assert!(version_req.is_ok());
        let version_req = version_req.unwrap();
        assert!(version_req.matches(&Version::parse(&version).unwrap()));
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_yarnrc_yml() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create .yarnrc.yml
        fs::write(
            temp_dir_path.join(".yarnrc.yml"),
            "nodeLinker: node-modules\nyarnPath: .yarn/releases/yarn-4.0.0.cjs",
        )
        .expect("Failed to write .yarnrc.yml");

        let result = PackageManager::builder(temp_dir_path.clone())
            .build()
            .await
            .expect("Should detect yarn from .yarnrc.yml");
        assert_eq!(result.bin_name, "yarn");
        assert_eq!(result.workspace_root, temp_dir_path);
        assert!(
            result.get_bin_prefix().ends_with("yarn/bin"),
            "bin_prefix should end with yarn/bin, but got {:?}",
            result.get_bin_prefix()
        );
        // package.json should have the `packageManager` field
        let package_json_path = temp_dir.path().join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("yarn@"));
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_pnpmfile_cjs() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create pnpmfile.cjs
        fs::write(temp_dir_path.join("pnpmfile.cjs"), "module.exports = { hooks: {} }")
            .expect("Failed to write pnpmfile.cjs");

        let result = PackageManager::builder(temp_dir_path.clone())
            .build()
            .await
            .expect("Should detect pnpm from pnpmfile.cjs");
        assert_eq!(result.bin_name, "pnpm");
        assert_eq!(result.workspace_root, temp_dir_path);
        assert!(
            result.get_bin_prefix().ends_with("pnpm/bin"),
            "bin_prefix should end with pnpm/bin, but got {:?}",
            result.get_bin_prefix()
        );
        // package.json should have the `packageManager` field
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("pnpm@"));
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_yarn_config_cjs() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create yarn.config.cjs
        fs::write(
            temp_dir_path.join("yarn.config.cjs"),
            "module.exports = { nodeLinker: 'node-modules' }",
        )
        .expect("Failed to write yarn.config.cjs");

        let result = PackageManager::builder(temp_dir_path.clone())
            .build()
            .await
            .expect("Should detect yarn from yarn.config.cjs");
        assert_eq!(result.bin_name, "yarn");
        assert_eq!(result.workspace_root, temp_dir_path);
        assert!(
            result.get_bin_prefix().ends_with("yarn/bin"),
            "bin_prefix should end with yarn/bin, but got {:?}",
            result.get_bin_prefix()
        );
        // package.json should have the `packageManager` field
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("yarn@"));
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    async fn test_detect_package_manager_priority_order_lock_over_config() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create multiple detection files to test priority order
        // According to vite-install.md, pnpmfile.cjs and yarn.config.cjs are lower priority than lock files

        // Create pnpmfile.cjs
        fs::write(temp_dir_path.join("pnpmfile.cjs"), "module.exports = { hooks: {} }")
            .expect("Failed to write pnpmfile.cjs");

        // Create yarn.config.cjs
        fs::write(
            temp_dir_path.join("yarn.config.cjs"),
            "module.exports = { nodeLinker: 'node-modules' }",
        )
        .expect("Failed to write yarn.config.cjs");

        // Create package-lock.json (should take precedence over pnpmfile.cjs and yarn.config.cjs)
        fs::write(temp_dir_path.join("package-lock.json"), r#"{"lockfileVersion": 3}"#)
            .expect("Failed to write package-lock.json");

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect npm from package-lock.json");
        assert_eq!(
            result.bin_name, "npm",
            "package-lock.json should take precedence over pnpmfile.cjs and yarn.config.cjs"
        );
    }

    #[tokio::test]
    async fn test_detect_package_manager_pnpmfile_over_yarn_config() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create both pnpmfile.cjs and yarn.config.cjs
        fs::write(temp_dir_path.join("pnpmfile.cjs"), "module.exports = { hooks: {} }")
            .expect("Failed to write pnpmfile.cjs");

        fs::write(
            temp_dir_path.join("yarn.config.cjs"),
            "module.exports = { nodeLinker: 'node-modules' }",
        )
        .expect("Failed to write yarn.config.cjs");

        // pnpmfile.cjs should be detected first (before yarn.config.cjs)
        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect pnpm from pnpmfile.cjs");
        assert_eq!(
            result.bin_name, "pnpm",
            "pnpmfile.cjs should be detected before yarn.config.cjs"
        );
    }
}
