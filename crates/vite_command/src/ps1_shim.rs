//! Windows-specific: when a resolved binary is a `.cmd` shim with a sibling
//! `.ps1`, rewrite the spawn to go through `powershell.exe -File <sibling.ps1>`.
//!
//! Running a `.cmd` from any shell makes `cmd.exe` prompt "Terminate batch
//! job (Y/N)?" on Ctrl+C, which leaves the terminal corrupt. Routing through
//! `PowerShell` sidesteps the prompt and lets Ctrl+C propagate cleanly.
//!
//! Unlike the task-layer rewrite (`vite_task_plan::ps1_shim`, scoped to
//! `node_modules/.bin/*.cmd` inside the workspace), this one applies to any
//! `.cmd` whose `.ps1` sibling exists. Package manager shims (`npm.cmd`,
//! `pnpm.cmd`, `yarn.cmd`, `npx.cmd`) live in `~/.vite-plus/js_runtime/...`
//! or system PATH — outside any `node_modules/.bin` — so the structural
//! check there is too narrow for this code path.
//!
//! The cross-platform primitives (`POWERSHELL_PREFIX`, `powershell_host`,
//! `find_ps1_sibling`) live in the `vite_powershell` crate and are shared
//! with `vite_task_plan::ps1_shim`. This module composes them with vp's
//! own conventions: absolute `.ps1` path in args (no cache fingerprint to
//! keep portable) and `Vec<OsString>` return type (matches the spawn API).
//!
//! See <https://github.com/voidzero-dev/vite-plus/issues/1489>
//! and <https://github.com/voidzero-dev/vite-plus/issues/1176>.

use std::{ffi::OsString, sync::Arc};

use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_powershell::{POWERSHELL_PREFIX, find_ps1_sibling, powershell_host};

/// Rewrite a resolved `.cmd` invocation to go through PowerShell.
///
/// Returns `Some((powershell_host, prefix_args))` when the rewrite applies,
/// where `prefix_args` is `["-NoProfile", "-NoLogo", "-ExecutionPolicy",
/// "Bypass", "-File", <abs ps1 path>]`. Caller prepends `prefix_args` to the
/// user args and spawns `powershell_host`.
///
/// Returns `None` when:
/// - not on Windows,
/// - no PowerShell host (`pwsh.exe` or `powershell.exe`) is on PATH,
/// - the resolved path is not a `.cmd` (case-insensitive),
/// - the `.cmd` has no sibling `.ps1`.
#[must_use]
pub fn rewrite_cmd_to_powershell(
    resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    let host = powershell_host()?;
    rewrite_with_host(resolved, host)
}

/// Pure rewrite logic. Factored out so tests can drive it on any platform
/// without depending on a real `powershell.exe`.
fn rewrite_with_host(
    resolved: &AbsolutePath,
    host: &Arc<AbsolutePath>,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    let ps1 = find_ps1_sibling(resolved)?;

    tracing::debug!(
        "rewriting .cmd to powershell: {} -> {} -File {}",
        resolved.as_path().display(),
        host.as_path().display(),
        ps1.as_path().display(),
    );

    let mut prefix_args: Vec<OsString> =
        POWERSHELL_PREFIX.iter().copied().map(OsString::from).collect();
    prefix_args.push(ps1.as_path().as_os_str().to_owned());

    Some((host.to_absolute_path_buf(), prefix_args))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[expect(clippy::disallowed_types, reason = "tempdir bridges std PathBuf into AbsolutePath")]
    fn abs(buf: std::path::PathBuf) -> AbsolutePathBuf {
        AbsolutePathBuf::new(buf).unwrap()
    }

    fn host_arc(root: &AbsolutePath) -> Arc<AbsolutePath> {
        Arc::from(abs(root.as_path().join("powershell.exe")))
    }

    #[test]
    fn rewrites_cmd_to_powershell_with_sibling_ps1() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("npm.cmd"), "").unwrap();
        fs::write(root.as_path().join("npm.ps1"), "").unwrap();

        let host = host_arc(&root);
        let resolved = abs(root.as_path().join("npm.cmd"));

        let (program, prefix_args) = rewrite_with_host(&resolved, &host).expect("should rewrite");

        assert_eq!(program.as_path(), host.as_path());
        let as_strs: Vec<&str> = prefix_args.iter().filter_map(|a| a.to_str()).collect();
        let ps1_path = root.as_path().join("npm.ps1");
        let ps1_str = ps1_path.to_str().unwrap();
        assert_eq!(
            as_strs,
            vec!["-NoProfile", "-NoLogo", "-ExecutionPolicy", "Bypass", "-File", ps1_str]
        );
    }

    #[test]
    fn returns_none_when_no_ps1_sibling() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("npm.cmd"), "").unwrap();

        let host = host_arc(&root);
        let resolved = abs(root.as_path().join("npm.cmd"));

        assert!(rewrite_with_host(&resolved, &host).is_none());
    }
}
