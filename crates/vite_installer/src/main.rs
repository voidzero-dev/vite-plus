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

mod cli;

#[cfg(windows)]
mod windows_path;

use std::io::{self, Write};

use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use vite_install::request::HttpClient;
use vite_path::AbsolutePathBuf;
use vite_setup::{install, integrity, platform, registry};

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
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| {
        print_error(&format!("Failed to create async runtime: {e}"));
        std::process::exit(1);
    });

    let code = rt.block_on(run(opts));
    std::process::exit(code);
}

#[allow(clippy::print_stdout, clippy::print_stderr)]
async fn run(mut opts: cli::Options) -> i32 {
    let install_dir = match resolve_install_dir(&opts) {
        Ok(dir) => dir,
        Err(e) => {
            print_error(&format!("Failed to resolve install directory: {e}"));
            return 1;
        }
    };
    let install_dir_display = install_dir.as_path().to_string_lossy().to_string();

    if !opts.yes {
        let proceed = show_interactive_menu(&mut opts, &install_dir_display);
        if !proceed {
            println!("Installation cancelled.");
            return 0;
        }
    }

    match do_install(&opts, &install_dir).await {
        Ok(()) => {
            print_success(&opts, &install_dir_display);
            0
        }
        Err(e) => {
            print_error(&format!("{e}"));
            1
        }
    }
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

    let version_or_tag = opts.version.as_deref().unwrap_or(&opts.tag);
    if !opts.quiet {
        print_info(&format!("resolving version '{version_or_tag}'..."));
    }
    let resolved =
        registry::resolve_version(version_or_tag, &platform_suffix, opts.registry.as_deref())
            .await?;
    if !opts.quiet {
        print_info(&format!("found vite-plus@{}", resolved.version));
    }

    tokio::fs::create_dir_all(install_dir).await?;

    let current_version = install::read_current_version(install_dir).await;
    if let Some(ref current) = current_version {
        if current == &resolved.version {
            if !opts.quiet {
                println!("\n{} Already installed ({})", "\u{2714}".green(), resolved.version);
            }
            return Ok(());
        }
        if !opts.quiet {
            print_info(&format!("upgrading from {current} to {}", resolved.version));
        }
    }

    if !opts.quiet {
        print_info(&format!(
            "downloading vite-plus@{} for {}...",
            resolved.version, platform_suffix
        ));
    }
    let client = HttpClient::new();
    let platform_data =
        download_with_progress(&client, &resolved.platform_tarball_url, opts.quiet).await?;

    if !opts.quiet {
        print_info("verifying integrity...");
    }
    integrity::verify_integrity(&platform_data, &resolved.platform_integrity)?;

    let version_dir = install_dir.join(&resolved.version);
    tokio::fs::create_dir_all(&version_dir).await?;

    if !opts.quiet {
        print_info("extracting binary...");
    }
    install::extract_platform_package(&platform_data, &version_dir).await?;

    let binary_name = if cfg!(windows) { "vp.exe" } else { "vp" };
    let binary_path = version_dir.join("bin").join(binary_name);
    if !tokio::fs::try_exists(&binary_path).await.unwrap_or(false) {
        return Err("Binary not found after extraction. The download may be corrupted.".into());
    }

    install::generate_wrapper_package_json(&version_dir, &resolved.version).await?;
    install::write_release_age_overrides(&version_dir).await?;

    if !opts.quiet {
        print_info("installing dependencies (this may take a moment)...");
    }
    install::install_production_deps(&version_dir, opts.registry.as_deref()).await?;

    if current_version.is_some() {
        install::save_previous_version(install_dir).await?;
    }
    install::swap_current_link(install_dir, &resolved.version).await?;

    if !opts.quiet {
        print_info("setting up shims...");
    }
    setup_bin_shims(install_dir).await?;

    if !opts.no_node_manager {
        if !opts.quiet {
            print_info("setting up Node.js version manager...");
        }
        install::refresh_shims(install_dir).await?;
    } else {
        // When skipping Node.js manager, still create shell env files
        create_env_files(install_dir).await;
    }

    if let Err(e) = install::cleanup_old_versions(
        install_dir,
        vite_setup::MAX_VERSIONS_KEEP,
        &[&resolved.version],
    )
    .await
    {
        tracing::warn!("Old version cleanup failed (non-fatal): {e}");
    }

    if !opts.no_modify_path {
        let bin_dir_str = install_dir.join("bin").as_path().to_string_lossy().to_string();
        modify_path(&bin_dir_str, opts.quiet)?;
    }

    Ok(())
}

