//! Local CLI delegation (Category C).
//!
//! These commands delegate to the local `vite-plus` package installed in the
//! project's `node_modules`. Uses managed Node.js from `vite_js_runtime` with
//! the project's `devEngines.runtime` configuration.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute a delegated command.
///
/// Delegates the command to the local `vite-plus` CLI installed in the
/// project's `node_modules`. Uses the project's Node.js version from
/// `devEngines.runtime` in package.json.
///
/// # Arguments
/// * `cwd` - Current working directory (project root)
/// * `command` - Command name (e.g., "dev", "build", "test")
/// * `args` - Additional arguments to pass to the command
pub async fn execute(
    cwd: AbsolutePathBuf,
    command: &str,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);

    // Build the full argument list for the local CLI
    let mut full_args = vec![command.to_string()];
    full_args.extend(args.iter().cloned());

    // Delegate to the local CLI
    executor.delegate_to_local_cli(&cwd, &full_args).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_delegate_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
