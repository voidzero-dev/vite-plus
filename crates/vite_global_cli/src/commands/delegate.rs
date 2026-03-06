//! JavaScript command delegation — resolves local vite-plus first, falls back to global.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute a command by delegating to the local `vite-plus` CLI.
pub async fn execute(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_to_local_cli(&cwd, &full_args).await
}

/// Execute a command by delegating to the global `vite-plus` CLI.
pub async fn execute_global(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_to_global_cli(&cwd, &full_args).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_delegate_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
