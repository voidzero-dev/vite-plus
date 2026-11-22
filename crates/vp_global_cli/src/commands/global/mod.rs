//! Managed global package utilities.

use std::{fs::File, io::Read};

use flate2::read::GzDecoder;
use tar::Archive;
use vt_path::{AbsolutePathBuf, current_dir};

use crate::error::Error;

pub(crate) mod bin_lock;
pub mod install;
pub mod outdated;
pub mod packages;

/// Legacy managed globals superseded by the default package-manager shims.
pub(crate) const LEGACY_PACKAGE_MANAGER_PACKAGES: &[&str] = &["yarn", "pnpm", "bun", "corepack"];

/// Return true for package specs that refer to local filesystem content.
pub(crate) fn is_local_package_spec(spec: &str) -> bool {
    spec == "."
        || spec == ".."
        || spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with('/')
        || spec.starts_with("file:")
        || (cfg!(windows)
            && spec.len() >= 3
            && spec.as_bytes()[1] == b':'
            && (spec.as_bytes()[2] == b'\\' || spec.as_bytes()[2] == b'/'))
}

/// Parse package spec into name and optional version.
/// For local packages, read package.json from a directory or package tarball.
///
/// It will never return an `Err()` if it is not a local package
pub(crate) fn parse_package_spec(spec: &str) -> Result<(String, Option<String>), Error> {
    if is_local_package_spec(spec) {
        let package_json = read_local_package_json(spec)?;
        let Some(package_name) = package_json.get("name").and_then(|name| name.as_str()) else {
            return Err(Error::Other(
                format!("Local package {spec} must have a string name in package.json").into(),
            ));
        };

        Ok((package_name.to_string(), None))
    } else {
        if spec.starts_with('@') {
            if let Some(idx) = spec[1..].find('@') {
                let idx = idx + 1;
                return Ok((spec[..idx].to_string(), Some(spec[idx + 1..].to_string())));
            }
            return Ok((spec.to_string(), None));
        }

        if let Some(idx) = spec.find('@') {
            return Ok((spec[..idx].to_string(), Some(spec[idx + 1..].to_string())));
        }

        Ok((spec.to_string(), None))
    }
}

/// Return the version part of a registry package spec that update flows
/// should keep resolving within: a dist-tag (e.g. `nightly` in
/// `some-pkg@nightly`), a range, an exact version, or an npm alias
/// (`npm:other` / `npm:other@^1`). The implicit `latest` tag and other
/// non-registry version parts (URLs, git, tarballs) yield `None`.
pub(crate) fn update_version_spec(spec: &str) -> Option<String> {
    if is_local_package_spec(spec) {
        return None;
    }
    let (_, version_spec) = parse_package_spec(spec).ok()?;
    let version_spec = version_spec?;
    // Keep the alias so updates reinstall the aliased package, not one named after the alias.
    if let Some(target) = version_spec.strip_prefix("npm:") {
        let target_name = alias_target(&version_spec)?;
        return Some(match update_version_spec(target) {
            Some(target_spec) => format!("npm:{target_name}@{target_spec}"),
            None => format!("npm:{target_name}"),
        });
    }
    if version_spec.is_empty()
        || version_spec == "latest"
        || version_spec.contains(':')
        || version_spec.contains('/')
        // npm treats tarball-suffixed spec parts as file installs.
        || version_spec.ends_with(".tgz")
        || version_spec.ends_with(".tar.gz")
        || version_spec.ends_with(".tar")
    {
        None
    } else {
        Some(version_spec)
    }
}

/// Package name behind an npm alias version part (`npm:<target>[@<spec>]`).
pub(crate) fn alias_target(version_spec: &str) -> Option<String> {
    let target = version_spec.strip_prefix("npm:")?;
    if is_local_package_spec(target) {
        return None;
    }
    parse_package_spec(target).ok().map(|(name, _)| name).filter(|name| !name.is_empty())
}

fn resolve_local_package_path(spec: &str) -> Result<AbsolutePathBuf, Error> {
    let path_spec = spec.strip_prefix("file:").unwrap_or(spec);
    let path = std::path::Path::new(path_spec);
    if path.is_absolute() {
        AbsolutePathBuf::new(path.to_path_buf())
            .ok_or_else(|| Error::Other(format!("Invalid local package path {spec}").into()))
    } else {
        Ok(current_dir()?.join(path))
    }
}

