//! Select removal units without granting ownership to a legacy installation's parent.

use std::{fs, path::Path};

use vp_setup::{
    install::is_install_dir_for_version,
    lockfile::{INSTALL_LOCKFILE, INSTALL_RECEIPT, matches_install_receipt},
};
use vp_shared::{VP_BINARY_NAME, VpDirs};
use vt_path::AbsolutePathBuf;

use crate::{error::Error, install_dirs};

pub(super) struct RemovalPlan {
    pub directories: Vec<AbsolutePathBuf>,
    pub current_link: Option<AbsolutePathBuf>,
    pub binary_targets: Vec<AbsolutePathBuf>,
    pub preserved_roots: Vec<AbsolutePathBuf>,
}

impl RemovalPlan {
    pub fn new(dirs: &VpDirs, roots: &[AbsolutePathBuf]) -> Result<Self, Error> {
        install_dirs::validate_root_normalization(dirs)?;
        let mut owned_roots = Vec::new();
        let mut unowned_roots = Vec::new();
        for root in roots {
            match fs::symlink_metadata(root) {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            }
            if install_dirs::is_owned(root)? {
                owned_roots.push(root.clone());
            } else {
                let canonical = match dunce::canonicalize(root) {
                    Ok(path) => path,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        root.as_path().to_path_buf()
                    }
                    Err(error) => return Err(error.into()),
                };
                unowned_roots.push((root.clone(), canonical));
            }
        }
        // Recursive removal must preserve unmarked categories, including aliases.
        let contains_unowned_root = |directory: &AbsolutePathBuf, canonical: &Path| {
            unowned_roots.iter().any(|(path, boundary)| {
                path.as_path().starts_with(directory.as_path()) || boundary.starts_with(canonical)
            })
        };
        let mut plan = Self {
            directories: Vec::new(),
            current_link: None,
            binary_targets: vec![
                AbsolutePathBuf::new(dunce::canonicalize(std::env::current_exe()?)?)
                    .ok_or(Error::CliBinaryNotFound)?,
            ],
            preserved_roots: Vec::new(),
        };
        if install_dirs::is_owned(&dirs.data)? {
            plan.binary_targets.push(dirs.data.join("current/bin").join(VP_BINARY_NAME));
        }
        for root in owned_roots {
            let canonical = dunce::canonicalize(&root)?;
            if !contains_unowned_root(&root, &canonical) {
                plan.directories.push(root);
            }
        }
        let current = dirs.data.join("current");
        let current_target = fs::symlink_metadata(&current)
            .ok()
            .filter(|metadata| metadata.is_symlink())
            .and_then(|_| dunce::canonicalize(&current).ok());
        if !covered(&dirs.data, &plan.directories) {
            match fs::read_dir(&dirs.data) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry?;
                        if !entry.file_type()?.is_dir() {
                            continue;
                        }
                        let version_dir = dirs.data.join(entry.file_name());
                        if !verified_version(&version_dir)? {
                            continue;
                        }
                        plan.binary_targets.push(version_dir.join("bin").join(VP_BINARY_NAME));
                        let canonical = dunce::canonicalize(&version_dir)?;
                        if contains_unowned_root(&version_dir, &canonical) {
                            continue;
                        }
                        if current_target.as_ref() == Some(&canonical) {
                            plan.current_link = Some(current.clone());
                            plan.binary_targets.push(current.join("bin").join(VP_BINARY_NAME));
                        }
                        plan.directories.push(version_dir);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        plan.directories.sort();
        plan.directories.dedup_by(|child, parent| child.as_path().starts_with(parent.as_path()));
        plan.preserved_roots =
            roots.iter().filter(|root| !covered(root, &plan.directories)).cloned().collect();
        plan.preserved_roots
            .dedup_by(|child, parent| child.as_path().starts_with(parent.as_path()));
        Ok(plan)
    }
}

fn covered(path: &AbsolutePathBuf, directories: &[AbsolutePathBuf]) -> bool {
    directories.iter().any(|directory| path.as_path().starts_with(directory.as_path()))
}

