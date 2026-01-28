//! Project scaffolding command (Category B).
//!
//! This command creates new projects using templates. It uses managed Node.js
//! from `vite_js_runtime` to execute the bundled JavaScript scaffolding scripts.
//!
//! The command supports:
//! - Builtin templates: vite:monorepo, vite:application, vite:library, vite:generator
//! - Remote templates: npm packages like create-vite, @tanstack/create-start
//! - GitHub templates: github:user/repo or full URLs
//! - Local templates: workspace packages or local directories

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the new command.
///
/// This delegates to the bundled JavaScript implementation which handles:
/// - Template discovery and resolution
/// - Interactive prompts for template selection
/// - Template execution (via package manager dlx, degit, or local execution)
/// - Post-processing (package name updates, workspace configuration)
///
/// # Arguments
/// * `cwd` - Current working directory
/// * `args` - All arguments passed to the command (template name, options, template args)
///
/// # Examples
///
/// ```text
/// vite new                              # Interactive mode
/// vite new vite:monorepo                # Create a monorepo
/// vite new create-vite                  # Use create-vite template
/// vite new create-vite -- --template react-ts  # Pass options to template
/// vite new --list                       # List available templates
/// vite new --help                       # Show help
/// ```
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);

    // Execute the bundled JS script with the "new" command
    // The JS script handles all argument parsing, template discovery, and execution
    executor.execute_cli_script("index.js", "new", args, &cwd).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
