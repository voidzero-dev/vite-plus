//! Windows-specific: when a vp-managed package-manager `.cmd` shim has a
//! sibling `.ps1`, rewrite the spawn to go through
//! `powershell.exe -File <sibling.ps1>`.
//!
//! Running a `.cmd` from any shell makes `cmd.exe` prompt "Terminate batch
//! job (Y/N)?" on Ctrl+C, which leaves the terminal corrupt. Routing through
//! `PowerShell` sidesteps the prompt and lets Ctrl+C propagate cleanly.
//!
//! The rewrite is scoped to two patterns:
//!   - Inside `$VP_HOME` (`~/.vite-plus` by default) — vp's managed shims:
//!     - `$VP_HOME/js_runtime/node/<ver>/{npm,npx}.cmd`,
//!     - `$VP_HOME/package_manager/<pm>/<ver>/<pm>/bin/<pm>.cmd`.
//!   - Any `<...>/node_modules/.bin/*.cmd` — the canonical layout for
//!     npm/pnpm/yarn-emitted shims (cmd-shim writes both `.cmd` and `.ps1`
//!     so the wrappers stay equivalent).
//!
//! Anything outside both patterns — system tools, third-party CLIs whose
//! `.cmd` and `.ps1` wrappers may diverge — keeps its existing `.cmd`
//! path (Ctrl+C corruption included), so we don't silently change
//! execution semantics for unrelated commands or bypass execution
//! policies on locked-down hosts.
//!
//! The rewrite is also skipped when stdin is not a terminal. The
//! `pnpm`/`npm`/`yarn` `.ps1` wrappers introspect stdin (e.g.
//! `$MyInvocation.ExpectingInput`) and hang when stdin is piped or
//! null; in that environment there is no terminal to corrupt with the
//! Ctrl+C prompt anyway, so falling back to `.cmd` is strictly safer.
//!
//! See <https://github.com/voidzero-dev/vite-plus/issues/1489>
//! and <https://github.com/voidzero-dev/vite-plus/issues/1176>.

use std::ffi::OsString;

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
/// - stdin is not a terminal (the `.ps1` wrappers hang on piped/null
///   stdin and the Ctrl+C concern doesn't apply without a TTY),
/// - the resolved path is outside `$VP_HOME` (or `$VP_HOME` is
///   unresolvable) AND not under any `node_modules/.bin/`,
/// - the resolved path is not a `.cmd` (case-insensitive),
/// - the `.cmd` has no sibling `.ps1`.
#[must_use]
pub fn rewrite_cmd_to_powershell(
    resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    let host = powershell_host()?;
    rewrite_in_scope(resolved, vp_home().map(AsRef::as_ref), host, is_stdin_terminal())
}

/// Cached `stdin.is_terminal()`. The TTY-ness of the parent's stdin
/// is fixed for the process lifetime, and `build_command` always
/// inherits stdin into spawned children — so a TTY here means a TTY
/// in the child too.
fn is_stdin_terminal() -> bool {
    use std::{io::IsTerminal, sync::LazyLock};

    static IS_TTY: LazyLock<bool> = LazyLock::new(|| std::io::stdin().is_terminal());
    *IS_TTY
}

