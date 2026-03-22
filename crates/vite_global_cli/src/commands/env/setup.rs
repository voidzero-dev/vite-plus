//! Setup command implementation for creating bin directory and shims.
//!
//! Creates the following structure:
//! - ~/.vite-plus/bin/     - Contains vp symlink and node/npm/npx shims
//! - ~/.vite-plus/current/ - Contains the actual vp CLI binary
//!
//! On Unix:
//! - bin/vp is a symlink to ../current/bin/vp
//! - bin/node, bin/npm, bin/npx are symlinks to ../current/bin/vp
//! - Symlinks preserve argv[0], allowing tool detection via the symlink name
//!
//! On Windows:
//! - bin/vp.exe, bin/node.exe, bin/npm.exe, bin/npx.exe are trampoline executables
//! - Each trampoline detects its tool name from its own filename and spawns
//!   current\bin\vp.exe with VITE_PLUS_SHIM_TOOL env var set
//! - This avoids the "Terminate batch job (Y/N)?" prompt from .cmd wrappers

use std::process::ExitStatus;

use clap::CommandFactory;
use owo_colors::OwoColorize;

use super::config::{get_bin_dir, get_vite_plus_home};
use crate::{cli::Args, error::Error, help};

/// Tools to create shims for (node, npm, npx, vpx)
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx", "vpx"];

fn accent_command(command: &str) -> String {
    if help::should_style_help() {
        format!("`{}`", command.bright_blue())
    } else {
        format!("`{command}`")
    }
}

/// Execute the setup command.
pub async fn execute(refresh: bool, env_only: bool) -> Result<ExitStatus, Error> {
    let vite_plus_home = get_vite_plus_home()?;

    // Ensure home directory exists (env files are written here)
    tokio::fs::create_dir_all(&vite_plus_home).await?;

    // Generate completion scripts
    generate_completion_scripts(&vite_plus_home).await?;

    // Create env files with PATH guard (prevents duplicate PATH entries)
    create_env_files(&vite_plus_home).await?;

    if env_only {
        println!("{}", help::render_heading("Setup"));
        println!("  Updated shell environment files.");
        println!("  Run {} to verify setup.", accent_command("vp env doctor"));
        return Ok(ExitStatus::default());
    }

    let bin_dir = get_bin_dir()?;

    println!("{}", help::render_heading("Setup"));
    println!("  Preparing vite-plus environment.");
    println!();

    // Ensure bin directory exists
    tokio::fs::create_dir_all(&bin_dir).await?;

    // Get the current executable path (for shims)
    let current_exe = std::env::current_exe()
        .map_err(|e| Error::ConfigError(format!("Cannot find current executable: {e}").into()))?;

    // Create wrapper script in bin/
    setup_vp_wrapper(&bin_dir, refresh).await?;

    // Create shims for node, npm, npx
    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for tool in SHIM_TOOLS {
        let result = create_shim(&current_exe, &bin_dir, tool, refresh).await?;
        if result {
            created.push(*tool);
        } else {
            skipped.push(*tool);
        }
    }

    // Best-effort cleanup of .old files from rename-before-copy on Windows
    #[cfg(windows)]
    if refresh {
        cleanup_old_files(&bin_dir).await;
    }

    // Print results
    if !created.is_empty() {
        println!("{}", help::render_heading("Created Shims"));
        for tool in &created {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
    }

    if !skipped.is_empty() && !refresh {
        if !created.is_empty() {
            println!();
        }
        println!("{}", help::render_heading("Skipped Shims"));
        for tool in &skipped {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
        println!();
        println!("  Use --refresh to update existing shims.");
    }

    println!();
    print_path_instructions(&bin_dir);

    Ok(ExitStatus::default())
}

/// Create symlink in bin/ that points to current/bin/vp.
async fn setup_vp_wrapper(bin_dir: &vite_path::AbsolutePath, refresh: bool) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let bin_vp = bin_dir.join("vp");

        // Create symlink bin/vp -> ../current/bin/vp
        let should_create_symlink = refresh
            || !tokio::fs::try_exists(&bin_vp).await.unwrap_or(false)
            || !is_symlink(&bin_vp).await; // Replace non-symlink with symlink

        if should_create_symlink {
            // Remove existing if present (could be old wrapper script or file)
            if tokio::fs::try_exists(&bin_vp).await.unwrap_or(false) {
                tokio::fs::remove_file(&bin_vp).await?;
            }
            // Create relative symlink
            tokio::fs::symlink("../current/bin/vp", &bin_vp).await?;
            tracing::debug!("Created symlink {:?} -> ../current/bin/vp", bin_vp);
        }
    }

    #[cfg(windows)]
    {
        let bin_vp_exe = bin_dir.join("vp.exe");

        // Create trampoline bin/vp.exe that forwards to current\bin\vp.exe
        let should_create = refresh || !tokio::fs::try_exists(&bin_vp_exe).await.unwrap_or(false);

        if should_create {
            let trampoline_src = get_trampoline_path()?;
            // On refresh, the existing vp.exe may still be running (the trampoline
            // that launched us). Windows prevents overwriting a running exe, so we
            // rename it to a timestamped .old file first, then copy the new one.
            if tokio::fs::try_exists(&bin_vp_exe).await.unwrap_or(false) {
                rename_to_old(&bin_vp_exe).await;
            }

            tokio::fs::copy(trampoline_src.as_path(), &bin_vp_exe).await?;
            tracing::debug!("Created trampoline {:?}", bin_vp_exe);
        }

        // Clean up legacy .cmd and shell script wrappers from previous versions
        if refresh {
            cleanup_legacy_windows_shim(bin_dir, "vp").await;
        }
    }

    Ok(())
}

