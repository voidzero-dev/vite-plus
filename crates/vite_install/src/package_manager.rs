use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Write},
    path::Path,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tokio::fs::remove_dir_all;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::OnFail;
use vite_str::Str;
#[cfg(test)]
use vite_workspace::find_package_root;
use vite_workspace::{WorkspaceFile, WorkspaceRoot, find_workspace_root};

use crate::{
    config::{get_npm_package_metadata_url, get_npm_package_tgz_url, get_npm_package_version_url},
    request::{HttpClient, download_and_extract_tgz_with_hash},
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
    Bun,
}

impl fmt::Display for PackageManagerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pnpm => write!(f, "pnpm"),
            Self::Yarn => write!(f, "yarn"),
            Self::Npm => write!(f, "npm"),
            Self::Bun => write!(f, "bun"),
        }
    }
}

impl PackageManagerType {
    /// Map an invoked shim tool name (including aliases like `npx`, `pnpx`,
    /// `yarnpkg`, `bunx`) to the package-manager family that provides it.
    #[must_use]
    pub fn from_tool(tool: &str) -> Option<Self> {
        match tool {
            "npm" | "npx" => Some(Self::Npm),
            "pnpm" | "pnpx" => Some(Self::Pnpm),
            "yarn" | "yarnpkg" => Some(Self::Yarn),
            "bun" | "bunx" => Some(Self::Bun),
            _ => None,
        }
    }

    /// Resolve the bin file name for an invoked tool, preserving alias names
    /// that the managed PM installs alongside its primary binary.
    #[must_use]
    pub fn bin_name_for_tool(self, tool: &str) -> &'static str {
        match (tool, self) {
            ("npx", Self::Npm) => "npx",
            ("pnpx", Self::Pnpm) => "pnpx",
            ("yarnpkg", Self::Yarn) => "yarnpkg",
            ("bunx", Self::Bun) => "bunx",
            (_, Self::Npm) => "npm",
            (_, Self::Pnpm) => "pnpm",
            (_, Self::Yarn) => "yarn",
            (_, Self::Bun) => "bun",
        }
    }
}

/// Package-manager resolution from an explicit project `packageManager` field.
#[derive(Debug, Clone)]
pub struct PackageManagerResolution {
    pub package_manager_type: PackageManagerType,
    pub version: Str,
    pub hash: Option<Str>,
    pub source: Str,
    pub source_path: AbsolutePathBuf,
    pub project_root: AbsolutePathBuf,
}

/// Where the package manager selection came from (see rfcs/dev-engines.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManagerSource {
    /// Top-level `packageManager` field in package.json
    PackageManagerField,
    /// `devEngines.packageManager` field in package.json
    DevEnginesPackageManager,
    /// Lockfiles or package-manager config files
    LockfileOrConfig,
    /// Caller-provided default
    Default,
}

impl fmt::Display for PackageManagerSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PackageManagerField => write!(f, "packageManager"),
            Self::DevEnginesPackageManager => write!(f, "devEngines.packageManager"),
            Self::LockfileOrConfig => write!(f, "lockfile"),
            Self::Default => write!(f, "default"),
        }
    }
}

// TODO(@fengmk2): should move ResolveCommandResult to vite-common crate
#[derive(Debug)]
pub struct ResolveCommandResult {
    pub bin_path: String,
    pub args: Vec<String>,
    pub envs: HashMap<String, String>,
}

/// The package manager.
/// Use `PackageManager::builder()` to create a package manager.
/// Then use `PackageManager::resolve_command()` to resolve the command result.
#[derive(Debug)]
pub struct PackageManager {
    pub client: PackageManagerType,
    pub package_name: Str,
    pub version: Str,
    pub hash: Option<Str>,
    pub bin_name: Str,
    pub workspace_root: AbsolutePathBuf,
    /// Whether the workspace is a monorepo.
    pub is_monorepo: bool,
    pub install_dir: AbsolutePathBuf,
}

#[derive(Debug)]
pub struct PackageManagerBuilder {
    client_override: Option<PackageManagerType>,
    cwd: AbsolutePathBuf,
}

impl PackageManagerBuilder {
    pub fn new(cwd: impl AsRef<AbsolutePath>) -> Self {
        Self { client_override: None, cwd: cwd.as_ref().to_absolute_path_buf() }
    }

    #[must_use]
    pub const fn package_manager_type(mut self, package_manager_type: PackageManagerType) -> Self {
        self.client_override = Some(package_manager_type);
        self
    }

    /// Build the package manager.
    /// Detect the package manager from the current working directory.
    pub async fn build(&self) -> Result<PackageManager, Error> {
        let (workspace_root, _cwd) = find_workspace_root(&self.cwd)?;
        let (package_manager_type, version_or_req, hash, source) =
            get_package_manager_type_and_version(&workspace_root, self.client_override)?;

        // only download the package manager if it's not already downloaded
        let (install_dir, package_name, version) =
            download_package_manager(package_manager_type, &version_or_req, hash.as_deref())
                .await?;

        // Auto-pin the resolved version when detection had no explicit field
        // (lockfiles, config files, or caller default). A devEngines range is
        // the user's source of truth and is never frozen into an exact pin.
        // See rfcs/dev-engines.md.
        if matches!(source, PackageManagerSource::LockfileOrConfig | PackageManagerSource::Default)
            && version_or_req != version
        {
            let package_json_path = workspace_root.path.join("package.json");
            set_dev_engines_package_manager_field(
                &package_json_path,
                package_manager_type,
                &version,
            )
            .await?;
        }

        let is_monorepo = matches!(
            workspace_root.workspace_file,
            WorkspaceFile::PnpmWorkspaceYaml(_) | WorkspaceFile::NpmWorkspaceJson(_)
        );

        Ok(PackageManager {
            client: package_manager_type,
            package_name,
            version,
            hash,
            bin_name: package_manager_type.to_string().into(),
            workspace_root: workspace_root.path.to_absolute_path_buf(),
            is_monorepo,
            install_dir,
        })
    }

    /// Build the package manager with default package manager.
    /// If the package manager is not specified, prompt the user to select a package manager.
    pub async fn build_with_default(&self) -> Result<PackageManager, Error> {
        let package_manager = match self.build().await {
            Ok(pm) => pm,
            Err(Error::UnrecognizedPackageManager) => {
                // Prompt user to select a package manager
                let selected_type = prompt_package_manager_selection()?;
                Self::new(&self.cwd).package_manager_type(selected_type).build().await?
            }
            Err(e) => return Err(e),
        };
        Ok(package_manager)
    }
}

impl PackageManager {
    pub fn builder(cwd: impl AsRef<AbsolutePath>) -> PackageManagerBuilder {
        PackageManagerBuilder::new(cwd)
    }

    #[must_use]
    pub fn get_bin_prefix(&self) -> AbsolutePathBuf {
        self.install_dir.join("bin")
    }
}

/// Get the package manager name, version, optional hash, and detection source
/// from the workspace root.
///
/// The returned version is exact when detected from the `packageManager` field,
/// `"latest"` when detected from lockfiles/config files/default, and may be a
/// semver range (or `"*"` for an absent version) when detected from
/// `devEngines.packageManager` (see rfcs/dev-engines.md).
pub fn get_package_manager_type_and_version(
    workspace_root: &WorkspaceRoot,
    default: Option<PackageManagerType>,
) -> Result<(PackageManagerType, Str, Option<Str>, PackageManagerSource), Error> {
    // check packageManager field in package.json
    if let Some(resolution) = get_package_manager_from_package_json(workspace_root)? {
        warn_on_dev_engines_package_manager_conflict(workspace_root, &resolution);
        return Ok((
            resolution.package_manager_type,
            resolution.version,
            resolution.hash,
            PackageManagerSource::PackageManagerField,
        ));
    }

    // check devEngines.packageManager field in package.json (see rfcs/dev-engines.md)
    if let Some((package_manager_type, version_req)) =
        get_package_manager_from_dev_engines(workspace_root)?
    {
        // an absent version means any version satisfies (devEngines spec)
        let version_req = version_req.unwrap_or_else(|| "*".into());
        return Ok((
            package_manager_type,
            version_req,
            None,
            PackageManagerSource::DevEnginesPackageManager,
        ));
    }

    let version = Str::from("latest");
    let source = PackageManagerSource::LockfileOrConfig;
    // if pnpm-workspace.yaml exists, use pnpm@latest
    if matches!(workspace_root.workspace_file, WorkspaceFile::PnpmWorkspaceYaml(_)) {
        return Ok((PackageManagerType::Pnpm, version, None, source));
    }

    // if pnpm-lock.yaml exists, use pnpm@latest
    let pnpm_lock_yaml_path = workspace_root.path.join("pnpm-lock.yaml");
    if is_exists_file(&pnpm_lock_yaml_path)? {
        return Ok((PackageManagerType::Pnpm, version, None, source));
    }

    // if yarn.lock or .yarnrc.yml exists, use yarn@latest
    let yarn_lock_path = workspace_root.path.join("yarn.lock");
    let yarnrc_yml_path = workspace_root.path.join(".yarnrc.yml");
    if is_exists_file(&yarn_lock_path)? || is_exists_file(&yarnrc_yml_path)? {
        return Ok((PackageManagerType::Yarn, version, None, source));
    }

    // if package-lock.json exists, use npm@latest
    let package_lock_json_path = workspace_root.path.join("package-lock.json");
    if is_exists_file(&package_lock_json_path)? {
        return Ok((PackageManagerType::Npm, version, None, source));
    }

    // if bun.lock (text format) or bun.lockb (binary format) exists, use bun@latest
    let bun_lock_path = workspace_root.path.join("bun.lock");
    if is_exists_file(&bun_lock_path)? {
        return Ok((PackageManagerType::Bun, version, None, source));
    }
    let bun_lockb_path = workspace_root.path.join("bun.lockb");
    if is_exists_file(&bun_lockb_path)? {
        return Ok((PackageManagerType::Bun, version, None, source));
    }

    // if .pnpmfile.cjs exists, use pnpm@latest
    let pnpmfile_cjs_path = workspace_root.path.join(".pnpmfile.cjs");
    if is_exists_file(&pnpmfile_cjs_path)? {
        return Ok((PackageManagerType::Pnpm, version, None, source));
    }
    // if legacy pnpmfile.cjs exists, use pnpm@latest
    // https://newreleases.io/project/npm/pnpm/release/6.0.0
    let legacy_pnpmfile_cjs_path = workspace_root.path.join("pnpmfile.cjs");
    if is_exists_file(&legacy_pnpmfile_cjs_path)? {
        return Ok((PackageManagerType::Pnpm, version, None, source));
    }

    // if bunfig.toml exists, use bun@latest
    let bunfig_toml_path = workspace_root.path.join("bunfig.toml");
    if is_exists_file(&bunfig_toml_path)? {
        return Ok((PackageManagerType::Bun, version, None, source));
    }

    // if yarn.config.cjs exists, use yarn@latest (yarn 2.0+)
    let yarn_config_cjs_path = workspace_root.path.join("yarn.config.cjs");
    if is_exists_file(&yarn_config_cjs_path)? {
        return Ok((PackageManagerType::Yarn, version, None, source));
    }

    // if default is specified, use it
    if let Some(default) = default {
        return Ok((default, version, None, PackageManagerSource::Default));
    }

    // unrecognized package manager, let user specify the package manager
    Err(Error::UnrecognizedPackageManager)
}