/// Cached `$VP_HOME` (`~/.vite-plus` by default; overridable via env var).
/// Returns `None` if `vite_shared::get_vp_home()` failed; the rewrite still
/// applies to `node_modules/.bin/*.cmd` paths in that case (the two scopes
/// are independent).
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
    vp_home: Option<&AbsolutePath>,
    host: &AbsolutePath,
    is_interactive: bool,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    if !is_interactive {
        return None;
    }
    if !is_in_managed_scope(resolved, vp_home) {
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

fn is_in_managed_scope(resolved: &AbsolutePath, vp_home: Option<&AbsolutePath>) -> bool {
    let in_vp_home = vp_home.is_some_and(|home| resolved.as_path().starts_with(home.as_path()));
    in_vp_home || is_in_node_modules_bin(resolved)
}

/// `true` when `resolved` is `<...>/node_modules/.bin/<file>` (matched
/// case-insensitively on the `.bin`/`node_modules` components — Windows
/// is case-insensitive, and pnpm's hoisted layouts can vary in casing).
fn is_in_node_modules_bin(resolved: &AbsolutePath) -> bool {
    let mut parents = resolved.as_path().components().rev();
    parents.next(); // shim filename
    let Some(bin) = parents.next() else { return false };
    if !bin.as_os_str().eq_ignore_ascii_case(".bin") {
        return false;
    }
    let Some(node_modules) = parents.next() else { return false };
    node_modules.as_os_str().eq_ignore_ascii_case("node_modules")
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

    fn host_buf(root: &AbsolutePath) -> AbsolutePathBuf {
        abs(root.as_path().join("powershell.exe"))
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

        let host = host_buf(&vp_home);
        let resolved = abs(bin_dir.join("npm.cmd"));

        let (program, prefix_args) =
            rewrite_in_scope(&resolved, Some(&vp_home), &host, true).expect("should rewrite");

        assert_eq!(program.as_path(), host.as_path());
        let as_strs: Vec<&str> = prefix_args.iter().filter_map(|a| a.to_str()).collect();
        let ps1_path = bin_dir.join("npm.ps1");
        let ps1_str = ps1_path.to_str().unwrap();
        assert_eq!(
            as_strs,
            vec!["-NoProfile", "-NoLogo", "-ExecutionPolicy", "Bypass", "-File", ps1_str]
        );
    }

    /// Any `<...>/node_modules/.bin/*.cmd` rewrites, regardless of where
    /// the project root sits — covers single-package projects, hoisted
    /// monorepos, and globally-installed shims uniformly.
    #[test]
    fn rewrites_cmd_in_node_modules_bin() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        // vp_home points elsewhere — this scope is the node_modules path.
        let vp_home_path = root.as_path().join("vite-plus");
        fs::create_dir_all(&vp_home_path).unwrap();
        let vp_home = abs(vp_home_path);

        let bin = root.as_path().join("my-project").join("node_modules").join(".bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("vite.cmd"), "").unwrap();
        fs::write(bin.join("vite.ps1"), "").unwrap();

        let host = host_buf(&root);
        let resolved = abs(bin.join("vite.cmd"));

        let result = rewrite_in_scope(&resolved, Some(&vp_home), &host, true);
        assert!(result.is_some(), "any node_modules/.bin/*.cmd must rewrite");
    }

    /// `pnpm`/`npm`/`yarn` `.ps1` wrappers introspect stdin and hang
    /// when stdin is piped or null (CI, snap tests, scripted invocations).
    /// In that environment the rewrite is unwanted; the spawn falls back
    /// to `.cmd` directly.
    #[test]
    fn skips_rewrite_when_not_interactive() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        let bin = root.as_path().join("my-project").join("node_modules").join(".bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("vite.cmd"), "").unwrap();
        fs::write(bin.join("vite.ps1"), "").unwrap();

        let host = host_buf(&root);
        let resolved = abs(bin.join("vite.cmd"));

        assert!(
            rewrite_in_scope(&resolved, None, &host, false).is_none(),
            "non-interactive spawns must not be rewritten through PowerShell"
        );
    }

    /// `vp_home` may be unresolvable in unusual environments (CI containers
    /// missing $HOME, sandboxed shells); when that happens the
    /// `node_modules/.bin` scope must still rewrite, since it is
    /// architecturally independent from the `$VP_HOME` scope.
    #[test]
    fn rewrites_cmd_in_node_modules_bin_when_vp_home_unresolved() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        let bin = root.as_path().join("my-project").join("node_modules").join(".bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("vite.cmd"), "").unwrap();
        fs::write(bin.join("vite.ps1"), "").unwrap();

        let host = host_buf(&root);
        let resolved = abs(bin.join("vite.cmd"));

        assert!(
            rewrite_in_scope(&resolved, None, &host, true).is_some(),
            "node_modules/.bin must rewrite even without a resolvable vp_home"
        );
    }

    /// The `.bin`/`node_modules` component check is case-insensitive so
    /// a `.CMD` shim under `Node_Modules\.Bin\` (or any casing variant)
    /// still matches.
    #[test]
    fn rewrites_cmd_in_node_modules_bin_case_insensitive() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        let vp_home = abs(root.as_path().join("vite-plus"));
        fs::create_dir_all(vp_home.as_path()).unwrap();

        let bin = root.as_path().join("Node_Modules").join(".Bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("vite.cmd"), "").unwrap();
        fs::write(bin.join("vite.ps1"), "").unwrap();

        let host = host_buf(&root);
        let resolved = abs(bin.join("vite.cmd"));

        assert!(rewrite_in_scope(&resolved, Some(&vp_home), &host, true).is_some());
    }

    /// A `.cmd`+`.ps1` pair outside `$VP_HOME` AND outside any
    /// `node_modules/.bin/` (e.g. a system tool living at `<root>/global/bin/foo.cmd`)
    /// must NOT be retargeted.
    #[test]
    fn returns_none_for_cmd_outside_managed_scope() {
        let dir = tempdir().unwrap();
        let root = abs(dir.path().canonicalize().unwrap());
        let vp_home_path = root.as_path().join("vite-plus");
        fs::create_dir_all(&vp_home_path).unwrap();
        let vp_home = abs(vp_home_path);

        let outside_bin = root.as_path().join("global").join("bin");
        fs::create_dir_all(&outside_bin).unwrap();
        fs::write(outside_bin.join("foo.cmd"), "").unwrap();
        fs::write(outside_bin.join("foo.ps1"), "").unwrap();

        let host = host_buf(&root);
        let resolved = abs(outside_bin.join("foo.cmd"));

        assert!(
            rewrite_in_scope(&resolved, Some(&vp_home), &host, true).is_none(),
            "rewrite must stay hands-off for .cmd outside both vp_home and node_modules/.bin"
        );
    }

    #[test]
    fn returns_none_when_no_ps1_sibling() {
        let dir = tempdir().unwrap();
        let vp_home = abs(dir.path().canonicalize().unwrap());
        fs::write(vp_home.as_path().join("npm.cmd"), "").unwrap();

        let host = host_buf(&vp_home);
        let resolved = abs(vp_home.as_path().join("npm.cmd"));

        assert!(rewrite_in_scope(&resolved, Some(&vp_home), &host, true).is_none());
    }
}
