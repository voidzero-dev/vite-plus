//! Windows-specific: when a vp-managed package-manager `.cmd` shim has a
//! sibling `.ps1`, rewrite the spawn to go through
//! `powershell.exe -File <sibling.ps1>`.
//!
//! Running a `.cmd` from any shell makes `cmd.exe` prompt "Terminate batch
//! job (Y/N)?" on Ctrl+C, which leaves the terminal corrupt. Routing through
//! `PowerShell` sidesteps the prompt and lets Ctrl+C propagate cleanly.
//!
//! The rewrite is intentionally **scoped to paths inside `$VP_HOME`**
//! (`~/.vite-plus` by default). vp's managed shims live there:
//!   - `$VP_HOME/js_runtime/node/<ver>/{npm,npx}.cmd` (npm/npx shipped with
//!     the managed Node.js),
//!   - `$VP_HOME/package_manager/<pm>/<ver>/<pm>/bin/<pm>.cmd` (downloaded
//!     pnpm/yarn/bun).
//!
//! Anything outside `$VP_HOME` — system tools, globally-installed npm
//! shims, third-party CLIs whose `.cmd` and `.ps1` wrappers may diverge —
//! keeps its existing `.cmd` path (Ctrl+C corruption included), so we
//! don't silently change execution semantics for unrelated commands or
//! bypass execution policies on locked-down hosts.
//!
//! The task-layer rewrite (`vite_task_plan::ps1_shim`) is scoped
//! differently — to `node_modules/.bin/*.cmd` inside the workspace — and
//! covers `vp run <script>`. The two scopes don't overlap.
//!
//! See <https://github.com/voidzero-dev/vite-plus/issues/1489>
//! and <https://github.com/voidzero-dev/vite-plus/issues/1176>.

use std::{ffi::OsString, sync::Arc};

use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_powershell::{POWERSHELL_PREFIX, find_ps1_sibling, powershell_host};

/// Rewrite a vp-managed `.cmd` invocation to go through PowerShell.
///
/// Returns `Some((powershell_host, prefix_args))` when the rewrite applies.
/// `prefix_args` is `["-NoProfile", "-NoLogo", "-ExecutionPolicy", "Bypass",
/// "-File", <abs ps1 path>]`; callers prepend it to the user args and spawn
/// `powershell_host`.
///
/// Returns `None` when:
/// - not on Windows,
/// - no PowerShell host (`pwsh.exe` or `powershell.exe`) is on PATH,
/// - `$VP_HOME` could not be resolved,
/// - the resolved path is **outside** `$VP_HOME`,
/// - the resolved path is not a `.cmd` (case-insensitive),
/// - the `.cmd` has no sibling `.ps1`.
#[must_use]
pub fn rewrite_cmd_to_powershell(
    resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    let vp_home = vp_home()?;
    let host = powershell_host()?;
    rewrite_in_scope(resolved, vp_home, host)
}

/// Cached `$VP_HOME` (`~/.vite-plus` by default; overridable via env var).
/// `None` only if `vite_shared::get_vp_home()` failed to resolve a home —
/// in that case we conservatively skip the rewrite rather than retarget
/// arbitrary PATH commands.
fn vp_home() -> Option<&'static AbsolutePathBuf> {
    use std::sync::LazyLock;

    static VP_HOME: LazyLock<Option<AbsolutePathBuf>> =
        LazyLock::new(|| vite_shared::get_vp_home().ok());
    VP_HOME.as_ref()
}

/// Pure rewrite logic. Factored out so tests can drive it on any platform
/// without depending on a real `powershell.exe` or a real `$VP_HOME`.
fn rewrite_in_scope(
    resolved: &AbsolutePath,
    vp_home: &AbsolutePath,
    host: &Arc<AbsolutePath>,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    if !resolved.as_path().starts_with(vp_home.as_path()) {
        return None;
    }
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
    fn rewrites_cmd_inside_vp_home_to_powershell() {
        let dir = tempdir().unwrap();
        let vp_home = abs(dir.path().canonicalize().unwrap());
        // Mimic the real layout: $VP_HOME/js_runtime/node/<ver>/npm.cmd.
        let bin_dir = vp_home.as_path().join("js_runtime").join("node").join("24.0.0");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("npm.cmd"), "").unwrap();
        fs::write(bin_dir.join("npm.ps1"), "").unwrap();

        let host = host_arc(&vp_home);
        let resolved = abs(bin_dir.join("npm.cmd"));

        let (program, prefix_args) =
            rewrite_in_scope(&resolved, &vp_home, &host).expect("should rewrite");

        assert_eq!(program.as_path(), host.as_path());
        let as_strs: Vec<&str> = prefix_args.iter().filter_map(|a| a.to_str()).collect();
        let ps1_path = bin_dir.join("npm.ps1");
        let ps1_str = ps1_path.to_str().unwrap();
        assert_eq!(
            as_strs,
            vec!["-NoProfile", "-NoLogo", "-ExecutionPolicy", "Bypass", "-File", ps1_str]
        );
    }

    /// Regression for the Codex review: the rewrite must NOT retarget
    /// `.cmd` files that live outside `$VP_HOME` even when a sibling
    /// `.ps1` exists. Otherwise unrelated PATH commands (system tools,
    /// globally installed npm wrappers, third-party CLIs whose `.cmd`
    /// and `.ps1` wrappers diverge) would silently switch to PowerShell
    /// with `-ExecutionPolicy Bypass`.
    #[test]
    fn returns_none_for_cmd_outside_vp_home() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        let vp_home_path = root.as_path().join("vite-plus");
        fs::create_dir_all(&vp_home_path).unwrap();
        let vp_home = abs(vp_home_path);

        // A `.cmd`+`.ps1` pair *outside* vp_home — e.g. a global install at
        // `%AppData%\npm\node_modules\.bin\foo.cmd` or any third-party tool.
        let outside_bin = root.as_path().join("global").join("bin");
        fs::create_dir_all(&outside_bin).unwrap();
        fs::write(outside_bin.join("foo.cmd"), "").unwrap();
        fs::write(outside_bin.join("foo.ps1"), "").unwrap();

        let host = host_arc(&root);
        let resolved = abs(outside_bin.join("foo.cmd"));

        assert!(
            rewrite_in_scope(&resolved, &vp_home, &host).is_none(),
            "rewrite must stay hands-off for .cmd outside $VP_HOME"
        );
    }

    #[test]
    fn returns_none_when_no_ps1_sibling() {
        let dir = tempdir().unwrap();
        let vp_home = abs(dir.path().canonicalize().unwrap());
        fs::write(vp_home.as_path().join("npm.cmd"), "").unwrap();

        let host = host_arc(&vp_home);
        let resolved = abs(vp_home.as_path().join("npm.cmd"));

        assert!(rewrite_in_scope(&resolved, &vp_home, &host).is_none());
    }
}
