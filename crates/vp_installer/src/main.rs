//! Standalone Windows installer for the Vite+ CLI (`vp-setup.exe`).
//!
//! This binary provides a download-and-run installation experience for Windows,
//! complementing the existing PowerShell installer (`install.ps1`).
//!
//! Modeled after `rustup-init.exe`:
//! - Console-based (no GUI)
//! - Interactive prompts with numbered menu
//! - Silent mode via `-y` for CI
//! - Works from cmd.exe, PowerShell, Git Bash, or double-click

#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::print_stdout
)]

mod cli;

#[cfg(windows)]
mod windows_path;

use std::io::{self, Write};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use vp_pm_cli::HttpClient;
use vp_setup::{VP_BINARY_NAME, install, integrity, platform, registry};
use vp_shared::VpDirs;
use vt_path::AbsolutePathBuf;

/// Restrict DLL search to system32 only to prevent DLL hijacking
/// when the installer is run from a Downloads folder.
#[cfg(windows)]
fn init_dll_security() {
    unsafe extern "system" {
        fn SetDefaultDllDirectories(directory_flags: u32) -> i32;
    }
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
}

#[cfg(not(windows))]
fn init_dll_security() {}

fn main() {
    init_dll_security();

    let opts = cli::parse();

    // Validate and resolve the category roots before the installer creates any
    // directories.
    let dirs = match prepare_dirs() {
        Ok(dirs) => dirs,
        Err(e) => {
            print_error(&format!("Failed to resolve install directory: {e}"));
            std::process::exit(1);
        }
    };

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| {
        print_error(&format!("Failed to create async runtime: {e}"));
        std::process::exit(1);
    });

    let code = rt.block_on(run(opts, dirs));
    std::process::exit(code);
}

