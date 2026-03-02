mod args;
mod workspace;

pub(crate) use args::ExecArgs;
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_task::ExitStatus;

use self::workspace::execute_exec_workspace;

/// Execute `vp exec` command in the local CLI.
///
/// Resolves the workspace, selects packages (defaulting to the current package
/// when no flags are given), and executes the command in each selected package.
pub async fn execute(exec_args: ExecArgs, cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    // No command specified
    if exec_args.command.is_empty() {
        vite_shared::output::error(
            "'vp exec' requires a command to run\n\n\
             Usage: vp exec [--] <command> [args...]\n\n\
             Examples:\n\
             \x20 vp exec node --version\n\
             \x20 vp exec tsc --noEmit",
        );
        return Ok(ExitStatus(1));
    }

    execute_exec_workspace(exec_args, cwd).await
}
