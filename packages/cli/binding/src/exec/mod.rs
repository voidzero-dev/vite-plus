mod args;
mod workspace;

pub(crate) use args::ExecArgs;
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_shared::{PrependOptions, prepend_to_path_env};
use vite_task::ExitStatus;

use self::workspace::execute_exec_workspace;

/// Execute `vp exec` command in the local CLI.
///
/// Prepends `./node_modules/.bin` and package manager bin directory to PATH,
/// then spawns the specified command.
pub async fn execute(exec_args: ExecArgs, cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    // No command specified
    if exec_args.command.is_empty() {
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
    if exec_args.packages.is_recursive()
        || exec_args.packages.is_workspace_root()
        || !exec_args.packages.filter().is_empty()
    {
        return execute_exec_workspace(exec_args, cwd).await;
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

    if exec_args.shell_mode {
        let shell_cmd = exec_args.command.join(" ");
        let mut cmd = vite_command::build_shell_command(&shell_cmd, cwd);
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    } else {
        let bin_path = match vite_command::resolve_bin(&exec_args.command[0], None, cwd) {
            Ok(p) => p,
            Err(_) => {
                vite_shared::output::error(&format!(
                    "Command '{}' not found in node_modules/.bin\n\n\
                     Hint: Run 'vp install' to install dependencies, or use 'vpx' for remote fallback.",
                    exec_args.command[0]
                ));
                return Ok(ExitStatus(1));
            }
        };
        let mut cmd = vite_command::build_command(&bin_path, cwd);
        if exec_args.command.len() > 1 {
            cmd.args(&exec_args.command[1..]);
        }
        let mut child = cmd.spawn().map_err(|e| Error::Anyhow(e.into()))?;
        let status = child.wait().await.map_err(|e| Error::Anyhow(e.into()))?;
        Ok(ExitStatus(status.code().unwrap_or(1) as u8))
    }
}
