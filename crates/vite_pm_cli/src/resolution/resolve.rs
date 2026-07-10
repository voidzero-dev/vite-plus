use semver::Version;

use crate::{
    Error, PackageManager, PackageManagerType,
    resolution::{
        Bun, CommandResolution, Diagnosis, Diagnostics, Npm, PackageManagerDialect, Pnpm, Yarn,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolution {
    pub(crate) outcome: CommandResolution,
    pub(crate) diagnostics: Diagnostics,
}

pub(crate) trait Resolve<A>: PackageManagerDialect {
    fn resolve(&self, args: &A, diag: &mut Diagnostics) -> CommandResolution;
}

pub(crate) fn resolve<Dialect, A>(dialect: &Dialect, args: A) -> Resolution
where
    Dialect: Resolve<A>,
    A: Diagnosis,
{
    let mut diagnostics = Diagnostics::default();
    let args = args.diagnose(dialect, &mut diagnostics);
    let outcome = dialect.resolve(&args, &mut diagnostics);
    Resolution { outcome, diagnostics }
}

pub(crate) fn resolve_for_manager<A>(manager: &PackageManager, args: A) -> Result<Resolution, Error>
where
    A: Diagnosis,
    Npm: Resolve<A>,
    Pnpm: Resolve<A>,
    Yarn: Resolve<A>,
    Bun: Resolve<A>,
{
    let mut resolution = match manager.client {
        PackageManagerType::Npm => {
            let dialect =
                Version::parse(&manager.version).map_or_else(|_| Npm::unknown_version(), Npm::new);
            resolve(&dialect, args)
        }
        PackageManagerType::Pnpm => {
            let dialect = Pnpm::new(parse_version(manager)?);
            resolve(&dialect, args)
        }
        PackageManagerType::Yarn => {
            let dialect = Yarn::new(parse_version(manager)?);
            resolve(&dialect, args)
        }
        PackageManagerType::Bun => {
            let dialect = Bun::new(parse_version(manager)?);
            resolve(&dialect, args)
        }
    };

    if let CommandResolution::Run(command) = &mut resolution.outcome {
        command.env.insert(
            "PATH".to_string(),
            vite_shared::format_path_prepended(manager.get_bin_prefix()),
        );
    }

    Ok(resolution)
}

fn parse_version(manager: &PackageManager) -> Result<Version, Error> {
    Version::parse(&manager.version).map_err(|source| Error::InvalidPackageManagerVersion {
        manager: manager.client,
        version: manager.version.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::ApproveBuildsArgs;

    fn package_manager(client: PackageManagerType, version: &str) -> PackageManager {
        let workspace_root = vite_path::current_dir().unwrap();
        PackageManager {
            client,
            version: version.into(),
            install_dir: workspace_root.join(".test-package-manager"),
        }
    }

    #[test]
    fn npm_latest_uses_unknown_version_fallback() {
        let manager = package_manager(PackageManagerType::Npm, "latest");
        let resolution = resolve_for_manager(
            &manager,
            ApproveBuildsArgs { packages: vec!["esbuild".to_string()], ..Default::default() },
        )
        .unwrap();
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected command resolution");
        };

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["approve-scripts", "esbuild"]);
        let path = command.env.get("PATH").expect("resolved command should bind PATH");
        assert_eq!(
            std::env::split_paths(path).next().as_deref(),
            Some(manager.get_bin_prefix().as_path())
        );
    }

    #[test]
    fn invalid_non_npm_version_is_an_error() {
        let manager = package_manager(PackageManagerType::Pnpm, "latest");
        let error = resolve_for_manager(&manager, ApproveBuildsArgs::default()).unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidPackageManagerVersion {
                manager: PackageManagerType::Pnpm,
                ref version,
                ..
            } if version == "latest"
        ));
    }
}