fn dir_displays(dirs: &VpDirs) -> (String, String) {
    (
        dirs.data.as_path().to_string_lossy().to_string(),
        dirs.bin.as_path().to_string_lossy().to_string(),
    )
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
async fn run(mut opts: cli::Options, dirs: VpDirs) -> i32 {
    let (data_dir_display, bin_dir_display) = dir_displays(&dirs);

    // Pre-compute Node.js manager default before showing the menu,
    // so the user sees the resolved value and can override it.
    if !opts.no_node_manager {
        opts.no_node_manager = !auto_detect_node_manager(&dirs, !opts.yes);
    }

    if !opts.yes {
        let proceed = show_interactive_menu(&mut opts, &data_dir_display, &bin_dir_display);
        if !proceed {
            println!("Installation cancelled.");
            return 0;
        }
    }

    let code = match do_install(&opts, &dirs).await {
        Ok(()) => {
            print_success(&opts, &data_dir_display, &bin_dir_display);
            0
        }
        Err(e) => {
            print_error(&format!("{e}"));
            1
        }
    };

    // When running interactively (double-click), pause so the user can
    // read the output before the console window closes.
    if !opts.yes {
        read_input("  Press Enter to close...");
    }

    code
}

/// Install the resolved version.
#[allow(clippy::print_stdout)]
async fn do_install(opts: &cli::Options, dirs: &VpDirs) -> Result<(), Box<dyn std::error::Error>> {
    let platform_suffix = platform::detect_platform_suffix()?;
    if !opts.quiet {
        print_info(&format!("detected platform: {platform_suffix}"));
    }

    // Read the installed version first. This operation does not create a
    // directory. Create the installation root after target-version validation.
    let current_version = install::read_current_version(&dirs.data).await;

    let version_or_tag = opts.version.as_deref().unwrap_or(&opts.tag);

    // Resolve the target version first. If it matches the installed version,
    // skip the platform package request.
    if !opts.quiet {
        print_info(&format!("resolving version '{version_or_tag}'..."));
    }
    let target_version =
        registry::resolve_version_string(version_or_tag, opts.registry.as_deref()).await?;
    if !vp_setup::supports_split_layout(&target_version) {
        return Err(format!(
            "vp-setup does not support vite-plus {target_version}. Install vite-plus 0.3.0 or later."
        )
        .into());
    }

    // Same version only if the binary is intact — a corrupted install needs a full reinstall.
    // `is_install_dir_for_version` also matches `{version}+force.*` dirs left by a forced
    // reinstall (`vp upgrade <version> --force`), so re-running setup recognizes them as
    // already installed instead of re-downloading.
    let same_version = current_version
        .as_deref()
        .is_some_and(|current| install::is_install_dir_for_version(current, &target_version))
        && tokio::fs::try_exists(dirs.data.join("current").join("bin").join(VP_BINARY_NAME))
            .await
            .unwrap_or(false);

    if same_version {
        if !opts.quiet {
            print_info(&format!("version {target_version} already installed, verifying setup..."));
        }
    } else if let Some(ref current) = current_version {
        if !opts.quiet {
            print_info(&format!("upgrading from {current} to {target_version}"));
        }
    }

    if !same_version {
        // Only fetch platform metadata + download when we actually need to install
        let resolved = registry::resolve_platform_package(
            &target_version,
            &platform_suffix,
            opts.registry.as_deref(),
        )
        .await?;

        if !opts.quiet {
            print_info(&format!("downloading vite-plus@{target_version} for {platform_suffix}..."));
        }
        let client = HttpClient::new();
        let platform_data =
            download_with_progress(&client, &resolved.platform_tarball_url, opts.quiet).await?;

        if !opts.quiet {
            print_info("verifying integrity...");
        }
        integrity::verify_integrity(&platform_data, &resolved.platform_integrity)?;

        let install_dir = &dirs.data;
        let version_dir = install_dir.join(&target_version);
        tokio::fs::create_dir_all(&version_dir).await?;

        let result = install_new_version(
            opts,
            &platform_data,
            &version_dir,
            install_dir,
            &target_version,
            current_version.is_some(),
        )
        .await;

        // On failure, clean up the partial version directory (matches vp upgrade behavior)
        if result.is_err() {
            let _ = tokio::fs::remove_dir_all(&version_dir).await;
        }

        result?;
    }

    // --- Post-activation setup (always runs, even for same-version repair) ---
    // All steps below are best-effort: the core install succeeded once `current`
    // points at the right version.

    if !opts.quiet {
        print_info("setting up shims...");
    }
    if let Err(e) = setup_bin_shims(&dirs).await {
        print_warn(&format!("Shim setup failed (non-fatal): {e}"));
    }

    if !opts.no_node_manager {
        if !opts.quiet {
            print_info("setting up Node.js version manager...");
        }
        if let Err(e) = install::refresh_shims(&dirs.data).await {
            print_warn(&format!("Node.js manager setup failed (non-fatal): {e}"));
        }
    } else if let Err(e) = install::create_env_files(&dirs.data).await {
        print_warn(&format!("Env file creation failed (non-fatal): {e}"));
    }

    if !opts.no_modify_path {
        let bin_dir_str = dirs.bin.as_path().to_string_lossy().to_string();
        if let Err(e) = modify_path(&bin_dir_str, opts.quiet) {
            print_warn(&format!("PATH modification failed (non-fatal): {e}"));
        }
    }

    Ok(())
}

/// Auto-detect whether the Node.js version manager should be enabled.
///
/// Pure logic — no user prompts. Called once before the interactive menu
/// so the user sees the resolved default and can override it.
///
/// Matches install.ps1/install.sh auto-detect logic:
/// 1. VP_NODE_MANAGER=yes → enable; VP_NODE_MANAGER=no → disable
/// 2. Vite+-owned Node shim → enable (refresh); foreign target-bin Node → require consent
/// 3. CI / Codespaces / DevContainer / DevPod → enable
/// 4. No system `node` found → enable
/// 5. System node present, interactive → enable (matching install.ps1's default-Y prompt;
///    user can disable via customize menu before proceeding)
/// 6. System node present, silent → disable (don't silently take over)
fn auto_detect_node_manager(dirs: &VpDirs, interactive: bool) -> bool {
    auto_detect_node_manager_for_state(existing_node_shim_state(dirs), interactive)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeShimState {
    Absent,
    Owned,
    Foreign,
}

fn existing_node_shim_state(dirs: &VpDirs) -> NodeShimState {
    #[cfg(windows)]
    {
        let node = dirs.bin.join("node.exe");
        if !node.as_path().exists() {
            return NodeShimState::Absent;
        }
        return if dirs.owns_windows_trampoline(node.as_path()) {
            NodeShimState::Owned
        } else {
            NodeShimState::Foreign
        };
    }

    #[cfg(not(windows))]
    {
        let node = dirs.bin.join("node");
        let Ok(metadata) = std::fs::symlink_metadata(node.as_path()) else {
            return NodeShimState::Absent;
        };
        if !metadata.file_type().is_symlink() {
            return NodeShimState::Foreign;
        }
        let expected = dirs.data.join("current").join("bin").join(vp_shared::VP_BINARY_NAME);
        let owned = std::fs::canonicalize(node.as_path()).is_ok_and(|target| {
            std::fs::canonicalize(expected.as_path()).is_ok_and(|expected| target == expected)
        });
        if owned { NodeShimState::Owned } else { NodeShimState::Foreign }
    }
}

fn auto_detect_node_manager_for_state(state: NodeShimState, interactive: bool) -> bool {
    // VP_NODE_MANAGER env var: only "yes" and "no" are recognized;
    // unrecognized values fall through to normal auto-detection
    // (matching install.ps1/install.sh behavior).
    if let Ok(val) = std::env::var("VP_NODE_MANAGER") {
        if val.eq_ignore_ascii_case("yes") {
            return true;
        }
        if val.eq_ignore_ascii_case("no") {
            return false;
        }
    }

    match state {
        NodeShimState::Owned => return true,
        // Silent setup must not overwrite an unrelated executable. In the
        // interactive menu, proceeding with the enabled default is consent.
        NodeShimState::Foreign => return interactive,
        NodeShimState::Absent => {}
    }

    // Auto-enable on CI / devcontainer environments
    if std::env::var_os("CI").is_some()
        || std::env::var_os("CODESPACES").is_some()
        || std::env::var_os("REMOTE_CONTAINERS").is_some()
        || std::env::var_os("DEVPOD").is_some()
    {
        return true;
    }

    // Auto-enable if no system node available
    if which::which("node").is_err() {
        return true;
    }

    // System node exists: in interactive mode, default to enabled (matching
    // install.ps1's Y/n prompt where Enter = yes). The user can disable it
    // in the customize menu. In silent mode, don't take over.
    interactive
}

/// Extract, install deps, and activate a new version. Separated so the caller
/// can clean up the version directory on failure.
async fn install_new_version(
    opts: &cli::Options,
    platform_data: &[u8],
    version_dir: &AbsolutePathBuf,
    install_dir: &AbsolutePathBuf,
    version: &str,
    has_previous: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !opts.quiet {
        print_info("extracting binary...");
    }
    install::extract_platform_package(platform_data, version_dir).await?;

    let binary_path = version_dir.join("bin").join(VP_BINARY_NAME);
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        return Err("Binary not found after extraction. The download may be corrupted.".into());
    }
    #[cfg(windows)]
    if !tokio::fs::try_exists(version_dir.join("bin").join("vp-shim.exe")).await.unwrap_or(false) {
        return Err(
            "vp-setup did not find vp-shim.exe after extraction. The downloaded package can be corrupt."
                .into(),
        );
    }

    install::generate_wrapper_package_json(version_dir, version).await?;

    if !opts.quiet {
        print_info("installing dependencies (this may take a moment)...");
    }
    install::install_production_deps(version_dir, opts.registry.as_deref(), opts.yes, version)
        .await?;

    let previous_version =
        if has_previous { install::save_previous_version(install_dir).await? } else { None };
    install::swap_current_link(install_dir, version).await?;

    // Cleanup with both new and previous versions protected (matches vp upgrade)
    let mut protected = vec![version];
    if let Some(ref prev) = previous_version {
        protected.push(prev.as_str());
    }
    if let Err(e) =
        install::cleanup_old_versions(install_dir, vp_setup::MAX_VERSIONS_KEEP, &protected).await
    {
        print_warn(&format!("Old version cleanup failed (non-fatal): {e}"));
    }

    Ok(())
}

/// Windows locks running `.exe` files — rename the old one out of the way before copying.
#[cfg(windows)]
async fn replace_windows_exe(
    src: &vt_path::AbsolutePathBuf,
    dst: &vt_path::AbsolutePathBuf,
    bin_dir: &vt_path::AbsolutePathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let old_name = format!(
        "vp.exe.{}.old",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let _ = tokio::fs::rename(dst, &bin_dir.join(&old_name)).await;
    tokio::fs::copy(src, dst).await?;
    Ok(())
}

/// Set up the `<BIN>/vp` entry point (trampoline copy on Windows, symlink on Unix).
async fn setup_bin_shims(dirs: &VpDirs) -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = &dirs.bin;
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(windows)]
    {
        let shim_src = dirs.data.join("current").join("bin").join("vp-shim.exe");
        let shim_dst = bin_dir.join("vp.exe");

        replace_windows_exe(&shim_src, &shim_dst, &bin_dir).await?;
        dirs.write_shim_pointer("vp")?;

        // Best-effort cleanup of old shim files
        if let Ok(mut entries) = tokio::fs::read_dir(&bin_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if entry.file_name().to_string_lossy().ends_with(".old") {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let current_vp = dirs.data.join("current").join("bin").join("vp");
        let link_path = bin_dir.join("vp");
        let _ = tokio::fs::remove_file(&link_path).await;
        tokio::fs::symlink(current_vp.as_path(), &link_path).await?;
    }

    Ok(())
}

async fn download_with_progress(
    client: &HttpClient,
    url: &str,
    quiet: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if quiet {
        return Ok(client.get_bytes(url).await?);
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.set_message("downloading...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let data = client.get_bytes(url).await?;

    pb.finish_and_clear();
    Ok(data)
}

/// Validate installer directory overrides. Then resolve category roots from
/// [`vp_shared::EnvConfig`].
fn prepare_dirs() -> Result<VpDirs, Box<dyn std::error::Error>> {
    vp_shared::validate_vp_dir_env()?;
    Ok(vp_shared::EnvConfig::get().dirs.clone())
}

#[allow(clippy::print_stdout)]
fn modify_path(bin_dir: &str, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        windows_path::add_to_user_path(bin_dir)?;
        if !quiet {
            print_info("added to User PATH (restart your terminal to pick up changes)");
        }
    }

    #[cfg(not(windows))]
    {
        if !quiet {
            print_info(&format!("add {bin_dir} to your shell's PATH"));
        }
    }

    Ok(())
}

#[allow(clippy::print_stdout)]
fn show_interactive_menu(opts: &mut cli::Options, data_dir: &str, bin_dir: &str) -> bool {
    loop {
        let version = opts.version.as_deref().unwrap_or(&opts.tag);

        println!();
        println!("  {}", style("Welcome to Vite+ Installer!").bold());
        println!();
        println!("  This will install the {} CLI and monorepo task runner.", style("vp").cyan());
        println!();
        println!("    Data directory:    {}", style(data_dir).cyan());
        println!("    Bin directory:     {}", style(bin_dir).cyan());
        println!(
            "    PATH modification: {}",
            style(if opts.no_modify_path {
                "no".to_string()
            } else {
                format!("{bin_dir} \u{2192} User PATH")
            })
            .cyan()
        );
        println!("    Version:           {}", style(version).cyan());
        println!(
            "    Node.js manager:   {}",
            style(if opts.no_node_manager { "disabled" } else { "enabled" }).cyan()
        );
        println!();
        println!("  1) {} (default)", style("Proceed with installation").bold());
        println!("  2) Customize installation");
        println!("  3) Cancel");
        println!();

        let choice = read_input("  > ");
        match choice.as_str() {
            "" | "1" => return true,
            "2" => show_customize_menu(opts),
            "3" => return false,
            _ => {
                println!("  Invalid choice. Please enter 1, 2, or 3.");
            }
        }
    }
}

#[allow(clippy::print_stdout)]
fn show_customize_menu(opts: &mut cli::Options) {
    loop {
        let version_display = opts.version.as_deref().unwrap_or(&opts.tag);
        let registry_display = opts.registry.as_deref().unwrap_or("(default)");

        println!();
        println!("  {}", style("Customize installation:").bold());
        println!();
        println!("    1) Version:        [{}]", style(version_display).cyan());
        println!("    2) npm registry:   [{}]", style(registry_display).cyan());
        println!(
            "    3) Node.js manager: [{}]",
            style(if opts.no_node_manager { "disabled" } else { "enabled" }).cyan()
        );
        println!(
            "    4) Modify PATH:    [{}]",
            style(if opts.no_modify_path { "no" } else { "yes" }).cyan()
        );
        println!();

        let choice = read_input("  Enter option number to change, or press Enter to go back: ");
        match choice.as_str() {
            "" => return,
            "1" => {
                let v = read_input("    Version (e.g. 0.3.0 or latest, Enter to keep): ");
                if v.is_empty() {
                    // Keep current value
                } else if v == opts.tag {
                    opts.version = None;
                } else {
                    opts.version = Some(v);
                }
            }
            "2" => {
                let r = read_input("    npm registry URL (or empty for default): ");
                opts.registry = if r.is_empty() { None } else { Some(r) };
            }
            "3" => opts.no_node_manager = !opts.no_node_manager,
            "4" => opts.no_modify_path = !opts.no_modify_path,
            _ => println!("  Invalid option."),
        }
    }
}

fn read_input(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    input.trim().to_string()
}

#[allow(clippy::print_stdout)]
fn print_success(opts: &cli::Options, data_dir: &str, bin_dir: &str) {
    if opts.quiet {
        return;
    }

    println!();
    println!("  {} Vite+ has been installed successfully!", style("\u{2714}").green().bold());
    println!();
    println!("  To get started, restart your terminal, then run:");
    println!();
    println!("    {}", style("vp --help").cyan());
    println!();
    println!("  Data directory: {data_dir}");
    println!("  Bin directory:  {bin_dir}");
    println!("  Documentation:  {}", "https://viteplus.dev/guide/");
    println!();
}

#[allow(clippy::print_stderr)]
fn print_info(msg: &str) {
    eprintln!("{}{msg}", style("info: ").blue().for_stderr());
}

#[allow(clippy::print_stderr)]
fn print_warn(msg: &str) {
    eprintln!("{}{msg}", style("warn: ").yellow().for_stderr());
}

#[allow(clippy::print_stderr)]
fn print_error(msg: &str) {
    eprintln!("{}{msg}", style("error: ").red().for_stderr());
}

#[cfg(test)]
mod tests {
    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    fn with_clean_home<R>(home: &std::path::Path, f: impl FnOnce() -> R) -> R {
        let mut vars =
            vec![("HOME", Some(home.as_os_str())), ("USERPROFILE", Some(home.as_os_str()))];
        vars.extend(env_vars::LAYOUT_OVERRIDE_VARS.iter().map(|name| (*name, None)));
        EnvConfig::with_vars(vars, |_| f())
    }

    #[test]
    fn foreign_node_requires_interactive_or_explicit_consent() {
        EnvConfig::with_vars([("VP_NODE_MANAGER", None), ("CI", Some("true"))], |_| {
            assert!(
                !auto_detect_node_manager_for_state(NodeShimState::Foreign, false),
                "silent CI setup must preserve a foreign Node executable"
            );
            assert!(
                auto_detect_node_manager_for_state(NodeShimState::Foreign, true),
                "the interactive menu can obtain consent to replace it"
            );
        });

        EnvConfig::with_vars([("VP_NODE_MANAGER", Some("yes")), ("CI", None)], |_| {
            assert!(auto_detect_node_manager_for_state(NodeShimState::Foreign, false));
        });
        EnvConfig::with_vars([("VP_NODE_MANAGER", Some("no")), ("CI", None)], |_| {
            assert!(!auto_detect_node_manager_for_state(NodeShimState::Owned, true));
        });
    }

    #[cfg(windows)]
    #[test]
    fn existing_windows_node_requires_matching_sidecar() {
        EnvConfig::scoped(|config| {
            let node = config.dirs.bin.join("node.exe");
            std::fs::create_dir_all(&config.dirs.bin).unwrap();
            std::fs::write(node.as_path(), b"foreign-node").unwrap();
            assert_eq!(existing_node_shim_state(&config.dirs), NodeShimState::Foreign);

            config.dirs.write_shim_pointer("node").unwrap();
            assert_eq!(existing_node_shim_state(&config.dirs), NodeShimState::Owned);
        });
    }

    #[test]
    fn fresh_home_uses_resolved_split_dirs_and_does_not_set_vp_home() {
        let tmp = tempfile::tempdir().unwrap();
        with_clean_home(tmp.path(), || {
            let expected = EnvConfig::get().dirs.clone();
            let dirs = prepare_dirs().unwrap();
            assert_eq!(dirs, expected);
            assert!(std::env::var_os(env_vars::VP_HOME).is_none());
            #[cfg(not(windows))]
            assert_eq!(
                dirs.bin.as_path(),
                dirs.data.join("bin").as_path(),
                "fresh Unix installs keep executables in the Vite+-owned data tree"
            );
            #[cfg(windows)]
            {
                let windows_bin = dirs.data.as_path().parent().unwrap().join("bin");
                assert_eq!(
                    dirs.bin.as_path(),
                    windows_bin.as_path(),
                    "fresh Windows installs keep bin and data as sibling application directories"
                );
            }
        });
    }

    #[test]
    fn existing_vite_plus_reuses_single_root_without_setting_vp_home() {
        let tmp = tempfile::tempdir().unwrap();
        let legacy = tmp.path().join(".vite-plus");
        // Grandfathering requires a real install: the `current` link, not a
        // bare directory.
        std::fs::create_dir_all(legacy.join("current")).unwrap();

        with_clean_home(tmp.path(), || {
            let dirs = prepare_dirs().unwrap();
            assert_eq!(dirs.data.as_path(), legacy.as_path());
            assert_eq!(dirs.bin.as_path(), legacy.join("bin").as_path());
            assert_eq!(dirs.config.as_path(), legacy.as_path());
            assert_eq!(dirs.state.as_path(), legacy.as_path());
            assert_eq!(dirs.cache.as_path(), legacy.join("cache").as_path());
            assert!(std::env::var_os(env_vars::VP_HOME).is_none());
        });
    }

    #[test]
    fn vp_home_pins_single_root() {
        let tmp = tempfile::tempdir().unwrap();
        let custom = tmp.path().join("custom");

        with_clean_home(tmp.path(), || {
            EnvConfig::with_vars([(env_vars::VP_HOME, Some(custom.as_os_str()))], |_| {
                let dirs = prepare_dirs().unwrap();
                assert_eq!(dirs.data.as_path(), custom.as_path());
                assert_eq!(dirs.bin.as_path(), custom.join("bin").as_path());
                assert_eq!(dirs.cache.as_path(), custom.join("cache").as_path());
                assert_eq!(dirs.config.as_path(), custom.as_path());
            });
        });
    }

    #[test]
    fn complete_vp_dir_group_is_used_when_fresh() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        let data = tmp.path().join("data");
        let cache = tmp.path().join("cache");
        std::fs::create_dir_all(&data).unwrap();

        EnvConfig::with_vars(
            [
                ("HOME", Some(tmp.path().as_os_str())),
                ("USERPROFILE", Some(tmp.path().as_os_str())),
                (env_vars::VP_HOME, None),
                (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                (env_vars::XDG_DATA_HOME, None),
                (env_vars::XDG_CACHE_HOME, None),
                (env_vars::XDG_CONFIG_HOME, None),
                (env_vars::XDG_STATE_HOME, None),
            ],
            |config| {
                let dirs = prepare_dirs().unwrap();
                assert_eq!(dirs.data.as_path(), data.as_path());
                assert_eq!(dirs.bin.as_path(), bin.as_path());
                assert_eq!(dirs.cache.as_path(), cache.as_path());
                assert_eq!(dirs, config.dirs);
                assert!(std::env::var_os(env_vars::VP_HOME).is_none());
            },
        );
    }

    #[test]
    fn invalid_vp_dir_env_is_rejected_without_creating_roots() {
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("requested-data");

        with_clean_home(tmp.path(), || {
            let default_dirs = EnvConfig::get().dirs.clone();
            let default_roots = [
                default_dirs.bin.as_path().to_path_buf(),
                default_dirs.data.as_path().to_path_buf(),
                default_dirs.cache.as_path().to_path_buf(),
                default_dirs.config.as_path().to_path_buf(),
                default_dirs.state.as_path().to_path_buf(),
            ];
            let default_existed = default_roots.each_ref().map(|root| root.exists());
            EnvConfig::with_vars(
                [
                    (env_vars::VP_HOME, None),
                    (env_vars::VP_BIN_DIR, None),
                    (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                    (env_vars::VP_CACHE_DIR, None),
                ],
                |_| {
                    let error = prepare_dirs().unwrap_err().to_string();
                    assert_eq!(
                        error,
                        "Set all three variables together: VP_BIN_DIR, VP_DATA_DIR, and VP_CACHE_DIR. Otherwise, do not set any of them."
                    );
                },
            );

            assert!(!data.exists(), "validation created {}", data.display());
            for (root, existed) in default_roots.iter().zip(default_existed) {
                if !existed {
                    assert!(!root.exists(), "validation created default root {}", root.display());
                }
            }
        });
    }
}
