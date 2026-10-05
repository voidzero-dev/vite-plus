//! First-start installation followed by command execution through the deployed binary.

mod external;
mod shell;

use std::{path::Path, process::ExitCode};

use dialoguer::{Confirm, theme::ColorfulTheme};
use vp_pm_cli::PackageManagerType;
use vp_setup::{SELF_SETUP_MARKER, VP_BINARY_NAME, install, is_commit_preview_version};
use vp_shared::{EnvConfig, env_vars, output};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::{
    commands::env::{config, setup},
    error::Error,
};

pub(crate) async fn maybe_run() -> Result<Option<ExitCode>, Error> {
    let shell = std::env::var(env_vars::VP_SELF_SETUP_SHELL).ok();
    if let Some(shell) = shell.as_deref() {
        if !matches!(shell, "sh" | "powershell") {
            return Err(Error::Other("VP_SELF_SETUP_SHELL must be sh or powershell".into()));
        }
        vp_shared::validate_vp_dir_env().map_err(|error| Error::Other(error.to_string().into()))?;
        output::route_user_output_to_stderr();
    }
    let binary = std::env::current_exe()?;
    // macOS can return the invoking symlink; its parent is the shared shim directory.
    #[cfg(target_os = "macos")]
    let binary = std::fs::canonicalize(binary)?;
    let bin = binary.parent().ok_or(Error::CliBinaryNotFound)?;
    // Once the binary path is resolved, normal commands only check marker existence.
    if !crate::homebrew::owns_current_exe() && bin.join(SELF_SETUP_MARKER).try_exists()? {
        if let Some(shell) = shell.as_deref() {
            print_shell_result(shell);
            return Ok(Some(ExitCode::SUCCESS));
        }
        return Ok(None);
    }

    vp_shared::validate_vp_dir_env().map_err(|error| Error::Other(error.to_string().into()))?;
    let data = normalize_target(&EnvConfig::get().dirs.data)?;
    let external = if !crate::homebrew::owns_current_exe()
        && dunce::simplified(&binary).starts_with(dunce::simplified(data.as_path()))
    {
        None
    } else {
        Some(external::SetupState::new(&binary, local_install_version().as_deref())?)
    };
    let bundled = external.is_some() && has_bundled_package(&binary);
    let user_package = crate::homebrew::user_package_dir()?;
    let retains_binary = bundled || user_package.is_some();
    let reuse =
        shell.is_none() && std::env::var_os(env_vars::VP_SELF_SETUP_REPLACE_EXISTING).is_none();
    // Installers explicitly request setup, including same-version reinstalls.
    if reuse
        && let Some(state) = &external
        && let Some(installed_binary) =
            completed_external(state, retains_binary, user_package.as_deref()).await?
    {
        if installed_binary.as_path() == binary {
            return Ok(None);
        }
        return execute_installed(&installed_binary, false);
    }
    let lock = if let Some(state) = &external { Some(state.lock().await?) } else { None };
    // Another invocation may have finished while this one waited for setup.
    if reuse
        && let Some(state) = &external
        && let Some(installed_binary) =
            completed_external(state, retains_binary, user_package.as_deref()).await?
    {
        drop(lock);
        if installed_binary.as_path() == binary {
            return Ok(None);
        }
        return execute_installed(&installed_binary, false);
    }
    // Setup diagnostics must not pollute the original command's machine-readable stdout.
    output::route_user_output_to_stderr();
    let installed_binary = run(&binary, bundled, user_package.as_deref()).await?;
    if let Some(state) = external {
        state.save(&installed_binary).await?;
    }
    drop(lock);
    output::success("Vite+ setup complete.");
    if let Some(shell) = shell.as_deref() {
        print_shell_result(shell);
        return Ok(Some(ExitCode::SUCCESS));
    }
    execute_installed(&installed_binary, true)
}

async fn completed_external(
    state: &external::SetupState,
    retains_binary: bool,
    user_package: Option<&AbsolutePath>,
) -> Result<Option<AbsolutePathBuf>, Error> {
    if user_package.is_some_and(|package| !external::package_complete(package)) {
        return Ok(None);
    }
    state.completed_binary(retains_binary).await
}

