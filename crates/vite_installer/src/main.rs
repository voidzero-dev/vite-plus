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

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use vite_install::request::HttpClient;
use vite_path::AbsolutePathBuf;
use vite_setup::{VP_BINARY_NAME, install, integrity, platform, registry};

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

/// Enable ANSI color support on Windows.
///
/// Older Windows consoles (cmd.exe) don't process ANSI escape codes by default.
/// We try to enable virtual terminal processing; if that fails (e.g. redirected
/// output, legacy console), we disable colors globally via owo_colors.
#[cfg(windows)]
fn init_colors() {
    // Respect NO_COLOR (https://no-color.org/)
    if std::env::var_os("NO_COLOR").is_some() {
        owo_colors::set_override(false);
        return;
    }

    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> isize;
        fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> i32;
    }
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // -11i32 as u32
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4; // -12i32 as u32
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    let enable_vt = |std_handle: u32| -> bool {
        unsafe {
            let handle = GetStdHandle(std_handle);
            // INVALID_HANDLE_VALUE (-1) or NULL (0, no console attached)
            if handle == -1_isize || handle == 0 {
                return false;
            }
            let mut mode: u32 = 0;
            if GetConsoleMode(handle, &mut mode) != 0 {
                SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
            } else {
                false
            }
        }
    };

    let stdout_ok = enable_vt(STD_OUTPUT_HANDLE);
    let stderr_ok = enable_vt(STD_ERROR_HANDLE);

    if !stdout_ok && !stderr_ok {
        owo_colors::set_override(false);
    }
}

#[cfg(not(windows))]
fn init_colors() {
    if std::env::var_os("NO_COLOR").is_some() {
        owo_colors::set_override(false);
    }
}