/// A usable executable or familiar layout alone is not an installation receipt.
fn verified_version(directory: &AbsolutePathBuf) -> Result<bool, Error> {
    let Some(name) = directory.as_path().file_name().and_then(|name| name.to_str()) else {
        return Ok(false);
    };
    if js_semver::Version::parse(name).is_err() {
        return Ok(false);
    }
    let canonical = dunce::canonicalize(directory)?;
    let Some(receipt) = read_contained_file(&canonical, INSTALL_RECEIPT)? else {
        return Ok(false);
    };
    let Some(lockfile) =
        read_contained_file(&canonical, &vt_str::format!("bin/{INSTALL_LOCKFILE}"))?
    else {
        return Ok(false);
    };
    if !matches_install_receipt(&lockfile, &receipt) {
        return Ok(false);
    }
    let Some(wrapper) = read_contained_file(&canonical, "package.json")? else {
        return Ok(false);
    };
    let Some(package) = read_contained_file(&canonical, "node_modules/vite-plus/package.json")?
    else {
        return Ok(false);
    };
    let (Ok(wrapper), Ok(package)) = (
        serde_json::from_slice::<serde_json::Value>(&wrapper),
        serde_json::from_slice::<serde_json::Value>(&package),
    ) else {
        return Ok(false);
    };
    let Some(version) = wrapper["version"].as_str() else { return Ok(false) };
    Ok(js_semver::Version::parse(version).is_ok()
        && is_install_dir_for_version(name, version)
        && wrapper["name"] == "vp-global"
        && wrapper["private"] == true
        && wrapper["dependencies"]["vite-plus"] == version
        && package["name"] == "vite-plus"
        && package["version"] == version)
}