/// Check if a path is a symlink.
#[cfg(unix)]
async fn is_symlink(path: &vite_path::AbsolutePath) -> bool {
    match tokio::fs::symlink_metadata(path).await {
        Ok(m) => m.file_type().is_symlink(),
        Err(_) => false,
    }
}

/// Create a single shim for node/npm/npx.
///
/// Returns `true` if the shim was created, `false` if it already exists.
async fn create_shim(
    source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
    refresh: bool,
) -> Result<bool, Error> {
    let shim_path = bin_dir.join(shim_filename(tool));

    // Check if shim already exists
    if tokio::fs::try_exists(&shim_path).await.unwrap_or(false) {
        if !refresh {
            return Ok(false);
        }
        // Remove existing shim for refresh.
        // On Windows, .exe files may be locked (by antivirus, indexer, or
        // still-running processes), so rename to .old first instead of deleting.
        #[cfg(windows)]
        rename_to_old(&shim_path).await;
        #[cfg(not(windows))]
        {
            tokio::fs::remove_file(&shim_path).await?;
        }
    }

    #[cfg(unix)]
    {
        create_unix_shim(source, &shim_path, tool).await?;
    }

    #[cfg(windows)]
    {
        create_windows_shim(source, bin_dir, tool).await?;
    }

    Ok(true)
}

/// Get the filename for a shim (platform-specific).
fn shim_filename(tool: &str) -> String {
    #[cfg(windows)]
    {
        // All tools use trampoline .exe files on Windows
        format!("{tool}.exe")
    }

    #[cfg(not(windows))]
    {
        tool.to_string()
    }
}

/// Create a Unix shim using symlink to ../current/bin/vp.
///
/// Symlinks preserve argv[0], allowing the vp binary to detect which tool
/// was invoked. This is the same pattern used by Volta.
#[cfg(unix)]
async fn create_unix_shim(
    _source: &std::path::Path,
    shim_path: &vite_path::AbsolutePath,
    _tool: &str,
) -> Result<(), Error> {
    // Create symlink to ../current/bin/vp (relative path)
    tokio::fs::symlink("../current/bin/vp", shim_path).await?;
    tracing::debug!("Created symlink shim at {:?} -> ../current/bin/vp", shim_path);

    Ok(())
}

