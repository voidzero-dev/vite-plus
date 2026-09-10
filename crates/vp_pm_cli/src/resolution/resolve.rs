use semver::Version;
use vp_shared::{PrependOptions, ToolPathEnv};

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
        let mut env = ToolPathEnv::from_env();
        env.prepend(manager.get_bin_prefix(), &manager.bin_names(), PrependOptions::default())?;
        for (key, value) in env.into_envs() {
            command.env.insert(key.to_string(), value.to_string_lossy().into_owned());
        }
        match manager.client {
            PackageManagerType::Pnpm => {
                // Vite+ manages pnpm, so its self-update notification is not useful here.
                command.env.insert("PNPM_CONFIG_UPDATE_NOTIFIER".to_string(), "false".to_string());
            }
            PackageManagerType::Yarn if command.program == "yarn" => {
                match parse_version(manager)?.major {
                    0 | 1 => {
                        command.env.insert(
                            "YARN_DISABLE_SELF_UPDATE_CHECK".to_string(),
                            "true".to_string(),
                        );
                    }
                    4.. => {
                        // Yarn 4 includes version notices in its daily tips.
                        command.env.insert("YARN_ENABLE_TIPS".to_string(), "false".to_string());
                    }
                    // Yarn 2 and 3 reject these settings and have no version notices.
                    _ => {}
                }
            }
            _ => {}
        }
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
    use crate::resolution::{ApproveBuildsArgs, DlxArgs, InstallArgs};

    fn package_manager(client: PackageManagerType, version: &str) -> PackageManager {
        let workspace_root = vt_path::current_dir().unwrap();
        PackageManager {
            client,
            version: version.into(),
            bin_prefix: workspace_root.join(".test-package-manager").join("bin"),
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

    #[test]
    fn yarn_update_settings_match_the_major_version() {
        for (client, version, classic, tips) in [
            (PackageManagerType::Yarn, "1.22.22", Some("true"), None),
            (PackageManagerType::Yarn, "2.4.2", None, None),
            (PackageManagerType::Yarn, "3.8.7", None, None),
            (PackageManagerType::Yarn, "4.0.0", None, Some("false")),
            (PackageManagerType::Yarn, "4.12.0", None, Some("false")),
            (PackageManagerType::Npm, "11.13.0", None, None),
            (PackageManagerType::Pnpm, "12.3.4", None, None),
            (PackageManagerType::Bun, "1.0.0", None, None),
        ] {
            let manager = package_manager(client, version);
            let resolution = resolve_for_manager(&manager, InstallArgs::default()).unwrap();
            let CommandResolution::Run(command) = resolution.outcome else {
                panic!("expected install command");
            };

            assert_eq!(
                command.env.get("YARN_DISABLE_SELF_UPDATE_CHECK").map(String::as_str),
                classic,
                "{client}@{version}"
            );
            assert_eq!(
                command.env.get("YARN_ENABLE_TIPS").map(String::as_str),
                tips,
                "{client}@{version}"
            );
            assert!(!command.env.contains_key("YARN_ENABLE_TELEMETRY"));
        }
    }

    #[test]
    fn yarn_classic_npx_fallback_uses_only_npm_update_settings() {
        let manager = package_manager(PackageManagerType::Yarn, "1.22.22");
        let resolution = resolve_for_manager(
            &manager,
            DlxArgs { args: vec!["create-vue".to_string()], ..Default::default() },
        )
        .unwrap();
        let CommandResolution::Run(command) = resolution.outcome else {
            panic!("expected npx command");
        };

        assert_eq!(command.program, "npx");
        assert_eq!(
            command.env.get("npm_config_update_notifier").map(String::as_str),
            Some("false")
        );
        assert!(!command.env.contains_key("YARN_DISABLE_SELF_UPDATE_CHECK"));
        assert!(!command.env.contains_key("YARN_ENABLE_TIPS"));
    }

    #[test]
    fn only_npm_installs_disable_npm_update_notifications() {
        for (client, version, expected) in [
            (PackageManagerType::Npm, "11.13.0", Some("false")),
            (PackageManagerType::Npm, "12.0.2", Some("false")),
            (PackageManagerType::Npm, "latest", Some("false")),
            (PackageManagerType::Pnpm, "12.3.4", None),
            (PackageManagerType::Yarn, "4.0.0", None),
            (PackageManagerType::Bun, "1.0.0", None),
        ] {
            let manager = package_manager(client, version);
            let resolution = resolve_for_manager(&manager, InstallArgs::default()).unwrap();
            let CommandResolution::Run(command) = resolution.outcome else {
                panic!("expected install command");
            };

            assert_eq!(
                command.env.get("npm_config_update_notifier").map(String::as_str),
                expected,
                "{client}@{version}"
            );
        }
    }

    #[test]
    fn only_pnpm_installs_disable_update_notifications() {
        for (client, version, expected) in [
            (PackageManagerType::Pnpm, "11.25.0", Some("false")),
            (PackageManagerType::Pnpm, "12.3.4", Some("false")),
            (PackageManagerType::Npm, "11.0.0", None),
            (PackageManagerType::Yarn, "4.0.0", None),
            (PackageManagerType::Bun, "1.0.0", None),
        ] {
            let manager = package_manager(client, version);
            let resolution = resolve_for_manager(&manager, InstallArgs::default()).unwrap();
            let CommandResolution::Run(command) = resolution.outcome else {
                panic!("expected install command");
            };

            assert_eq!(
                command.env.get("PNPM_CONFIG_UPDATE_NOTIFIER").map(String::as_str),
                expected,
                "{client}@{version}"
            );
        }
    }
}