fn main() {
    init_dll_security();
    init_colors();

    let opts = cli::parse();

    // Resolve install dir and set VP_HOME before starting the tokio runtime,
    // so the unsafe set_var runs while we're still single-threaded.
    let install_dir = match resolve_install_dir(&opts) {
        Ok(dir) => dir,
        Err(e) => {
            print_error(&format!("Failed to resolve install directory: {e}"));
            std::process::exit(1);
        }
    };
    // Safety: called in main() before any threads are spawned.
    unsafe { std::env::set_var("VP_HOME", install_dir.as_path()) };

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| {
        print_error(&format!("Failed to create async runtime: {e}"));
        std::process::exit(1);
    });

    let code = rt.block_on(run(opts, install_dir));
    std::process::exit(code);
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
async fn run(mut opts: cli::Options, install_dir: AbsolutePathBuf) -> i32 {
    let install_dir_display = install_dir.as_path().to_string_lossy().to_string();

    // Pre-compute Node.js manager default before showing the menu,
    // so the user sees the resolved value and can override it.
    if !opts.no_node_manager {
        opts.no_node_manager = !auto_detect_node_manager(&install_dir, !opts.yes);
    }

    if !opts.yes {
        let proceed = show_interactive_menu(&mut opts, &install_dir_display);
        if !proceed {
            println!("Installation cancelled.");
            return 0;
        }
    }

    let code = match do_install(&opts, &install_dir).await {
        Ok(()) => {
            print_success(&opts, &install_dir_display);
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

#[allow(clippy::print_stdout)]
async fn do_install(
    opts: &cli::Options,
    install_dir: &AbsolutePathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let platform_suffix = platform::detect_platform_suffix()?;
    if !opts.quiet {
        print_info(&format!("detected platform: {platform_suffix}"));
    }

    // Check local version first to potentially skip HTTP requests
    tokio::fs::create_dir_all(install_dir).await?;
    let current_version = install::read_current_version(install_dir).await;

    let version_or_tag = opts.version.as_deref().unwrap_or(&opts.tag);

    // Resolve the target version — use resolve_version_string first so we can
    // skip the platform package fetch if the version is already installed
    if !opts.quiet {
        print_info(&format!("resolving version '{version_or_tag}'..."));
    }
    let target_version =
        registry::resolve_version_string(version_or_tag, opts.registry.as_deref()).await?;

    // Same version only if the binary is intact — a corrupted install needs a full reinstall
    let same_version = current_version.as_deref() == Some(target_version.as_str())
        && tokio::fs::try_exists(install_dir.join("current").join("bin").join(VP_BINARY_NAME))
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
    if let Err(e) = setup_bin_shims(install_dir).await {
        print_warn(&format!("Shim setup failed (non-fatal): {e}"));
    }

    if !opts.no_node_manager {
        if !opts.quiet {
            print_info("setting up Node.js version manager...");
        }
        if let Err(e) = install::refresh_shims(install_dir).await {
            print_warn(&format!("Node.js manager setup failed (non-fatal): {e}"));
        }
    } else if let Err(e) = install::create_env_files(install_dir).await {
        print_warn(&format!("Env file creation failed (non-fatal): {e}"));
    }

    if !opts.no_modify_path {
        let bin_dir_str = install_dir.join("bin").as_path().to_string_lossy().to_string();
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
/// 2. Already managing Node (bin/node.exe exists) → enable (refresh)
/// 3. CI / Codespaces / DevContainer / DevPod → enable
/// 4. No system `node` found → enable
/// 5. System node present, interactive → enable (matching install.ps1's default-Y prompt;
///    user can disable via customize menu before proceeding)
/// 6. System node present, silent → disable (don't silently take over)
fn auto_detect_node_manager(install_dir: &vite_path::AbsolutePath, interactive: bool) -> bool {
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

    // Already managing Node (shims exist from a previous install)
    let node_shim = install_dir.join("bin").join(if cfg!(windows) { "node.exe" } else { "node" });
    if node_shim.as_path().exists() {
        return true;
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
        install::cleanup_old_versions(install_dir, vite_setup::MAX_VERSIONS_KEEP, &protected).await
    {
        print_warn(&format!("Old version cleanup failed (non-fatal): {e}"));
    }

    Ok(())
}

/// Windows locks running `.exe` files — rename the old one out of the way before copying.
#[cfg(windows)]
async fn replace_windows_exe(
    src: &vite_path::AbsolutePathBuf,
    dst: &vite_path::AbsolutePathBuf,
    bin_dir: &vite_path::AbsolutePathBuf,
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

/// Set up the `bin/vp` entry point (trampoline copy on Windows, symlink on Unix).
async fn setup_bin_shims(
    install_dir: &vite_path::AbsolutePath,
) -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = install_dir.join("bin");
    tokio::fs::create_dir_all(&bin_dir).await?;

    #[cfg(windows)]
    {
        let shim_src = install_dir.join("current").join("bin").join("vp-shim.exe");
        let shim_dst = bin_dir.join("vp.exe");

        // Prefer vp-shim.exe (trampoline); fall back to vp.exe for pre-trampoline releases
        let src = if tokio::fs::try_exists(&shim_src).await.unwrap_or(false) {
            shim_src
        } else {
            install_dir.join("current").join("bin").join("vp.exe")
        };

        if tokio::fs::try_exists(&src).await.unwrap_or(false) {
            replace_windows_exe(&src, &shim_dst, &bin_dir).await?;
        }

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
        let link_target = std::path::PathBuf::from("../current/bin/vp");
        let link_path = bin_dir.join("vp");
        let _ = tokio::fs::remove_file(&link_path).await;
        tokio::fs::symlink(&link_target, &link_path).await?;
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

fn resolve_install_dir(opts: &cli::Options) -> Result<AbsolutePathBuf, Box<dyn std::error::Error>> {
    if let Some(ref dir) = opts.install_dir {
        let path = std::path::PathBuf::from(dir);
        let abs = if path.is_absolute() { path } else { std::env::current_dir()?.join(path) };
        AbsolutePathBuf::new(abs).ok_or_else(|| "Invalid installation directory".into())
    } else {
        Ok(vite_shared::get_vp_home()?)
    }
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
fn show_interactive_menu(opts: &mut cli::Options, install_dir: &str) -> bool {
    loop {
        let version = opts.version.as_deref().unwrap_or(&opts.tag);
        let bin_dir = format!("{install_dir}{sep}bin", sep = std::path::MAIN_SEPARATOR);

        println!();
        println!("  {}", "Welcome to Vite+ Installer!".bold());
        println!();
        println!("  This will install the {} CLI and monorepo task runner.", "vp".cyan());
        println!();
        println!("    Install directory: {}", install_dir.cyan());
        println!(
            "    PATH modification: {}",
            if opts.no_modify_path {
                "no".to_string()
            } else {
                format!("{bin_dir} \u{2192} User PATH")
            }
            .cyan()
        );
        println!("    Version:           {}", version.cyan());
        println!(
            "    Node.js manager:   {}",
            if opts.no_node_manager { "disabled" } else { "enabled" }.cyan()
        );
        println!();
        println!("  1) {} (default)", "Proceed with installation".bold());
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
        println!("  {}", "Customize installation:".bold());
        println!();
        println!("    1) Version:        [{}]", version_display.cyan());
        println!("    2) npm registry:   [{}]", registry_display.cyan());
        println!(
            "    3) Node.js manager: [{}]",
            if opts.no_node_manager { "disabled" } else { "enabled" }.cyan()
        );
        println!(
            "    4) Modify PATH:    [{}]",
            if opts.no_modify_path { "no" } else { "yes" }.cyan()
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
fn print_success(opts: &cli::Options, install_dir: &str) {
    if opts.quiet {
        return;
    }

    println!();
    println!("  {} Vite+ has been installed successfully!", "\u{2714}".green().bold());
    println!();
    println!("  To get started, restart your terminal, then run:");
    println!();
    println!("    {}", "vp --help".cyan());
    println!();
    println!("  Install directory: {install_dir}");
    println!("  Documentation:     {}", "https://viteplus.dev/guide/");
    println!();
}

#[allow(clippy::print_stderr)]
fn print_info(msg: &str) {
    eprint!("{}", "info: ".blue());
    eprintln!("{msg}");
}

#[allow(clippy::print_stderr)]
fn print_warn(msg: &str) {
    eprint!("{}", "warn: ".yellow());
    eprintln!("{msg}");
}

#[allow(clippy::print_stderr)]
fn print_error(msg: &str) {
    eprint!("{}", "error: ".red());
    eprintln!("{msg}");
}