/// Resolve the project-declared package manager for the current workspace:
/// the explicit `packageManager` field first, then `devEngines.packageManager`
/// (rfcs/dev-engines.md).
///
/// This is intentionally non-mutating: it does not prompt, download a package manager, or write the
/// resolved version back to `package.json`. A `devEngines.packageManager` range resolves
/// against already-downloaded versions when possible; otherwise the raw requirement is
/// kept in `version` and resolved at download time.
pub fn resolve_package_manager_from_package_json(
    cwd: impl AsRef<AbsolutePath>,
) -> Result<Option<PackageManagerResolution>, Error> {
    let (workspace_root, _) = match find_workspace_root(cwd.as_ref()) {
        Ok(result) => result,
        Err(vite_workspace::Error::PackageJsonNotFound(_)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if let Some(resolution) = get_package_manager_from_package_json(&workspace_root)? {
        return Ok(Some(resolution));
    }

    // Fall back to devEngines.packageManager (see rfcs/dev-engines.md)
    let Some((package_manager_type, version_req)) =
        get_package_manager_from_dev_engines(&workspace_root)?
    else {
        return Ok(None);
    };
    // An absent version means any version satisfies (devEngines spec)
    let version_req = version_req.unwrap_or_else(|| "*".into());
    let version = if Version::parse(&version_req).is_ok() {
        version_req
    } else if let Ok(range) = node_semver::Range::parse(version_req.as_str())
        && let Some(cached) = find_cached_package_manager_version(package_manager_type, &range)?
    {
        cached
    } else {
        version_req
    };

    let package_json_path = workspace_root.path.join("package.json");
    Ok(Some(PackageManagerResolution {
        package_manager_type,
        version,
        hash: None,
        source: "devEngines.packageManager".into(),
        source_path: package_json_path.to_absolute_path_buf(),
        project_root: workspace_root.path.to_absolute_path_buf(),
    }))
}

/// Return the managed install directory for a package manager version.
#[must_use]
pub fn package_manager_install_dir(
    package_manager_type: PackageManagerType,
    version: &str,
) -> Option<AbsolutePathBuf> {
    let home_dir = vite_shared::get_vp_home().ok()?;
    let bin_name = package_manager_type.to_string();
    Some(home_dir.join("package_manager").join(&bin_name).join(version).join(&bin_name))
}

/// Return the executable shim path for a package manager binary inside an install directory.
#[must_use]
pub fn package_manager_bin_path(install_dir: &AbsolutePath, bin_name: &str) -> AbsolutePathBuf {
    let bin_path = install_dir.join("bin").join(bin_name);
    if cfg!(windows) { bin_path.with_extension("cmd") } else { bin_path }
}

fn get_package_manager_from_package_json(
    workspace_root: &WorkspaceRoot,
) -> Result<Option<PackageManagerResolution>, Error> {
    let package_json_path = workspace_root.path.join("package.json");
    let Some(file) = open_exists_file(&package_json_path)? else {
        return Ok(None);
    };

    let package_json: PackageJson = serde_json::from_reader(BufReader::new(&file))?;
    if package_json.package_manager.is_empty() {
        return Ok(None);
    }

    let Some((package_manager_type, version, hash)) =
        parse_package_manager_field(&package_json.package_manager, &package_json_path)?
    else {
        return Ok(None);
    };

    Ok(Some(PackageManagerResolution {
        package_manager_type,
        version,
        hash,
        source: "packageManager".into(),
        source_path: package_json_path.to_absolute_path_buf(),
        project_root: workspace_root.path.to_absolute_path_buf(),
    }))
}

/// Read the `devEngines.packageManager` field from the workspace root package.json.
fn read_dev_engines_package_manager(
    workspace_root: &WorkspaceRoot,
) -> Option<vite_shared::DevEngineField> {
    let package_json_path = workspace_root.path.join("package.json");
    let file = open_exists_file(&package_json_path).ok()??;
    // Lenient: a package.json we cannot parse here is reported by other paths
    let pkg: vite_shared::PackageJson = serde_json::from_reader(BufReader::new(&file)).ok()?;
    pkg.dev_engines.and_then(|dev_engines| dev_engines.package_manager)
}

/// Resolve the package manager from `devEngines.packageManager` in package.json.
///
/// Entries are evaluated in order and the first supported entry wins (devEngines
/// spec: "the first acceptable option would be used"). When no entry names a
/// supported package manager, the effective `onFail` of the last entry decides:
/// `ignore`/`warn` fall through to lockfile detection, `error` (the default) and
/// `download` fail with a clear message. See rfcs/dev-engines.md.
///
/// Returns the package manager type and the raw version requirement
/// (`None` means any version satisfies).
fn get_package_manager_from_dev_engines(
    workspace_root: &WorkspaceRoot,
) -> Result<Option<(PackageManagerType, Option<Str>)>, Error> {
    let Some(field) = read_dev_engines_package_manager(workspace_root) else {
        return Ok(None);
    };
    let entries = field.entries();
    if entries.is_empty() {
        return Ok(None);
    }

    for entry in entries {
        let Some(package_manager_type) = parse_package_manager_name(&entry.name) else {
            continue;
        };
        // Lenient read: an invalid version range is treated as any version,
        // surfaced as a warning here and by `vp env doctor`
        let version_req = entry.version.clone().filter(|version| {
            let valid = Version::parse(version).is_ok()
                || node_semver::Range::parse(version.as_str()).is_ok();
            if !valid {
                vite_shared::output::warn(&format!(
                    "invalid devEngines.packageManager version {version:?} for \
                     {package_manager_type}, treating as any version"
                ));
            }
            valid
        });
        return Ok(Some((package_manager_type, version_req)));
    }

    // No supported entry: the effective onFail of the last entry decides
    let names: Str = entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ").into();
    match field.effective_on_fail(entries.len() - 1) {
        OnFail::Ignore => Ok(None),
        OnFail::Warn => {
            vite_shared::output::warn(&format!(
                "devEngines.packageManager {names:?} is not supported \
                 (supported: pnpm, yarn, npm, bun)"
            ));
            Ok(None)
        }
        OnFail::Error | OnFail::Download => Err(Error::UnsupportedDevEnginesPackageManager(names)),
    }
}

/// Warn when the explicit `packageManager` field does not satisfy the
/// `devEngines.packageManager` constraint.
///
/// Per rfcs/dev-engines.md this is a warning for now and becomes a hard error
/// in a future release; npm already errors in this situation.
fn warn_on_dev_engines_package_manager_conflict(
    workspace_root: &WorkspaceRoot,
    resolution: &PackageManagerResolution,
) {
    let Some(field) = read_dev_engines_package_manager(workspace_root) else {
        return;
    };
    if let Some(message) = dev_engines_package_manager_conflict_message(&field, resolution) {
        vite_shared::output::warn(&message);
    }
}

/// Build the conflict message for an explicit `packageManager` field that does
/// not satisfy the `devEngines.packageManager` constraint.
///
/// Returns `None` when the field is consistent with the constraint (semver-aware:
/// an exact version satisfying a declared range is not a conflict), when the
/// constraint is empty, or when the declared range is not valid semver
/// (`vp env doctor` reports that case).
fn dev_engines_package_manager_conflict_message(
    field: &vite_shared::DevEngineField,
    resolution: &PackageManagerResolution,
) -> Option<Str> {
    let entries = field.entries();
    if entries.is_empty() {
        return None;
    }

    let name = resolution.package_manager_type.to_string();
    let Some(entry) = field.find_by_name(&name) else {
        let names = entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ");
        return Some(
            format!(
                "packageManager is {name}@{version} but devEngines.packageManager \
                 requires {names:?}. This will become an error in a future release.",
                version = resolution.version
            )
            .into(),
        );
    };
    if let Some(required) = &entry.version
        && let Ok(range) = node_semver::Range::parse(required.as_str())
        && let Ok(version) = node_semver::Version::parse(resolution.version.as_str())
        && !range.satisfies(&version)
    {
        return Some(
            format!(
                "packageManager {name}@{version} does not satisfy \
                 devEngines.packageManager {required:?}. This will become an error in a \
                 future release.",
                version = resolution.version
            )
            .into(),
        );
    }
    None
}

/// Parse a package manager name into a supported [`PackageManagerType`].
fn parse_package_manager_name(name: &str) -> Option<PackageManagerType> {
    match name {
        "pnpm" => Some(PackageManagerType::Pnpm),
        "yarn" => Some(PackageManagerType::Yarn),
        "npm" => Some(PackageManagerType::Npm),
        "bun" => Some(PackageManagerType::Bun),
        _ => None,
    }
}

fn parse_package_manager_field(
    package_manager: &str,
    package_json_path: &AbsolutePath,
) -> Result<Option<(PackageManagerType, Str, Option<Str>)>, Error> {
    let Some((name, version_with_hash)) = package_manager.split_once('@') else {
        return Ok(None);
    };

    // Parse version and optional hash (format: version+sha512.hash)
    let (version, hash) = if let Some((ver, hash_part)) = version_with_hash.split_once('+') {
        (ver, Some(hash_part.into()))
    } else {
        (version_with_hash, None)
    };

    // check if the version is a valid semver
    semver::Version::parse(version).map_err(|_| Error::PackageManagerVersionInvalid {
        name: name.into(),
        version: version.into(),
        package_json_path: package_json_path.to_absolute_path_buf(),
    })?;
    let package_manager_type = parse_package_manager_name(name)
        .ok_or_else(|| Error::UnsupportedPackageManager(name.into()))?;

    Ok(Some((package_manager_type, version.into(), hash)))
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

/// Abbreviated registry metadata: only the version list is needed.
#[derive(Deserialize)]
struct RegistryPackument {
    // a map with ignored values is the idiomatic serde way to read only the keys
    #[allow(clippy::zero_sized_map_values)]
    #[serde(default)]
    versions: HashMap<String, serde::de::IgnoredAny>,
}

/// Fetch all published versions of a package from the npm registry.
async fn fetch_registry_versions(package_name: &str) -> Result<Vec<node_semver::Version>, Error> {
    let url = get_npm_package_metadata_url(package_name);
    let packument: RegistryPackument = HttpClient::new().get_json(&url).await?;
    Ok(packument
        .versions
        .keys()
        .filter_map(|version| node_semver::Version::parse(version).ok())
        .collect())
}

/// Resolve the latest registry version satisfying `range` (prereleases excluded).
async fn resolve_latest_satisfying_version(
    package_manager_type: PackageManagerType,
    range: &node_semver::Range,
    version_req: &str,
) -> Result<Str, Error> {
    let package_name = package_manager_type.to_string();
    let mut versions = fetch_registry_versions(&package_name).await?;
    // yarn >= 2.0.0 is published as `@yarnpkg/cli-dist`; merge both version lists
    if matches!(package_manager_type, PackageManagerType::Yarn) {
        versions.extend(fetch_registry_versions("@yarnpkg/cli-dist").await?);
    }

    versions
        .into_iter()
        .filter(|version| !version.is_prerelease() && range.satisfies(version))
        .max()
        .map(|version| Str::from(version.to_string()))
        .ok_or_else(|| Error::PackageManagerVersionNotFound {
            name: package_name.clone().into(),
            version: version_req.into(),
            url: get_npm_package_metadata_url(&package_name).into(),
        })
}

/// Find the highest already-downloaded package manager version satisfying `range`
/// under `$VP_HOME/package_manager/<name>/`.
fn find_cached_package_manager_version(
    package_manager_type: PackageManagerType,
    range: &node_semver::Range,
) -> Result<Option<Str>, Error> {
    let home_dir = vite_shared::get_vp_home()?;
    let bin_name = package_manager_type.to_string();
    let versions_dir = home_dir.join("package_manager").join(&bin_name);
    let entries = match fs::read_dir(&versions_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let mut best: Option<node_semver::Version> = None;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else { continue };
        let Ok(version) = node_semver::Version::parse(name) else { continue };
        if !range.satisfies(&version) {
            continue;
        }
        // Only consider completed installs (the bin shim exists)
        let bin_file = versions_dir.join(name).join(&bin_name).join("bin").join(&bin_name);
        if !is_exists_file(&bin_file)? {
            continue;
        }
        if best.as_ref().is_none_or(|b| version > *b) {
            best = Some(version);
        }
    }
    Ok(best.map(|version| Str::from(version.to_string())))
}

/// Resolve a semver range (e.g. from `devEngines.packageManager`) to an exact
/// version: prefer the highest already-downloaded satisfying version, falling
/// back to the latest satisfying version from the npm registry.
/// See rfcs/dev-engines.md.
async fn resolve_package_manager_range(
    package_manager_type: PackageManagerType,
    version_req: &str,
) -> Result<Str, Error> {
    let range = node_semver::Range::parse(version_req).map_err(|_| {
        Error::InvalidArgument(
            format!(
                "invalid {package_manager_type} version {version_req:?}: expected semver \
                 'major.minor.patch' or a semver range"
            )
            .into(),
        )
    })?;

    if let Some(cached) = find_cached_package_manager_version(package_manager_type, &range)? {
        tracing::debug!("Found cached {package_manager_type} {cached} satisfying {version_req}");
        return Ok(cached);
    }

    // `*` (any version) resolves to the registry's latest stable
    if version_req == "*" {
        return get_latest_version(package_manager_type).await;
    }

    resolve_latest_satisfying_version(package_manager_type, &range, version_req).await
}

/// Download the package manager and extract it to the vite-plus home directory.
/// Return the install directory, e.g. `$VP_HOME/package_manager/pnpm/10.0.0/pnpm`
pub async fn download_package_manager(
    package_manager_type: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<(AbsolutePathBuf, Str, Str), Error> {
    let version: Str = if version_or_latest == "latest" {
        get_latest_version(package_manager_type).await?
    } else if Version::parse(version_or_latest).is_ok() {
        version_or_latest.into()
    } else {
        // semver range (e.g. from devEngines.packageManager): prefer an already
        // downloaded satisfying version, otherwise resolve from the registry
        resolve_package_manager_range(package_manager_type, version_or_latest).await?
    };

    // Reject anything that is not strict semver `major.minor.patch[-prerelease][+build]`.
    // This prevents path traversal via the version being interpolated into
    // `$VP_HOME/package_manager/{name}/{version}` below, since `AbsolutePath::join`
    // does not normalize `..` components. Also guards against registry-controlled
    // "latest" lookups returning a malicious value.
    let parsed_version = Version::parse(&version).map_err(|_| {
        Error::InvalidArgument(
            format!(
                "invalid {package_manager_type} version {version:?}: expected semver 'major.minor.patch'"
            )
            .into(),
        )
    })?;

    let mut package_name: Str = package_manager_type.to_string().into();
    // handle yarn >= 2.0.0 to use `@yarnpkg/cli-dist` as package name
    // @see https://github.com/nodejs/corepack/blob/main/config.json#L135
    if matches!(package_manager_type, PackageManagerType::Yarn)
        && VersionReq::parse(">=2.0.0")?.matches(&parsed_version)
    {
        package_name = "@yarnpkg/cli-dist".into();
    }

    let home_dir = vite_shared::get_vp_home()?;
    let bin_name = package_manager_type.to_string();

    // For bun, use platform-specific download flow.
    // The hash from `packageManager` field belongs to the main `bun` npm package,
    // not the platform-specific binary, so we don't pass it through.
    if matches!(package_manager_type, PackageManagerType::Bun) {
        return download_bun_package_manager(&version, &home_dir).await;
    }

    let tgz_url = get_npm_package_tgz_url(&package_name, &version);
    // $VP_HOME/package_manager/pnpm/10.0.0
    let target_dir = home_dir.join("package_manager").join(&bin_name).join(&version);
    let install_dir = target_dir.join(&bin_name);

    // If all shims already exist, return the target directory
    // $VP_HOME/package_manager/pnpm/10.0.0/pnpm/bin/(pnpm|pnpm.cmd|pnpm.ps1)
    let bin_prefix = install_dir.join("bin");
    let bin_file = bin_prefix.join(&bin_name);
    if is_exists_file(&bin_file)?
        && is_exists_file(bin_file.with_extension("cmd"))?
        && is_exists_file(bin_file.with_extension("ps1"))?
    {
        return Ok((install_dir, package_name, version));
    }

    // $VP_HOME/package_manager/pnpm/{tmp_name}
    // Use tempfile::TempDir for robust temporary directory creation
    let parent_dir = target_dir.parent().unwrap();
    tokio::fs::create_dir_all(parent_dir).await?;
    let target_dir_tmp = tempfile::tempdir_in(parent_dir)?.path().to_path_buf();

    download_and_extract_tgz_with_hash(&tgz_url, &target_dir_tmp, expected_hash).await.map_err(
        |err| {
            // status 404 means the version is not found, convert to PackageManagerVersionNotFound error
            if let Error::Reqwest(e) = &err
                && let Some(status) = e.status()
                && status == reqwest::StatusCode::NOT_FOUND
            {
                Error::PackageManagerVersionNotFound {
                    name: package_manager_type.to_string().into(),
                    version: version.clone(),
                    url: tgz_url.into(),
                }
            } else {
                err
            }
        },
    )?;

    // rename $target_dir_tmp/package to $target_dir_tmp/{bin_name}
    tracing::debug!("Rename package dir to {}", bin_name);
    tokio::fs::rename(&target_dir_tmp.join("package"), &target_dir_tmp.join(&bin_name)).await?;

    // Use a file-based lock to ensure atomicity of remove + rename operations
    // This prevents DirectoryNotEmpty error when multiple processes/threads
    // try to install the same package manager version concurrently.
    // The lock is automatically skipped on NFS filesystems where locking is unreliable.
    let lock_path = parent_dir.join(format!("{version}.lock"));
    tracing::debug!("Acquire lock file: {:?}", lock_path);
    let lock_file = open_lock_file(lock_path.as_path())?;
    // Acquire exclusive lock (blocks until available)
    lock_file.lock()?;
    tracing::debug!("Lock acquired: {:?}", lock_path);

    // Check again after acquiring the lock, in case another thread completed
    // the installation while we were downloading
    if is_exists_file(&bin_file)? {
        tracing::debug!("bin_file already exists after lock acquisition, skip rename");
        return Ok((install_dir, package_name, version));
    }

    // rename $target_dir_tmp to $target_dir
    tracing::debug!("Rename {:?} to {:?}", target_dir_tmp, target_dir);
    remove_dir_all_force(&target_dir).await?;
    tokio::fs::rename(&target_dir_tmp, &target_dir).await?;

    // create shim file
    tracing::debug!("Create shim files for {}", bin_name);
    create_shim_files(package_manager_type, &bin_prefix).await?;

    Ok((install_dir, package_name, version))
}

/// Open a lock file without truncating it. This is required on Windows
/// where `File::create` implies truncation, which is forbidden when another
/// process holds an exclusive lock on the file.
fn open_lock_file(lock_path: &Path) -> io::Result<File> {
    fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(lock_path)
}

/// Get the platform-specific npm package name for bun.
/// Returns the `@oven/bun-{os}-{arch}` package name for the current platform.
fn get_bun_platform_package_name() -> Result<&'static str, Error> {
    let name = match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => "@oven/bun-darwin-aarch64",
        ("macos", "x86_64") => "@oven/bun-darwin-x64",
        #[cfg(target_env = "musl")]
        ("linux", "aarch64") => "@oven/bun-linux-aarch64-musl",
        #[cfg(not(target_env = "musl"))]
        ("linux", "aarch64") => "@oven/bun-linux-aarch64",
        #[cfg(target_env = "musl")]
        ("linux", "x86_64") => "@oven/bun-linux-x64-musl",
        #[cfg(not(target_env = "musl"))]
        ("linux", "x86_64") => "@oven/bun-linux-x64",
        ("windows", "x86_64") => "@oven/bun-windows-x64",
        ("windows", "aarch64") => "@oven/bun-windows-aarch64",
        (os, arch) => {
            return Err(Error::UnsupportedPackageManager(
                format!("bun (unsupported platform: {os}-{arch})").into(),
            ));
        }
    };
    Ok(name)
}

/// Download bun package manager (native binary) from npm.
///
/// Unlike JS-based package managers (pnpm/npm/yarn), bun is a native binary
/// distributed via platform-specific npm packages (`@oven/bun-{os}-{arch}`).
///
/// Layout: `$VP_HOME/package_manager/bun/{version}/bun/bin/bun.native`
async fn download_bun_package_manager(
    version: &Str,
    home_dir: &AbsolutePath,
) -> Result<(AbsolutePathBuf, Str, Str), Error> {
    let package_name: Str = "bun".into();
    let platform_package_name = get_bun_platform_package_name()?;

    // $VP_HOME/package_manager/bun/{version}
    let target_dir = home_dir.join("package_manager").join("bun").join(version.as_str());
    let install_dir = target_dir.join("bun");
    let bin_prefix = install_dir.join("bin");
    let bin_file = bin_prefix.join("bun");

    // If shims already exist, return early
    if is_exists_file(&bin_file)?
        && is_exists_file(bin_file.with_extension("cmd"))?
        && is_exists_file(bin_file.with_extension("ps1"))?
    {
        return Ok((install_dir, package_name, version.clone()));
    }

    let parent_dir = target_dir.parent().unwrap();
    tokio::fs::create_dir_all(parent_dir).await?;

    // Download the platform-specific package directly
    let platform_tgz_url = get_npm_package_tgz_url(platform_package_name, version);
    let target_dir_tmp = tempfile::tempdir_in(parent_dir)?.path().to_path_buf();

    download_and_extract_tgz_with_hash(&platform_tgz_url, &target_dir_tmp, None).await.map_err(
        |err| {
            if let Error::Reqwest(e) = &err
                && let Some(status) = e.status()
                && status == reqwest::StatusCode::NOT_FOUND
            {
                Error::PackageManagerVersionNotFound {
                    name: "bun".into(),
                    version: version.clone(),
                    url: platform_tgz_url.into(),
                }
            } else {
                err
            }
        },
    )?;

    // Create the expected directory structure: bun/bin/
    let tmp_bun_dir = target_dir_tmp.join("bun");
    let tmp_bin_dir = tmp_bun_dir.join("bin");
    tokio::fs::create_dir_all(&tmp_bin_dir).await?;

    // The platform package extracts to `package/bin/` with the bun binary inside
    // Find the native binary in the extracted package
    let package_dir = target_dir_tmp.join("package");
    let package_bin_dir = package_dir.join("bin");
    let native_bin_src =
        if cfg!(windows) { package_bin_dir.join("bun.exe") } else { package_bin_dir.join("bun") };

    // Move native binary to bin/bun.native
    let native_bin_dest = if cfg!(windows) {
        tmp_bin_dir.join("bun.native.exe")
    } else {
        tmp_bin_dir.join("bun.native")
    };
    tokio::fs::rename(&native_bin_src, &native_bin_dest).await?;

    // Set executable permission on the native binary
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&native_bin_dest, fs::Permissions::from_mode(0o755)).await?;
    }

    // Clean up the extracted package directory
    remove_dir_all_force(&package_dir).await?;

    // Acquire lock for atomic rename
    let lock_path = parent_dir.join(format!("{version}.lock"));
    tracing::debug!("Acquire lock file: {:?}", lock_path);
    let lock_file = open_lock_file(lock_path.as_path())?;
    lock_file.lock()?;
    tracing::debug!("Lock acquired: {:?}", lock_path);

    if is_exists_file(&bin_file)? {
        tracing::debug!("bun bin_file already exists after lock acquisition, skip rename");
        return Ok((install_dir, package_name, version.clone()));
    }

    // Rename temp dir to final location
    tracing::debug!("Rename {:?} to {:?}", target_dir_tmp, target_dir);
    remove_dir_all_force(&target_dir).await?;
    tokio::fs::rename(&target_dir_tmp, &target_dir).await?;

    // Create native binary shims
    tracing::debug!("Create shim files for bun");
    create_shim_files(PackageManagerType::Bun, &bin_prefix).await?;

    Ok((install_dir, package_name, version.clone()))
}

