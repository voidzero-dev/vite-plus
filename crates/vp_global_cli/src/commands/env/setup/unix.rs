//! Keep external shims linked through the package manager's public entrypoint.

use std::path::{Path, PathBuf};

use vp_shared::EnvConfig;

pub(super) fn external_shim_target(binary: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(binary).ok()?;
    let env = EnvConfig::get();
    let bin = env.dirs.bin.as_path();
    let fallback_bin = env.dirs.fallback_bin();
    let cwd = vt_path::current_dir().ok()?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut candidates: Vec<_> = std::env::split_paths(&path).map(|dir| dir.join("vp")).collect();
    // An explicit invocation need not be on PATH. vpx/vpr use the sibling vp.
    if let Some(invoked) = std::env::args_os().next().map(PathBuf::from)
        && let Some(parent) = invoked.parent().filter(|parent| !parent.as_os_str().is_empty())
    {
        candidates.push(parent.join("vp"));
    }
    // Retain a previously selected entrypoint when only the user shims are on PATH.
    if let Ok(target) = std::fs::read_link(bin.join("vp")) {
        candidates.push(bin.join(target));
    }
    // Preserve the supplied path when it is itself an external entrypoint.
    candidates.push(binary.to_path_buf());
    candidates.into_iter().map(|path| cwd.as_path().join(path)).find(|candidate| {
        candidate != &canonical
            && std::fs::canonicalize(candidate).is_ok_and(|target| target == canonical)
            && !passes_through_shims(candidate, bin)
            && !passes_through_shims(candidate, fallback_bin.as_path())
    })
}

// Canonical equality alone would accept aliases back to our own vp/node shims,
// creating a cycle as soon as setup replaces them. Check every link in the chain.
pub(super) fn passes_through_shims(candidate: &Path, bin: &Path) -> bool {
    let bin = std::fs::canonicalize(bin).unwrap_or_else(|_| bin.to_path_buf());
    let mut path = candidate.to_path_buf();
    for _ in 0..40 {
        let Some(parent) = path.parent() else { return true };
        if std::fs::canonicalize(parent).is_ok_and(|parent| parent.starts_with(&bin)) {
            return true;
        }
        match std::fs::read_link(&path) {
            Ok(target) => path = parent.join(target),
            Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => return false,
            Err(_) => return true,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn prefers_public_entrypoint_and_keeps_it_without_path() {
        EnvConfig::scoped(|env| {
            let root = tempfile::tempdir().unwrap();
            let binary = root.path().join("package/vp");
            let public = root.path().join("bin/vp");
            std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
            std::fs::create_dir_all(public.parent().unwrap()).unwrap();
            std::fs::create_dir_all(&env.dirs.bin).unwrap();
            std::fs::write(&binary, b"vp").unwrap();
            let binary = std::fs::canonicalize(binary).unwrap();
            symlink("../package/vp", &public).unwrap();
            symlink(&binary, env.dirs.bin.join("vp")).unwrap();
            let path =
                std::env::join_paths([env.dirs.bin.as_path(), public.parent().unwrap()]).unwrap();
            EnvConfig::with_vars([("PATH", Some(path))], |_| {
                assert_eq!(external_shim_target(&binary), Some(public.clone()));
            });
            std::fs::remove_file(env.dirs.bin.join("vp")).unwrap();
            symlink(&public, env.dirs.bin.join("vp")).unwrap();
            EnvConfig::with_vars([("PATH", None::<&str>)], |_| {
                assert_eq!(external_shim_target(&binary), Some(public));
            });
        });
    }

    #[test]
    fn rejects_aliases_through_user_shims_and_unrelated_binaries() {
        EnvConfig::scoped(|env| {
            let root = tempfile::tempdir().unwrap();
            let binary = root.path().join("vp");
            let alias = root.path().join("alias");
            let foreign = root.path().join("foreign");
            std::fs::create_dir_all(&alias).unwrap();
            std::fs::create_dir_all(&foreign).unwrap();
            std::fs::create_dir_all(&env.dirs.bin).unwrap();
            std::fs::write(&binary, b"vp").unwrap();
            let binary = std::fs::canonicalize(binary).unwrap();
            std::fs::write(foreign.join("vp"), b"another vp").unwrap();
            symlink(&binary, env.dirs.bin.join("node")).unwrap();
            symlink(env.dirs.bin.join("node"), alias.join("vp")).unwrap();
            let directory_alias = root.path().join("directory-alias");
            symlink(&env.dirs.bin, &directory_alias).unwrap();
            symlink(&binary, env.dirs.bin.join("vp")).unwrap();
            let path = std::env::join_paths([alias, directory_alias, foreign]).unwrap();
            EnvConfig::with_vars([("PATH", Some(path))], |_| {
                assert_eq!(external_shim_target(&binary), None);
            });
        });
    }
}