/// Create Windows shims using trampoline `.exe` files.
///
/// Each tool gets a copy of the trampoline binary renamed to `<tool>.exe`.
/// The trampoline detects its tool name from its own filename and spawns
/// vp.exe with `VITE_PLUS_SHIM_TOOL` set, avoiding the "Terminate batch job?"
/// prompt that `.cmd` wrappers cause on Ctrl+C.
///
/// See: <https://github.com/voidzero-dev/vite-plus/issues/835>
#[cfg(windows)]
async fn create_windows_shim(
    _source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let trampoline_src = get_trampoline_path()?;
    let shim_path = bin_dir.join(format!("{tool}.exe"));
    tokio::fs::copy(trampoline_src.as_path(), &shim_path).await?;

    // Clean up legacy .cmd and shell script wrappers from previous versions
    cleanup_legacy_windows_shim(bin_dir, tool).await;

    tracing::debug!("Created trampoline shim {:?}", shim_path);

    Ok(())
}

/// Creates completion scripts in `~/.vite-plus/completion/`:
/// - `vp.bash` (bash)
/// - `_vp` (zsh, following zsh convention)
/// - `vp.fish` (fish shell)
/// - `vp.ps1` (PowerShell)
async fn generate_completion_scripts(
    vite_plus_home: &vite_path::AbsolutePath,
) -> Result<(), Error> {
    let mut cmd = Args::command();

    // Create completion directory
    let completion_dir = vite_plus_home.join("completion");
    tokio::fs::create_dir_all(&completion_dir).await?;

    // Generate shell completion scripts
    let completions = [
        (clap_complete::Shell::Bash, "vp.bash"),
        (clap_complete::Shell::Zsh, "_vp"),
        (clap_complete::Shell::Fish, "vp.fish"),
        (clap_complete::Shell::PowerShell, "vp.ps1"),
    ];

    for (shell, filename) in completions {
        let path = completion_dir.join(filename);
        let mut file = std::fs::File::create(&path)?;
        clap_complete::generate(shell, &mut cmd, "vp", &mut file);
    }

    tracing::debug!("Generated completion scripts in {:?}", completion_dir);

    Ok(())
}

/// Get the path to the trampoline template binary (vp-shim.exe).
///
/// The trampoline binary is distributed alongside vp.exe in the same directory.
/// In tests, `VITE_PLUS_TRAMPOLINE_PATH` can override the resolved path.
#[cfg(windows)]
pub(crate) fn get_trampoline_path() -> Result<vite_path::AbsolutePathBuf, Error> {
    // Allow tests to override the trampoline path
    if let Ok(override_path) = std::env::var(vite_shared::env_vars::VITE_PLUS_TRAMPOLINE_PATH) {
        let path = std::path::PathBuf::from(override_path);
        if path.exists() {
            return vite_path::AbsolutePathBuf::new(path)
                .ok_or_else(|| Error::ConfigError("Invalid trampoline override path".into()));
        }
    }

    let current_exe = std::env::current_exe()
        .map_err(|e| Error::ConfigError(format!("Cannot find current executable: {e}").into()))?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| Error::ConfigError("Cannot find parent directory of vp.exe".into()))?;
    let trampoline = bin_dir.join("vp-shim.exe");

    if !trampoline.exists() {
        return Err(Error::ConfigError(
            format!(
                "Trampoline binary not found at {}. Re-install vite-plus to fix this.",
                trampoline.display()
            )
            .into(),
        ));
    }

    vite_path::AbsolutePathBuf::new(trampoline)
        .ok_or_else(|| Error::ConfigError("Invalid trampoline path".into()))
}

/// Rename an existing `.exe` to a timestamped `.old` file instead of deleting.
///
/// On Windows, running `.exe` files can't be deleted or overwritten, but they can
/// be renamed. The `.old` files are cleaned up by `cleanup_old_files()`.
#[cfg(windows)]
async fn rename_to_old(path: &vite_path::AbsolutePath) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(name) = path.as_path().file_name().and_then(|n| n.to_str()) {
        let old_name = format!("{name}.{timestamp}.old");
        let old_path = path.as_path().with_file_name(&old_name);
        if let Err(e) = tokio::fs::rename(path, &old_path).await {
            tracing::warn!("Failed to rename {} to {}: {}", name, old_name, e);
        }
    }
}

