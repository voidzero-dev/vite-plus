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
//! check there is too narrow for this code path. If a `.ps1` sibling exists,
//! the same tool that emitted the `.cmd` (npm cmd-shim, pnpm, yarn) emitted
//! the `.ps1` with equivalent semantics.
//!
//! See <https://github.com/voidzero-dev/vite-plus/issues/1489>
//! and <https://github.com/voidzero-dev/vite-plus/issues/1176>.

use std::ffi::OsString;

use vite_path::{AbsolutePath, AbsolutePathBuf};

/// Fixed arguments prepended before the `.ps1` path. `-NoProfile`/`-NoLogo`
/// skip user profile loading; `-ExecutionPolicy Bypass` allows running the
/// unsigned shims that npm/pnpm/yarn install.
#[cfg(any(windows, test))]
const POWERSHELL_PREFIX: &[&str] =
    &["-NoProfile", "-NoLogo", "-ExecutionPolicy", "Bypass", "-File"];

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
#[cfg(windows)]
#[must_use]
pub fn rewrite_cmd_to_powershell(
    resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    let host = powershell_host()?;
    rewrite_with_host(resolved, host)
}

#[cfg(not(windows))]
#[must_use]
pub const fn rewrite_cmd_to_powershell(
    _resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    None
}

/// Cached location of the PowerShell host. Prefers cross-platform `pwsh.exe`
/// when present, falling back to the Windows built-in `powershell.exe`.
#[cfg(windows)]
fn powershell_host() -> Option<&'static AbsolutePathBuf> {
    use std::sync::LazyLock;

    static POWERSHELL_HOST: LazyLock<Option<AbsolutePathBuf>> = LazyLock::new(|| {
        let resolved = which::which("pwsh.exe").or_else(|_| which::which("powershell.exe")).ok()?;
        AbsolutePathBuf::new(resolved)
    });
    POWERSHELL_HOST.as_ref()
}

/// Pure rewrite logic. Factored out so tests can drive it on any platform
/// without depending on a real `powershell.exe`.
#[cfg(any(windows, test))]
fn rewrite_with_host(
    resolved: &AbsolutePath,
    host: &AbsolutePathBuf,
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

    Some((host.clone(), prefix_args))
}

#[cfg(any(windows, test))]
fn find_ps1_sibling(resolved: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let ext = resolved.as_path().extension().and_then(|e| e.to_str())?;
    if !ext.eq_ignore_ascii_case("cmd") {
        return None;
    }

    let ps1 = resolved.with_extension("ps1");
    if !ps1.as_path().is_file() {
        return None;
    }

    Some(ps1)
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

    #[test]
    fn rewrites_cmd_to_powershell_with_sibling_ps1() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("npm.cmd"), "").unwrap();
        fs::write(root.as_path().join("npm.ps1"), "").unwrap();

        let host = abs(root.as_path().join("powershell.exe"));
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
    fn rewrites_uppercase_cmd_extension() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("pnpm.CMD"), "").unwrap();
        fs::write(root.as_path().join("pnpm.ps1"), "").unwrap();

        let host = abs(root.as_path().join("powershell.exe"));
        let resolved = abs(root.as_path().join("pnpm.CMD"));

        let result = rewrite_with_host(&resolved, &host);
        assert!(result.is_some(), "case-insensitive .CMD should still rewrite");
    }

    #[test]
    fn returns_none_when_no_ps1_sibling() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("npm.cmd"), "").unwrap();

        let host = abs(root.as_path().join("powershell.exe"));
        let resolved = abs(root.as_path().join("npm.cmd"));

        assert!(rewrite_with_host(&resolved, &host).is_none());
    }

    #[test]
    fn returns_none_for_exe() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("bun.exe"), "").unwrap();
        fs::write(root.as_path().join("bun.ps1"), "").unwrap();

        let host = abs(root.as_path().join("powershell.exe"));
        let resolved = abs(root.as_path().join("bun.exe"));

        assert!(rewrite_with_host(&resolved, &host).is_none());
    }

    #[test]
    fn returns_none_for_no_extension() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        fs::write(root.as_path().join("node"), "").unwrap();
        fs::write(root.as_path().join("node.ps1"), "").unwrap();

        let host = abs(root.as_path().join("powershell.exe"));
        let resolved = abs(root.as_path().join("node"));

        assert!(rewrite_with_host(&resolved, &host).is_none());
    }
}
