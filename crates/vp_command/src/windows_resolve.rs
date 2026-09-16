//! Resolve Windows executable candidates using their on-disk filename casing.
//!
//! `which` 8 checks whether a PATHEXT candidate exists before correcting its
//! casing. In a case-sensitive directory, `.CMD` therefore misses pnpm's `.cmd`
//! shim. Correct each candidate before checking it, preserving PATH/PATHEXT order.

use std::{
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use vt_path::AbsolutePath;
use which::sys::{RealSys, Sys};

#[cfg(windows)]
pub(super) fn resolve(bin: &str, paths: &OsStr, cwd: &AbsolutePath) -> Option<PathBuf> {
    resolve_with_extensions(bin, paths, cwd, &RealSys.env_windows_path_ext())
}

fn resolve_with_extensions(
    bin: &str,
    paths: &OsStr,
    cwd: &AbsolutePath,
    extensions: &[String],
) -> Option<PathBuf> {
    let bin = Path::new(bin);
    if bin.is_absolute() || bin.components().count() > 1 {
        return resolve_path(&cwd.as_path().join(bin), extensions);
    }

    std::env::split_paths(paths).filter(|dir| !dir.as_os_str().is_empty()).find_map(|dir| {
        let mut components = dir.components();
        let dir = if components.next() == Some(Component::Normal(OsStr::new("~"))) {
            RealSys.home_dir().map_or_else(|| dir.clone(), |home| home.join(components.as_path()))
        } else {
            dir
        };
        resolve_path(&cwd.as_path().join(dir).join(bin), extensions)
    })
}

fn resolve_path(path: &Path, extensions: &[String]) -> Option<PathBuf> {
    let parent = path.parent()?;
    let name = path.file_name()?;
    // One listing for all extensions. If enumeration is unavailable, retain
    // direct lookup so searchable directories do not require list permission.
    let names = std::fs::read_dir(parent)
        .and_then(|entries| {
            entries
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<std::io::Result<Vec<_>>>()
        })
        .ok();
    let has_extension = path.extension().is_some_and(|ext| {
        extensions.iter().any(|candidate| {
            candidate.strip_prefix('.').is_some_and(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
    });
    let candidates = std::iter::once(name.to_owned()).chain(
        extensions.iter().take(if has_extension { 0 } else { extensions.len() }).map(|ext| {
            let mut candidate = name.to_owned();
            candidate.push(ext);
            candidate
        }),
    );
    for candidate in candidates {
        // Retain native lookup when ASCII matching finds nothing: an ordinary
        // Windows directory may also equate non-ASCII filename casing.
        let actual =
            names.as_ref().and_then(|names| match_name(names, &candidate)).unwrap_or(&candidate);
        let path = parent.join(actual);
        // Match which's Windows validation, including executable files without
        // an extension and application execution aliases (reparse points).
        if std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_file() || meta.is_symlink())
            && (path.extension().is_some() || RealSys.is_valid_executable(&path).unwrap_or(false))
        {
            return Some(path);
        }
    }
    None
}

fn match_name<'a>(names: &'a [OsString], candidate: &OsStr) -> Option<&'a OsStr> {
    names
        .iter()
        .find(|name| *name == candidate)
        .or_else(|| names.iter().find(|name| name.eq_ignore_ascii_case(candidate)))
        .map(OsString::as_os_str)
}

#[cfg(test)]
mod tests {
    use vt_path::AbsolutePathBuf;

    use super::*;

    fn resolve_in(
        bin: &str,
        dirs: &[PathBuf],
        cwd: &AbsolutePath,
        ext: &[&str],
    ) -> Option<PathBuf> {
        resolve_with_extensions(
            bin,
            &std::env::join_paths(dirs).unwrap(),
            cwd,
            &ext.iter().map(|ext| (*ext).to_owned()).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn resolves_shim_casing_before_trying_later_path_directories() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        let local = cwd.as_path().join("local");
        let global = cwd.as_path().join("global");
        std::fs::create_dir(&local).unwrap();
        std::fs::create_dir(&global).unwrap();
        std::fs::write(local.join("astro.cmd"), "").unwrap();
        std::fs::write(global.join("astro.EXE"), "").unwrap();
        assert_eq!(
            resolve_in("astro", &[local.clone(), global], &cwd, &[".EXE", ".CMD"]),
            Some(local.join("astro.cmd"))
        );
    }

    #[test]
    fn preserves_pathext_order_and_matches_mixed_case() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::write(temp.path().join("AsTrO.cMd"), "").unwrap();
        std::fs::write(temp.path().join("astro.BAT"), "").unwrap();
        let dirs = [cwd.as_path().to_owned()];
        assert_eq!(
            resolve_in("astro", &dirs, &cwd, &[".CMD", ".BAT"]),
            Some(cwd.as_path().join("AsTrO.cMd"))
        );
        assert_eq!(
            resolve_in("astro", &dirs, &cwd, &[".BAT", ".CMD"]),
            Some(cwd.as_path().join("astro.BAT"))
        );
    }

    #[test]
    fn prefers_exact_spelling_when_both_casings_exist() {
        let names = [OsString::from("astro.cmd"), OsString::from("astro.CMD")];
        assert_eq!(match_name(&names, OsStr::new("astro.CMD")), Some(OsStr::new("astro.CMD")));
        assert_eq!(match_name(&names, OsStr::new("astro.cmd")), Some(OsStr::new("astro.cmd")));
    }

    #[test]
    fn resolves_relative_path_entries_against_task_cwd_and_skips_missing_entries() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(temp.path().join("bin")).unwrap();
        std::fs::write(temp.path().join("bin/tool.cmd"), "").unwrap();
        assert_eq!(
            resolve_in("tool", &["missing".into(), "./bin".into()], &cwd, &[".CMD"]),
            Some(cwd.as_path().join("./bin/tool.cmd"))
        );
    }

    #[test]
    fn resolves_explicit_paths_without_searching_path() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(temp.path().join("bin")).unwrap();
        let actual = cwd.as_path().join("bin/tool.cmd");
        std::fs::write(&actual, "").unwrap();
        for bin in ["bin/tool", "bin/tool.CMD", actual.to_str().unwrap()] {
            assert_eq!(resolve_in(bin, &[], &cwd, &[".CMD"]), Some(actual.clone()));
        }
    }

    #[test]
    fn skips_directories_and_does_not_append_to_a_recognized_extension() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(temp.path().join("tool.EXE")).unwrap();
        std::fs::write(temp.path().join("tool.cmd"), "").unwrap();
        std::fs::write(temp.path().join("tool.EXE.cmd"), "").unwrap();
        let dirs = [cwd.as_path().to_owned()];
        assert_eq!(
            resolve_in("tool", &dirs, &cwd, &[".EXE", ".CMD"]),
            Some(cwd.as_path().join("tool.cmd"))
        );
        assert_eq!(resolve_in("tool.EXE", &dirs, &cwd, &[".EXE", ".CMD"]), None);
    }

    #[test]
    fn respects_empty_path_and_pathext() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::write(temp.path().join("tool.cmd"), "").unwrap();
        assert_eq!(resolve_in("tool", &[PathBuf::new()], &cwd, &[".CMD"]), None);
        let dirs = [cwd.as_path().to_owned()];
        assert_eq!(resolve_in("tool", &dirs, &cwd, &[]), None);
        assert_eq!(resolve_in("tool.cmd", &dirs, &cwd, &[]), Some(cwd.as_path().join("tool.cmd")));
    }

    #[test]
    fn tries_unrecognized_extensions_verbatim_before_appending_pathext() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::write(temp.path().join("tool.js"), "").unwrap();
        std::fs::write(temp.path().join("other.js.cmd"), "").unwrap();
        let dirs = [cwd.as_path().to_owned()];
        assert_eq!(
            resolve_in("tool.js", &dirs, &cwd, &[".CMD"]),
            Some(cwd.as_path().join("tool.js"))
        );
        assert_eq!(
            resolve_in("other.js", &dirs, &cwd, &[".CMD"]),
            Some(cwd.as_path().join("other.js.cmd"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn retains_native_non_ascii_case_matching() {
        let temp = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp.path().canonicalize().unwrap()).unwrap();
        std::fs::write(temp.path().join("über.cmd"), "").unwrap();
        let result = resolve_in("ÜBER", &[cwd.as_path().to_owned()], &cwd, &[".CMD"]);
        assert!(result.unwrap().is_file());
    }
}
