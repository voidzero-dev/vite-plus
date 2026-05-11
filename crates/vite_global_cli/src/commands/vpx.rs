//! `vpx` command implementation.
//!
//! Executes a command from a local or remote npm package (like `npx`).
//! Resolution order:
//! 1. Local `node_modules/.bin` (walk up from cwd)
//! 2. Global vp packages (installed via `vp install -g`)
//! 3. System PATH (excluding vite-plus bin directory)
//! 4. Remote download via `vp dlx`

use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::{PrependOptions, output, prepend_to_path_env};

use crate::{commands::env::config, shim::dispatch};

/// Parsed vpx flags.
#[derive(Debug, Default)]
pub struct VpxFlags {
    /// Packages to install (from --package/-p)
    pub packages: Vec<String>,
    /// Execute within a shell environment (-c/--shell-mode)
    pub shell_mode: bool,
    /// Suppress output (-s/--silent)
    pub silent: bool,
    /// Show help (-h/--help)
    pub help: bool,
}

/// Help text for vpx.
const VPX_HELP: &str = "\
Execute a command from a local or remote npm package

Usage: vpx [OPTIONS] <pkg[@version]> [args...]

Arguments:
  <pkg[@version]>  Package binary to execute
  [args...]        Arguments to pass to the command

Options:
  -p, --package <NAME>  Package(s) to install if not found locally
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the command's output
  -h, --help            Print help

Examples:
  vpx eslint .                                           # Run local eslint (or download)
  vpx create-vue my-app                                  # Download and run create-vue
  vpx typescript@5.5.4 tsc --version                     # Run specific version
  vpx -p cowsay -c 'echo \"hi\" | cowsay'                  # Shell mode with package";

/// A globally installed binary found via `vp install -g`.
struct GlobalBinary {
    path: AbsolutePathBuf,
    is_js: bool,
    node_version: String,
}

/// Main entry point for vpx execution.
///
/// Called from shim dispatch when `argv[0]` is `vpx`.
pub async fn execute_vpx(args: &[String], cwd: &AbsolutePath) -> i32 {
    let (flags, positional) = parse_vpx_args(args);

    // Show help
    if flags.help {
        println!("{VPX_HELP}");
        return 0;
    }

    // No command specified
    if positional.is_empty() {
        output::error("vpx requires a command to run");
        eprintln!();
        eprintln!("Usage: vpx <pkg[@version]> [args...]");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  vpx eslint .");
        eprintln!("  vpx create-vue my-app");
        return 1;
    }

    let cmd_spec = &positional[0];

    // Extract the command name (binary to look for in node_modules/.bin)
    let cmd_name = extract_command_name(cmd_spec);

    // If no version spec and no --package flag, try local → global → PATH lookup
    if !has_version_spec(cmd_spec) && flags.packages.is_empty() && !flags.shell_mode {
        // 1. Try local node_modules/.bin
        if let Some(local_bin) = find_local_binary(cwd, &cmd_name) {
            tracing::debug!("vpx: found local binary at {}", local_bin.as_path().display());
            prepend_node_modules_bin_to_path(cwd);
            let cmd_args: Vec<String> = positional[1..].to_vec();
            return crate::shim::exec::exec_tool(&local_bin, &cmd_args);
        }

        // 2. Try global vp packages
        if let Some(global_bin) = find_global_binary(&cmd_name).await {
            tracing::debug!("vpx: found global binary at {}", global_bin.path.as_path().display());
            return execute_global_binary(global_bin, &positional[1..], cwd).await;
        }

        // 3. Try system PATH (excluding vite-plus bin dir)
        if let Some(path_bin) = find_on_path(&cmd_name) {
            tracing::debug!("vpx: found on PATH at {}", path_bin.as_path().display());
            prepend_node_modules_bin_to_path(cwd);
            let cmd_args: Vec<String> = positional[1..].to_vec();
            return crate::shim::exec::exec_tool(&path_bin, &cmd_args);
        }
    }

    // 4. Fall back to dlx (remote download)
    if let Err(e) = super::prepend_js_runtime_to_path_env(cwd).await {
        output::error(&format!("vpx: {e}"));
        return 1;
    }
    let dlx = vite_pm_cli::PackageManagerCommand::Dlx {
        package: flags.packages,
        shell_mode: flags.shell_mode,
        silent: flags.silent,
        args: positional,
    };
    match vite_pm_cli::dispatch(cwd, dlx).await {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            output::error(&format!("vpx: {e}"));
            1
        }
    }
}

