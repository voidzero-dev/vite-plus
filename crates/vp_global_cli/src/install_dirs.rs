//! Ownership of directories that `vp implode` may remove recursively.

use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use vp_shared::VpDirs;
use vt_path::AbsolutePathBuf;

use crate::error::Error;

pub(crate) const OWNERSHIP_MARKER: &str = ".vp-owned";
const OWNERSHIP_CONTENT: &[u8] = b"vite-plus-owned-directory\n";

pub(crate) fn lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

/// Reject parent traversal when lexical normalization would change the directory target.
pub(crate) fn validate_root_normalization(dirs: &VpDirs) -> Result<(), Error> {
    for root in [&dirs.data, &dirs.cache, &dirs.config, &dirs.state] {
        let mut prefix = PathBuf::new();
        for component in root.as_path().components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    validate_parent_traversal(root, &prefix)?;
                    prefix.pop();
                }
                _ => prefix.push(component.as_os_str()),
            }
        }
    }
    Ok(())
}

fn validate_parent_traversal(root: &AbsolutePathBuf, prefix: &Path) -> Result<(), Error> {
    let Some(parent) = prefix.parent() else {
        return Ok(());
    };
    match fs::symlink_metadata(prefix) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    }

    let resolved = dunce::canonicalize(prefix)?;
    let expected_parent = dunce::canonicalize(parent)?;
    if resolved.parent() != Some(expected_parent.as_path()) {
        return Err(Error::Other(vt_str::format!(
            "Refusing to normalize directory {}: resolving '..' across a link changes its target. Specify the directory without '..'.",
            root.as_path().display()
        )));
    }
    Ok(())
}

/// Distinct data, cache, config, and state roots, including nested categories.
pub(crate) fn roots(dirs: &VpDirs) -> Vec<AbsolutePathBuf> {
    let mut roots: Vec<_> = [&dirs.data, &dirs.cache, &dirs.config, &dirs.state]
        .into_iter()
        .map(|root| {
            AbsolutePathBuf::new(lexical_path(root.as_path()))
                .expect("resolved Vite+ roots remain absolute after lexical normalization")
        })
        .collect();
    roots.sort_by(|a, b| a.as_path().cmp(b.as_path()));
    roots.dedup();
    roots
}