/// Remove the directory and all its contents.
/// Ignore the error if the directory is not found.
async fn remove_dir_all_force(path: impl AsRef<Path>) -> Result<(), std::io::Error> {
    let path = path.as_ref();
    remove_dir_all(path).await.or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            tracing::error!("remove_dir_all_force path: {:?} error: {e:?}", path);
            Err(e)
        }
    })
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
        PackageManagerType::Bun => {
            // bun is a native binary, not a JS package.
            // Create native binary shims instead of Node.js-based shims.
            let bin_prefix = bin_prefix.as_ref();
            return create_bun_shim_files(bin_prefix).await;
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

/// Create shim files for bun's native binary.
///
/// Bun is a native binary distributed via platform-specific npm packages.
/// The native binary is placed at `bin_prefix/bun.native` (unix) or
/// `bin_prefix/bun.native.exe` (windows), and we create shim wrappers
/// that exec it directly (without Node.js).
async fn create_bun_shim_files(bin_prefix: &AbsolutePath) -> Result<(), Error> {
    // The native binary should already be at bin_prefix/bun.native (unix) or
    // bin_prefix/bun.native.exe (windows), placed there by download_bun_platform_binary.
    let native_bin = if cfg!(windows) {
        bin_prefix.join("bun.native.exe")
    } else {
        bin_prefix.join("bun.native")
    };
    if !is_exists_file(&native_bin)? {
        return Err(Error::CannotFindBinaryPath(
            "bun native binary not found. Expected bin/bun.native".into(),
        ));
    }

    // Create bun shim -> bun.native
    let bun_shim = bin_prefix.join("bun");
    shim::write_native_shims(&native_bin, &bun_shim).await?;

    // Create bunx shim -> bun.native (bunx is just bun with different argv[0])
    let bunx_shim = bin_prefix.join("bunx");
    shim::write_native_shims(&native_bin, &bunx_shim).await?;

    Ok(())
}

/// Write the resolved package manager into `devEngines.packageManager`.
///
/// Used by auto-pin when detection had no explicit field (rfcs/dev-engines.md):
/// the exact resolved version is recorded with `onFail: "download"` so future
/// runs are deterministic. Preserves the file's key order and formatting style,
/// placing `devEngines` next to `engines` when present.
async fn set_dev_engines_package_manager_field(
    package_json_path: impl AsRef<AbsolutePath>,
    package_manager_type: PackageManagerType,
    version: &str,
) -> Result<(), Error> {
    let package_json_path = package_json_path.as_ref();
    let content = if is_exists_file(package_json_path)? {
        tokio::fs::read_to_string(&package_json_path).await?
    } else {
        "{}\n".to_string()
    };
    let entry = serde_json::json!({
        "name": package_manager_type.to_string(),
        "version": version,
        "onFail": "download",
    });
    let updated = vite_shared::edit_json_object(&content, |obj| {
        if let Some(dev_engines) = obj.get_mut("devEngines").and_then(|v| v.as_object_mut()) {
            dev_engines.insert("packageManager".into(), entry);
        } else {
            vite_shared::insert_after(
                obj,
                "engines",
                "devEngines",
                serde_json::json!({ "packageManager": entry }),
            );
        }
    })?;
    tokio::fs::write(&package_json_path, updated).await?;
    tracing::debug!(
        "set_dev_engines_package_manager_field: {:?} to {}@{}",
        package_json_path,
        package_manager_type,
        version
    );
    Ok(())
}

pub(crate) use vite_shared::format_path_prepended as format_path_env;

/// Common CI environment variables
const CI_ENV_VARS: &[&str] = &[
    "CI",
    "CONTINUOUS_INTEGRATION",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "TRAVIS",
    "JENKINS_URL",
    "BUILDKITE",
    "DRONE",
    "CODEBUILD_BUILD_ID", // AWS CodeBuild
    "TF_BUILD",           // Azure Pipelines
];

/// Check if running in a CI environment
fn is_ci_environment() -> bool {
    CI_ENV_VARS.iter().any(|key| env::var(key).is_ok())
}

/// Interactive menu for selecting a package manager with keyboard navigation
fn interactive_package_manager_menu() -> Result<PackageManagerType, Error> {
    let options = [
        ("pnpm (recommended)", PackageManagerType::Pnpm),
        ("npm", PackageManagerType::Npm),
        ("yarn", PackageManagerType::Yarn),
        ("bun", PackageManagerType::Bun),
    ];

    let mut selected_index = 0;

    // Print header and instructions with proper line breaks
    println!("\nNo package manager detected. Please select one:");
    println!(
        "   Use ↑↓ arrows to navigate, Enter to select, 1-{} for quick selection",
        options.len()
    );
    println!("   Press Esc, q, or Ctrl+C to cancel installation\n");

    // Enable raw mode for keyboard input
    terminal::enable_raw_mode()?;

    // Clear the selection area and hide cursor
    execute!(io::stdout(), cursor::Hide)?;

    let result = loop {
        // Display menu with current selection
        for (i, (name, _)) in options.iter().enumerate() {
            execute!(io::stdout(), cursor::MoveToColumn(2))?;

            if i == selected_index {
                // Highlight selected item
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Blue),
                    Print("▶ "),
                    Print(format!("[{}] ", i + 1)),
                    Print(name),
                    ResetColor,
                    Print(" ← ")
                )?;
            } else {
                execute!(
                    io::stdout(),
                    Print("  "),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("[{}] ", i + 1)),
                    ResetColor,
                    Print(name),
                    Print("   ")
                )?;
            }

            if i < options.len() - 1 {
                execute!(io::stdout(), Print("\n"))?;
            }
        }

        // Move cursor back up for next iteration
        if options.len() > 1 {
            execute!(io::stdout(), cursor::MoveUp((options.len() - 1) as u16))?;
        }

        // Read keyboard input, skipping non-Press events (e.g. Release on Windows)
        let (code, modifiers) = loop {
            if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event::read()?
                && kind == KeyEventKind::Press
            {
                break (code, modifiers);
            }
        };

        match code {
            // Handle Ctrl+C for exit
            KeyCode::Char('c') if modifiers.contains(event::KeyModifiers::CONTROL) => {
                // Clean up terminal before exiting
                terminal::disable_raw_mode()?;
                execute!(
                    io::stdout(),
                    cursor::Show,
                    cursor::MoveDown(options.len() as u16),
                    Print("\n\n"),
                    SetForegroundColor(Color::Yellow),
                    Print("⚠ Installation cancelled by user\n"),
                    ResetColor
                )?;
                return Err(Error::UserCancelled);
            }
            KeyCode::Up => {
                selected_index = selected_index.saturating_sub(1);
            }
            KeyCode::Down if selected_index < options.len() - 1 => {
                selected_index += 1;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                break Ok(options[selected_index].1);
            }
            KeyCode::Char('1') => {
                break Ok(options[0].1);
            }
            KeyCode::Char('2') if options.len() > 1 => {
                break Ok(options[1].1);
            }
            KeyCode::Char('3') if options.len() > 2 => {
                break Ok(options[2].1);
            }
            KeyCode::Char('4') if options.len() > 3 => {
                break Ok(options[3].1);
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Exit on escape/quit
                terminal::disable_raw_mode()?;
                execute!(
                    io::stdout(),
                    cursor::Show,
                    cursor::MoveDown(options.len() as u16),
                    Print("\n\n"),
                    SetForegroundColor(Color::Yellow),
                    Print("⚠ Installation cancelled by user\n"),
                    ResetColor
                )?;
                return Err(Error::UserCancelled);
            }
            _ => {}
        }
    };

    // Clean up: disable raw mode and show cursor
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), cursor::Show, cursor::MoveDown(options.len() as u16), Print("\n"))?;

    // Print selection confirmation
    if let Ok(pm) = &result {
        let name = match pm {
            PackageManagerType::Pnpm => "pnpm",
            PackageManagerType::Npm => "npm",
            PackageManagerType::Yarn => "yarn",
            PackageManagerType::Bun => "bun",
        };
        println!("\n✓ Selected package manager: {name}\n");
    }

    result
}

