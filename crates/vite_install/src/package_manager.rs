use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{self, BufReader, IsTerminal, Write},
    path::Path,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use tokio::fs::remove_dir_all;
use vite_error::Error;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;
#[cfg(test)]
use vite_workspace::find_package_root;
use vite_workspace::{WorkspaceFile, WorkspaceRoot, find_workspace_root, load_package_graph};

use crate::{
    config::{get_cache_dir, get_npm_package_tgz_url, get_npm_package_version_url},
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
        let (package_manager_type, version_or_latest, hash) =
            get_package_manager_type_and_version(&workspace_root, self.client_override)?;

        // only download the package manager if it's not already downloaded
        let (install_dir, package_name, version) =
            download_package_manager(package_manager_type, &version_or_latest, hash.as_deref())
                .await?;

        if version_or_latest != version {
            // auto set `packageManager` field in package.json
            let package_json_path = workspace_root.path.join("package.json");
            set_package_manager_field(&package_json_path, package_manager_type, &version).await?;
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
                PackageManagerBuilder::new(&self.cwd)
                    .package_manager_type(selected_type)
                    .build()
                    .await?
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

    #[must_use]
    pub fn get_fingerprint_ignores(&self) -> Result<Vec<Str>, Error> {
        let mut ignores: Vec<Str> = vec![
            // ignore all files by default, the package manager will traverse all subdirectories
            "**/*".into(),
            // keep all package.json files except under node_modules
            "!**/package.json".into(),
            "!**/.npmrc".into(),
        ];
        match self.client {
            PackageManagerType::Pnpm => {
                ignores.push("!**/pnpm-workspace.yaml".into());
                ignores.push("!**/pnpm-lock.yaml".into());
                // https://pnpm.io/pnpmfile
                ignores.push("!**/.pnpmfile.cjs".into());
                ignores.push("!**/pnpmfile.cjs".into());
                // pnpm support Plug'n'Play https://pnpm.io/blog/2020/10/17/node-modules-configuration-options-with-pnpm#plugnplay-the-strictest-configuration
                ignores.push("!**/.pnp.cjs".into());
            }
            PackageManagerType::Yarn => {
                ignores.push("!**/.yarnrc".into()); // yarn 1.x
                ignores.push("!**/.yarnrc.yml".into()); // yarn 2.x
                ignores.push("!**/yarn.config.cjs".into()); // yarn 2.x
                ignores.push("!**/yarn.lock".into());
                // .yarn/patches, .yarn/releases
                ignores.push("!**/.yarn/**/*".into());
                // .pnp.cjs https://yarnpkg.com/features/pnp
                ignores.push("!**/.pnp.cjs".into());
            }
            PackageManagerType::Npm => {
                ignores.push("!**/package-lock.json".into());
                ignores.push("!**/npm-shrinkwrap.json".into());
            }
        }

        // if the workspace is a monorepo, keep workspace packages' parent directories to watch for new packages being added
        if self.is_monorepo {
            // TODO(@fengmk2): should use a more efficient way to get the workspace packages parent directories
            let (workspace_root_info, _) = find_workspace_root(&self.workspace_root)?;
            let package_graph = load_package_graph(&workspace_root_info)?;
            for node_index in package_graph.node_indices() {
                let package_info = &package_graph[node_index];
                if let Some(parent_path) = package_info.path.as_path().parent() {
                    let rule: Str = format!("!{}", parent_path.display()).into();
                    // check if the rule is already in the ignores
                    if ignores.contains(&rule) {
                        continue;
                    }
                    ignores.push(rule);
                }
            }
        }

        // ignore all files under node_modules
        // e.g. node_modules/mqtt/package.json
        ignores.push("**/node_modules/**/*".into());
        // keep the node_modules directory
        ignores.push("!**/node_modules".into());
        // keep the scoped directory
        ignores.push("!**/node_modules/@*".into());
        // ignore all patterns under nested node_modules
        // e.g. node_modules/mqtt/node_modules/mqtt-packet/node_modules
        ignores.push("**/node_modules/**/node_modules/**".into());

        Ok(ignores)
    }
}

/// Get the package manager name, version and optional hash from the workspace root.
pub fn get_package_manager_type_and_version(
    workspace_root: &WorkspaceRoot,
    default: Option<PackageManagerType>,
) -> Result<(PackageManagerType, Str, Option<Str>), Error> {
    // check packageManager field in package.json
    let package_json_path = workspace_root.path.join("package.json");
    if let Some(file) = open_exists_file(&package_json_path)? {
        let package_json: PackageJson = serde_json::from_reader(BufReader::new(&file))?;
        if !package_json.package_manager.is_empty()
            && let Some((name, version_with_hash)) = package_json.package_manager.split_once('@')
        {
            // Parse version and optional hash (format: version+sha512.hash)
            let (version, hash) = if let Some((ver, hash_part)) = version_with_hash.split_once('+')
            {
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
            match name {
                "pnpm" => return Ok((PackageManagerType::Pnpm, version.into(), hash)),
                "yarn" => return Ok((PackageManagerType::Yarn, version.into(), hash)),
                "npm" => return Ok((PackageManagerType::Npm, version.into(), hash)),
                _ => return Err(Error::UnsupportedPackageManager(name.into())),
            }
        }
    }

    // TODO(@fengmk2): check devEngines.packageManager field in package.json

    let version = Str::from("latest");
    // if pnpm-workspace.yaml exists, use pnpm@latest
    if matches!(workspace_root.workspace_file, WorkspaceFile::PnpmWorkspaceYaml(_)) {
        return Ok((PackageManagerType::Pnpm, version, None));
    }

    // if pnpm-lock.yaml exists, use pnpm@latest
    let pnpm_lock_yaml_path = workspace_root.path.join("pnpm-lock.yaml");
    if is_exists_file(&pnpm_lock_yaml_path)? {
        return Ok((PackageManagerType::Pnpm, version, None));
    }

    // if yarn.lock or .yarnrc.yml exists, use yarn@latest
    let yarn_lock_path = workspace_root.path.join("yarn.lock");
    let yarnrc_yml_path = workspace_root.path.join(".yarnrc.yml");
    if is_exists_file(&yarn_lock_path)? || is_exists_file(&yarnrc_yml_path)? {
        return Ok((PackageManagerType::Yarn, version, None));
    }

    // if package-lock.json exists, use npm@latest
    let package_lock_json_path = workspace_root.path.join("package-lock.json");
    if is_exists_file(&package_lock_json_path)? {
        return Ok((PackageManagerType::Npm, version, None));
    }

    // if .pnpmfile.cjs exists, use pnpm@latest
    let pnpmfile_cjs_path = workspace_root.path.join(".pnpmfile.cjs");
    if is_exists_file(&pnpmfile_cjs_path)? {
        return Ok((PackageManagerType::Pnpm, version, None));
    }
    // if legacy pnpmfile.cjs exists, use pnpm@latest
    // https://newreleases.io/project/npm/pnpm/release/6.0.0
    let legacy_pnpmfile_cjs_path = workspace_root.path.join("pnpmfile.cjs");
    if is_exists_file(&legacy_pnpmfile_cjs_path)? {
        return Ok((PackageManagerType::Pnpm, version, None));
    }

    // if yarn.config.cjs exists, use yarn@latest (yarn 2.0+)
    let yarn_config_cjs_path = workspace_root.path.join("yarn.config.cjs");
    if is_exists_file(&yarn_config_cjs_path)? {
        return Ok((PackageManagerType::Yarn, version, None));
    }

    // if default is specified, use it
    if let Some(default) = default {
        return Ok((default, version, None));
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
pub async fn download_package_manager(
    package_manager_type: PackageManagerType,
    version_or_latest: &str,
    expected_hash: Option<&str>,
) -> Result<(AbsolutePathBuf, Str, Str), Error> {
    let version: Str = if version_or_latest == "latest" {
        get_latest_version(package_manager_type).await?
    } else {
        version_or_latest.into()
    };

    let mut package_name: Str = package_manager_type.to_string().into();
    // handle yarn >= 2.0.0 to use `@yarnpkg/cli-dist` as package name
    // @see https://github.com/nodejs/corepack/blob/main/config.json#L135
    if matches!(package_manager_type, PackageManagerType::Yarn) {
        let version_req = VersionReq::parse(">=2.0.0")?;
        if version_req.matches(&Version::parse(&version)?) {
            package_name = "@yarnpkg/cli-dist".into();
        }
    }

    let tgz_url = get_npm_package_tgz_url(&package_name, &version);
    let cache_dir = get_cache_dir()?;
    let bin_name = package_manager_type.to_string();
    // $CACHE_DIR/vite/package_manager/pnpm/10.0.0
    let target_dir = cache_dir.join("package_manager").join(&bin_name).join(&version);
    let install_dir = target_dir.join(&bin_name);

    // If all shims are already exists, return the target directory
    // $CACHE_DIR/vite/package_manager/pnpm/10.0.0/pnpm/bin/(pnpm|pnpm.cmd|pnpm.ps1)
    let bin_prefix = install_dir.join("bin");
    let bin_file = bin_prefix.join(&bin_name);
    if is_exists_file(&bin_file)?
        && is_exists_file(bin_file.with_extension("cmd"))?
        && is_exists_file(bin_file.with_extension("ps1"))?
    {
        return Ok((install_dir, package_name, version));
    }

    // $CACHE_DIR/vite/package_manager/pnpm/{tmp_name}
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
    let lock_file = File::create(lock_path.as_path())?;
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

pub fn format_path_env(bin_prefix: impl AsRef<Path>) -> String {
    let mut paths = env::split_paths(&env::var_os("PATH").unwrap_or_default()).collect::<Vec<_>>();
    paths.insert(0, bin_prefix.as_ref().to_path_buf());
    env::join_paths(paths).unwrap().to_string_lossy().to_string()
}

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
    ];

    let mut selected_index = 0;

    // Print header and instructions with proper line breaks
    println!("\n📦 No package manager detected. Please select one:");
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
                    SetForegroundColor(Color::Cyan),
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

        // Read keyboard input
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
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
                KeyCode::Down => {
                    if selected_index < options.len() - 1 {
                        selected_index += 1;
                    }
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
        };
        println!("\n✓ Selected package manager: {name}\n");
    }

    result
}

/// Prompt the user to select a package manager
fn prompt_package_manager_selection() -> Result<PackageManagerType, Error> {
    // In CI environment, automatically use pnpm without prompting
    if is_ci_environment() {
        println!("CI environment detected. Using default package manager: pnpm");
        return Ok(PackageManagerType::Pnpm);
    }

    // Check if stdin is a TTY (terminal) - if not, use default
    if !io::stdin().is_terminal() {
        println!("Non-interactive environment detected. Using default package manager: pnpm");
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
        println!("package_json: {package_json:?}");
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
        println!("package_json: {package_json:?}");
        assert!(package_json["packageManager"].as_str().unwrap().starts_with("yarn@"));
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

        let workspace_root = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash) =
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
    async fn test_parse_package_manager_with_sha1_hash() {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Test with sha1 hash
        let package_content = r#"{"name": "test-package", "packageManager": "npm@10.5.0+sha1.abcd1234567890abcdef1234567890abcdef1234"}"#;
        create_package_json(&temp_dir_path, package_content);

        let workspace_root = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash) =
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

        let workspace_root = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash) =
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

        let workspace_root = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash) =
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

        let workspace_root = find_workspace_root(&temp_dir_path).unwrap();
        let (pm_type, version, hash) =
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
    // Tests for get_fingerprint_ignores method
    mod get_fingerprint_ignores_tests {
        use vite_glob::GlobPatternSet;

        use super::*;

        fn create_mock_package_manager(
            temp_dir: TempDir,
            pm_type: PackageManagerType,
            is_monorepo: bool,
        ) -> PackageManager {
            let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
            let install_dir = temp_dir_path.join("install");

            PackageManager {
                client: pm_type,
                package_name: pm_type.to_string().into(),
                version: "1.0.0".into(),
                hash: None,
                bin_name: pm_type.to_string().into(),
                workspace_root: temp_dir_path,
                is_monorepo,
                install_dir,
            }
        }

        #[test]
        fn test_get_fingerprint_ignores_monorepo() {
            let temp_dir: TempDir = create_temp_dir();
            let pm = create_mock_package_manager(temp_dir, PackageManagerType::Pnpm, true);
            // mkdir packages/app
            fs::create_dir_all(pm.workspace_root.join("packages/app"))
                .expect("Failed to create packages/app directory");
            // create pnpm-workspace.yaml
            fs::write(
                pm.workspace_root.join("pnpm-workspace.yaml"),
                "packages:
  - 'packages/*'
",
            )
            .expect("Failed to write pnpm-workspace.yaml");
            // create package.json
            fs::write(pm.workspace_root.join("package.json"), "{\"name\": \"test-package\"}")
                .expect("Failed to write package.json");
            // create packages/app/package.json
            fs::write(
                pm.workspace_root.join("packages/app/package.json"),
                "{\"name\": \"test-package-app\"}",
            )
            .expect("Failed to write packages/app/package.json");
            let ignores = pm.get_fingerprint_ignores().expect("Should get fingerprint ignores");
            let matcher = GlobPatternSet::new(&ignores).expect("Should compile patterns");
            assert!(!matcher.is_match("packages"), "Should not ignore packages directory");
            assert!(matcher.is_match("packages/app"), "Should ignore packages/app directory");
            assert_eq!(
                ignores,
                [
                    "**/*",
                    "!**/package.json",
                    "!**/.npmrc",
                    "!**/pnpm-workspace.yaml",
                    "!**/pnpm-lock.yaml",
                    "!**/.pnpmfile.cjs",
                    "!**/pnpmfile.cjs",
                    "!**/.pnp.cjs",
                    "!packages",
                    "**/node_modules/**/*",
                    "!**/node_modules",
                    "!**/node_modules/@*",
                    "**/node_modules/**/node_modules/**"
                ]
            );
        }

        #[test]
        fn test_pnpm_fingerprint_ignores() {
            let temp_dir: TempDir = create_temp_dir();
            let pm = create_mock_package_manager(temp_dir, PackageManagerType::Pnpm, false);
            let ignores = pm.get_fingerprint_ignores().expect("Should get fingerprint ignores");
            let matcher = GlobPatternSet::new(&ignores).expect("Should compile patterns");

            // Should ignore most files in node_modules
            assert!(
                matcher.is_match("node_modules/pkg-a/index.js"),
                "Should ignore implementation files"
            );
            assert!(
                matcher.is_match("foo/bar/node_modules/pkg-a/lib/util.js"),
                "Should ignore nested files"
            );
            assert!(matcher.is_match("node_modules/.bin/cli"), "Should ignore binaries");

            // Should NOT ignore package.json files (including in node_modules)
            assert!(!matcher.is_match("package.json"), "Should NOT ignore root package.json");
            assert!(
                !matcher.is_match("packages/app/package.json"),
                "Should NOT ignore package package.json"
            );

            // Should ignore package.json files under node_modules
            assert!(
                matcher.is_match("node_modules/pkg-a/package.json"),
                "Should ignore package.json in node_modules"
            );
            assert!(
                matcher.is_match("foo/bar/node_modules/pkg-a/package.json"),
                "Should ignore package.json in node_modules"
            );
            assert!(
                matcher.is_match("node_modules/@scope/pkg-a/package.json"),
                "Should ignore package.json in node_modules"
            );

            // Should keep node_modules directories themselves
            assert!(!matcher.is_match("node_modules"), "Should NOT ignore node_modules directory");
            assert!(
                !matcher.is_match("packages/app/node_modules"),
                "Should NOT ignore nested node_modules"
            );
            assert!(
                matcher.is_match("node_modules/mqtt/node_modules"),
                "Should ignore sub node_modules under node_modules"
            );
            assert!(
                matcher
                    .is_match("node_modules/minimatch/node_modules/brace-expansion/node_modules"),
                "Should ignore sub node_modules under node_modules"
            );
            assert!(
                matcher.is_match("packages/app/node_modules/@octokit/graphql/node_modules"),
                "Should ignore sub node_modules under node_modules"
            );

            // Should keep the root scoped directory under node_modules
            assert!(!matcher.is_match("node_modules/@types"), "Should NOT ignore scoped directory");
            assert!(
                matcher.is_match("node_modules/@types/node"),
                "Should ignore scoped sub directory"
            );

            // Pnpm-specific files should NOT be ignored
            assert!(
                !matcher.is_match("pnpm-workspace.yaml"),
                "Should NOT ignore pnpm-workspace.yaml"
            );
            assert!(!matcher.is_match("pnpm-lock.yaml"), "Should NOT ignore pnpm-lock.yaml");
            assert!(!matcher.is_match(".pnpmfile.cjs"), "Should NOT ignore .pnpmfile.cjs");
            assert!(!matcher.is_match("pnpmfile.cjs"), "Should NOT ignore pnpmfile.cjs");
            assert!(!matcher.is_match(".pnp.cjs"), "Should NOT ignore .pnp.cjs");
            assert!(!matcher.is_match(".npmrc"), "Should NOT ignore .npmrc");

            // Other package manager files should be ignored
            assert!(matcher.is_match("yarn.lock"), "Should ignore yarn.lock");
            assert!(matcher.is_match("package-lock.json"), "Should ignore package-lock.json");

            // Regular source files should be ignored
            assert!(matcher.is_match("src/index.js"), "Should ignore source files");
            assert!(matcher.is_match("dist/bundle.js"), "Should ignore build outputs");
        }

        #[test]
        fn test_yarn_fingerprint_ignores() {
            let temp_dir: TempDir = create_temp_dir();
            let pm = create_mock_package_manager(temp_dir, PackageManagerType::Yarn, false);
            let ignores = pm.get_fingerprint_ignores().expect("Should get fingerprint ignores");
            let matcher = GlobPatternSet::new(&ignores).expect("Should compile patterns");

            // Should ignore most files in node_modules
            assert!(
                matcher.is_match("node_modules/react/index.js"),
                "Should ignore implementation files"
            );
            assert!(
                matcher.is_match("node_modules/react/cjs/react.production.js"),
                "Should ignore nested files"
            );

            // Should NOT ignore package.json files (including in node_modules)
            assert!(!matcher.is_match("package.json"), "Should NOT ignore root package.json");
            assert!(
                !matcher.is_match("apps/web/package.json"),
                "Should NOT ignore app package.json"
            );

            // Should ignore package.json files under node_modules
            assert!(
                matcher.is_match("node_modules/react/package.json"),
                "Should ignore package.json in node_modules"
            );

            // Should keep node_modules directories
            assert!(!matcher.is_match("node_modules"), "Should NOT ignore node_modules directory");
            assert!(!matcher.is_match("node_modules/@types"), "Should NOT ignore scoped packages");

            // Yarn-specific files should NOT be ignored
            assert!(!matcher.is_match(".yarnrc"), "Should NOT ignore .yarnrc");
            assert!(!matcher.is_match(".yarnrc.yml"), "Should NOT ignore .yarnrc.yml");
            assert!(!matcher.is_match("yarn.config.cjs"), "Should NOT ignore yarn.config.cjs");
            assert!(!matcher.is_match("yarn.lock"), "Should NOT ignore yarn.lock");
            assert!(
                !matcher.is_match(".yarn/releases/yarn-4.0.0.cjs"),
                "Should NOT ignore .yarn contents"
            );
            assert!(
                !matcher.is_match(".yarn/patches/package.patch"),
                "Should NOT ignore .yarn patches"
            );
            assert!(
                !matcher.is_match(".yarn/patches/yjs-npm-13.6.21-c9f1f3397c.patch"),
                "Should NOT ignore .yarn patches"
            );
            assert!(!matcher.is_match(".pnp.cjs"), "Should NOT ignore .pnp.cjs");
            assert!(!matcher.is_match(".npmrc"), "Should NOT ignore .npmrc");

            // Other package manager files should be ignored
            assert!(matcher.is_match("pnpm-lock.yaml"), "Should ignore pnpm-lock.yaml");
            assert!(matcher.is_match("package-lock.json"), "Should ignore package-lock.json");

            // Regular source files should be ignored
            assert!(matcher.is_match("src/components/Button.tsx"), "Should ignore source files");

            // Should ignore nested node_modules
            assert!(
                matcher.is_match(
                    "node_modules/@mixmark-io/domino/.yarn/plugins/@yarnpkg/plugin-version.cjs"
                ),
                "Should ignore sub node_modules under node_modules"
            );
            assert!(
                matcher.is_match("node_modules/touch/node_modules"),
                "Should ignore sub node_modules under node_modules"
            );
        }

        #[test]
        fn test_npm_fingerprint_ignores() {
            let temp_dir: TempDir = create_temp_dir();
            let pm = create_mock_package_manager(temp_dir, PackageManagerType::Npm, false);
            let ignores = pm.get_fingerprint_ignores().expect("Should get fingerprint ignores");
            let matcher = GlobPatternSet::new(&ignores).expect("Should compile patterns");

            // Should ignore most files in node_modules
            assert!(
                matcher.is_match("node_modules/express/index.js"),
                "Should ignore implementation files"
            );
            assert!(
                matcher.is_match("node_modules/express/lib/application.js"),
                "Should ignore nested files"
            );

            // Should NOT ignore package.json files (including in node_modules)
            assert!(!matcher.is_match("package.json"), "Should NOT ignore root package.json");
            assert!(!matcher.is_match("src/package.json"), "Should NOT ignore nested package.json");

            // Should ignore package.json files under node_modules
            assert!(
                matcher.is_match("node_modules/express/package.json"),
                "Should ignore package.json in node_modules"
            );

            // Should keep node_modules directories
            assert!(!matcher.is_match("node_modules"), "Should NOT ignore node_modules directory");
            assert!(!matcher.is_match("node_modules/@babel"), "Should NOT ignore scoped packages");

            // Npm-specific files should NOT be ignored
            assert!(!matcher.is_match("package-lock.json"), "Should NOT ignore package-lock.json");
            assert!(
                !matcher.is_match("npm-shrinkwrap.json"),
                "Should NOT ignore npm-shrinkwrap.json"
            );
            assert!(!matcher.is_match(".npmrc"), "Should NOT ignore .npmrc");

            // Other package manager files should be ignored
            assert!(matcher.is_match("pnpm-lock.yaml"), "Should ignore pnpm-lock.yaml");
            assert!(matcher.is_match("yarn.lock"), "Should ignore yarn.lock");

            // Regular files should be ignored
            assert!(matcher.is_match("README.md"), "Should ignore docs");
            assert!(matcher.is_match("src/app.ts"), "Should ignore source files");
        }
    }
}