pub(crate) fn is_owned(root: &AbsolutePathBuf) -> Result<bool, Error> {
    match fs::symlink_metadata(root.join(OWNERSHIP_MARKER)) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Claim only empty or newly created roots. Installing into an unowned, nonempty
/// directory must never grant permission to delete its existing contents.
pub(crate) fn prepare(dirs: &VpDirs) -> Result<(), Error> {
    validate_root_normalization(dirs)?;
    let mut empty_roots = Vec::new();
    // Preflight all roots before creating directories or ownership markers.
    for root in roots(dirs) {
        if is_owned(&root)? {
            continue;
        }
        let is_empty = match fs::read_dir(&root) {
            Ok(mut entries) => entries.next().transpose()?.is_none(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };
        if is_empty {
            empty_roots.push(root);
        }
    }
    for root in empty_roots {
        // An earlier root may have created the marker through a filesystem alias.
        if is_owned(&root)? {
            continue;
        }
        fs::create_dir_all(&root)?;
        let mut marker = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(OWNERSHIP_MARKER))?;
        marker.write_all(OWNERSHIP_CONTENT)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use vp_shared::EnvConfig;

    use super::*;

    #[test]
    fn roots_normalize_category_paths_without_discarding_nested_roots() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            dirs.data = env.dirs.data.join("data");
            dirs.cache = dirs.data.join("../cache");
            dirs.config = dirs.data.join("./config");
            dirs.state = dirs.data.join("../data");
            assert_eq!(
                roots(&dirs),
                vec![env.dirs.data.join("cache"), dirs.data.clone(), dirs.data.join("config")]
            );
        });
    }

    #[test]
    fn prepare_claims_empty_roots_and_preserves_shared_bin() {
        for split in [false, true] {
            EnvConfig::scoped(|env| {
                let mut dirs = env.dirs.clone();
                if split {
                    dirs.data = env.dirs.data.join("data");
                    dirs.cache = env.dirs.data.join("cache");
                    dirs.config = env.dirs.data.join("config");
                    dirs.state = env.dirs.data.join("state");
                    fs::create_dir_all(&dirs.bin).unwrap();
                    fs::write(dirs.bin.join("other-tool"), b"keep").unwrap();
                }
                prepare(&dirs).unwrap();
                for root in roots(&dirs) {
                    assert!(is_owned(&root).unwrap());
                }
                fs::write(dirs.data.join("payload"), b"vp").unwrap();
                prepare(&dirs).unwrap();
                assert_eq!(fs::read(dirs.data.join("payload")).unwrap(), b"vp");
                assert!(!dirs.bin.join(OWNERSHIP_MARKER).as_path().exists());
                if split {
                    assert_eq!(fs::read(dirs.bin.join("other-tool")).unwrap(), b"keep");
                }
            });
        }
    }

    #[cfg(unix)]
    #[test]
    fn prepare_accepts_aliased_empty_roots() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            dirs.data = env.dirs.data.join("data");
            dirs.cache = env.dirs.data.join("cache");
            dirs.config = dirs.data.clone();
            dirs.state = dirs.data.clone();
            fs::create_dir(&dirs.data).unwrap();
            std::os::unix::fs::symlink(&dirs.data, &dirs.cache).unwrap();

            prepare(&dirs).unwrap();

            assert!(is_owned(&dirs.data).unwrap());
            assert!(is_owned(&dirs.cache).unwrap());
        });
    }

    #[test]
    fn prepare_preflights_all_roots_and_never_claims_existing_contents() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            dirs.data = env.dirs.data.join("a-data");
            dirs.config = env.dirs.data.join("b-config");
            dirs.state = env.dirs.data.join("c-state");
            dirs.cache = env.dirs.data.join("z-cache");
            fs::create_dir_all(&env.dirs.data).unwrap();
            fs::write(&dirs.cache, b"not a directory").unwrap();
            assert!(prepare(&dirs).is_err());
            for root in [&dirs.data, &dirs.config, &dirs.state] {
                assert!(!root.as_path().exists());
            }
            fs::remove_file(&dirs.cache).unwrap();
            fs::create_dir_all(&dirs.cache).unwrap();
            fs::write(dirs.cache.join("personal.txt"), b"keep").unwrap();
            prepare(&dirs).unwrap();
            for root in [&dirs.data, &dirs.config, &dirs.state] {
                assert!(is_owned(root).unwrap());
            }
            assert!(!dirs.cache.join(OWNERSHIP_MARKER).as_path().exists());
            assert_eq!(fs::read(dirs.cache.join("personal.txt")).unwrap(), b"keep");
            assert!(!is_owned(&dirs.cache).unwrap());
        });
    }

    #[test]
    fn prepare_allows_parent_components_in_existing_and_missing_directories() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            fs::create_dir(env.dirs.data.join("existing")).unwrap();
            dirs.data = env.dirs.data.join("existing/../data");
            dirs.cache = env.dirs.data.join("missing/../cache");
            dirs.config = dirs.data.clone();
            dirs.state = dirs.data.clone();

            prepare(&dirs).unwrap();

            assert!(is_owned(&env.dirs.data.join("data")).unwrap());
            assert!(is_owned(&env.dirs.data.join("cache")).unwrap());
            assert!(!env.dirs.data.join("missing").as_path().exists());
        });
    }

    #[cfg(unix)]
    #[test]
    fn prepare_rejects_changed_parent_before_creating_any_roots() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            let personal = env.dirs.data.join("personal");
            fs::create_dir_all(personal.join("child")).unwrap();
            fs::write(personal.join("notes.txt"), b"keep").unwrap();
            let route = env.dirs.data.join("route");
            std::os::unix::fs::symlink(personal.join("child"), &route).unwrap();
            dirs.data = env.dirs.data.join("new-data");
            dirs.cache = route.join("../cache");
            dirs.config = env.dirs.data.join("new-config");
            dirs.state = env.dirs.data.join("new-state");

            let error = prepare(&dirs).unwrap_err();

            assert!(error.to_string().contains("Refusing to normalize directory"));
            for root in roots(&dirs) {
                assert!(!root.as_path().exists());
            }
            assert_eq!(fs::read(personal.join("notes.txt")).unwrap(), b"keep");
            assert!(!personal.join(OWNERSHIP_MARKER).as_path().exists());
        });
    }

    #[cfg(unix)]
    #[test]
    fn validate_root_normalization_allows_links_that_preserve_parent_traversal() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            let target = env.dirs.data.join("target");
            fs::create_dir_all(target.join("child")).unwrap();
            let alias = env.dirs.data.join("alias");
            std::os::unix::fs::symlink(&target, &alias).unwrap();
            for cache in [alias.clone(), alias.join("child/.."), alias.join("../new-cache")] {
                dirs.cache = cache;
                validate_root_normalization(&dirs).unwrap();
            }
        });
    }

    #[cfg(unix)]
    #[test]
    fn validate_root_normalization_rejects_dangling_links_before_parent_traversal() {
        EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            let alias = env.dirs.data.join("alias");
            std::os::unix::fs::symlink(env.dirs.data.join("missing"), &alias).unwrap();
            dirs.cache = alias.join("../cache");

            assert!(validate_root_normalization(&dirs).is_err());
        });
    }

    #[test]
    fn ownership_requires_a_regular_marker_file() {
        EnvConfig::scoped(|env| {
            prepare(&env.dirs).unwrap();
            let marker = env.dirs.cache.join(OWNERSHIP_MARKER);
            fs::remove_file(&marker).unwrap();
            assert!(!is_owned(&env.dirs.cache).unwrap());
            fs::create_dir(&marker).unwrap();
            assert!(!is_owned(&env.dirs.cache).unwrap());
            fs::remove_dir(&marker).unwrap();
            #[cfg(unix)]
            {
                let target = env.dirs.data.join("elsewhere");
                fs::write(&target, OWNERSHIP_CONTENT).unwrap();
                std::os::unix::fs::symlink(&target, &marker).unwrap();
                assert!(!is_owned(&env.dirs.cache).unwrap());
                fs::remove_file(&marker).unwrap();
            }
            fs::write(&marker, b"").unwrap();
            assert!(is_owned(&env.dirs.cache).unwrap());
        });
    }
}
