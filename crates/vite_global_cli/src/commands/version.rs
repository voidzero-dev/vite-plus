//! Version command (Category B).
//!
//! This command displays version information by delegating to the bundled
//! JavaScript scripts. This ensures the version displayed matches the
//! JS-based CLI and includes all relevant package versions.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the version command.
///
/// This delegates to the bundled JavaScript implementation which displays:
/// - Vite+ version
/// - Node.js version being used
/// - Any other relevant version information
pub async fn execute(cwd: AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);

    // Execute the bundled JS script with "--version" flag
    // The JS index.js checks for "--version" or "-V" as the first argument
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