/// Best-effort cleanup of accumulated `.old` files from previous rename-before-copy operations.
///
/// When refreshing `bin/vp.exe` on Windows, the running trampoline is renamed to a
/// timestamped `.old` file. This function tries to delete all such files. Files still
/// in use by a running process will silently fail to delete and be cleaned up next time.
#[cfg(windows)]
async fn cleanup_old_files(bin_dir: &vite_path::AbsolutePath) {
    let Ok(mut entries) = tokio::fs::read_dir(bin_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.ends_with(".old") {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

/// Remove legacy `.cmd` and shell script wrappers from previous versions.
#[cfg(windows)]
pub(crate) async fn cleanup_legacy_windows_shim(bin_dir: &vite_path::AbsolutePath, tool: &str) {
    // Remove old .cmd wrapper (best-effort, ignore NotFound)
    let cmd_path = bin_dir.join(format!("{tool}.cmd"));
    let _ = tokio::fs::remove_file(&cmd_path).await;

    // Remove old shell script wrapper (extensionless, for Git Bash)
    // Only remove if it starts with #!/bin/sh (not a binary or other file)
    // Read only the first 9 bytes to avoid loading large files into memory
    let sh_path = bin_dir.join(tool);
    let is_shell_script = async {
        use tokio::io::AsyncReadExt;
        let mut file = tokio::fs::File::open(&sh_path).await.ok()?;
        let mut buf = [0u8; 9]; // b"#!/bin/sh".len()
        let n = file.read(&mut buf).await.ok()?;
        Some(buf[..n].starts_with(b"#!/bin/sh"))
        // file handle dropped here before remove_file
    }
    .await;
    if is_shell_script == Some(true) {
        let _ = tokio::fs::remove_file(&sh_path).await;
    }
}

/// Create env files with PATH guard (prevents duplicate PATH entries).
///
/// Creates:
/// - `~/.vite-plus/env` (POSIX shell — bash/zsh) with `vp()` wrapper function
/// - `~/.vite-plus/env.fish` (fish shell) with `vp` wrapper function
/// - `~/.vite-plus/env.ps1` (PowerShell) with PATH setup + `vp` function
/// - `~/.vite-plus/bin/vp-use.cmd` (cmd.exe wrapper for `vp env use`)
async fn create_env_files(vite_plus_home: &vite_path::AbsolutePath) -> Result<(), Error> {
    let bin_path = vite_plus_home.join("bin");
    let completion_path = vite_plus_home.join("completion");

    // Use $HOME-relative path if install dir is under HOME (like rustup's ~/.cargo/env)
    // This makes the env file portable across sessions where HOME may differ
    let home_dir = vite_shared::EnvConfig::get().user_home;
    let to_ref = |path: &vite_path::AbsolutePath| -> String {
        home_dir
            .as_ref()
            .and_then(|h| path.as_path().strip_prefix(h).ok())
            .map(|s| {
                // Normalize to forward slashes for $HOME/... paths (POSIX-style)
                format!("$HOME/{}", s.display().to_string().replace('\\', "/"))
            })
            .unwrap_or_else(|| path.as_path().display().to_string())
    };
    let bin_path_ref = to_ref(&bin_path);

    // POSIX env file (bash/zsh)
    // When sourced multiple times, removes existing entry and re-prepends to front
    // Uses parameter expansion to split PATH around the bin entry in O(1) operations
    // Includes vp() shell function wrapper for `vp env use` (evals stdout)
    // Includes shell completion support
    let env_content = r#"#!/bin/sh
# Vite+ environment setup (https://viteplus.dev)
__vp_bin="__VP_BIN__"
case ":${PATH}:" in
    *":${__vp_bin}:"*)
        __vp_tmp=":${PATH}:"
        __vp_before="${__vp_tmp%%":${__vp_bin}:"*}"
        __vp_before="${__vp_before#:}"
        __vp_after="${__vp_tmp#*":${__vp_bin}:"}"
        __vp_after="${__vp_after%:}"
        export PATH="${__vp_bin}${__vp_before:+:${__vp_before}}${__vp_after:+:${__vp_after}}"
        unset __vp_tmp __vp_before __vp_after
        ;;
    *)
        export PATH="$__vp_bin:$PATH"
        ;;
esac
unset __vp_bin

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VITE_PLUS_NODE_VERSION in the current shell session.
vp() {
    if [ "$1" = "env" ] && [ "$2" = "use" ]; then
        case " $* " in *" -h "*|*" --help "*) command vp "$@"; return; esac
        __vp_out="$(VITE_PLUS_ENV_USE_EVAL_ENABLE=1 command vp "$@")" || return $?
        eval "$__vp_out"
    else
        command vp "$@"
    fi
}

# Shell completion for bash/zsh
# Source appropriate completion script based on current shell
# Only load completion in interactive shells with required builtins
if [ -n "$BASH_VERSION" ] && type complete >/dev/null 2>&1; then
    # Bash shell with completion support
    __vp_completion="__VP_COMPLETION_BASH__"
    if [ -f "$__vp_completion" ]; then
        . "$__vp_completion"
    fi
    unset __vp_completion
elif [ -n "$ZSH_VERSION" ] && type compdef >/dev/null 2>&1; then
    # Zsh shell with completion support
    __vp_completion="__VP_COMPLETION_ZSH__"
    if [ -f "$__vp_completion" ]; then
        . "$__vp_completion"
    fi
    unset __vp_completion
fi
"#
    .replace("__VP_BIN__", &bin_path_ref)
    .replace("__VP_COMPLETION_BASH__", &to_ref(&completion_path.join("vp.bash")))
    .replace("__VP_COMPLETION_ZSH__", &to_ref(&completion_path.join("_vp")));
    let env_file = vite_plus_home.join("env");
    tokio::fs::write(&env_file, env_content).await?;

    // Fish env file with vp wrapper function
    let env_fish_content = r#"# Vite+ environment setup (https://viteplus.dev)
set -l __vp_idx (contains -i -- __VP_BIN__ $PATH)
and set -e PATH[$__vp_idx]
set -gx PATH __VP_BIN__ $PATH

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VITE_PLUS_NODE_VERSION in the current shell session.
function vp
    if test (count $argv) -ge 2; and test "$argv[1]" = "env"; and test "$argv[2]" = "use"
        if contains -- -h $argv; or contains -- --help $argv
            command vp $argv; return
        end
        set -lx VITE_PLUS_ENV_USE_EVAL_ENABLE 1
        set -l __vp_out (command vp $argv); or return $status
        eval $__vp_out
    else
        command vp $argv
    end
end

# Shell completion for fish
if not set -q __vp_completion_sourced
    set -l __vp_completion "__VP_COMPLETION_FISH__"
    if test -f "$__vp_completion"
        source "$__vp_completion"
        set -g __vp_completion_sourced 1
    end
end
"#
    .replace("__VP_BIN__", &bin_path_ref)
    .replace("__VP_COMPLETION_FISH__", &to_ref(&completion_path.join("vp.fish")));
    let env_fish_file = vite_plus_home.join("env.fish");
    tokio::fs::write(&env_fish_file, env_fish_content).await?;

    // PowerShell env file
    let env_ps1_content = r#"# Vite+ environment setup (https://viteplus.dev)
$__vp_bin = "__VP_BIN_WIN__"
if ($env:Path -split ';' -notcontains $__vp_bin) {
    $env:Path = "$__vp_bin;$env:Path"
}

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VITE_PLUS_NODE_VERSION in the current shell session.
function vp {
    if ($args.Count -ge 2 -and $args[0] -eq "env" -and $args[1] -eq "use") {
        if ($args -contains "-h" -or $args -contains "--help") {
            & (Join-Path $__vp_bin "vp.exe") @args; return
        }
        $env:VITE_PLUS_ENV_USE_EVAL_ENABLE = "1"
        $output = & (Join-Path $__vp_bin "vp.exe") @args 2>&1 | ForEach-Object {
            if ($_ -is [System.Management.Automation.ErrorRecord]) {
                Write-Host $_.Exception.Message
            } else {
                $_
            }
        }
        Remove-Item Env:VITE_PLUS_ENV_USE_EVAL_ENABLE -ErrorAction SilentlyContinue
        if ($LASTEXITCODE -eq 0 -and $output) {
            Invoke-Expression ($output -join "`n")
        }
    } else {
        & (Join-Path $__vp_bin "vp.exe") @args
    }
}

# Shell completion for PowerShell
$__vp_completion = "__VP_COMPLETION_PS1__"
if (Test-Path $__vp_completion) {
    . $__vp_completion
}
"#;

    // For PowerShell, use the actual absolute path (not $HOME-relative)
    let bin_path_win = bin_path.as_path().display().to_string();
    let completion_ps1_win = completion_path.join("vp.ps1").as_path().display().to_string();
    let env_ps1_content = env_ps1_content
        .replace("__VP_BIN_WIN__", &bin_path_win)
        .replace("__VP_COMPLETION_PS1__", &completion_ps1_win);
    let env_ps1_file = vite_plus_home.join("env.ps1");
    tokio::fs::write(&env_ps1_file, env_ps1_content).await?;

    // cmd.exe wrapper for `vp env use` (cmd.exe cannot define shell functions)
    // Users run `vp-use 24` in cmd.exe instead of `vp env use 24`
    let vp_use_cmd_content = "@echo off\r\nset VITE_PLUS_ENV_USE_EVAL_ENABLE=1\r\nfor /f \"delims=\" %%i in ('%~dp0..\\current\\bin\\vp.exe env use %*') do %%i\r\nset VITE_PLUS_ENV_USE_EVAL_ENABLE=\r\n";
    // Only write if bin directory exists (it may not during --env-only)
    if tokio::fs::try_exists(&bin_path).await.unwrap_or(false) {
        let vp_use_cmd_file = bin_path.join("vp-use.cmd");
        tokio::fs::write(&vp_use_cmd_file, vp_use_cmd_content).await?;
    }

    Ok(())
}