/// Find a binary in globally installed vp packages.
///
/// Uses the dispatch helpers to look up BinConfig and PackageMetadata.
async fn find_global_binary(cmd: &str) -> Option<GlobalBinary> {
    let metadata = match dispatch::find_package_for_binary(cmd).await {
        Ok(Some(m)) => m,
        _ => return None,
    };

    let path = match dispatch::locate_package_binary(&metadata.name, cmd) {
        Ok(p) => p,
        Err(_) => return None,
    };

    Some(GlobalBinary {
        is_js: metadata.is_js_binary(cmd),
        node_version: metadata.platform.node.clone(),
        path,
    })
}

/// Execute a globally installed binary.
///
/// Ensures the required Node.js version is installed, prepends its bin dir
/// and local node_modules/.bin dirs to PATH, then executes.
async fn execute_global_binary(bin: GlobalBinary, args: &[String], cwd: &AbsolutePath) -> i32 {
    // Ensure Node.js is installed
    if let Err(e) = dispatch::ensure_installed(&bin.node_version).await {
        output::error(&format!("vpx: Failed to install Node {}: {e}", bin.node_version));
        return 1;
    }

    // Locate node binary for this version
    let node_path = match dispatch::locate_tool(&bin.node_version, "node") {
        Ok(p) => p,
        Err(e) => {
            output::error(&format!("vpx: Node not found: {e}"));
            return 1;
        }
    };

    // Prepend Node.js bin dir to PATH
    let node_bin_dir = node_path.parent().expect("Node has no parent directory");
    prepend_to_path_env(node_bin_dir, PrependOptions::default());

    // Prepend local node_modules/.bin dirs to PATH
    prepend_node_modules_bin_to_path(cwd);

    if bin.is_js {
        // Execute: node <binary_path> <args>
        let mut full_args = vec![bin.path.as_path().display().to_string()];
        full_args.extend(args.iter().cloned());
        crate::shim::exec::exec_tool(&node_path, &full_args)
    } else {
        crate::shim::exec::exec_tool(&bin.path, args)
    }
}

/// Find a command on system PATH, excluding the vite-plus bin directory.
///
/// This prevents vpx from finding itself (or other vite-plus shims) on PATH.
fn find_on_path(cmd: &str) -> Option<AbsolutePathBuf> {
    let bin_dir = config::get_bin_dir().ok();
    let path_var = std::env::var_os("PATH")?;

    // Filter PATH to exclude vite-plus bin directory
    let filtered_paths: Vec<_> = std::env::split_paths(&path_var)
        .filter(|p| {
            if let Some(ref bin) = bin_dir {
                if p == bin.as_path() {
                    return false;
                }
            }
            true
        })
        .collect();

    let filtered_path = std::env::join_paths(filtered_paths).ok()?;
    let cwd = vite_path::current_dir().ok()?;
    vite_command::resolve_bin(cmd, Some(&filtered_path), &cwd).ok()
}

