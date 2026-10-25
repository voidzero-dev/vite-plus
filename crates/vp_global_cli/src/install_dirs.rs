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
    let mut empty = Vec::new();
    // Preflight all roots before creating directories or ownership markers.
    for root in roots(dirs) {
        if is_owned(&root)? {
            continue;
        }
        let nonempty = match fs::read_dir(&root) {
            Ok(mut entries) => entries.next().transpose()?.is_some(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if nonempty {
            continue;
        }
        empty.push(root);
    }
    for root in empty {
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
