//! `vpx` command implementation.
//!
//! Executes a command from a local or remote npm package (like `npx`).
//! First checks local `node_modules/.bin` for the command, then falls back
//! to `vp dlx` behavior for remote download.

use vite_path::{AbsolutePath, AbsolutePathBuf};

use super::DlxCommand;

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
        eprintln!("Error: vpx requires a command to run");
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

    // If no version spec and no --package flag, try local lookup first
    if !has_version_spec(cmd_spec) && flags.packages.is_empty() && !flags.shell_mode {
        if let Some(local_bin) = find_local_binary(cwd, &cmd_name) {
            tracing::debug!("vpx: found local binary at {}", local_bin.as_path().display());
            let cmd_args: Vec<String> = positional[1..].to_vec();
            return crate::shim::exec::exec_tool(&local_bin, &cmd_args);
        }
    }

    // Fall back to dlx
    let cwd_buf = cwd.to_absolute_path_buf();
    match DlxCommand::new(cwd_buf)
        .execute(flags.packages, flags.shell_mode, flags.silent, positional)
        .await
    {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("vpx: {e}");
            1
        }
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
}