/// Prepend all `node_modules/.bin` directories from cwd upward to PATH.
///
/// Walks up from cwd and prepends each existing `node_modules/.bin` directory
/// to PATH so that sub-processes also resolve local binaries first.
fn prepend_node_modules_bin_to_path(cwd: &AbsolutePath) {
    // Collect dirs bottom-up, then prepend in reverse so nearest is first
    let mut bin_dirs = Vec::new();
    let mut current = cwd;
    loop {
        let bin_dir = current.join("node_modules").join(".bin");
        if bin_dir.as_path().is_dir() {
            bin_dirs.push(bin_dir);
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }

    // Prepend in reverse order so the nearest (deepest) directory ends up first
    for dir in bin_dirs.iter().rev() {
        prepend_to_path_env(dir, PrependOptions { dedupe_anywhere: true });
    }
}

/// Walk up from `cwd` looking for `node_modules/.bin/<cmd>`.
///
/// On Windows, also checks for `.cmd` extension.
/// Returns the absolute path to the binary if found.
pub fn find_local_binary(cwd: &AbsolutePath, cmd: &str) -> Option<AbsolutePathBuf> {
    let mut current = cwd;
    loop {
        let bin_dir = current.join("node_modules").join(".bin");
        let bin_path = bin_dir.join(cmd);

        if bin_path.as_path().exists() {
            return Some(bin_path);
        }

        // On Windows, check for .cmd extension
        #[cfg(windows)]
        {
            let cmd_path = bin_dir.join(format!("{cmd}.cmd"));
            if cmd_path.as_path().exists() {
                return Some(cmd_path);
            }
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None, // Reached filesystem root
        }
    }
}

/// Check if a package spec includes a version (e.g., `eslint@9`).
///
/// Scoped packages like `@vue/cli` are not version specs, but
/// `@vue/cli@5.0.0` is.
pub fn has_version_spec(spec: &str) -> bool {
    if spec.starts_with('@') {
        // Scoped package: @scope/pkg@version
        if let Some(slash_pos) = spec.find('/') {
            return spec[slash_pos + 1..].contains('@');
        }
        // Just "@scope" with no slash — not a valid spec, no version
        return false;
    }
    spec.contains('@')
}

/// Extract the command/binary name from a package spec.
///
/// Examples:
/// - `eslint` → `eslint`
/// - `eslint@9` → `eslint`
/// - `@vue/cli` → `cli`
/// - `@vue/cli@5.0.0` → `cli`
fn extract_command_name(spec: &str) -> String {
    if spec.starts_with('@') {
        // Scoped package: @scope/pkg or @scope/pkg@version
        if let Some(slash_pos) = spec.find('/') {
            let after_slash = &spec[slash_pos + 1..];
            // Strip version if present
            if let Some(at_pos) = after_slash.find('@') {
                return after_slash[..at_pos].to_string();
            }
            return after_slash.to_string();
        }
        // Just "@scope" — use as-is (unusual case)
        return spec.to_string();
    }
    // Unscoped: pkg or pkg@version
    if let Some(at_pos) = spec.find('@') { spec[..at_pos].to_string() } else { spec.to_string() }
}

/// Parse vpx flags from the argument slice.
///
/// All flags must come before the first positional argument (npx-style).
/// Returns the parsed flags and remaining positional arguments.
pub fn parse_vpx_args(args: &[String]) -> (VpxFlags, Vec<String>) {
    let mut flags = VpxFlags::default();
    let mut positional = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Once we see a non-flag argument, everything else is positional
        if !arg.starts_with('-') {
            positional.extend_from_slice(&args[i..]);
            break;
        }

        match arg.as_str() {
            "-p" | "--package" => {
                i += 1;
                if i < args.len() {
                    flags.packages.push(args[i].clone());
                }
            }
            "-c" | "--shell-mode" => {
                flags.shell_mode = true;
            }
            "-s" | "--silent" => {
                flags.silent = true;
            }
            "-h" | "--help" => {
                flags.help = true;
            }
            other => {
                // Handle --package=VALUE
                if let Some(value) = other.strip_prefix("--package=") {
                    flags.packages.push(value.to_string());
                } else if let Some(value) = other.strip_prefix("-p=") {
                    flags.packages.push(value.to_string());
                } else {
                    // Unknown flag — treat as start of positional args
                    positional.extend_from_slice(&args[i..]);
                    break;
                }
            }
        }

        i += 1;
    }

    (flags, positional)
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    // =========================================================================
    // has_version_spec tests
    // =========================================================================

    #[test]
    fn test_has_version_spec_simple_package() {
        assert!(!has_version_spec("eslint"));
    }

    #[test]
    fn test_has_version_spec_with_version() {
        assert!(has_version_spec("eslint@9"));
    }

    #[test]
    fn test_has_version_spec_with_full_version() {
        assert!(has_version_spec("typescript@5.5.4"));
    }

    #[test]
    fn test_has_version_spec_scoped_package_no_version() {
        assert!(!has_version_spec("@vue/cli"));
    }

    #[test]
    fn test_has_version_spec_scoped_package_with_version() {
        assert!(has_version_spec("@vue/cli@5.0.0"));
    }

    #[test]
    fn test_has_version_spec_scoped_no_slash() {
        assert!(!has_version_spec("@vue"));
    }

    #[test]
    fn test_has_version_spec_with_tag() {
        assert!(has_version_spec("eslint@latest"));
    }

    // =========================================================================
    // extract_command_name tests
    // =========================================================================

    #[test]
    fn test_extract_command_name_simple() {
        assert_eq!(extract_command_name("eslint"), "eslint");
    }

    #[test]
    fn test_extract_command_name_with_version() {
        assert_eq!(extract_command_name("eslint@9"), "eslint");
    }

    #[test]
    fn test_extract_command_name_scoped() {
        assert_eq!(extract_command_name("@vue/cli"), "cli");
    }

    #[test]
    fn test_extract_command_name_scoped_with_version() {
        assert_eq!(extract_command_name("@vue/cli@5.0.0"), "cli");
    }

    #[test]
    fn test_extract_command_name_create_vue() {
        assert_eq!(extract_command_name("create-vue"), "create-vue");
    }

    // =========================================================================
    // parse_vpx_args tests
    // =========================================================================

    #[test]
    fn test_parse_vpx_args_simple_command() {
        let args: Vec<String> = vec!["eslint".into(), ".".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert!(flags.packages.is_empty());
        assert!(!flags.shell_mode);
        assert!(!flags.silent);
        assert!(!flags.help);
        assert_eq!(positional, vec!["eslint", "."]);
    }

    #[test]
    fn test_parse_vpx_args_with_package_flag() {
        let args: Vec<String> =
            vec!["-p".into(), "cowsay".into(), "-c".into(), "echo hi | cowsay".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert_eq!(flags.packages, vec!["cowsay"]);
        assert!(flags.shell_mode);
        assert_eq!(positional, vec!["echo hi | cowsay"]);
    }

    #[test]
    fn test_parse_vpx_args_with_long_package_flag() {
        let args: Vec<String> = vec!["--package".into(), "yo".into(), "yo".into(), "webapp".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert_eq!(flags.packages, vec!["yo"]);
        assert_eq!(positional, vec!["yo", "webapp"]);
    }

    #[test]
    fn test_parse_vpx_args_with_package_equals() {
        let args: Vec<String> = vec!["--package=cowsay".into(), "cowsay".into(), "hello".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert_eq!(flags.packages, vec!["cowsay"]);
        assert_eq!(positional, vec!["cowsay", "hello"]);
    }

    #[test]
    fn test_parse_vpx_args_multiple_packages() {
        let args: Vec<String> = vec![
            "-p".into(),
            "cowsay".into(),
            "-p".into(),
            "lolcatjs".into(),
            "-c".into(),
            "echo hi | cowsay | lolcatjs".into(),
        ];
        let (flags, positional) = parse_vpx_args(&args);
        assert_eq!(flags.packages, vec!["cowsay", "lolcatjs"]);
        assert!(flags.shell_mode);
        assert_eq!(positional, vec!["echo hi | cowsay | lolcatjs"]);
    }

    #[test]
    fn test_parse_vpx_args_silent() {
        let args: Vec<String> = vec!["-s".into(), "create-vue".into(), "my-app".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert!(flags.silent);
        assert_eq!(positional, vec!["create-vue", "my-app"]);
    }

    #[test]
    fn test_parse_vpx_args_help() {
        let args: Vec<String> = vec!["--help".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert!(flags.help);
        assert!(positional.is_empty());
    }

    #[test]
    fn test_parse_vpx_args_no_args() {
        let args: Vec<String> = vec![];
        let (flags, positional) = parse_vpx_args(&args);
        assert!(flags.packages.is_empty());
        assert!(!flags.shell_mode);
        assert!(!flags.silent);
        assert!(!flags.help);
        assert!(positional.is_empty());
    }

    #[test]
    fn test_parse_vpx_args_unknown_flag_becomes_positional() {
        let args: Vec<String> = vec!["--version".into()];
        let (flags, positional) = parse_vpx_args(&args);
        assert!(!flags.help);
        assert_eq!(positional, vec!["--version"]);
    }

    // =========================================================================
    // find_local_binary tests
    // =========================================================================

    #[test]
    fn test_find_local_binary_in_cwd() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create node_modules/.bin/eslint
        let bin_dir = temp_path.join("node_modules").join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let eslint_path = bin_dir.join("eslint");
        std::fs::write(&eslint_path, "#!/bin/sh\n").unwrap();

        let result = find_local_binary(&temp_path, "eslint");
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_path(), eslint_path.as_path());
    }

    #[test]
    fn test_find_local_binary_walks_up() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create node_modules/.bin/eslint at root
        let bin_dir = temp_path.join("node_modules").join(".bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let eslint_path = bin_dir.join("eslint");
        std::fs::write(&eslint_path, "#!/bin/sh\n").unwrap();

        // Create nested directory
        let nested_dir = temp_path.join("packages").join("app");
        std::fs::create_dir_all(&nested_dir).unwrap();

        let nested_abs = AbsolutePathBuf::new(nested_dir.as_path().to_path_buf()).unwrap();
        let result = find_local_binary(&nested_abs, "eslint");
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_path(), eslint_path.as_path());
    }

    #[test]
    fn test_find_local_binary_not_found() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        let result = find_local_binary(&temp_path, "nonexistent-tool");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_local_binary_prefers_nearest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create eslint at root
        let root_bin = temp_path.join("node_modules").join(".bin");
        std::fs::create_dir_all(&root_bin).unwrap();
        std::fs::write(root_bin.join("eslint"), "root").unwrap();

        // Create eslint in nested package
        let nested = temp_path.join("packages").join("app");
        let nested_bin = nested.join("node_modules").join(".bin");
        std::fs::create_dir_all(&nested_bin).unwrap();
        std::fs::write(nested_bin.join("eslint"), "nested").unwrap();

        let nested_abs = AbsolutePathBuf::new(nested.as_path().to_path_buf()).unwrap();
        let result = find_local_binary(&nested_abs, "eslint");
        assert!(result.is_some());
        // Should find the nested one first
        let found = result.unwrap();
        assert_eq!(found.as_path(), nested_bin.join("eslint").as_path());
    }

    // =========================================================================
    // find_global_binary tests
    // =========================================================================

    #[tokio::test]
    async fn test_find_global_binary_not_installed() {
        // A binary that doesn't exist in any global package should return None
        let result = find_global_binary("nonexistent-vpx-test-binary-xyz").await;
        assert!(result.is_none());
    }

    // =========================================================================
    // find_on_path tests
    // =========================================================================

    #[cfg(unix)]
    fn create_fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[cfg(windows)]
    fn create_fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{name}.exe"));
        std::fs::write(&path, "fake").unwrap();
        path
    }

    #[test]
    #[serial]
    fn test_find_on_path_finds_tool() {
        let original_path = std::env::var_os("PATH");
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("bin_test");
        std::fs::create_dir_all(&dir).unwrap();
        create_fake_executable(&dir, "vpx-test-tool-abc");

        // SAFETY: serial test
        unsafe {
            std::env::set_var("PATH", &dir);
        }

        let result = find_on_path("vpx-test-tool-abc");
        assert!(result.is_some());

        unsafe {
            match &original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    #[test]
    #[serial]
    fn test_find_on_path_excludes_vp_bin_dir() {
        let original_path = std::env::var_os("PATH");
        let original_home = std::env::var_os("VP_HOME");
        let temp = tempfile::tempdir().unwrap();

        // Set up a fake vite-plus home with bin dir
        let fake_home = temp.path().join("vite-plus-home");
        let fake_bin = fake_home.join("bin");
        std::fs::create_dir_all(&fake_bin).unwrap();
        create_fake_executable(&fake_bin, "vpx-excluded-tool");

        // Set up another directory with the same tool
        let other_dir = temp.path().join("other_bin");
        std::fs::create_dir_all(&other_dir).unwrap();
        create_fake_executable(&other_dir, "vpx-excluded-tool");

        let path = std::env::join_paths([fake_bin.as_path(), other_dir.as_path()]).unwrap();

        // SAFETY: serial test
        unsafe {
            std::env::set_var("PATH", &path);
            std::env::set_var("VP_HOME", fake_home.as_os_str());
        }

        let result = find_on_path("vpx-excluded-tool");
        assert!(result.is_some());
        // Should find the one in other_dir, not fake_bin
        assert!(
            result.unwrap().as_path().starts_with(&other_dir),
            "Should skip vite-plus bin dir and find tool in other directory"
        );

        unsafe {
            match &original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
            match &original_home {
                Some(v) => std::env::set_var("VP_HOME", v),
                None => std::env::remove_var("VP_HOME"),
            }
        }
    }

    // =========================================================================
    // prepend_node_modules_bin_to_path tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_prepend_node_modules_bin_to_path() {
        let original_path = std::env::var_os("PATH");
        let temp = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        // Create node_modules/.bin at root
        let root_bin = temp_path.join("node_modules").join(".bin");
        std::fs::create_dir_all(&root_bin).unwrap();

        // Create node_modules/.bin in nested package
        let nested = temp_path.join("packages").join("app");
        let nested_bin = nested.join("node_modules").join(".bin");
        std::fs::create_dir_all(&nested_bin).unwrap();

        // SAFETY: serial test
        unsafe {
            std::env::set_var("PATH", "/usr/bin");
        }

        prepend_node_modules_bin_to_path(&nested);

        let new_path = std::env::var_os("PATH").unwrap();
        let paths: Vec<_> = std::env::split_paths(&new_path).collect();

        // Nearest (nested) should be first
        assert_eq!(paths[0], nested_bin.as_path().to_path_buf());
        // Root should be second
        assert_eq!(paths[1], root_bin.as_path().to_path_buf());

        unsafe {
            match &original_path {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }
    }
}