/// On Windows, rename a running exe to `.old` then copy the new one in place.
#[cfg(windows)]
async fn replace_windows_exe(
    src: &vite_path::AbsolutePathBuf,
    dst: &vite_path::AbsolutePathBuf,
    bin_dir: &vite_path::AbsolutePathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    if dst.as_path().exists() {
        let old_name = format!(
            "vp.exe.{}.old",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        let _ = tokio::fs::rename(dst, &bin_dir.join(&old_name)).await;
    }
    tokio::fs::copy(src, dst).await?;
    Ok(())
}

/// Set up the bin/ directory with the initial `vp` shim.
///
/// On Windows, copies `vp-shim.exe` from `current/bin/` to `bin/vp.exe`.
/// On Unix, creates a symlink from `bin/vp` to `../current/bin/vp`.
async fn setup_bin_shims(
    install_dir: &vite_path::AbsolutePath,
) -> Result<(), Box<dyn std::error::Error>> {
    let bin_dir = install_dir.join("bin");
    tokio::fs::create_dir_all(&bin_dir).await?;

    #[cfg(windows)]
    {
        let shim_dst = bin_dir.join("vp.exe");
        let shim_src = install_dir.join("current").join("bin").join("vp-shim.exe");

        // Prefer vp-shim.exe (lightweight trampoline), fall back to vp.exe
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

/// Create shell env files by spawning `vp env setup --env-only`.
async fn create_env_files(install_dir: &vite_path::AbsolutePath) {
    let vp_binary =
        install_dir.join("current").join("bin").join(if cfg!(windows) { "vp.exe" } else { "vp" });

    if !tokio::fs::try_exists(&vp_binary).await.unwrap_or(false) {
        return;
    }

    let output = tokio::process::Command::new(vp_binary.as_path())
        .args(["env", "setup", "--env-only"])
        .output()
        .await;

    if let Err(e) = output {
        tracing::warn!("Failed to create env files (non-fatal): {e}");
    }
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
        let version = opts.version.as_deref().unwrap_or("latest");
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
            if opts.no_node_manager { "disabled" } else { "auto-detect" }.cyan()
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
        let version_display = opts.version.as_deref().unwrap_or("latest");
        let registry_display = opts.registry.as_deref().unwrap_or("(default)");

        println!();
        println!("  {}", "Customize installation:".bold());
        println!();
        println!("    1) Version:        [{}]", version_display.cyan());
        println!("    2) npm registry:   [{}]", registry_display.cyan());
        println!(
            "    3) Node.js manager: [{}]",
            if opts.no_node_manager { "disabled" } else { "auto-detect" }.cyan()
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
                let v = read_input("    Version (e.g. 0.3.0, or 'latest'): ");
                if v == "latest" || v.is_empty() {
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
    println!("  Documentation:     {}", "https://github.com/voidzero-dev/vite-plus");
    println!();
}

#[allow(clippy::print_stderr)]
fn print_info(msg: &str) {
    eprint!("{}", "info: ".blue());
    eprintln!("{msg}");
}

#[allow(clippy::print_stderr)]
fn print_error(msg: &str) {
    eprint!("{}", "error: ".red());
    eprintln!("{msg}");
}
