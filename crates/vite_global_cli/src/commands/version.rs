//! Version command (Category B: JS Script Command).

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the `--version` command by delegating to the bundled JavaScript implementation.
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);
    executor.execute_cli_script("index.js", "--version", &[], &cwd).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_version_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
