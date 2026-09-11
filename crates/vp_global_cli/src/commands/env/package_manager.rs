use vp_pm_cli::{
    EnvironmentPackageManagerResolution, PackageManagerType, resolve_environment_package_manager,
    resolve_environment_package_manager_spec, resolve_package_manager_version,
};
use vt_path::AbsolutePath;

use super::{config, spec::parse_package_manager_spec_with_hash};
use crate::error::Error;

pub(crate) async fn resolve_current(
    cwd: &AbsolutePath,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    resolve_current_for(cwd, None).await
}

/// Direct shims have independent overrides; selecting a manager for vp commands must not change them.
pub(crate) async fn resolve_shim_for(
    cwd: &AbsolutePath,
    expected: PackageManagerType,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let (version, source, source_path) = if let Some(version) = environment_version(expected) {
        (Some(version), version_env_var(expected).to_string(), None)
    } else {
        (
            config::read_session_package_manager(expected).await,
            format!(".session-{expected}-version"),
            config::get_session_package_manager_path(expected).ok(),
        )
    };
    let override_spec = version
        .map(|version| parse_package_manager_spec_with_hash(&format!("{expected}@{version}")))
        .transpose()?;
    let default = configured_default_for(&config::load_config().await?, expected)?;
    let mut resolution = resolve_environment_package_manager(
        cwd,
        override_spec
            .as_ref()
            .map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref())),
        default.as_ref().map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref())),
        Some(expected),
    )
    .await?;
    if override_spec.is_some()
        && let Some(resolution) = &mut resolution
    {
        resolution.source = source.into();
        resolution.source_path = source_path;
    }
    Ok(resolution)
}

pub(crate) fn version_env_var(kind: PackageManagerType) -> &'static str {
    use vp_shared::env_vars;
    match kind {
        PackageManagerType::Npm => env_vars::VP_NPM_VERSION,
        PackageManagerType::Pnpm => env_vars::VP_PNPM_VERSION,
        PackageManagerType::Yarn => env_vars::VP_YARN_VERSION,
        PackageManagerType::Bun => env_vars::VP_BUN_VERSION,
    }
}

pub(crate) fn environment_version(kind: PackageManagerType) -> Option<String> {
    let env = vp_shared::EnvConfig::get();
    match kind {
        PackageManagerType::Npm => env.npm_version.as_deref(),
        PackageManagerType::Pnpm => env.pnpm_version.as_deref(),
        PackageManagerType::Yarn => env.yarn_version.as_deref(),
        PackageManagerType::Bun => env.bun_version.as_deref(),
    }
    .map(str::trim)
    .filter(|version| !version.is_empty())
    .map(str::to_string)
}

pub(crate) async fn resolve_current_for(
    cwd: &AbsolutePath,
    expected: Option<PackageManagerType>,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let specs = current_specs(expected).await?;
    let mut resolution = resolve_environment_package_manager(
        cwd,
        specs.override_spec(),
        specs.default_spec(),
        expected,
    )
    .await?;
    specs.apply_override_source(&mut resolution);
    Ok(resolution)
}

pub(crate) async fn resolve_current_or_fallback_for(
    cwd: &AbsolutePath,
    package_manager: PackageManagerType,
) -> Result<EnvironmentPackageManagerResolution, Error> {
    if let Some(resolution) = resolve_shim_for(cwd, package_manager).await? {
        return Ok(resolution);
    }

    registry_fallback_for(package_manager).await
}

pub(crate) async fn resolve_current_spec(
    cwd: &AbsolutePath,
) -> Result<Option<EnvironmentPackageManagerResolution>, Error> {
    let specs = current_specs(None).await?;

    let mut resolution =
        resolve_environment_package_manager_spec(cwd, specs.override_spec(), specs.default_spec())
            .map_err(Error::from)?;
    specs.apply_override_source(&mut resolution);
    Ok(resolution)
}

pub(crate) type PackageManagerSpec = (PackageManagerType, String, Option<String>);

struct CurrentSpecs {
    selected: Option<PackageManagerSpec>,
    default: Option<PackageManagerSpec>,
}

impl CurrentSpecs {
    fn override_spec(&self) -> Option<(PackageManagerType, &str, Option<&str>)> {
        self.selected
            .as_ref()
            .map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref()))
    }

    fn default_spec(&self) -> Option<(PackageManagerType, &str, Option<&str>)> {
        self.default
            .as_ref()
            .map(|(kind, version, hash)| (*kind, version.as_str(), hash.as_deref()))
    }

    fn apply_override_source(&self, resolution: &mut Option<EnvironmentPackageManagerResolution>) {
        if self.selected.is_some()
            && let Some(resolution) = resolution
        {
            resolution.source = config::PACKAGE_MANAGER_ENV_VAR.into();
            resolution.source_path = None;
        }
    }
}

async fn current_specs(expected: Option<PackageManagerType>) -> Result<CurrentSpecs, Error> {
    let env = vp_shared::EnvConfig::get();
    let selected = env
        .package_manager
        .as_deref()
        .map(str::trim)
        .filter(|spec| !spec.is_empty())
        .map(parse_package_manager_spec_with_hash)
        .transpose()?;
    let config = config::load_config().await?;
    let default = expected
        .map(|package_manager| configured_default_for(&config, package_manager))
        .transpose()?
        .flatten();
    Ok(CurrentSpecs { selected, default })
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
