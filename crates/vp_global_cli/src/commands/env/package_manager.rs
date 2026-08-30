use vp_pm_cli::{
    EnvironmentPackageManagerResolution, PackageManagerType, resolve_environment_package_manager,
    resolve_environment_package_manager_spec, resolve_package_manager_version,
};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::{config, spec::parse_package_manager_spec_with_hash};
use crate::error::Error;

pub(crate) async fn resolve_current(
    cwd: &AbsolutePath,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    resolve_current_for(cwd, None).await
}

pub(crate) async fn resolve_current_for(
    cwd: &AbsolutePath,
    expected: Option<PackageManagerType>,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let specs = current_specs(expected).await?;
    let mut resolution = resolve_environment_package_manager(
        cwd,
        specs.session_spec(),
        specs.default_spec(),
        expected,
    )
    .await?;
    specs.apply_session_source(&mut resolution);
    Ok(resolution)
}

pub(crate) async fn resolve_current_or_fallback_for(
    cwd: &AbsolutePath,
    package_manager: PackageManagerType,
) -> Result<EnvironmentPackageManagerResolution, Error> {
    if let Some(resolution) = resolve_current_for(cwd, Some(package_manager)).await? {
        return Ok(resolution);
    }

    registry_fallback_for(package_manager).await
}

pub(crate) async fn resolve_current_spec(
    cwd: &AbsolutePath,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let specs = current_specs(None).await?;

    let mut resolution =
        resolve_environment_package_manager_spec(cwd, specs.session_spec(), specs.default_spec())
            .map_err(Error::from)?;
    specs.apply_session_source(&mut resolution);
    Ok(resolution)
}

pub(crate) type PackageManagerSpec = (PackageManagerType, String, Option<String>);

struct CurrentSpecs {
    session: Option<PackageManagerSpec>,
    session_source: Option<&'static str>,
    session_source_path: Option<AbsolutePathBuf>,
    default: Option<PackageManagerSpec>,
}

impl CurrentSpecs {
    fn session_spec(&self) -> Option<(PackageManagerType, &str, Option<&str>)> {
        self.session
            .as_ref()
            .map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref()))
    }

    fn default_spec(&self) -> Option<(PackageManagerType, &str, Option<&str>)> {
        self.default
            .as_ref()
            .map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref()))
    }

    fn apply_session_source(&self, resolution: &mut Option<EnvironmentPackageManagerResolution>) {
        if let (Some(resolution), Some(source)) = (resolution, self.session_source) {
            resolution.source = source.into();
            resolution.source_path.clone_from(&self.session_source_path);
        }
    }
}

async fn current_specs(expected: Option<PackageManagerType>) -> Result<CurrentSpecs, Error> {
    let config = vp_shared::EnvConfig::get();
    let (session, session_source, session_source_path) = if let Some(spec) =
        config.package_manager.as_deref().map(str::trim).filter(|spec| !spec.is_empty())
    {
        (
            Some(parse_package_manager_spec_with_hash(spec)?),
            Some(config::PACKAGE_MANAGER_ENV_VAR),
            None,
        )
    } else if let Some(spec) = config::read_session_package_manager().await {
        (
            Some(parse_package_manager_spec_with_hash(spec.trim())?),
            Some(config::SESSION_PACKAGE_MANAGER_FILE),
            config::get_session_package_manager_path().ok(),
        )
    } else {
        (None, None, None)
    };
    let config = config::load_config().await?;
    let default = expected
        .map(|package_manager| configured_default_for(&config, package_manager))
        .transpose()?
        .flatten();
    Ok(CurrentSpecs { session, session_source, session_source_path, default })
}

pub(crate) fn configured_default_for(
    config: &config::Config,
    package_manager: PackageManagerType,
) -> Result<Option<PackageManagerSpec>, Error> {
    config
        .default_package_manager_version_for(package_manager)
        .map(|version| {
            parse_package_manager_spec_with_hash(&format!("{package_manager}@{version}"))
        })
        .transpose()
}

pub(crate) async fn resolve_from_files_for(
    cwd: &AbsolutePath,
    expected: Option<PackageManagerType>,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let config = config::load_config().await?;
    let default = expected
        .map(|package_manager| configured_default_for(&config, package_manager))
        .transpose()?
        .flatten();
    resolve_environment_package_manager(
        cwd,
        None,
        default.as_ref().map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref())),
        expected,
    )
    .await
    .map_err(Error::from)
}

pub(crate) async fn resolve_from_files_or_fallback_for(
    cwd: &AbsolutePath,
    package_manager: PackageManagerType,
) -> Result<EnvironmentPackageManagerResolution, Error> {
    if let Some(resolution) = resolve_from_files_for(cwd, Some(package_manager)).await? {
        return Ok(resolution);
    }

    registry_fallback_for(package_manager).await
}

async fn registry_fallback_for(
    package_manager: PackageManagerType,
) -> Result<EnvironmentPackageManagerResolution, Error> {
    Ok(EnvironmentPackageManagerResolution {
        package_manager_type: package_manager,
        version: resolve_package_manager_version(package_manager, "latest").await?,
        hash: None,
        source: "registry fallback".into(),
        source_path: None,
        project_root: None,
    })
}

pub(crate) async fn warn_if_target_differs(cwd: &AbsolutePath, target: PackageManagerType) {
    let Ok(Some(current)) = resolve_current_spec(cwd).await else {
        return;
    };
    if current.source != "default" && current.package_manager_type != target {
        vp_shared::output::warn(&format!(
            "Current environment resolves to {} from {}, but {target} was requested.",
            current.package_manager_type, current.source
        ));
    }
}

pub(crate) const ALL_PACKAGE_MANAGERS: [PackageManagerType; 4] = [
    PackageManagerType::Npm,
    PackageManagerType::Pnpm,
    PackageManagerType::Yarn,
    PackageManagerType::Bun,
];

pub(crate) fn selected(scope: super::spec::EnvScope) -> Vec<PackageManagerType> {
    match scope {
        super::spec::EnvScope::All | super::spec::EnvScope::PackageManagers => {
            ALL_PACKAGE_MANAGERS.to_vec()
        }
        super::spec::EnvScope::PackageManager(kind) => vec![kind],
        super::spec::EnvScope::Node => Vec::new(),
    }
}

pub(crate) const fn title(kind: PackageManagerType) -> &'static str {
    match kind {
        PackageManagerType::Npm => "npm",
        PackageManagerType::Pnpm => "pnpm",
        PackageManagerType::Yarn => "Yarn",
        PackageManagerType::Bun => "Bun",
    }
}