/// Print instructions for adding bin directory to PATH.
fn print_path_instructions(bin_dir: &vite_path::AbsolutePath) {
    // Derive vite_plus_home from bin_dir (parent), using $HOME prefix for readability
    let home_path = bin_dir
        .parent()
        .map(|p| p.as_path().display().to_string())
        .unwrap_or_else(|| bin_dir.as_path().display().to_string());
    let home_path = if let Ok(home_dir) = std::env::var("HOME") {
        if let Some(suffix) = home_path.strip_prefix(&home_dir) {
            format!("$HOME{suffix}")
        } else {
            home_path
        }
    } else {
        home_path
    };

    println!("{}", help::render_heading("Next Steps"));
    println!("  Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
    println!();
    println!("  . \"{home_path}/env\"");
    println!();
    println!("  For fish shell, add to ~/.config/fish/config.fish:");
    println!();
    println!("  source \"{home_path}/env.fish\"");
    println!();
    println!("  For PowerShell, add to your $PROFILE:");
    println!();
    println!("  . \"{home_path}/env.ps1\"");
    println!();
    println!("  For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:");

    #[cfg(target_os = "macos")]
    {
        println!("  - macOS: Add to ~/.profile or use launchd");
    }

    #[cfg(target_os = "linux")]
    {
        println!("  - Linux: Add to ~/.profile for display manager integration");
    }

    #[cfg(target_os = "windows")]
    {
        println!("  - Windows: System Properties -> Environment Variables -> Path");
    }

    println!();
    println!(
        "  Restart your terminal and IDE, then run {} to verify.",
        accent_command("vp env doctor")
    );
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vite_path::AbsolutePathBuf;

    use super::*;

    /// Helper: create a test_guard with user_home set to the given path.
    fn home_guard(home: impl Into<std::path::PathBuf>) -> vite_shared::TestEnvGuard {
        vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            user_home: Some(home.into()),
            ..vite_shared::EnvConfig::for_test()
        })
    }

    #[tokio::test]
    async fn test_create_env_files_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let env_path = home.join("env");
        let env_fish_path = home.join("env.fish");
        let env_ps1_path = home.join("env.ps1");
        assert!(env_path.as_path().exists(), "env file should be created");
        assert!(env_fish_path.as_path().exists(), "env.fish file should be created");
        assert!(env_ps1_path.as_path().exists(), "env.ps1 file should be created");
    }

    #[tokio::test]
    async fn test_create_env_files_replaces_placeholder_with_home_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
        let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

        // Placeholder should be fully replaced
        assert!(
            !env_content.contains("__VP_BIN__"),
            "env file should not contain __VP_BIN__ placeholder"
        );
        assert!(
            !fish_content.contains("__VP_BIN__"),
            "env.fish file should not contain __VP_BIN__ placeholder"
        );

        // Should use $HOME-relative path since install dir is under HOME
        assert!(
            env_content.contains("$HOME/bin"),
            "env file should reference $HOME/bin, got: {env_content}"
        );
        assert!(
            fish_content.contains("$HOME/bin"),
            "env.fish file should reference $HOME/bin, got: {fish_content}"
        );
    }

    #[tokio::test]
    async fn test_create_env_files_uses_absolute_path_when_not_under_home() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // Set user_home to a different path so install dir is NOT under HOME
        let _guard = home_guard("/nonexistent-home-dir");

        create_env_files(&home).await.unwrap();

        let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
        let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

        // Should use absolute path since install dir is not under HOME
        let expected_bin = home.join("bin");
        let expected_str = expected_bin.as_path().display().to_string();
        assert!(
            env_content.contains(&expected_str),
            "env file should use absolute path {expected_str}, got: {env_content}"
        );
        assert!(
            fish_content.contains(&expected_str),
            "env.fish file should use absolute path {expected_str}, got: {fish_content}"
        );

        // Should NOT use $HOME-relative path
        assert!(!env_content.contains("$HOME/bin"), "env file should not reference $HOME/bin");
    }

    #[tokio::test]
    async fn test_create_env_files_posix_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();

        // Verify PATH guard structure: case statement checks for duplicate
        assert!(
            env_content.contains("case \":${PATH}:\" in"),
            "env file should contain PATH guard case statement"
        );
        assert!(
            env_content.contains("*\":${__vp_bin}:\"*)"),
            "env file should check for existing bin in PATH"
        );
        // Verify it re-prepends to front when already present
        assert!(
            env_content.contains("export PATH=\"${__vp_bin}"),
            "env file should re-prepend bin to front of PATH"
        );
        // Verify simple prepend for new entry
        assert!(
            env_content.contains("export PATH=\"$__vp_bin:$PATH\""),
            "env file should prepend bin to PATH for new entry"
        );
    }

    #[tokio::test]
    async fn test_create_env_files_fish_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

        // Verify fish PATH guard: remove existing entry before prepending
        assert!(
            fish_content.contains("contains -i --"),
            "env.fish should check for existing bin in PATH"
        );
        assert!(
            fish_content.contains("set -e PATH[$__vp_idx]"),
            "env.fish should remove existing entry"
        );
        assert!(fish_content.contains("set -gx PATH"), "env.fish should set PATH globally");
    }

    #[tokio::test]
    async fn test_create_env_files_is_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        // Create env files twice
        create_env_files(&home).await.unwrap();
        let first_env = tokio::fs::read_to_string(home.join("env")).await.unwrap();
        let first_fish = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
        let first_ps1 = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

        create_env_files(&home).await.unwrap();
        let second_env = tokio::fs::read_to_string(home.join("env")).await.unwrap();
        let second_fish = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
        let second_ps1 = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

        assert_eq!(first_env, second_env, "env file should be identical after second write");
        assert_eq!(first_fish, second_fish, "env.fish file should be identical after second write");
        assert_eq!(first_ps1, second_ps1, "env.ps1 file should be identical after second write");
    }

    #[tokio::test]
    async fn test_create_env_files_posix_contains_vp_shell_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();

        // Verify vp() shell function wrapper is present
        assert!(env_content.contains("vp() {"), "env file should contain vp() shell function");
        assert!(
            env_content.contains("\"$1\" = \"env\""),
            "env file should check for 'env' subcommand"
        );
        assert!(
            env_content.contains("\"$2\" = \"use\""),
            "env file should check for 'use' subcommand"
        );
        assert!(env_content.contains("eval \"$__vp_out\""), "env file should eval the output");
        assert!(
            env_content.contains("command vp \"$@\""),
            "env file should use 'command vp' for passthrough"
        );
    }

    #[tokio::test]
    async fn test_create_env_files_fish_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

        // Verify fish vp function wrapper is present
        assert!(fish_content.contains("function vp"), "env.fish file should contain vp function");
        assert!(
            fish_content.contains("\"$argv[1]\" = \"env\""),
            "env.fish should check for 'env' subcommand"
        );
        assert!(
            fish_content.contains("\"$argv[2]\" = \"use\""),
            "env.fish should check for 'use' subcommand"
        );
        assert!(
            fish_content.contains("command vp $argv"),
            "env.fish should use 'command vp' for passthrough"
        );
    }

    #[tokio::test]
    async fn test_create_env_files_ps1_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let ps1_content = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

        // Verify PowerShell function is present
        assert!(ps1_content.contains("function vp {"), "env.ps1 should contain vp function");
        assert!(ps1_content.contains("Invoke-Expression"), "env.ps1 should use Invoke-Expression");
        // Should not contain placeholders
        assert!(
            !ps1_content.contains("__VP_BIN_WIN__"),
            "env.ps1 should not contain __VP_BIN_WIN__ placeholder"
        );
    }

    #[tokio::test]
    async fn test_execute_env_only_creates_home_dir_and_env_files() {
        let temp_dir = TempDir::new().unwrap();
        let fresh_home = temp_dir.path().join("new-vite-plus");
        // Directory does NOT exist yet — execute should create it
        let _guard = vite_shared::EnvConfig::test_guard(vite_shared::EnvConfig {
            vite_plus_home: Some(fresh_home.clone()),
            user_home: Some(temp_dir.path().to_path_buf()),
            ..vite_shared::EnvConfig::for_test()
        });

        let status = execute(false, true).await.unwrap();
        assert!(status.success(), "execute --env-only should succeed");

        // Directory should now exist
        assert!(fresh_home.exists(), "VITE_PLUS_HOME directory should be created");

        // Env files should be written
        assert!(fresh_home.join("env").exists(), "env file should be created");
        assert!(fresh_home.join("env.fish").exists(), "env.fish file should be created");
        assert!(fresh_home.join("env.ps1").exists(), "env.ps1 file should be created");
    }

    #[tokio::test]
    async fn test_generate_completion_scripts_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        generate_completion_scripts(&home).await.unwrap();

        let completion_dir = home.join("completion");

        // Verify all completion scripts are created
        let bash_completion = completion_dir.join("vp.bash");
        let zsh_completion = completion_dir.join("_vp");
        let fish_completion = completion_dir.join("vp.fish");
        let ps1_completion = completion_dir.join("vp.ps1");

        assert!(bash_completion.as_path().exists(), "bash completion (vp.bash) should be created");
        assert!(zsh_completion.as_path().exists(), "zsh completion (_vp) should be created");
        assert!(fish_completion.as_path().exists(), "fish completion (vp.fish) should be created");
        assert!(
            ps1_completion.as_path().exists(),
            "PowerShell completion (vp.ps1) should be created"
        );
    }

    #[tokio::test]
    async fn test_create_env_files_contains_completion() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let _guard = home_guard(temp_dir.path());

        create_env_files(&home).await.unwrap();

        let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
        let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
        let ps1_content = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

        assert!(
            env_content.contains("Shell completion")
                && env_content.contains("/completion/vp.bash\""),
            "env file should contain bash completion"
        );
        assert!(
            fish_content.contains("Shell completion")
                && fish_content.contains("/completion/vp.fish\""),
            "env.fish file should contain fish completion"
        );
        assert!(
            ps1_content.contains("Shell completion")
                && ps1_content.contains(&format!(
                    "{}completion{}vp.ps1\"",
                    std::path::MAIN_SEPARATOR_STR,
                    std::path::MAIN_SEPARATOR_STR
                )),
            "env.ps1 file should contain PowerShell completion"
        );

        // Verify placeholders are replaced
        assert!(
            !env_content.contains("__VP_COMPLETION_BASH__")
                && !env_content.contains("__VP_COMPLETION_ZSH__"),
            "env file should not contain __VP_COMPLETION_* placeholders"
        );
        assert!(
            !fish_content.contains("__VP_COMPLETION_FISH__"),
            "env.fish file should not contain __VP_COMPLETION_FISH__ placeholder"
        );
        assert!(
            !ps1_content.contains("__VP_COMPLETION_PS1__"),
            "env.ps1 file should not contain __VP_COMPLETION_PS1__ placeholder"
        );
    }
}
