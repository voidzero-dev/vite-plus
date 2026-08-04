//! Migration command (Category B: JavaScript Command).

use std::process::ExitStatus;

use vite_path::AbsolutePathBuf;

use crate::{error::Error, js_executor::JsExecutor};

/// Execute the `migrate` command by delegating to local or global vite-plus.
///
/// Routes through [`JsExecutor::delegate_migrate`], which escalates to the
/// global CLI when the project's local `vite-plus` is older than this global
/// `vp` (the upgrade scenario). Otherwise it keeps local-first semantics.
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    let mut executor = JsExecutor::new(None);
    let mut full_args = vec!["migrate".to_string()];
    full_args.extend(args.iter().cloned());
    executor.delegate_migrate(&cwd, &full_args).await
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_migrate_command_module_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