fn read_local_package_json(spec: &str) -> Result<serde_json::Value, Error> {
    let package_path = resolve_local_package_path(spec)?;
    if package_path.as_path().is_file() && is_package_tarball(package_path.as_path()) {
        return read_package_json_from_tarball(spec, &package_path);
    }

    let package_json_path = package_path.join("package.json");
    let package_json_content =
        std::fs::read_to_string(package_json_path.as_path()).map_err(|error| {
            Error::Other(
                format!(
                    "Failed to read package.json for local package {spec} at {}: {error}",
                    package_json_path.as_path().display()
                )
                .into(),
            )
        })?;
    serde_json::from_str(&package_json_content).map_err(Error::JsonError)
}

fn is_package_tarball(path: &std::path::Path) -> bool {
    let path = path.to_string_lossy();
    path.ends_with(".tgz") || path.ends_with(".tar.gz")
}

fn read_package_json_from_tarball(
    spec: &str,
    package_path: &AbsolutePathBuf,
) -> Result<serde_json::Value, Error> {
    let file = File::open(package_path.as_path()).map_err(|error| {
        Error::Other(
            format!(
                "Failed to read package tarball {spec} at {}: {error}",
                package_path.as_path().display()
            )
            .into(),
        )
    })?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries().map_err(|error| {
        Error::Other(format!("Failed to read package tarball {spec}: {error}").into())
    })? {
        let mut entry = entry.map_err(|error| {
            Error::Other(format!("Failed to read package tarball {spec}: {error}").into())
        })?;
        let path = entry.path().map_err(|error| {
            Error::Other(format!("Failed to read package tarball {spec}: {error}").into())
        })?;
        if path.as_ref() != std::path::Path::new("package/package.json") {
            continue;
        }

        let mut package_json_content = String::new();
        entry.read_to_string(&mut package_json_content).map_err(|error| {
            Error::Other(
                format!("Failed to read package.json from package tarball {spec}: {error}").into(),
            )
        })?;
        return serde_json::from_str(&package_json_content).map_err(Error::JsonError);
    }

    Err(Error::Other(format!("Package tarball {spec} must contain package/package.json").into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_version_spec_keeps_dist_tags() {
        assert_eq!(update_version_spec("some-pkg@nightly"), Some("nightly".to_string()));
        assert_eq!(update_version_spec("some-pkg@beta"), Some("beta".to_string()));
        assert_eq!(update_version_spec("@scope/pkg@canary"), Some("canary".to_string()));
    }

    #[test]
    fn update_version_spec_keeps_versions_and_ranges() {
        assert_eq!(update_version_spec("some-pkg@1.2.3"), Some("1.2.3".to_string()));
        assert_eq!(
            update_version_spec("some-pkg@1.2.3-nightly.0"),
            Some("1.2.3-nightly.0".to_string())
        );
        assert_eq!(update_version_spec("some-pkg@^1.0.0"), Some("^1.0.0".to_string()));
        assert_eq!(update_version_spec("some-pkg@>=2"), Some(">=2".to_string()));
    }

    #[test]
    fn update_version_spec_ignores_bare_names_and_latest() {
        assert_eq!(update_version_spec("some-pkg"), None);
        assert_eq!(update_version_spec("@scope/pkg"), None);
        assert_eq!(update_version_spec("some-pkg@latest"), None);
        assert_eq!(update_version_spec("some-pkg@"), None);
    }

    #[test]
    fn update_version_spec_ignores_non_registry_specs() {
        assert_eq!(update_version_spec("./local-pkg"), None);
        assert_eq!(update_version_spec("file:../pkg"), None);
        assert_eq!(update_version_spec("some-pkg@github:user/repo"), None);
        assert_eq!(update_version_spec("some-pkg@archive.tgz"), None);
        assert_eq!(update_version_spec("some-pkg@archive.tar.gz"), None);
        assert_eq!(update_version_spec("some-pkg@archive.tar"), None);
    }

    #[test]
    fn update_version_spec_keeps_npm_aliases() {
        assert_eq!(update_version_spec("typescript@npm:cowsay"), Some("npm:cowsay".to_string()));
        assert_eq!(
            update_version_spec("typescript@npm:cowsay@latest"),
            Some("npm:cowsay".to_string())
        );
        assert_eq!(
            update_version_spec("typescript@npm:cowsay@^1.5"),
            Some("npm:cowsay@^1.5".to_string())
        );
        assert_eq!(
            update_version_spec("@scope/alias@npm:@types/node@20"),
            Some("npm:@types/node@20".to_string())
        );
        assert_eq!(update_version_spec("some-pkg@npm:"), None);
        assert_eq!(update_version_spec("some-pkg@npm:./local"), None);
    }
}
