//! Migration command (Category B: JS Script Command).

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the `migrate` command by delegating to the bundled JavaScript implementation.
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);

    // Execute the bundled JS script with the "migrate" command
    // The JS script handles all migration logic
    executor.execute_cli_script("index.js", "migrate", args, &cwd).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_migrate_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
