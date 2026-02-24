mod args;
mod filter;
mod workspace;

use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_shared::{PrependOptions, prepend_to_path_env};
use vite_task::ExitStatus;

use self::{args::parse_exec_args, workspace::execute_exec_workspace};

/// Help text for `vp exec`.
const EXEC_HELP: &str = "\
Execute a command from local node_modules/.bin

Usage: vp exec [OPTIONS] [--] <command> [args...]

Arguments:
  <command>  Command to execute from node_modules/.bin
  [args...]  Arguments to pass to the command

Options:
  -c, --shell-mode              Execute the command within a shell environment
  -r, --recursive               Run in every workspace package
  -w, --workspace-root          Run on the workspace root package only
      --include-workspace-root  Include workspace root when running recursively
      --filter <PATTERN>        Filter packages (can be used multiple times)
      --parallel                Run concurrently without topological ordering
      --reverse                 Reverse execution order
      --resume-from <PACKAGE>   Resume from a specific package
      --report-summary          Save results to vp-exec-summary.json
  -h, --help                    Print help

Examples:
  vp exec eslint .                            # Run local eslint
  vp exec tsc --noEmit                        # Run local TypeScript compiler
  vp exec -c 'eslint . && prettier --check .' # Shell mode
  vp exec -r -- eslint .                      # Run in all workspace packages
  vp exec --filter 'app...' -- tsc            # Run in filtered packages";

/// Execute `vp exec` command in the local CLI.
///
/// Prepends `./node_modules/.bin` and package manager bin directory to PATH,
/// then spawns the specified command.
pub async fn execute(args: &[String], cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let (flags, positional) = parse_exec_args(args);

    // Show help
    if flags.help {
        println!("{EXEC_HELP}");
        return Ok(ExitStatus::SUCCESS);
    }

    // No command specified
    if positional.is_empty() {
        vite_shared::output::error(
            "'vp exec' requires a command to run\n\n\
             Usage: vp exec [--] <command> [args...]\n\n\
             Examples:\n\
             \x20 vp exec eslint .\n\
             \x20 vp exec tsc --noEmit",
        );
        return Ok(ExitStatus(1));
    }

    // Workspace mode: --recursive, --workspace-root, or --filter
    if flags.recursive || flags.workspace_root || !flags.filters.is_empty() {
        return execute_exec_workspace(&flags, &positional, cwd).await;
    }

    // Single-package mode
    // Prepend package manager bin dir to PATH
    if let Ok(pm) = vite_install::PackageManager::builder(cwd).build().await {
        let bin_prefix = pm.get_bin_prefix();
        prepend_to_path_env(&bin_prefix, PrependOptions::default());
    }

    // Prepend ./node_modules/.bin to PATH (current dir only, no walk-up)
    let bin_dir = cwd.join("node_modules").join(".bin");
    if bin_dir.as_path().is_dir() {
        prepend_to_path_env(&bin_dir, PrependOptions { dedupe_anywhere: true });
    }

    // Set VITE_PLUS_PACKAGE_NAME from package.json if available
    if let Ok(pkg_json) = std::fs::read_to_string(cwd.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&pkg_json) {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                // SAFETY: only called from single-threaded local CLI context
                unsafe {
                    std::env::set_var("VITE_PLUS_PACKAGE_NAME", name);
                }
            }
        }
    }

    if flags.shell_mode {
        let shell_cmd = positional.join(" ");
        let mut cmd = vite_command::build_shell_command(&shell_cmd, cwd);
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    } else {
        let bin_path = match vite_command::resolve_bin(&positional[0], None, cwd) {
            Ok(p) => p,
            Err(_) => {
                vite_shared::output::error(&format!(
                    "Command '{}' not found in node_modules/.bin\n\n\
                     Hint: Run 'vp install' to install dependencies, or use 'vpx' for remote fallback.",
                    positional[0]
                ));
                return Ok(ExitStatus(1));
            }
        };
        let mut cmd = vite_command::build_command(&bin_path, cwd);
        if positional.len() > 1 {
            cmd.args(&positional[1..]);
        }
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    }
}
