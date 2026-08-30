use vp_pm_cli::PackageManagerType;

use crate::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnvScope {
    All,
    Node,
    PackageManagers,
    PackageManager(PackageManagerType),
}

impl EnvScope {
    pub(crate) fn parse(value: Option<&str>) -> Result<Self, Error> {
        let Some(value) = value else {
            return Ok(Self::All);
        };
        match value {
            "node" => Ok(Self::Node),
            "pm" => Ok(Self::PackageManagers),
            name => PackageManagerType::from_name(name)
                .map(Self::PackageManager)
                .ok_or_else(|| invalid_scope(name)),
        }
    }

    pub(crate) fn includes_node(self) -> bool {
        matches!(self, Self::All | Self::Node)
    }

    pub(crate) fn includes_package_managers(self) -> bool {
        !matches!(self, Self::Node)
    }

    pub(crate) fn package_manager(self) -> Option<PackageManagerType> {
        match self {
            Self::PackageManager(kind) => Some(kind),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EnvSpecs {
    pub(crate) node: Option<String>,
    pub(crate) package_manager: Option<(PackageManagerType, String, Option<String>)>,
}

impl EnvSpecs {
    pub(crate) fn parse(values: &[String]) -> Result<Self, Error> {
        let mut parsed = Self::default();
        for value in values {
            if let Some((name, version)) = value.split_once('@') {
                if version.is_empty() {
                    return Err(invalid_spec(value));
                }
                if name == "node" {
                    if parsed.node.replace(version.to_string()).is_some() {
                        return Err(duplicate("Node.js"));
                    }
                } else {
                    let package_manager = parse_package_manager_spec_with_hash(value)?;
                    if parsed.package_manager.replace(package_manager).is_some() {
                        return Err(duplicate("package manager"));
                    }
                }
            } else if EnvScope::parse(Some(value)).is_ok() {
                return Err(Error::Other(
                    format!("{value:?} is a component selector, not a version specification")
                        .into(),
                ));
            } else if parsed.node.replace(value.clone()).is_some() {
                return Err(duplicate("Node.js"));
            }
        }
        Ok(parsed)
    }

    pub(crate) fn parse_requests(values: &[String]) -> Result<(EnvScope, Self), Error> {
        if values.len() == 1
            && let Ok(scope) = EnvScope::parse(Some(&values[0]))
        {
            return Ok((scope, Self::default()));
        }
        let specs = Self::parse(values)?;
        let scope = match (&specs.node, &specs.package_manager) {
            (Some(_), Some(_)) | (None, None) => EnvScope::All,
            (Some(_), None) => EnvScope::Node,
            (None, Some((kind, _, _))) => EnvScope::PackageManager(*kind),
        };
        Ok((scope, specs))
    }
}

pub(crate) fn parse_package_manager_spec(
    value: &str,
) -> Result<(PackageManagerType, String), Error> {
    let (package_manager, version, _) = parse_package_manager_spec_with_hash(value)?;
    Ok((package_manager, version))
}

pub(crate) fn parse_package_manager_spec_with_hash(
    value: &str,
) -> Result<(PackageManagerType, String, Option<String>), Error> {
    let Some((name, version)) = value.split_once('@') else {
        return Err(invalid_spec(value));
    };
    let package_manager = PackageManagerType::from_name(name).ok_or_else(|| invalid_spec(value))?;
    if version.is_empty() {
        return Err(invalid_spec(value));
    }
    let (version, hash) = version
        .split_once('+')
        .map_or((version, None), |(version, hash)| (version, Some(hash.to_string())));
    if version.is_empty() || hash.as_deref() == Some("") {
        return Err(invalid_spec(value));
    }
    Ok((package_manager, version.to_string(), hash))
}

fn invalid_scope(value: &str) -> Error {
    Error::Other(
        format!("invalid environment scope {value:?}; expected node, pm, npm, pnpm, yarn, or bun")
            .into(),
    )
}

fn invalid_spec(value: &str) -> Error {
    Error::Other(
        format!(
            "invalid environment specification {value:?}; expected a Node.js version or node|npm|pnpm|yarn|bun@<version>"
        )
        .into(),
    )
}

fn duplicate(component: &str) -> Error {
    Error::Other(format!("only one {component} specification may be supplied").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_node_and_package_manager_specs() {
        let parsed = EnvSpecs::parse(&["22.0.0".into(), "pnpm@10.18.0".into()]).unwrap();
        assert_eq!(parsed.node.as_deref(), Some("22.0.0"));
        assert_eq!(
            parsed.package_manager,
            Some((PackageManagerType::Pnpm, "10.18.0".into(), None))
        );
    }

    #[test]
    fn bare_version_request_selects_node() {
        let (scope, specs) = EnvSpecs::parse_requests(&["22.0.0".into()]).unwrap();
        assert_eq!(scope, EnvScope::Node);
        assert_eq!(specs.node.as_deref(), Some("22.0.0"));
    }

    #[test]
    fn package_manager_session_spec_preserves_hash() {
        let parsed =
            parse_package_manager_spec_with_hash("yarn@4.17.1+sha512.0123456789abcdef").unwrap();
        assert_eq!(
            parsed,
            (PackageManagerType::Yarn, "4.17.1".into(), Some("sha512.0123456789abcdef".into()))
        );
    }
}
