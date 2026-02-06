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
//! - bin/vp.cmd is a wrapper script that calls ..\current\bin\vp.exe
//! - bin/node.cmd, bin/npm.cmd, bin/npx.cmd are wrappers calling `vp env run <tool>`

use std::process::ExitStatus;

use super::config::{get_bin_dir, get_vite_plus_home};
use crate::error::Error;

/// Tools to create shims for (node, npm, npx)
const SHIM_TOOLS: &[&str] = &["node", "npm", "npx"];

/// Execute the setup command.
pub async fn execute(refresh: bool, env_only: bool) -> Result<ExitStatus, Error> {
    let vite_plus_home = get_vite_plus_home()?;

    // Ensure home directory exists (env files are written here)
    tokio::fs::create_dir_all(&vite_plus_home).await?;

    // Create env files with PATH guard (prevents duplicate PATH entries)
    create_env_files(&vite_plus_home).await?;

    if env_only {
        return Ok(ExitStatus::default());
    }

    let bin_dir = get_bin_dir()?;

    println!("Setting up vite-plus environment...");
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

    // Print results
    if !created.is_empty() {
        println!("Created shims:");
        for tool in &created {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
    }

    if !skipped.is_empty() && !refresh {
        println!("Skipped existing shims:");
        for tool in &skipped {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
        println!();
        println!("Use --refresh to update existing shims.");
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
        let bin_vp_cmd = bin_dir.join("vp.cmd");

        // Create wrapper script bin/vp.cmd that calls current\bin\vp.exe
        let should_create_wrapper =
            refresh || !tokio::fs::try_exists(&bin_vp_cmd).await.unwrap_or(false);

        if should_create_wrapper {
            // Set VITE_PLUS_HOME using a for loop to canonicalize the path.
            // %~dp0.. would produce paths like C:\Users\x\.vite-plus\bin\..
            // The for loop resolves this to a clean C:\Users\x\.vite-plus
            let cmd_content = "@echo off\r\nfor %%I in (\"%~dp0..\") do set VITE_PLUS_HOME=%%~fI\r\n\"%VITE_PLUS_HOME%\\current\\bin\\vp.exe\" %*\r\nexit /b %ERRORLEVEL%\r\n";
            tokio::fs::write(&bin_vp_cmd, cmd_content).await?;
            tracing::debug!("Created wrapper script {:?}", bin_vp_cmd);
        }

        // Also create shell script for Git Bash (vp without extension)
        // Note: We call vp.exe directly, not via symlink, because Windows
        // symlinks require admin privileges and Git Bash support is unreliable
        let bin_vp = bin_dir.join("vp");
        let should_create_sh = refresh || !tokio::fs::try_exists(&bin_vp).await.unwrap_or(false);

        if should_create_sh {
            let sh_content = r#"#!/bin/sh
VITE_PLUS_HOME="$(dirname "$(dirname "$(readlink -f "$0" 2>/dev/null || echo "$0")")")"
export VITE_PLUS_HOME
exec "$VITE_PLUS_HOME/current/bin/vp.exe" "$@"
"#;
            tokio::fs::write(&bin_vp, sh_content).await?;
            tracing::debug!("Created shell wrapper script {:?}", bin_vp);
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
        // Remove existing shim for refresh
        tokio::fs::remove_file(&shim_path).await?;
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
        // All tools use .cmd wrappers on Windows (including node)
        format!("{tool}.cmd")
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

/// Create Windows shims using .cmd wrappers that call `vp env run <tool>`.
///
/// All tools (node, npm, npx) get .cmd wrappers that invoke `vp env run`.
/// Also creates shell scripts (without extension) for Git Bash compatibility.
/// This is consistent with Volta's Windows approach.
#[cfg(windows)]
async fn create_windows_shim(
    _source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let cmd_path = bin_dir.join(format!("{tool}.cmd"));

    // Create .cmd wrapper that calls vp env run <tool>
    // Use a for loop to canonicalize VITE_PLUS_HOME path.
    // %~dp0.. would produce paths like C:\Users\x\.vite-plus\bin\..
    // The for loop resolves this to a clean C:\Users\x\.vite-plus
    let cmd_content = format!(
        "@echo off\r\nfor %%I in (\"%~dp0..\") do set VITE_PLUS_HOME=%%~fI\r\n\"%VITE_PLUS_HOME%\\current\\bin\\vp.exe\" env run {} %*\r\nexit /b %ERRORLEVEL%\r\n",
        tool
    );

    tokio::fs::write(&cmd_path, cmd_content).await?;

    // Also create shell script for Git Bash (tool without extension)
    // Uses explicit "vp env run <tool>" instead of symlink+argv[0] because
    // Windows symlinks require admin privileges
    let sh_path = bin_dir.join(tool);
    let sh_content = format!(
        r#"#!/bin/sh
VITE_PLUS_HOME="$(dirname "$(dirname "$(readlink -f "$0" 2>/dev/null || echo "$0")")")"
export VITE_PLUS_HOME
exec "$VITE_PLUS_HOME/current/bin/vp.exe" env run {} "$@"
"#,
        tool
    );
    tokio::fs::write(&sh_path, sh_content).await?;

    tracing::debug!("Created Windows wrappers for {} (.cmd and shell script)", tool);

    Ok(())
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

    // Use $HOME-relative path if install dir is under HOME (like rustup's ~/.cargo/env)
    // This makes the env file portable across sessions where HOME may differ
    let bin_path_ref = if let Ok(home_dir) = std::env::var("HOME") {
        let home = std::path::Path::new(&home_dir);
        if let Ok(suffix) = bin_path.as_path().strip_prefix(home) {
            format!("$HOME/{}", suffix.display())
        } else {
            bin_path.as_path().display().to_string()
        }
    } else {
        bin_path.as_path().display().to_string()
    };

    // POSIX env file (bash/zsh)
    // When sourced multiple times, removes existing entry and re-prepends to front
    // Uses parameter expansion to split PATH around the bin entry in O(1) operations
    // Includes vp() shell function wrapper for `vp env use` (evals stdout)
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
"#
    .replace("__VP_BIN__", &bin_path_ref);
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
"#
    .replace("__VP_BIN__", &bin_path_ref);
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
"#;

    // For PowerShell, use the actual absolute path (not $HOME-relative)
    let bin_path_win = bin_path.as_path().display().to_string();
    let env_ps1_content = env_ps1_content.replace("__VP_BIN_WIN__", &bin_path_win);
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

    println!("Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
    println!();
    println!("  . \"{home_path}/env\"");
    println!();
    println!("For fish shell, add to ~/.config/fish/config.fish:");
    println!();
    println!("  source \"{home_path}/env.fish\"");
    println!();
    println!("For PowerShell, add to your $PROFILE:");
    println!();
    println!("  . \"{home_path}/env.ps1\"");
    println!();
    println!("For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:");

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
    println!("Restart your terminal and IDE, then run 'vp env doctor' to verify.");
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;
    use vite_path::AbsolutePathBuf;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

        create_env_files(&home).await.unwrap();

        let env_path = home.join("env");
        let env_fish_path = home.join("env.fish");
        let env_ps1_path = home.join("env.ps1");
        assert!(env_path.as_path().exists(), "env file should be created");
        assert!(env_fish_path.as_path().exists(), "env.fish file should be created");
        assert!(env_ps1_path.as_path().exists(), "env.ps1 file should be created");

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_replaces_placeholder_with_home_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_uses_absolute_path_when_not_under_home() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Set HOME to a different path so install dir is NOT under HOME
        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", "/nonexistent-home-dir");
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_posix_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_fish_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_is_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_posix_contains_vp_shell_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_fish_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_create_env_files_ps1_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
        }

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

        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_env_only_creates_home_dir_and_env_files() {
        let temp_dir = TempDir::new().unwrap();
        let fresh_home = temp_dir.path().join("new-vite-plus");
        // Directory does NOT exist yet — execute should create it

        // SAFETY: This test runs in isolation with serial_test
        unsafe {
            std::env::set_var("VITE_PLUS_HOME", &fresh_home);
            std::env::set_var("HOME", temp_dir.path());
        }

        let status = execute(false, true).await.unwrap();
        assert!(status.success(), "execute --env-only should succeed");

        // Directory should now exist
        assert!(fresh_home.exists(), "VITE_PLUS_HOME directory should be created");

        // Env files should be written
        assert!(fresh_home.join("env").exists(), "env file should be created");
        assert!(fresh_home.join("env.fish").exists(), "env.fish file should be created");
        assert!(fresh_home.join("env.ps1").exists(), "env.ps1 file should be created");

        unsafe {
            std::env::remove_var("VITE_PLUS_HOME");
            std::env::remove_var("HOME");
        }
    }
}
