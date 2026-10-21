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
}

/// Find `tool` on PATH outside Vite+ installations.
///
/// Skips the Vite+ bin directory, the bypass directories, shims from other
/// installations, and `self_exe`. Those would otherwise dispatch back into vp.
/// `cwd` only resolves relative PATH entries.
pub fn find_external_tool(
    tool: &str,
    query: &ExternalToolQuery<'_>,
    cwd: &AbsolutePath,
) -> Option<AbsolutePathBuf> {
    let env = vp_shared::EnvConfig::get();
    let bin_dir = env.dirs.bin.as_path();
    tracing::debug!("path_var: {:?}", query.path);
    tracing::debug!("bypass_paths: {:?}", query.bypass);

    // Filter PATH to exclude our bin directory and any bypass directories.
    // Relative entries are resolved against cwd because `which` reports
    // matches from them as relative paths, which `AbsolutePathBuf` rejects;
    // `~`-prefixed entries are left to `which`'s own tilde expansion.
    let mut filtered_paths: Vec<_> = std::env::split_paths(query.path)
        .filter(|p| p != bin_dir && !query.bypass.iter().any(|bp| p == bp))
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
    let Ok(target) = path.canonicalize() else {
        return false;
    };
    let data = &vp_shared::EnvConfig::get().dirs.data;
    ["js_runtime", "package_manager"].iter().any(|directory| {
        data.join(directory).as_path().canonicalize().is_ok_and(|root| target.starts_with(root))
    })
}
