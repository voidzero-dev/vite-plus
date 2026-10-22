use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::resolve_bin;

/// Inputs for [`find_external_tool`].
///
/// The caller supplies every process-dependent value, so the search does not
/// assume that `current_exe()` is a Vite+ binary. In a NAPI module, it is the
/// Node.js executable.
#[derive(Debug, Clone, Copy)]
pub struct ExternalToolQuery<'a> {
    /// PATH-style list of directories to search.
    pub path: &'a OsStr,
    /// Additional directories to skip, such as the `VP_BYPASS` entries.
    pub bypass: &'a [PathBuf],
    /// Canonicalized executable to exclude, or `None` to exclude no executable.
    pub self_exe: Option<&'a Path>,
    /// Also skip Vite+-managed runtimes and package managers (see
    /// [`is_managed_install`]) and the Vite+ fallback-bin directory, so that a
    /// managed install left on PATH never counts as the system tool.
    pub exclude_managed: bool,
}

/// Find `tool` on PATH outside Vite+ installations.
///
/// Skips the Vite+ bin directory, the bypass directories, shims from other
/// installations, and `self_exe`. Those would otherwise dispatch back into vp.
/// With [`ExternalToolQuery::exclude_managed`], also skips managed installs.
/// `cwd` only resolves relative PATH entries.
pub fn find_external_tool(
    tool: &str,
    query: &ExternalToolQuery<'_>,
    cwd: &AbsolutePath,
) -> Option<AbsolutePathBuf> {
    let env = vp_shared::EnvConfig::get();
    let bin_dir = env.dirs.bin.as_path();
    let fallback_bin = query.exclude_managed.then(|| env.dirs.fallback_bin());
    tracing::debug!("path_var: {:?}", query.path);
    tracing::debug!("bypass_paths: {:?}", query.bypass);

    // Filter PATH to exclude our bin directory and any bypass directories.
    // Relative entries are resolved against cwd because `which` reports
    // matches from them as relative paths, which `AbsolutePathBuf` rejects;
    // `~`-prefixed entries are left to `which`'s own tilde expansion.
    let mut filtered_paths: Vec<_> = std::env::split_paths(query.path)
        .filter(|p| {
            p != bin_dir
                && !query.bypass.iter().any(|bp| p == bp)
                && fallback_bin.as_ref().is_none_or(|fallback| p != fallback.as_path())
        })
        .map(|p| if p.is_absolute() || p.starts_with("~") { p } else { cwd.as_path().join(p) })
        .collect();

    loop {
        // Use resolve_bin with filtered PATH - stops at first match
        let search_path = std::env::join_paths(&filtered_paths).ok()?;
        let resolved = resolve_bin(tool, Some(&search_path), cwd).ok()?;
        let canonical = resolved.as_path().canonicalize().ok();
        // A sibling `vp` may itself be a shim sharing an external manager's executable.
        let is_unix_shim = cfg!(unix)
            && canonical
                .as_ref()
                .is_some_and(|target| target.file_name().is_some_and(|name| name == "vp"));
        if !vp_shared::is_windows_trampoline(resolved.as_path())
            && !is_unix_shim
            && (query.self_exe.is_none() || canonical.as_deref() != query.self_exe)
            && !(query.exclude_managed && is_managed_install(resolved.as_path()))
        {
            return Some(resolved);
        }
        // Canonicalize both sides of the comparison so symlink-aliased PATH
        // entries still match the resolved parent.
        let self_dir = resolved.as_path().parent()?.to_path_buf();
        let canonical_self_dir = self_dir.canonicalize().ok();
        let len_before = filtered_paths.len();
        filtered_paths.retain(|p| {
            *p != self_dir
                && (canonical_self_dir.is_none() || p.canonicalize().ok() != canonical_self_dir)
        });
        if filtered_paths.len() == len_before {
            // Nothing left to drop; give up rather than loop forever.
            return None;
        }
    }
}

/// Whether `path` resolves into a Vite+-managed Node.js runtime or package
/// manager, that is, under `<DATA>/js_runtime` or `<DATA>/package_manager`.
pub fn is_managed_install(path: &Path) -> bool {
    resolves_under_data(path, &["js_runtime", "package_manager"])
}

/// Whether `path` resolves into a Vite+-managed Node.js runtime, that is,
/// under `<DATA>/js_runtime`. This includes the npm bundled with it.
pub fn is_managed_runtime(path: &Path) -> bool {
    resolves_under_data(path, &["js_runtime"])
}

fn resolves_under_data(path: &Path, directories: &[&str]) -> bool {
    let Ok(target) = path.canonicalize() else {
        return false;
    };
    let data = &vp_shared::EnvConfig::get().dirs.data;
    directories.iter().any(|directory| {
        data.join(directory).as_path().canonicalize().is_ok_and(|root| target.starts_with(root))
    })
}

#[cfg(test)]
mod tests {
    use vp_shared::{EnvConfig, env_vars};

    use super::*;

    fn fake_executable(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        #[cfg(windows)]
        let path = dir.join(format!("{name}.exe"));
        #[cfg(not(windows))]
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    fn find(path: &[&Path], exclude_managed: bool, cwd: &AbsolutePath) -> Option<PathBuf> {
        let path = std::env::join_paths(path).unwrap();
        let query = ExternalToolQuery { path: &path, bypass: &[], self_exe: None, exclude_managed };
        find_external_tool("node", &query, cwd).map(|path| path.as_path().to_path_buf())
    }

    #[test]
    fn exclude_managed_skips_managed_installs_and_fallback_bin() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let runtime = home.join("js_runtime").join("node").join("24.0.0").join("bin");
        let fallback = home.join("fallback-bin");
        let system = temp.path().join("system");
        let managed_node = fake_executable(&runtime, "node");
        let fallback_node = fake_executable(&fallback, "node");
        let system_node = fake_executable(&system, "node");
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert!(is_managed_install(&managed_node));
            assert!(is_managed_runtime(&managed_node));
            assert!(!is_managed_install(&system_node));

            // Without the option, managed installs on PATH are reported as before.
            assert_eq!(find(&[&runtime, &system], false, &cwd), Some(managed_node.clone()));
            assert_eq!(find(&[&fallback, &system], false, &cwd), Some(fallback_node.clone()));

            assert_eq!(find(&[&runtime, &system], true, &cwd), Some(system_node.clone()));
            assert_eq!(find(&[&fallback, &system], true, &cwd), Some(system_node.clone()));
            assert_eq!(find(&[&runtime, &fallback], true, &cwd), None);
        });
    }

    #[test]
    fn managed_runtime_excludes_package_managers() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let pnpm = fake_executable(
            &home.join("package_manager").join("pnpm").join("10.0.0").join("pnpm").join("bin"),
            "pnpm",
        );
        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert!(is_managed_install(&pnpm));
            assert!(!is_managed_runtime(&pnpm));
        });
    }
}
