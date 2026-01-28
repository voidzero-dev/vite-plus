//! Project scaffolding command (Category B).
//!
//! This command creates new projects using templates. It uses managed Node.js
//! from `vite_js_runtime` to execute JavaScript-based templates.

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the new command.
///
/// # Arguments
/// * `cwd` - Current working directory
/// * `template` - Optional template name (e.g., "vite:monorepo", "create-vite")
/// * `name` - Optional project name or directory
/// * `args` - Additional arguments to pass to the template
pub async fn execute(
    _cwd: AbsolutePathBuf,
    template: Option<String>,
    name: Option<String>,
    args: &[String],
) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);

    // Build args for the JS script
    let mut script_args = Vec::new();
    if let Some(t) = &template {
        script_args.push(t.clone());
    }
    if let Some(n) = &name {
        script_args.push(n.clone());
    }
    script_args.extend(args.iter().cloned());

    // Execute the bundled JS script
    // The script handles template resolution and project creation
    executor.execute_cli_script("index.js", "new", &script_args).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_new_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