fn execute_installed(
    binary: &AbsolutePath,
    just_installed: bool,
) -> Result<Option<ExitCode>, Error> {
    let mut args = std::env::args_os();
    let argv0 = args.next();
    let shim_tool =
        argv0.as_deref().and_then(|name| name.to_str()).and_then(crate::shim::detect_shim_tool);
    if just_installed
        && args.len() == 0
        && shim_tool.is_none()
        && std::env::var_os("VP_COMPLETE").is_none()
    {
        return Ok(Some(ExitCode::SUCCESS));
    }
    // Re-enter through the marked installation, inheriting cwd, environment and stdio.
    let mut command = std::process::Command::new(binary.as_path());
    command.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        if let Some(argv0) = argv0 {
            command.arg0(argv0);
        }
        Err(command.exec().into())
    }
    #[cfg(windows)]
    {
        if let Some(tool) = shim_tool {
            command.env(env_vars::VP_SHIM_TOOL, tool);
        }
        let status = command.status()?;
        Ok(Some(ExitCode::from(vp_shared::exit_code_from_status(status) as u8)))
    }
}

pub(crate) fn has_bundled_package(binary: &Path) -> bool {
    let Some(prefix) = binary.parent().and_then(Path::parent) else { return false };
    let package = prefix.join("node_modules/vite-plus");
    // Unix shims can target an external binary; Windows trampolines need the managed layout.
    cfg!(unix) && package.join("package.json").is_file() && package.join("dist/bin.js").is_file()
}

fn local_install_version() -> Option<String> {
    let skip_deps = std::env::var_os("VP_SKIP_DEPS_INSTALL")?;
    if skip_deps.is_empty() {
        return None;
    }
    std::env::var("VP_VERSION").ok()
}

// Only successful setup emits executable output; logs use stderr in this mode.
fn print_shell_result(shell: &str) {
    let dirs = &EnvConfig::get().dirs;
    for (sh_name, powershell_name, path) in [
        ("INSTALL_DIR", "InstallDir", &dirs.data),
        ("SHIM_DIR", "ShimDir", &dirs.bin),
        ("CACHE_DIR", "CacheDir", &dirs.cache),
        ("CONFIG_DIR", "ConfigDir", &dirs.config),
        ("STATE_DIR", "StateDir", &dirs.state),
    ] {
        let value = path.to_string();
        if shell == "powershell" {
            vp_shared::output::print_stdout_line(format_args!(
                "$script:{powershell_name} = '{}'",
                setup::escape_powershell_single_quoted_string(&value)
            ));
        } else {
            vp_shared::output::print_stdout_line(format_args!(
                "{sh_name}=\"{}\"",
                setup::escape_posix_double_quoted_string(&value)
            ));
        }
    }
}

