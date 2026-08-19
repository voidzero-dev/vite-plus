//! On Windows, route a managed package-manager `.cmd` shim through
//! `powershell.exe -File <sibling.ps1>` when the sibling `.ps1` file exists.
//!
//! When a shell runs a `.cmd` file, Ctrl+C makes `cmd.exe` show a termination
//! prompt. This prompt can damage the terminal state. PowerShell does not show
//! the prompt and passes Ctrl+C to the child process.
//!
//! The rewrite is scoped to two patterns:
//!   - Managed shims in the Vite+ data root (`<DATA>`):
//!     - `<DATA>/js_runtime/node/<ver>/{npm,npx}.cmd`,
//!     - `<DATA>/package_manager/<pm>/<ver>/<pm>/bin/<pm>.cmd`.
//!   - Each `<...>/node_modules/.bin/*.cmd` shim. npm, pnpm, and Yarn use this
//!     standard layout. `cmd-shim` writes equivalent `.cmd` and `.ps1` files.
//!
//! Keep the `.cmd` path for files outside these patterns. This includes system
//! tools and third-party CLIs with different `.cmd` and `.ps1` behavior. This
//! rule keeps the execution behavior of unrelated commands. It also obeys host
//! execution policies.
//!
//! Do not rewrite when standard input is not a terminal. The pnpm, npm, and
//! Yarn `.ps1` wrappers inspect standard input. For example, they use
//! `$MyInvocation.ExpectingInput`. They can stop responding when input is a
//! pipe or null. In this case, there is no terminal that the Ctrl+C prompt can
//! damage. Use the `.cmd` file.
//!
//! See <https://github.com/voidzero-dev/vite-plus/issues/1489>
//! and <https://github.com/voidzero-dev/vite-plus/issues/1176>.

use std::ffi::OsString;

use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_powershell::{POWERSHELL_PREFIX, find_ps1_sibling, is_stdin_terminal, powershell_host};

/// Rewrite a vp-managed `.cmd` invocation to go through `PowerShell`.
///
/// Return `Some((powershell_host, prefix_args))` when the rewrite applies.
/// `prefix_args` contains `-NoProfile`, `-NoLogo`, `-ExecutionPolicy`,
/// `Bypass`, `-File`, and the absolute `.ps1` path. Add these arguments before
/// the user arguments. Then start `powershell_host`.
///
/// Returns `None` when:
/// - not on Windows,
/// - no `PowerShell` host (`pwsh.exe` or `powershell.exe`) is on PATH,
/// - standard input is not a terminal,
/// - the resolved path is not in the Vite+ data root or a
///   `node_modules/.bin/` directory,
/// - the resolved path is not a `.cmd` (case-insensitive),
/// - the `.cmd` has no sibling `.ps1`.
#[must_use]
pub fn rewrite_cmd_to_powershell(
    resolved: &AbsolutePath,
) -> Option<(AbsolutePathBuf, Vec<OsString>)> {
    // `build_command` gives its standard input to child processes. Thus, a TTY
    // here is also a TTY in the child. The `vt_powershell` crate shares
    // `is_stdin_terminal` with `vt_plan::ps1_shim`.
    let host = powershell_host()?;
    // Vite+ managed shims are under the data root (`<DATA>/js_runtime/…`,
    // `<DATA>/package_manager/…`).
    let config = vp_shared::EnvConfig::get();
    rewrite_in_scope(resolved, Some(config.dirs.data.as_absolute_path()), host, is_stdin_terminal())
}

/// Apply the rewrite without external state. Tests can call this function on
/// each platform without a real `powershell.exe` or Vite+ data root.
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

/// Return `true` when `resolved` is `<...>/node_modules/.bin/<file>`. Compare
/// the `.bin` and `node_modules` components without case sensitivity. Windows
/// is not case-sensitive, and pnpm hoisted layouts can use different case.
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
    /// when stdin is piped or null (CI, snapshot tests, scripted invocations).
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

    /// When no vp data root participates in the scope check (`None` here),
    /// the `node_modules/.bin` scope must still rewrite, since it is
    /// architecturally independent from the data-root scope.
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