// pnpm's package link may stay inside the version directory, but evidence from outside cannot authorize deletion.
fn read_contained_file(directory: &Path, relative: &str) -> Result<Option<Vec<u8>>, Error> {
    let path = directory.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => return Ok(None),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    }
    if !dunce::canonicalize(&path)?.starts_with(directory) {
        return Ok(None);
    }
    Ok(Some(fs::read(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCKFILE: &[u8] = b"lockfileVersion: '9.0'\n";
    const RECEIPT: &[u8] = b"\xf0\xbc\xde\x46\x3f\xa2\x01\x48\x00\x15\xb9\xca\xa7\xdb\x20\x17\xd3\xc1\xb6\xca\x9c\x7e\x13\x3d\xf9\x55\x03\x8c\x54\x33\x3d\x48";

    fn create_version(data: &AbsolutePathBuf, version: &str) -> AbsolutePathBuf {
        let directory = data.join(version);
        fs::create_dir_all(directory.join("bin")).unwrap();
        fs::create_dir_all(directory.join("node_modules/vite-plus")).unwrap();
        fs::write(directory.join("bin").join(INSTALL_LOCKFILE), LOCKFILE).unwrap();
        fs::write(directory.join(INSTALL_RECEIPT), RECEIPT).unwrap();
        fs::write(
            directory.join("package.json"),
            serde_json::json!({
                "name": "vp-global", "version": version, "private": true,
                "dependencies": { "vite-plus": version },
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            directory.join("node_modules/vite-plus/package.json"),
            serde_json::json!({
                "name": "vite-plus", "version": version,
            })
            .to_string(),
        )
        .unwrap();
        directory
    }

    #[test]
    fn versions_require_matching_metadata_and_a_frozen_install_receipt() {
        vp_shared::EnvConfig::scoped(|env| {
            let directory = create_version(&env.dirs.data, "1.4.0");
            assert!(verified_version(&directory).unwrap());
            let receipt = directory.join(INSTALL_RECEIPT);
            fs::remove_file(&receipt).unwrap();
            fs::write(directory.join("bin/.vp-setup-complete"), b"").unwrap();
            assert!(!verified_version(&directory).unwrap(), "setup completion is not ownership");
            for invalid in [b"".as_slice(), b"unlocked", b"different digest"] {
                fs::write(&receipt, invalid).unwrap();
                assert!(!verified_version(&directory).unwrap());
            }
            fs::write(&receipt, RECEIPT).unwrap();
            let lockfile = directory.join("bin").join(INSTALL_LOCKFILE);
            fs::remove_file(&lockfile).unwrap();
            assert!(!verified_version(&directory).unwrap());
            fs::write(&lockfile, b"changed lockfile").unwrap();
            assert!(!verified_version(&directory).unwrap());
            fs::write(&lockfile, LOCKFILE).unwrap();
            for (file, field, value) in [
                ("package.json", "/name", serde_json::json!("personal-project")),
                ("package.json", "/version", serde_json::json!("1.3.0")),
                ("package.json", "/private", serde_json::json!(false)),
                ("package.json", "/dependencies/vite-plus", serde_json::json!("1.3.0")),
                ("node_modules/vite-plus/package.json", "/name", serde_json::json!("other")),
                ("node_modules/vite-plus/package.json", "/version", serde_json::json!("1.3.0")),
            ] {
                let path = directory.join(file);
                let original = fs::read(&path).unwrap();
                let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
                *manifest.pointer_mut(field).unwrap() = value;
                fs::write(&path, manifest.to_string()).unwrap();
                assert!(!verified_version(&directory).unwrap(), "{file} {field}");
                fs::write(&path, b"malformed json").unwrap();
                assert!(!verified_version(&directory).unwrap());
                fs::write(&path, original).unwrap();
            }
            let forced = env.dirs.data.join("1.4.0+force.123.456");
            fs::rename(&directory, &forced).unwrap();
            assert!(verified_version(&forced).unwrap());
            let wrong_version = env.dirs.data.join("2.0.0");
            fs::rename(&forced, &wrong_version).unwrap();
            assert!(!verified_version(&wrong_version).unwrap());
        });
    }

    #[test]
    fn unowned_nested_categories_block_ancestor_and_version_removal() {
        vp_shared::EnvConfig::scoped(|env| {
            fs::write(env.dirs.data.join(install_dirs::OWNERSHIP_MARKER), b"").unwrap();
            fs::create_dir_all(&env.dirs.cache).unwrap();
            let version = create_version(&env.dirs.data, "1.4.0");
            let plan = RemovalPlan::new(&env.dirs, &install_dirs::roots(&env.dirs)).unwrap();
            assert_eq!(plan.directories, vec![version.clone()]);
            let nested = version.join("personal");
            fs::create_dir(&nested).unwrap();
            fs::write(nested.join("notes"), b"keep").unwrap();
            for category in ["cache", "config", "state"] {
                let mut dirs = env.dirs.clone();
                match category {
                    "cache" => dirs.cache = nested.clone(),
                    "config" => dirs.config = nested.clone(),
                    "state" => dirs.state = nested.clone(),
                    _ => unreachable!(),
                }
                let plan = RemovalPlan::new(&dirs, &install_dirs::roots(&dirs)).unwrap();
                assert!(plan.directories.is_empty(), "{category} must block recursive removal");
            }
            assert_eq!(fs::read(nested.join("notes")).unwrap(), b"keep");
        });
    }

    #[cfg(unix)]
    #[test]
    fn unowned_nested_cache_rejects_symlink_parent_path() {
        vp_shared::EnvConfig::scoped(|env| {
            let mut dirs = env.dirs.clone();
            dirs.data = env.dirs.data.join("data");
            dirs.config = dirs.data.clone();
            dirs.state = dirs.data.clone();
            let personal = dirs.data.join("personal");
            let links = env.dirs.data.join("links");
            fs::create_dir_all(dirs.data.join("child")).unwrap();
            fs::create_dir(&personal).unwrap();
            fs::create_dir(&links).unwrap();
            fs::write(dirs.data.join(install_dirs::OWNERSHIP_MARKER), b"").unwrap();
            fs::write(personal.join("notes.txt"), b"keep").unwrap();
            std::os::unix::fs::symlink(dirs.data.join("child"), links.join("route")).unwrap();
            let alias = links.join("route/../personal");
            assert!(!links.join("personal").as_path().exists());

            dirs.cache = personal.clone();
            let plan = RemovalPlan::new(&dirs, &install_dirs::roots(&dirs)).unwrap();
            assert!(plan.directories.is_empty());

            dirs.cache = alias;
            let error = RemovalPlan::new(&dirs, &install_dirs::roots(&dirs))
                .err()
                .expect("a symlink parent path must be rejected before removal");
            assert!(error.to_string().contains("Refusing to normalize directory"));
            assert_eq!(fs::read(personal.join("notes.txt")).unwrap(), b"keep");
        });
    }

    #[cfg(unix)]
    #[test]
    fn verified_version_links_and_shims_do_not_require_a_surviving_binary() {
        vp_shared::EnvConfig::scoped(|env| {
            let version = create_version(&env.dirs.data, "1.4.0");
            let current = env.dirs.data.join("current");
            std::os::unix::fs::symlink(&version, &current).unwrap();
            fs::create_dir_all(&env.dirs.bin).unwrap();
            let shim = env.dirs.bin.join("vp");
            std::os::unix::fs::symlink("../current/bin/vp", &shim).unwrap();
            let plan = RemovalPlan::new(&env.dirs, &install_dirs::roots(&env.dirs)).unwrap();
            assert_eq!(plan.current_link, Some(current));
            assert_eq!(plan.directories, vec![version]);
            assert_eq!(
                super::super::collect_shim_files(&env.dirs, &plan.binary_targets),
                vec![shim]
            );
        });
    }

    #[cfg(unix)]
    #[test]
    fn current_link_to_an_owned_cache_is_not_a_verified_cli_link() {
        vp_shared::EnvConfig::scoped(|env| {
            fs::create_dir_all(&env.dirs.cache).unwrap();
            fs::write(env.dirs.cache.join(install_dirs::OWNERSHIP_MARKER), b"").unwrap();
            std::os::unix::fs::symlink(&env.dirs.cache, env.dirs.data.join("current")).unwrap();
            let plan = RemovalPlan::new(&env.dirs, &install_dirs::roots(&env.dirs)).unwrap();
            assert_eq!(plan.directories, vec![env.dirs.cache.clone()]);
            assert!(plan.current_link.is_none());
        });
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_evidence_and_category_aliases_do_not_expand_ownership() {
        let temporary = tempfile::tempdir().unwrap();
        let outside_root = AbsolutePathBuf::new(temporary.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::scoped(|env| {
            let version = create_version(&env.dirs.data, "1.4.0");
            let modules = version.join("node_modules");
            let package = modules.join(".pnpm/vite-plus/node_modules/vite-plus");
            fs::create_dir_all(package.parent().unwrap()).unwrap();
            fs::rename(modules.join("vite-plus"), &package).unwrap();
            std::os::unix::fs::symlink(
                ".pnpm/vite-plus/node_modules/vite-plus",
                modules.join("vite-plus"),
            )
            .unwrap();
            assert!(verified_version(&version).unwrap(), "contained pnpm links are supported");

            let personal = version.join("personal");
            fs::create_dir(&personal).unwrap();
            let mut dirs = env.dirs.clone();
            dirs.cache = outside_root.join("cache-alias");
            std::os::unix::fs::symlink(&personal, &dirs.cache).unwrap();
            assert!(
                RemovalPlan::new(&dirs, &install_dirs::roots(&dirs))
                    .unwrap()
                    .directories
                    .is_empty()
            );

            let outside = outside_root.join("outside-modules");
            fs::rename(&modules, &outside).unwrap();
            std::os::unix::fs::symlink(&outside, &modules).unwrap();
            assert!(
                !verified_version(&version).unwrap(),
                "borrowed evidence cannot authorize removal"
            );
            fs::remove_file(&modules).unwrap();
            fs::rename(&outside, &modules).unwrap();

            let outside_version = outside_root.join("outside-version");
            fs::rename(&version, &outside_version).unwrap();
            std::os::unix::fs::symlink(&outside_version, &version).unwrap();
            assert!(
                RemovalPlan::new(&env.dirs, &install_dirs::roots(&env.dirs))
                    .unwrap()
                    .directories
                    .is_empty()
            );

            fs::write(env.dirs.data.join(install_dirs::OWNERSHIP_MARKER), b"").unwrap();
            fs::create_dir(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, &env.dirs.cache).unwrap();
            assert!(
                RemovalPlan::new(&env.dirs, &install_dirs::roots(&env.dirs))
                    .unwrap()
                    .directories
                    .is_empty(),
                "a marked parent must preserve an unowned category link"
            );
            assert!(outside_version.as_path().exists());
        });
    }
}