/// Setup Vite+ for the first run
async fn run(
    source: &Path,
    bundled: bool,
    user_package: Option<&AbsolutePath>,
) -> Result<AbsolutePathBuf, Error> {
    let env = EnvConfig::get();
    let dirs = &env.dirs;
    let active_binary = dirs.data.join("current").join("bin").join(VP_BINARY_NAME);
    let in_place = same_file::is_same_file(source, active_binary.as_path()).unwrap_or(false);
    // External package managers own their payload. Only set up the user's config and shims.
    let externally_owned = bundled || user_package.is_some();
    let deploy = !in_place && !externally_owned;
    #[cfg(windows)]
    if !in_place
        && ["vp.exe", "vpx.exe", "vpr.exe"]
            .iter()
            .any(|name| std::fs::symlink_metadata(dirs.bin.join(name)).is_ok())
        && !env.is_ci
        && std::env::var(env_vars::VP_SELF_SETUP_REPLACE_EXISTING).as_deref() != Ok("1")
        && !confirm(
            &format!("Replace existing Vite+ commands in {}?", dirs.bin.as_path().display()),
            false,
        )?
    {
        return Err(Error::Other(
            "Installation cancelled; existing Vite+ commands were kept.".into(),
        ));
    }
    let previous_install = if deploy { previous_install()? } else { None };
    let node_override = manager_mode("VP_NODE_MANAGER");
    // A package upgrade can change the executable path or expire its receipt.
    // Preferences belong to the user, not to that particular binary.
    let configured = externally_owned && config::get_config_path()?.as_path().is_file();
    // A supplied Node choice skips the combined prompt; upgrades preserve all saved choices.
    let default_mode = if in_place || configured || node_override.is_some() {
        None
    } else {
        management_default()?
    };
    let version = env!("CARGO_PKG_VERSION");
    let registry = std::env::var(env_vars::NPM_CONFIG_REGISTRY)
        .or_else(|_| std::env::var(env_vars::NPM_CONFIG_REGISTRY_UPPER))
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            is_commit_preview_version(version)
                .then(|| "https://registry-bridge.viteplus.dev/".to_string())
        });
    let registry = registry.as_deref();
    // The local bootstrap provisions JS dependencies itself after this invocation.
    let skip_deps = std::env::var_os("VP_SKIP_DEPS_INSTALL").is_some_and(|value| !value.is_empty());
    let local_version = local_install_version();
    let install_version = local_version.as_deref().unwrap_or(version);
    if !in_place
        && (install_version.is_empty()
            || Path::new(install_version).components().count() != 1
            || !matches!(
                Path::new(install_version).components().next(),
                Some(std::path::Component::Normal(_))
            ))
    {
        return Err(Error::Other("Invalid local installation version".into()));
    }

    // 1. Prepare the payload before activating it. Upgrade has already done this in the in-place case.
    let previous_version = install::read_current_version(&dirs.data).await;
    let version_dir = if let Some(package) = user_package {
        // Save choices before network work so a failed install does not ask again.
        save_management_preferences(node_override, default_mode).await?;
        tokio::fs::create_dir_all(package).await?;
        package.to_absolute_path_buf()
    } else if deploy {
        let name =
            install::target_install_dir_name(install_version, previous_version.as_deref(), true);
        dirs.data.join(name)
    } else {
        AbsolutePathBuf::new(
            source.parent().and_then(Path::parent).ok_or(Error::CliBinaryNotFound)?.to_path_buf(),
        )
        .ok_or(Error::CliBinaryNotFound)?
    };
    let binary = if externally_owned {
        AbsolutePathBuf::new(source.to_path_buf()).ok_or(Error::CliBinaryNotFound)?
    } else {
        version_dir.join("bin").join(VP_BINARY_NAME)
    };
    if deploy {
        tokio::fs::create_dir_all(version_dir.join("bin")).await?;
        install::clear_self_setup_marker(&version_dir).await?;
        if !same_file::is_same_file(source, binary.as_path()).unwrap_or(false) {
            // A failed install can leave a read-only copy from a package manager.
            // Replace it atomically instead of opening it for writing on retry.
            let temporary = tempfile::NamedTempFile::new_in(version_dir.join("bin"))?;
            tokio::fs::copy(source, temporary.path()).await?;
            temporary.persist(binary.as_path()).map_err(|error| error.error)?;
        }
    }
    if user_package.map_or_else(
        || !version_dir.join("node_modules/vite-plus/package.json").as_path().is_file(),
        |package| !external::package_complete(package),
    ) {
        output::info(&format!("installing vite-plus@{version}..."));
        install::generate_wrapper_package_json(&version_dir, version).await?;
        if !skip_deps {
            install::install_production_deps(&version_dir, registry, true).await?;
        }
        if user_package.is_some() {
            if !external::package_matches_binary(&version_dir) {
                return Err(Error::Other(
                    "Vite+ dependencies do not match this CLI version".into(),
                ));
            }
            tokio::fs::write(version_dir.join(external::PACKAGE_MARKER), b"").await?;
        }
    }
    #[cfg(windows)]
    if !bundled && !version_dir.join("bin/vp-shim.exe").as_path().is_file() {
        let sibling = source.with_file_name("vp-shim.exe");
        if sibling.is_file() {
            tokio::fs::copy(sibling, version_dir.join("bin/vp-shim.exe")).await?;
        } else {
            // The standalone Windows executable needs its companion trampoline, which is not a JS dependency.
            let suffix = vp_setup::platform::detect_platform_suffix()?;
            let resolved =
                vp_setup::registry::resolve_platform_package(version, &suffix, registry).await?;
            let data =
                vp_pm_cli::HttpClient::new().get_bytes(&resolved.platform_tarball_url).await?;
            vp_setup::integrity::verify_integrity(&data, &resolved.platform_integrity)?;
            let temporary = tempfile::tempdir()?;
            let temporary_dir = AbsolutePathBuf::new(temporary.path().to_path_buf())
                .ok_or(Error::CliBinaryNotFound)?;
            install::extract_platform_package(&data, &temporary_dir).await?;
            tokio::fs::copy(
                temporary_dir.join("bin/vp-shim.exe"),
                version_dir.join("bin/vp-shim.exe"),
            )
            .await?;
        }
    }

    if !in_place {
        // Prepare the payload first, then let the old uninstaller clean its shell entries before writing ours.
        if deploy {
            remove_previous_install(previous_install.as_deref()).await?;
        }
        if std::env::var(env_vars::VP_SELF_SETUP_NO_MODIFY_PATH).as_deref() != Ok("1") {
            if let Err(error) = shell::configure().await {
                output::warn(&format!(
                    "Could not configure shell profiles: {error}. Add {} to PATH manually.",
                    dirs.bin.as_path().display()
                ));
            }
        }
        if user_package.is_none() {
            save_management_preferences(node_override, default_mode).await?;
        }
    }

    // Existing Windows installs also need the fallback directory in their persistent PATH.
    #[cfg(windows)]
    if in_place && std::env::var(env_vars::VP_SELF_SETUP_NO_MODIFY_PATH).as_deref() != Ok("1") {
        shell::configure().await?;
    }

    // 2. Activate a standalone download; an upgrade hook must not overwrite rollback history.
    if deploy {
        install::save_previous_version(&dirs.data).await?;
        let name = version_dir
            .as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::CliBinaryNotFound)?;
        install::swap_current_link(&dirs.data, name).await?;
    }

    // 3. Run setup in this process. Spawning the unmarked binary here would reenter self-setup.
    tokio::fs::create_dir_all(&dirs.bin).await?;
    // Setup places each tool according to its effective management mode.
    setup::execute_for_binary(binary.as_path(), true, true, false).await?;
    if deploy {
        let name = version_dir
            .as_path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(Error::CliBinaryNotFound)?;
        let mut protected = vec![name];
        if let Some(previous) = previous_version.as_deref() {
            protected.push(previous);
        }
        if let Err(error) =
            install::cleanup_old_versions(&dirs.data, vp_setup::MAX_VERSIONS_KEEP, &protected).await
        {
            output::warn(&format!("Old version cleanup failed: {error}"));
        }
    }

    // A failure above leaves the marker absent so a later launch can retry.
    if !externally_owned {
        tokio::fs::write(version_dir.join("bin").join(SELF_SETUP_MARKER), b"").await?;
    }
    Ok(binary)
}