/// Prompt the user to select a package manager
fn prompt_package_manager_selection() -> Result<PackageManagerType, Error> {
    // In CI environment, automatically use pnpm without prompting
    if is_ci_environment() {
        tracing::info!("CI environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Check if stdin is a TTY (terminal) - if not, use default
    if !io::stdin().is_terminal() {
        tracing::info!("Non-interactive environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Try interactive menu first, fall back to simple prompt on error
    match interactive_package_manager_menu() {
        Ok(pm) => Ok(pm),
        Err(err) => {
            match err {
                Error::UserCancelled => Err(err),
                // Fallback to simple text prompt if interactive menu fails
                _ => simple_text_prompt(),
            }
        }
    }
}

/// Simple text-based prompt as fallback
fn simple_text_prompt() -> Result<PackageManagerType, Error> {
    let managers = [
        ("pnpm", PackageManagerType::Pnpm),
        ("npm", PackageManagerType::Npm),
        ("yarn", PackageManagerType::Yarn),
        ("bun", PackageManagerType::Bun),
    ];

    println!("\nNo package manager detected. Please select one:");
    println!("────────────────────────────────────────────────");

    for (i, (name, _)) in managers.iter().enumerate() {
        if i == 0 {
            println!("  [{}] {} (recommended)", i + 1, name);
        } else {
            println!("  [{}] {}", i + 1, name);
        }
    }

    print!("\nEnter your choice (1-{}) [default: 1]: ", managers.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let choice = input.trim();
    let index = if choice.is_empty() {
        0 // Default to pnpm
    } else {
        choice
            .parse::<usize>()
            .ok()
            .and_then(|n| if n > 0 && n <= managers.len() { Some(n - 1) } else { None })
            .unwrap_or(0) // Default to pnpm if invalid input
    };

    let (name, selected_type) = &managers[index];
    println!("✓ Selected package manager: {name}\n");

    Ok(*selected_type)
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
    fn test_package_manager_type_from_tool_includes_aliases() {
        assert_eq!(PackageManagerType::from_tool("npm"), Some(PackageManagerType::Npm));
        assert_eq!(PackageManagerType::from_tool("npx"), Some(PackageManagerType::Npm));
        assert_eq!(PackageManagerType::from_tool("pnpm"), Some(PackageManagerType::Pnpm));
        assert_eq!(PackageManagerType::from_tool("pnpx"), Some(PackageManagerType::Pnpm));
        assert_eq!(PackageManagerType::from_tool("yarn"), Some(PackageManagerType::Yarn));
        assert_eq!(PackageManagerType::from_tool("yarnpkg"), Some(PackageManagerType::Yarn));
        assert_eq!(PackageManagerType::from_tool("bun"), Some(PackageManagerType::Bun));
        assert_eq!(PackageManagerType::from_tool("bunx"), Some(PackageManagerType::Bun));
        assert_eq!(PackageManagerType::from_tool("node"), None);
        assert_eq!(PackageManagerType::from_tool("tsc"), None);
    }

    #[test]
    fn test_bin_name_for_tool_preserves_aliases() {
        assert_eq!(PackageManagerType::Npm.bin_name_for_tool("npm"), "npm");
        assert_eq!(PackageManagerType::Npm.bin_name_for_tool("npx"), "npx");
        assert_eq!(PackageManagerType::Pnpm.bin_name_for_tool("pnpm"), "pnpm");
        assert_eq!(PackageManagerType::Pnpm.bin_name_for_tool("pnpx"), "pnpx");
        assert_eq!(PackageManagerType::Yarn.bin_name_for_tool("yarn"), "yarn");
        assert_eq!(PackageManagerType::Yarn.bin_name_for_tool("yarnpkg"), "yarnpkg");
        assert_eq!(PackageManagerType::Bun.bin_name_for_tool("bun"), "bun");
        assert_eq!(PackageManagerType::Bun.bin_name_for_tool("bunx"), "bunx");
    }

    #[test]
    fn test_resolve_package_manager_from_package_json_npm() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        create_package_json(
            &temp_dir_path,
            r#"{"name": "test-package", "packageManager": "npm@11.14.0"}"#,
        );

        let resolution = resolve_package_manager_from_package_json(&temp_dir_path)
            .expect("Should resolve packageManager")
            .expect("Should find packageManager field");

        assert_eq!(resolution.package_manager_type, PackageManagerType::Npm);
        assert_eq!(resolution.version.as_str(), "11.14.0");
        assert!(resolution.hash.is_none());
        assert_eq!(resolution.source.as_str(), "packageManager");
        assert_eq!(resolution.project_root, temp_dir_path);
    }

    #[test]
    fn test_resolve_package_manager_from_package_json_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        create_package_json(
            &temp_dir_path,
            r#"{"name": "test-package", "packageManager": "pnpm@10.19.0+sha512.abc123"}"#,
        );

        let resolution = resolve_package_manager_from_package_json(&temp_dir_path)
            .expect("Should resolve packageManager")
            .expect("Should find packageManager field");

        assert_eq!(resolution.package_manager_type, PackageManagerType::Pnpm);
        assert_eq!(resolution.version.as_str(), "10.19.0");
        assert_eq!(resolution.hash.unwrap().as_str(), "sha512.abc123");
    }

    #[test]
    fn test_resolve_package_manager_from_package_json_missing() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        create_package_json(&temp_dir_path, r#"{"name": "test-package"}"#);

        let resolution = resolve_package_manager_from_package_json(&temp_dir_path)
            .expect("Missing packageManager should not error");

        assert!(resolution.is_none());
    }

    #[test]
    fn test_resolve_package_manager_from_package_json_without_package_json() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        let resolution = resolve_package_manager_from_package_json(&temp_dir_path)
            .expect("Missing package.json should not error");

        assert!(resolution.is_none());
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
        assert!(matches!(err, vite_workspace::Error::PackageJsonNotFound(_)));
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
        let (found, _) = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(&*found.path, &*temp_dir_path);
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
        let (found, _) = find_workspace_root(&temp_dir_path).unwrap();
        assert_eq!(&*found.path, &*temp_dir_path);
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
        let (found, _) = find_workspace_root(&nested_dir).unwrap();
        assert_eq!(&*found.path, &*temp_dir_path);
        assert!(matches!(found.workspace_file, WorkspaceFile::NonWorkspacePackage(_)));
        let package_root = find_package_root(&temp_dir_path).unwrap();
        // equal to workspace root
        assert_eq!(&*package_root.path, &*found.path);
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
        assert!(matches!(err, vite_workspace::Error::PackageJsonNotFound(_)));
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

        let (result, _) = find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        assert_eq!(&*result.path, &*temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_pnpm_workspace_yaml_in_parent_dir() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let (result, _) = find_workspace_root(&sub_dir).expect("Should find workspace root");
        assert_eq!(&*result.path, &*temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_package_json_workspaces() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-workspace", "workspaces": ["packages/*"]}"#;
        create_package_json(&temp_dir_path, package_content);

        let (result, _) = find_workspace_root(&temp_dir_path).unwrap();
        assert_eq!(&*result.path, &*temp_dir_path);
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

        let (result, _) = find_workspace_root(&sub_dir).unwrap();
        assert_eq!(&*result.path, &*temp_dir_path);
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

        let (result, _) = find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        assert_eq!(&*result.path, &*temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_falls_back_to_package_root_when_no_workspace_found() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let sub_dir = temp_dir_path.join("subdir");
        fs::create_dir(&sub_dir).expect("Failed to create subdirectory");

        let (result, _) = find_workspace_root(&sub_dir).expect("Should fall back to package root");
        assert_eq!(&*result.path, &*temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_with_nested_structure() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workspace_content = "packages:\n  - 'packages/*'";
        create_pnpm_workspace_yaml(&temp_dir_path, workspace_content);

        let nested_dir = temp_dir_path.join("packages").join("app").join("src");
        fs::create_dir_all(&nested_dir).expect("Failed to create nested directories");

        let (result, _) = find_workspace_root(&nested_dir).expect("Should find workspace root");
        assert_eq!(&*result.path, &*temp_dir_path);
    }

    #[test]
    fn test_find_workspace_root_without_workspace_files_returns_package_root() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (result, _) = find_workspace_root(&temp_dir_path).expect("Should return package root");
        assert_eq!(&*result.path, &*temp_dir_path);
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
        let (workspace_root, _) =
            find_workspace_root(&sub_dir).expect("Should find subdirectory package");
        assert_eq!(&*workspace_root.path, &*sub_dir);
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

        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir.path().join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        println!("package_json: {package_json:?}");
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "pnpm");
        assert!(Version::parse(entry["version"].as_str().unwrap()).is_ok());
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
        // the legacy field is not written
        assert!(package_json.get("packageManager").is_none());
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
        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        println!("package_json: {package_json:?}");
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "yarn");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
        // keep other fields
        assert_eq!(package_json["name"].as_str().unwrap(), "test-package");
    }

    #[tokio::test]
    #[cfg(not(windows))] // FIXME
    async fn test_detect_package_manager_with_package_lock_json() {
        use std::process::Command;

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
        assert!(is_exists_file(bin_prefix.join("npm")).unwrap());
        assert!(is_exists_file(bin_prefix.join("npm.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("npm.ps1")).unwrap());
        assert!(is_exists_file(bin_prefix.join("npx")).unwrap());
        assert!(is_exists_file(bin_prefix.join("npx.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("npx.ps1")).unwrap());

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
        use std::process::Command;

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
        assert!(is_exists_file(bin_prefix.join("pnpm.cjs")).unwrap());
        assert!(is_exists_file(bin_prefix.join("pnpm.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("pnpm.ps1")).unwrap());
        assert!(is_exists_file(bin_prefix.join("pnpx.cjs")).unwrap());
        assert!(is_exists_file(bin_prefix.join("pnpx.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("pnpx.ps1")).unwrap());

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
    async fn test_parse_package_manager_with_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test with sha512 hash
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@1.22.22+sha512.a6b2f7906b721bba3d67d4aff083df04dad64c399707841b7acf00f6b133b7ac24255f2652fa22ae3534329dc6180534e98d17432037ff6fd140556e2bb3137e"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Yarn);
        assert_eq!(version, "1.22.22");
        assert!(hash.is_some());
        assert_eq!(
            hash.unwrap(),
            "sha512.a6b2f7906b721bba3d67d4aff083df04dad64c399707841b7acf00f6b133b7ac24255f2652fa22ae3534329dc6180534e98d17432037ff6fd140556e2bb3137e"
        );
    }

    #[tokio::test]
    async fn test_resolve_package_manager_from_dev_engines_exact() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": {"name": "pnpm", "version": "9.15.0"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let resolution =
            resolve_package_manager_from_package_json(&temp_dir_path).unwrap().unwrap();
        assert_eq!(resolution.package_manager_type, PackageManagerType::Pnpm);
        assert_eq!(resolution.version, "9.15.0");
        assert!(resolution.hash.is_none());
        assert_eq!(resolution.source, "devEngines.packageManager");
    }

    #[tokio::test]
    async fn test_resolve_package_manager_from_dev_engines_uncached_range_kept() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // a range no downloaded version can satisfy: the raw requirement is kept
        // and resolved at download time
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": {"name": "pnpm", "version": ">=999.0.0"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let resolution =
            resolve_package_manager_from_package_json(&temp_dir_path).unwrap().unwrap();
        assert_eq!(resolution.package_manager_type, PackageManagerType::Pnpm);
        assert_eq!(resolution.version, ">=999.0.0");
        assert_eq!(resolution.source, "devEngines.packageManager");
    }

    #[tokio::test]
    async fn test_resolve_package_manager_field_wins_over_dev_engines() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "packageManager": "npm@10.5.0",
            "devEngines": {"packageManager": {"name": "pnpm", "version": "^9.0.0"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let resolution =
            resolve_package_manager_from_package_json(&temp_dir_path).unwrap().unwrap();
        assert_eq!(resolution.package_manager_type, PackageManagerType::Npm);
        assert_eq!(resolution.version, "10.5.0");
        assert_eq!(resolution.source, "packageManager");
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_exact_version() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": {"name": "pnpm", "version": "9.15.0", "onFail": "download"}
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);
        // a lockfile that would otherwise win: devEngines has higher priority
        fs::write(temp_dir_path.join("package-lock.json"), "{}").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "9.15.0");
        assert!(hash.is_none());
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_range_preserved() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": {"name": "pnpm", "version": "^9.0.0"}
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Pnpm);
        // the range is preserved here; download resolves it to an exact version
        assert_eq!(version, "^9.0.0");
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_absent_version_is_any() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": {"name": "bun"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version, "*");
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_invalid_version_is_any() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": {"name": "pnpm", "version": "not-a-version"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Pnpm);
        // lenient read: an invalid range is treated as any version
        assert_eq!(version, "*");
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_array_first_supported_wins() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": [
                    {"name": "vlt", "version": "^1.0.0"},
                    {"name": "yarn", "version": "4.9.2"},
                    {"name": "npm", "version": ">=10"}
                ]
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Yarn);
        assert_eq!(version, "4.9.2");
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_unsupported_single_errors() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // single entry defaults to onFail: error (devEngines spec)
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": {"name": "vlt", "version": "^1.0.0"}}
        }"#;
        create_package_json(&temp_dir_path, package_content);
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let result = get_package_manager_type_and_version(&workspace_root, None);
        assert!(matches!(result, Err(Error::UnsupportedDevEnginesPackageManager(_))));
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_unsupported_ignore_falls_through() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": {"name": "vlt", "version": "^1.0.0", "onFail": "ignore"}
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        // onFail: ignore continues down the detection chain to the lockfile
        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "latest");
        assert_eq!(source, PackageManagerSource::LockfileOrConfig);
    }

    // npm-install-checks: "noop options" / "empty array along side error"
    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_empty_array_falls_through() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {"packageManager": []}
        }"#;
        create_package_json(&temp_dir_path, package_content);
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        // an empty array imposes nothing: detection falls through to the lockfile
        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "latest");
        assert_eq!(source, PackageManagerSource::LockfileOrConfig);
    }

    // npm-install-checks: "returns the last failure" (array with no acceptable
    // entry applies the effective onFail of the last entry, which defaults to error)
    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_all_unsupported_array_errors() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": [
                    {"name": "vlt", "version": "^1.0.0"},
                    {"name": "deno", "version": "^2.0.0"}
                ]
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let result = get_package_manager_type_and_version(&workspace_root, None);
        assert!(matches!(result, Err(Error::UnsupportedDevEnginesPackageManager(_))));
    }

    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_array_last_warn_falls_through() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": [
                    {"name": "vlt", "version": "^1.0.0", "onFail": "ignore"},
                    {"name": "deno", "version": "^2.0.0", "onFail": "warn"}
                ]
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);
        fs::write(temp_dir_path.join("pnpm-lock.yaml"), "lockfileVersion: '6.0'").unwrap();

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        // onFail: warn on the last entry warns and continues down the chain
        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "latest");
        assert_eq!(source, PackageManagerSource::LockfileOrConfig);
    }

    // npm-install-checks: "spec 2" uses [bun, yarn] where npm matches the current
    // environment. Vite+ provisions the environment instead of validating it, so
    // the first supported entry wins (rfcs/dev-engines.md).
    #[tokio::test]
    async fn test_detect_dev_engines_package_manager_first_supported_entry_wins() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "devEngines": {
                "packageManager": [
                    {"name": "bun", "version": ">= 1.0.0", "onFail": "ignore"},
                    {"name": "yarn", "version": "3.2.3", "onFail": "download"}
                ]
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version, ">= 1.0.0");
        assert_eq!(source, PackageManagerSource::DevEnginesPackageManager);
    }

    /// Test helper: parse a `devEngines.packageManager` field from JSON.
    fn parse_dev_engines_pm_field(json: &str) -> vite_shared::DevEngineField {
        let pkg: vite_shared::PackageJson = serde_json::from_str(json).unwrap();
        pkg.dev_engines.unwrap().package_manager.unwrap()
    }

    /// Test helper: build a `PackageManagerResolution` for conflict-message tests.
    fn resolution_for_conflict_test(
        package_manager_type: PackageManagerType,
        version: &str,
    ) -> PackageManagerResolution {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        PackageManagerResolution {
            package_manager_type,
            version: version.into(),
            hash: None,
            source: "packageManager".into(),
            source_path: temp_dir_path.join("package.json"),
            project_root: temp_dir_path,
        }
    }

    // npm-install-checks: "invalid name"
    #[test]
    fn test_dev_engines_conflict_message_name_mismatch() {
        let field = parse_dev_engines_pm_field(
            r#"{"devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}}"#,
        );
        let resolution = resolution_for_conflict_test(PackageManagerType::Npm, "10.5.0");

        let message = dev_engines_package_manager_conflict_message(&field, &resolution).unwrap();
        assert!(message.contains("packageManager is npm@10.5.0"), "got: {message}");
        assert!(message.contains("requires \"pnpm\""), "got: {message}");
        assert!(message.contains("error in a future release"), "got: {message}");
    }

    // npm-install-checks: "semver version is not in range"
    #[test]
    fn test_dev_engines_conflict_message_version_not_satisfying() {
        let field = parse_dev_engines_pm_field(
            r#"{"devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}}"#,
        );
        let resolution = resolution_for_conflict_test(PackageManagerType::Pnpm, "10.9.0");

        let message = dev_engines_package_manager_conflict_message(&field, &resolution).unwrap();
        assert!(
            message.contains("pnpm@10.9.0 does not satisfy devEngines.packageManager"),
            "got: {message}"
        );
        assert!(message.contains("error in a future release"), "got: {message}");
    }

    // npm-install-checks: "semver version is in range" / "name only" /
    // non-semver wanted versions (doctor reports those, no conflict warning here)
    #[test]
    fn test_dev_engines_conflict_message_none_when_consistent() {
        let resolution = resolution_for_conflict_test(PackageManagerType::Pnpm, "11.5.1");

        // exact version satisfying the declared range is not a conflict
        let field = parse_dev_engines_pm_field(
            r#"{"devEngines": {"packageManager": {"name": "pnpm", "version": "^11.0.0"}}}"#,
        );
        assert!(dev_engines_package_manager_conflict_message(&field, &resolution).is_none());

        // name only: any version satisfies
        let field =
            parse_dev_engines_pm_field(r#"{"devEngines": {"packageManager": {"name": "pnpm"}}}"#);
        assert!(dev_engines_package_manager_conflict_message(&field, &resolution).is_none());

        // a non-semver wanted version is not range-checked here
        let field = parse_dev_engines_pm_field(
            r#"{"devEngines": {"packageManager": {"name": "pnpm", "version": "test-version"}}}"#,
        );
        assert!(dev_engines_package_manager_conflict_message(&field, &resolution).is_none());

        // empty array imposes nothing
        let field = parse_dev_engines_pm_field(r#"{"devEngines": {"packageManager": []}}"#);
        assert!(dev_engines_package_manager_conflict_message(&field, &resolution).is_none());
    }

    #[tokio::test]
    async fn test_detect_package_manager_field_priority_over_dev_engines() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{
            "name": "test-package",
            "packageManager": "npm@10.5.0",
            "devEngines": {
                "packageManager": {"name": "pnpm", "version": "^9.0.0"}
            }
        }"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, _, source) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        // packageManager field drives selection; a conflict warning is printed
        assert_eq!(pm_type, PackageManagerType::Npm);
        assert_eq!(version, "10.5.0");
        assert_eq!(source, PackageManagerSource::PackageManagerField);
    }

    #[tokio::test]
    async fn test_set_dev_engines_package_manager_field_preserves_format_and_engines() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_dir_path.join("package.json");
        // 4-space indentation and an existing engines.node that must stay unchanged
        let package_content = "{\n    \"name\": \"test-package\",\n    \"engines\": {\n        \"node\": \">=20.0.0\"\n    },\n    \"scripts\": {}\n}\n";
        fs::write(&package_json_path, package_content).unwrap();

        set_dev_engines_package_manager_field(
            &package_json_path,
            PackageManagerType::Pnpm,
            "9.15.0",
        )
        .await
        .unwrap();

        let updated = fs::read_to_string(&package_json_path).unwrap();
        // engines.node is kept unchanged and devEngines is placed right after it
        assert_eq!(
            updated,
            "{\n    \"name\": \"test-package\",\n    \"engines\": {\n        \"node\": \">=20.0.0\"\n    },\n    \"devEngines\": {\n        \"packageManager\": {\n            \"name\": \"pnpm\",\n            \"version\": \"9.15.0\",\n            \"onFail\": \"download\"\n        }\n    },\n    \"scripts\": {}\n}\n"
        );
    }

    #[tokio::test]
    async fn test_set_dev_engines_package_manager_field_keeps_existing_runtime() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_json_path = temp_dir_path.join("package.json");
        let package_content = r#"{
  "name": "test-package",
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^24.0.0"
    }
  }
}
"#;
        fs::write(&package_json_path, package_content).unwrap();

        set_dev_engines_package_manager_field(
            &package_json_path,
            PackageManagerType::Npm,
            "11.4.0",
        )
        .await
        .unwrap();

        let updated = fs::read_to_string(&package_json_path).unwrap();
        let package_json: serde_json::Value = serde_json::from_str(&updated).unwrap();
        // the existing runtime entry is preserved
        assert_eq!(package_json["devEngines"]["runtime"]["version"].as_str().unwrap(), "^24.0.0");
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "npm");
        assert_eq!(entry["version"].as_str().unwrap(), "11.4.0");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
    }

    #[tokio::test]
    async fn test_parse_package_manager_with_sha1_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test with sha1 hash
        let package_content = r#"{"name": "test-package", "packageManager": "npm@10.5.0+sha1.abcd1234567890abcdef1234567890abcdef1234"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Npm);
        assert_eq!(version, "10.5.0");
        assert!(hash.is_some());
        assert_eq!(hash.unwrap(), "sha1.abcd1234567890abcdef1234567890abcdef1234");
    }

    #[tokio::test]
    async fn test_parse_package_manager_with_sha224_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test with sha224 hash
        let package_content = r#"{"name": "test-package", "packageManager": "pnpm@8.15.0+sha224.1234567890abcdef1234567890abcdef1234567890abcdef12345678"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "8.15.0");
        assert!(hash.is_some());
        assert_eq!(
            hash.unwrap(),
            "sha224.1234567890abcdef1234567890abcdef1234567890abcdef12345678"
        );
    }

    #[tokio::test]
    async fn test_parse_package_manager_with_sha256_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test with sha256 hash
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@4.0.0+sha256.1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Yarn);
        assert_eq!(version, "4.0.0");
        assert!(hash.is_some());
        assert_eq!(
            hash.unwrap(),
            "sha256.1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        );
    }

    #[tokio::test]
    async fn test_parse_package_manager_without_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test without hash
        let package_content = r#"{"name": "test-package", "packageManager": "pnpm@8.15.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).unwrap();

        assert_eq!(pm_type, PackageManagerType::Pnpm);
        assert_eq!(version, "8.15.0");
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn test_download_success_package_manager_with_hash() {
        use std::process::Command;

        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@1.22.22+sha512.a6b2f7906b721bba3d67d4aff083df04dad64c399707841b7acf00f6b133b7ac24255f2652fa22ae3534329dc6180534e98d17432037ff6fd140556e2bb3137e"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect yarn with version and hash");
        assert_eq!(result.bin_name, "yarn");

        // check shim files
        let bin_prefix = result.get_bin_prefix();
        assert!(is_exists_file(bin_prefix.join("yarn.js")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn.ps1")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg.ps1")).unwrap());

        // run pnpm --version
        let mut paths =
            env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
        paths.insert(0, bin_prefix.into_path_buf());
        let mut cmd = "yarn";
        if cfg!(windows) {
            cmd = "yarn.cmd";
        }
        let output = Command::new(cmd)
            .arg("--version")
            .env("PATH", env::join_paths(paths).unwrap())
            .output()
            .expect("Failed to run yarn");
        // println!("pnpm --version: {:?}", output);
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1.22.22");
    }

    #[tokio::test]
    async fn test_download_failed_package_manager_with_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@1.22.21+sha512.a6b2f7906b721bba3d67d4aff083df04dad64c399707841b7acf00f6b133b7ac24255f2652fa22ae3534329dc6180534e98d17432037ff6fd140556e2bb3137e"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path).build().await;
        assert!(result.is_err());
        // Check if it's the expected error type
        if let Err(Error::HashMismatch { expected, actual }) = result {
            assert_eq!(
                expected,
                "sha512.a6b2f7906b721bba3d67d4aff083df04dad64c399707841b7acf00f6b133b7ac24255f2652fa22ae3534329dc6180534e98d17432037ff6fd140556e2bb3137e"
            );
            assert_eq!(
                actual,
                "sha512.ca75da26c00327d26267ce33536e5790f18ebd53266796fbb664d2a4a5116308042dd8ee7003b276a20eace7d3c5561c3577bdd71bcb67071187af124779620a"
            );
        } else {
            panic!("Expected HashMismatch error");
        }
    }

    #[tokio::test]
    async fn test_download_success_package_manager_with_sha1_and_sha224() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "yarn@1.22.20+sha1.167c8ab8d9c8c3826d3725d9579aaea8b47a2b18"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect yarn with version and hash");
        assert_eq!(result.bin_name, "yarn");

        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "pnpm@4.11.6+sha224.7783c4b01916b7a69e6ff05d328df6f83cb7f127e9c96be88739386d"}"#;
        create_package_json(&temp_dir_path, package_content);

        let result = PackageManager::builder(temp_dir_path)
            .build()
            .await
            .expect("Should detect pnpm with version and hash");
        assert_eq!(result.bin_name, "pnpm");
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_yarn_package_manager_field() {
        use std::process::Command;

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
        assert!(is_exists_file(bin_prefix.join("yarn.js")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarn.ps1")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg.cmd")).unwrap());
        assert!(is_exists_file(bin_prefix.join("yarnpkg.ps1")).unwrap());

        // run yarn --version
        let mut cmd = "yarn";
        if cfg!(windows) {
            cmd = "yarn.cmd";
        }
        let mut paths =
            env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
        paths.insert(0, bin_prefix.into_path_buf());
        let output = Command::new(cmd)
            .arg("--version")
            .env("PATH", env::join_paths(paths).unwrap())
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
        println!("result: {result:?}");
        // Check if it's the expected error type
        if let Err(Error::PackageManagerVersionNotFound { name, version, .. }) = result {
            assert_eq!(name, "yarn");
            assert_eq!(version, "10000000000.0.0");
        } else {
            panic!("Expected PackageManagerVersionNotFound error, got {result:?}");
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
        println!("result: {result:?}");
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
        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "yarn");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
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
        if matches!(result, Err(Error::UnrecognizedPackageManager)) {
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
    async fn test_download_package_manager_rejects_path_traversal_version() {
        // Versions containing path separators or traversal components must be
        // rejected before any filesystem operations: `AbsolutePath::join` does
        // not normalize `..`, so a bad version would escape the home dir.
        for bad in ["../../../escape", "..", "1.0.0/../../escape", "/foo/bar", "1.0.0\0", ""] {
            let result = download_package_manager(PackageManagerType::Pnpm, bad, None).await;
            match result {
                Err(Error::InvalidArgument(_)) => {}
                other => panic!("expected InvalidArgument for {bad:?}, got {other:?}"),
            }
        }

        // Bun takes a separate code path but shares the same pre-validation.
        let result = download_package_manager(PackageManagerType::Bun, "../../escape", None).await;
        assert!(matches!(result, Err(Error::InvalidArgument(_))));
    }

    #[tokio::test]
    async fn test_download_package_manager() {
        let result = download_package_manager(PackageManagerType::Yarn, "4.9.2", None).await;
        assert!(result.is_ok());
        let (target_dir, package_name, version) = result.unwrap();
        println!("result: {target_dir:?}");
        assert!(is_exists_file(target_dir.join("bin/yarn")).unwrap());
        assert!(is_exists_file(target_dir.join("bin/yarn.cmd")).unwrap());
        assert_eq!(package_name, "@yarnpkg/cli-dist");
        assert_eq!(version, "4.9.2");

        // again should skip download
        let result = download_package_manager(PackageManagerType::Yarn, "4.9.2", None).await;
        assert!(result.is_ok());
        let (target_dir, package_name, version) = result.unwrap();
        assert!(is_exists_file(target_dir.join("bin/yarn")).unwrap());
        assert!(is_exists_file(target_dir.join("bin/yarn.cmd")).unwrap());
        assert_eq!(package_name, "@yarnpkg/cli-dist");
        assert_eq!(version, "4.9.2");
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
        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir.path().join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "yarn");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
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
        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "pnpm");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
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
        // auto-pin writes devEngines.packageManager (see rfcs/dev-engines.md)
        let package_json_path = temp_dir_path.join("package.json");
        let package_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&package_json_path).unwrap()).unwrap();
        let entry = &package_json["devEngines"]["packageManager"];
        assert_eq!(entry["name"].as_str().unwrap(), "yarn");
        assert_eq!(entry["onFail"].as_str().unwrap(), "download");
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
    // Tests for bun package manager detection
    #[tokio::test]
    async fn test_detect_package_manager_with_bun_lock() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create bun.lock (text format)
        fs::write(temp_dir_path.join("bun.lock"), r#"# bun lockfile"#)
            .expect("Failed to write bun.lock");

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).expect("Should detect bun");
        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version.as_str(), "latest");
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_bun_lockb() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create bun.lockb (binary format)
        fs::write(temp_dir_path.join("bun.lockb"), b"\x00\x01\x02")
            .expect("Failed to write bun.lockb");

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).expect("Should detect bun");
        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version.as_str(), "latest");
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_bunfig_toml() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create bunfig.toml
        fs::write(temp_dir_path.join("bunfig.toml"), "[install]\noptional = true")
            .expect("Failed to write bunfig.toml");

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None).expect("Should detect bun");
        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version.as_str(), "latest");
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_package_manager_field_bun() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package", "packageManager": "bun@1.2.0"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None)
                .expect("Should detect bun from packageManager field");
        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version.as_str(), "1.2.0");
        assert!(hash.is_none());
    }

    #[tokio::test]
    async fn test_detect_package_manager_with_package_manager_field_bun_with_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content =
            r#"{"name": "test-package", "packageManager": "bun@1.2.0+sha512.abc123"}"#;
        create_package_json(&temp_dir_path, package_content);

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, version, hash, _) =
            get_package_manager_type_and_version(&workspace_root, None)
                .expect("Should detect bun with hash");
        assert_eq!(pm_type, PackageManagerType::Bun);
        assert_eq!(version.as_str(), "1.2.0");
        assert_eq!(hash.unwrap().as_str(), "sha512.abc123");
    }

    #[tokio::test]
    async fn test_detect_package_manager_bun_lock_priority_over_pnpmfile() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let package_content = r#"{"name": "test-package"}"#;
        create_package_json(&temp_dir_path, package_content);

        // Create both bun.lock and .pnpmfile.cjs
        fs::write(temp_dir_path.join("bun.lock"), "# bun lockfile")
            .expect("Failed to write bun.lock");
        fs::write(temp_dir_path.join(".pnpmfile.cjs"), "module.exports = {}")
            .expect("Failed to write .pnpmfile.cjs");

        let (workspace_root, _) =
            find_workspace_root(&temp_dir_path).expect("Should find workspace root");
        let (pm_type, _, _, _) =
            get_package_manager_type_and_version(&workspace_root, None).expect("Should detect bun");
        assert_eq!(
            pm_type,
            PackageManagerType::Bun,
            "bun.lock should be detected before .pnpmfile.cjs"
        );
    }

    #[test]
    fn test_get_bun_platform_package_name() {
        let result = get_bun_platform_package_name();
        assert!(result.is_ok(), "Should return a platform package name");
        let name = result.unwrap();
        assert!(
            name.starts_with("@oven/bun-"),
            "Package name should start with @oven/bun-, got: {name}"
        );
        // On musl targets, the package name should contain "-musl"
        #[cfg(target_env = "musl")]
        assert!(
            name.ends_with("-musl"),
            "On musl targets, package name should end with -musl, got: {name}"
        );
    }
    /// Note: The true ERROR_SHARING_VIOLATION occurs when *multiple processes*
    /// attempt to lock the file concurrently on Windows (e.g. during parallel MSBuild tasks).
    /// Standard cargo tests run in a single process, which the Windows OS allows to bypass
    /// the truncation violation. This test validates the safe `OpenOptions` syntax
    /// and ensures `open_lock_file` successfully acquires and releases locks without panicking.
    #[test]
    fn test_concurrent_lock_file_creation_windows_compat() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lock_path = temp_dir.path().join("test_concurrent.lock");

        // Process 1: Open and acquire exclusive lock using the new approach
        let lock_file1 = super::open_lock_file(&lock_path).expect("Failed to open lock file 1");

        // Acquire lock
        lock_file1.lock().expect("Failed to lock file 1");

        // Process 2: Attempt to open the same file while it is exclusively locked.
        // In the buggy implementation (`File::create`), this would throw ERROR_SHARING_VIOLATION
        // on Windows because `create` implies `truncate`, which Windows forbids for locked files.
        let open_result = super::open_lock_file(&lock_path);

        assert!(
            open_result.is_ok(),
            "Expected second handle to open successfully even when locked, but got: {:?}",
            open_result.err()
        );

        // Release lock
        lock_file1.unlock().expect("Failed to unlock file 1");
    }
}
