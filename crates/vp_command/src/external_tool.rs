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
    /// [`is_managed_install`]), global packages, and the Vite+ fallback-bin
    /// directory, so that a managed install left on PATH never counts as the
    /// system tool. The bin directory is then searched rather than skipped:
    /// a configured bin can be shared with foreign tools, and its shims are
    /// rejected by identity instead.
    pub exclude_managed: bool,
}

/// Find `tool` on PATH outside Vite+ installations.
///
/// Skips the Vite+ bin directory, the bypass directories, shims from other
/// installations, and `self_exe`. Those would otherwise dispatch back into vp.
/// With [`ExternalToolQuery::exclude_managed`], also skips managed installs,
/// but keeps foreign tools in the bin directory eligible.
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

    // Filter PATH to exclude our bin directory (system-only lookups keep it
    // and reject its shims by identity below) and any bypass directories.
    // Relative entries are resolved against cwd because `which` reports
    // matches from them as relative paths, which `AbsolutePathBuf` rejects;
    // `~`-prefixed entries are left to `which`'s own tilde expansion.
    let mut filtered_paths: Vec<_> = std::env::split_paths(query.path)
        .filter(|p| {
            (query.exclude_managed || p != bin_dir)
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
            && !(query.exclude_managed
                && (is_managed_install(resolved.as_path())
                    || is_global_package(resolved.as_path())))
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

/// The first PATH match of a tool, classified by who owns it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FirstPathMatch {
    /// A tool outside Vite+.
    System(AbsolutePathBuf),
    /// A Vite+ shim, from this or another installation.
    ViteShim(AbsolutePathBuf),
    /// A Vite+-owned install: a managed runtime or package manager (see
    /// [`is_managed_install`]), or a global package.
    VpOwned(AbsolutePathBuf),
    /// No match on PATH.
    NotFound,
}

/// Classify the first match of `tool` on `path`.
///
/// Only the first match counts: entries after it are never searched, so the
/// caller decides what a shim or a Vite+-owned install means instead of
/// skipping it. A shim is a Windows trampoline, a Unix link to `vp`, or
/// `self_exe` (canonicalized), as for [`find_external_tool`].
/// `cwd` only resolves relative PATH entries.
pub fn classify_first_path_match(
    tool: &str,
    path: &OsStr,
    self_exe: Option<&Path>,
    cwd: &AbsolutePath,
) -> FirstPathMatch {
    let paths = std::env::split_paths(path).map(|path| {
        if path.is_absolute() || path.starts_with("~") { path } else { cwd.as_path().join(path) }
    });
    let Ok(path) = std::env::join_paths(paths) else {
        return FirstPathMatch::NotFound;
    };
    let Ok(resolved) = resolve_bin(tool, Some(&path), cwd) else {
        return FirstPathMatch::NotFound;
    };
    let canonical = resolved.as_path().canonicalize().ok();
    let is_unix_shim = cfg!(unix)
        && canonical
            .as_ref()
            .is_some_and(|target| target.file_name().is_some_and(|name| name == "vp"));
    if vp_shared::is_windows_trampoline(resolved.as_path())
        || is_unix_shim
        || (self_exe.is_some() && canonical.as_deref() == self_exe)
    {
        FirstPathMatch::ViteShim(resolved)
    } else if is_managed_install(resolved.as_path()) || is_global_package(resolved.as_path()) {
        FirstPathMatch::VpOwned(resolved)
    } else {
        FirstPathMatch::System(resolved)
    }
}

/// Whether `path` resolves into a Vite+-managed Node.js runtime or package
/// manager, that is, under `<DATA>/js_runtime` or `<DATA>/package_manager`.
pub fn is_managed_install(path: &Path) -> bool {
    resolves_under_data(path, &[Path::new("js_runtime"), Path::new("package_manager")])
}

/// Whether `path` resolves into a Vite+-managed Node.js runtime, that is,
/// under `<DATA>/js_runtime`. This includes the npm bundled with it.
pub fn is_managed_runtime(path: &Path) -> bool {
    resolves_under_data(path, &[Path::new("js_runtime")])
}

/// Whether `path` resolves into a Vite+-managed install of one package-manager
/// family, that is, under `<DATA>/package_manager/<family>`.
pub fn is_managed_package_manager(path: &Path, family: &str) -> bool {
    resolves_under_data(path, &[&Path::new("package_manager").join(family)])
}

/// Whether `path` resolves into a `vp install -g` package, that is, under
/// `<DATA>/packages` (the directory of `get_packages_dir` in
/// `vp_global_cli`). An old `corepack enable` can leave links from the bin
/// directory into a Corepack installed there.
fn is_global_package(path: &Path) -> bool {
    resolves_under_data(path, &[Path::new("packages")])
}

fn resolves_under_data(path: &Path, directories: &[&Path]) -> bool {
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
    fn exclude_managed_keeps_foreign_tools_in_bin() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bin = home.join("bin");
        let system = temp.path().join("system");
        let foreign_node = fake_executable(&bin, "node");
        let system_node = fake_executable(&system, "node");
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |config| {
            assert_eq!(config.dirs.bin.as_path(), bin);
            // Without the option, the bin directory is skipped as before.
            assert_eq!(find(&[&bin, &system], false, &cwd), Some(system_node.clone()));
            assert_eq!(find(&[&bin, &system], true, &cwd), Some(foreign_node.clone()));

            // A shim in the same directory is rejected by identity.
            std::fs::remove_file(&foreign_node).unwrap();
            #[cfg(unix)]
            {
                let vp = fake_executable(&home.join("current").join("bin"), "vp");
                std::os::unix::fs::symlink(&vp, bin.join("node")).unwrap();
            }
            #[cfg(windows)]
            {
                fake_executable(&bin, "node");
                config.dirs.write_shim_pointer("node").unwrap();
            }
            assert_eq!(find(&[&bin, &system], true, &cwd), Some(system_node.clone()));
            assert_eq!(find(&[&bin], true, &cwd), None);
        });
    }

    #[cfg(unix)]
    #[test]
    fn exclude_managed_skips_links_into_global_packages() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bin = home.join("bin");
        let system = temp.path().join("system");
        let package_node = fake_executable(
            &home.join("packages").join("corepack").join("lib").join("node_modules"),
            "node",
        );
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(&package_node, bin.join("node")).unwrap();
        let system_node = fake_executable(&system, "node");
        let cwd = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert_eq!(find(&[&bin, &system], true, &cwd), Some(system_node.clone()));
            assert_eq!(find(&[&bin, &system], false, &cwd), Some(system_node.clone()));
        });
    }

    fn first(path: &[&Path], self_exe: Option<&Path>, cwd: &AbsolutePath) -> FirstPathMatch {
        let path = std::env::join_paths(path).unwrap();
        classify_first_path_match("node", &path, self_exe, cwd)
    }

    fn absolute(path: &Path) -> AbsolutePathBuf {
        AbsolutePathBuf::new(path.to_path_buf()).unwrap()
    }

    /// Make `dir/node` a shim: a link to `vp` on Unix, a trampoline on Windows.
    fn make_shim(config: &EnvConfig, home: &Path, dir: &Path) -> PathBuf {
        #[cfg(unix)]
        {
            let _ = config;
            let vp = fake_executable(&home.join("current").join("bin"), "vp");
            std::fs::create_dir_all(dir).unwrap();
            let shim = dir.join("node");
            std::os::unix::fs::symlink(&vp, &shim).unwrap();
            shim
        }
        #[cfg(windows)]
        {
            let _ = home;
            let shim = fake_executable(dir, "node");
            config.dirs.write_shim_pointer_beside(&shim).unwrap();
            shim
        }
    }

    #[test]
    fn first_match_classifies_system_tools_and_managed_installs() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bin = home.join("bin");
        let runtime = home.join("js_runtime").join("node").join("24.0.0").join("bin");
        let pm = home.join("package_manager").join("npm").join("10.0.0").join("bin");
        let system = temp.path().join("system");
        let managed_node = fake_executable(&runtime, "node");
        let pm_node = fake_executable(&pm, "node");
        let system_node = fake_executable(&system, "node");
        let foreign_node = fake_executable(&bin, "node");
        let cwd = absolute(temp.path());

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert_eq!(
                first(&[&system], None, &cwd),
                FirstPathMatch::System(absolute(&system_node))
            );
            // A foreign tool in a shared bin directory is a system tool.
            assert_eq!(
                first(&[&bin, &system], None, &cwd),
                FirstPathMatch::System(absolute(&foreign_node))
            );
            // Managed installs are reported, not skipped.
            assert_eq!(
                first(&[&runtime, &system], None, &cwd),
                FirstPathMatch::VpOwned(absolute(&managed_node))
            );
            assert_eq!(
                first(&[&pm, &system], None, &cwd),
                FirstPathMatch::VpOwned(absolute(&pm_node))
            );
            assert_eq!(first(&[&temp.path().join("empty")], None, &cwd), FirstPathMatch::NotFound);
        });
    }

    #[test]
    fn first_match_stops_at_a_shim() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bin = home.join("bin");
        let fallback = home.join("fallback-bin");
        let system = temp.path().join("system");
        fake_executable(&system, "node");
        let cwd = absolute(temp.path());

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |config| {
            // A system tool after the shim is never searched.
            for dir in [&bin, &fallback] {
                let shim = make_shim(&config, &home, dir);
                assert_eq!(
                    first(&[dir, &system], None, &cwd),
                    FirstPathMatch::ViteShim(absolute(&shim))
                );
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn first_match_reports_links_into_global_packages() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let bin = home.join("bin");
        let system = temp.path().join("system");
        fake_executable(&system, "node");
        let package_node = fake_executable(
            &home.join("packages").join("corepack").join("lib").join("node_modules"),
            "node",
        );
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(&package_node, bin.join("node")).unwrap();
        let cwd = absolute(temp.path());

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert_eq!(
                first(&[&bin, &system], None, &cwd),
                FirstPathMatch::VpOwned(absolute(&bin.join("node")))
            );
        });
    }

    #[test]
    fn first_match_treats_self_exe_as_a_shim_and_resolves_relative_entries() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let relative = temp.path().join("relative");
        let node = fake_executable(&relative, "node");
        let cwd = absolute(temp.path());
        let canonical = node.canonicalize().unwrap();

        EnvConfig::with_vars([(env_vars::VP_HOME, home.as_path())], |_| {
            assert_eq!(
                first(&[Path::new("relative")], None, &cwd),
                FirstPathMatch::System(absolute(&node))
            );
            assert_eq!(
                first(&[&relative], Some(&canonical), &cwd),
                FirstPathMatch::ViteShim(absolute(&node))
            );
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
            assert!(is_managed_package_manager(&pnpm, "pnpm"));
            assert!(!is_managed_package_manager(&pnpm, "yarn"));
        });
    }
}