async fn save_management_preferences(
    node_override: Option<config::ShimMode>,
    default_mode: Option<config::ShimMode>,
) -> Result<(), Error> {
    let mut settings = config::load_config().await?;
    if let Some(mode) = node_override.or(default_mode) {
        settings.node_shim_mode = mode;
    }
    let pm_mode = manager_mode("VP_PM_MANAGER").or(default_mode);
    for (family, variable) in [
        (PackageManagerType::Npm, "VP_NPM_MANAGER"),
        (PackageManagerType::Pnpm, "VP_PNPM_MANAGER"),
        (PackageManagerType::Yarn, "VP_YARN_MANAGER"),
        (PackageManagerType::Bun, "VP_BUN_MANAGER"),
    ] {
        if let Some(mode) = manager_mode(variable).or(pm_mode) {
            settings.set_package_manager_shim_mode(family, mode);
        }
    }
    config::save_config(&settings).await?;
    Ok(())
}

fn interactive() -> bool {
    std::env::var_os("CI").is_none() && vp_shared::is_stderr_terminal()
}

fn find_on_path(name: &str) -> Option<AbsolutePathBuf> {
    let cwd = vt_path::current_dir().ok()?;
    vp_command::resolve_bin(name, None, &cwd).ok()
}

fn confirm(prompt: &str, default: bool) -> Result<bool, Error> {
    if !interactive() {
        return Ok(false);
    }
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|error| Error::Other(error.to_string().into()))
}

fn manager_mode(variable: &str) -> Option<config::ShimMode> {
    match std::env::var(variable).as_deref() {
        Ok("yes") => Some(config::ShimMode::Managed),
        Ok("no") => Some(config::ShimMode::SystemFirst),
        _ => None,
    }
}

