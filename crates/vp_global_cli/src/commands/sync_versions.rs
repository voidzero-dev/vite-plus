//! Side-effect-free dependency version reconciliation for external automation.

use std::{path::PathBuf, process::ExitStatus};

use tokio::process::Command;
use vp_shared::env_vars;
use vt_path::AbsolutePathBuf;

use crate::error::Error;

fn packaged_sidecar(executable: &std::path::Path) -> Option<PathBuf> {
    let path = executable.parent()?.join("sync-versions").join("bin.mjs");
    path.is_file().then_some(path)
}

/// Execute the planner bundled next to the standalone `vp` binary.
///
/// Normal Vite+ installations keep JavaScript under `node_modules`, so they
/// fall back to the global package entrypoint. Official standalone archives
/// include this one self-contained bundle and need no npm installation.
pub async fn execute(
    cwd: AbsolutePathBuf,
    args: &[String],
    raw_subcommand: Option<&str>,
) -> Result<ExitStatus, Error> {
    let executable = std::env::current_exe()?;
    let executable = std::fs::canonicalize(executable)?;
    let Some(sidecar) = packaged_sidecar(&executable) else {
        return super::delegate::execute_global(cwd, "sync-versions", args, raw_subcommand).await;
    };

    let mut command = Command::new("node");
    command.arg(sidecar).args(args).current_dir(cwd.as_path()).env(env_vars::VP_BYPASS, "1");
    vp_command::sync_child_pwd(&mut command, &cwd);
    Ok(command.status().await?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::packaged_sidecar;

    #[test]
    fn finds_only_the_packaged_sync_versions_entrypoint() {
        let temp = tempdir().expect("temp directory");
        let executable = temp.path().join("vp");
        fs::write(&executable, []).expect("placeholder executable");

        assert_eq!(packaged_sidecar(&executable), None);

        let sidecar = temp.path().join("sync-versions/bin.mjs");
        fs::create_dir_all(sidecar.parent().expect("sidecar parent")).expect("sidecar directory");
        fs::write(&sidecar, []).expect("sidecar file");

        assert_eq!(packaged_sidecar(&executable), Some(sidecar));
    }
}
