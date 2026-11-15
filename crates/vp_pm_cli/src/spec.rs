use vp_shared::EnvConfig;

use crate::{Error, PackageManagerType};

pub type PackageManagerSpec = (PackageManagerType, String, Option<String>);

/// Parse a package manager requirement, including an optional Corepack integrity hash.
pub fn parse_package_manager_spec(value: &str) -> Result<PackageManagerSpec, Error> {
    let invalid_spec = || Error::InvalidPackageManagerSpec(value.to_string());
    let (name, version) = value.split_once('@').ok_or_else(invalid_spec)?;
    let package_manager = PackageManagerType::from_name(name).ok_or_else(invalid_spec)?;
    let (version, hash) = version
        .split_once('+')
        .map_or((version, None), |(version, hash)| (version, Some(hash.to_string())));
    if version.is_empty() || hash.as_deref() == Some("") {
        return Err(invalid_spec());
    }
    Ok((package_manager, version.to_string(), hash))
}

/// Read the package manager override used by `vp` commands.
pub fn package_manager_override() -> Result<Option<PackageManagerSpec>, Error> {
    EnvConfig::get()
        .package_manager
        .as_deref()
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
        .map(parse_package_manager_spec)
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_preserves_version_and_integrity() {
        EnvConfig::with_vars(
            [("VP_PACKAGE_MANAGER", " pnpm@10.6.1+sha512.0123456789abcdef ")],
            |_| {
                assert_eq!(
                    package_manager_override().unwrap(),
                    Some((
                        PackageManagerType::Pnpm,
                        "10.6.1".into(),
                        Some("sha512.0123456789abcdef".into())
                    ))
                );
            },
        );
    }

    #[test]
    fn empty_override_is_ignored() {
        EnvConfig::with_vars([("VP_PACKAGE_MANAGER", "  ")], |_| {
            assert_eq!(package_manager_override().unwrap(), None);
        });
    }

    #[test]
    fn invalid_override_is_rejected() {
        for value in ["pnpm", "pnpm@", "pnpm@+sha512.abc", "pnpm@10.6.1+", "other@1"] {
            EnvConfig::with_vars([("VP_PACKAGE_MANAGER", value)], |_| {
                assert!(package_manager_override().is_err(), "{value}");
            });
        }
    }
}