fn management_default() -> Result<Option<config::ShimMode>, Error> {
    let dirs = &EnvConfig::get().dirs;
    let node = dirs.bin.join(setup::shim_filename("node"));
    let exists = std::fs::symlink_metadata(&node).is_ok();
    #[cfg(unix)]
    let owned = std::fs::symlink_metadata(&node)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
        && same_file::is_same_file(&node, dirs.data.join("current/bin/vp")).unwrap_or(false);
    #[cfg(windows)]
    let owned = exists && dirs.owns_windows_trampoline(node.as_path());
    if owned {
        // Refresh existing shims without undoing a user's `vp env off` preference.
        return Ok(None);
    }
    let automatic = ["CI", "CODESPACES", "REMOTE_CONTAINERS", "DEVPOD"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
        || find_on_path("node").is_none();
    if !exists && automatic {
        return Ok(Some(config::ShimMode::Managed));
    }
    let enable =
        confirm("Would you like Vite+ to manage your Node.js and package-manager versions?", true)?;
    Ok(Some(if enable { config::ShimMode::Managed } else { config::ShimMode::SystemFirst }))
}

// PATH discovery only offers cleanup after an explicit move; VpDirs remains the authority for the target.
fn previous_install() -> Result<Option<AbsolutePathBuf>, Error> {
    let env = EnvConfig::get();
    if !env.dir_envs.contains_key(env_vars::VP_HOME) {
        return Ok(None);
    }
    let Some(vp) = find_on_path("vp") else { return Ok(None) };
    let Some(root) = vp.parent().and_then(|bin| bin.parent()) else { return Ok(None) };
    let root =
        AbsolutePathBuf::new(std::fs::canonicalize(root)?).ok_or(Error::CliBinaryNotFound)?;
    let target = normalize_target(&env.dirs.data)?;
    if root.parent().is_none()
        || root == target
        || root == env.user_home
        || ["/", "/bin", "/opt", "/usr", "/usr/bin", "/usr/local", "/usr/local/bin"]
            .iter()
            .any(|path| root.as_path() == Path::new(path))
    {
        return Ok(None);
    }
    let has_entrypoint = ["vp", "vp.exe", "vp.cmd"]
        .iter()
        .any(|name| root.join("bin").join(name).as_path().is_file());
    if !root.join("current").as_path().exists() || !has_entrypoint {
        return Ok(None);
    }
    if root.as_path().starts_with(target.as_path()) || target.as_path().starts_with(root.as_path())
    {
        return Err(Error::Other(format!("The previous Vite+ install at {} overlaps with the new installation at {}. Choose a directory that does not overlap.", root.as_path().display(), target.as_path().display()).into()));
    }
    Ok(Some(root))
}

fn normalize_target(path: &AbsolutePath) -> Result<AbsolutePathBuf, Error> {
    if path.as_path().exists() {
        return AbsolutePathBuf::new(std::fs::canonicalize(path)?).ok_or(Error::CliBinaryNotFound);
    }
    let parent = path.parent().ok_or(Error::CliBinaryNotFound)?;
    let name = path.as_path().file_name().ok_or(Error::CliBinaryNotFound)?;
    Ok(normalize_target(parent)?.join(name))
}

async fn remove_previous_install(previous: Option<&AbsolutePath>) -> Result<(), Error> {
    let Some(previous) = previous else { return Ok(()) };
    if !confirm(
        &format!(
            "Found a previous Vite+ install at {previous}. Remove the previous install directory?"
        ),
        false,
    )? {
        return Ok(());
    }
    let binary = previous.join("current/bin").join(VP_BINARY_NAME);
    // An unmarked new-style installation may consume the first invocation as setup.
    for _ in 0..2 {
        let result = tokio::process::Command::new(binary.as_path())
            .args(["implode", "--yes"])
            // The old installation must dispatch implode instead of returning bootstrap paths.
            .env_remove(env_vars::VP_SELF_SETUP_SHELL)
            .env(env_vars::VP_HOME, previous.as_path())
            .output()
            .await;
        match result {
            Ok(result) if result.status.success() => {
                if !binary.as_path().exists() {
                    output::success("Removed previous Vite+ install.");
                    return Ok(());
                }
            }
            Ok(result) => {
                output::warn(&format!(
                    "Could not remove previous Vite+ install: {}",
                    String::from_utf8_lossy(&result.stderr)
                ));
                return Ok(());
            }
            Err(error) => {
                output::warn(&format!("Could not remove previous Vite+ install: {error}"));
                return Ok(());
            }
        }
    }
    output::warn("The previous Vite+ installation is still present.");
    Ok(())
}
