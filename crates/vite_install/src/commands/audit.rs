use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the audit command.
#[derive(Debug)]
pub struct AuditCommandOptions<'a> {
    pub fix: bool,
    pub json: bool,
    pub level: Option<&'a str>,
    pub production: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the audit command with the package manager.
    /// Returns ExitStatus with success (0) if the command is not supported.
    #[must_use]
    pub async fn run_audit_command(
        &self,
        options: &AuditCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolve_command) = self.resolve_audit_command(options) else {
            return Ok(ExitStatus::default());
        };
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the audit command.
    /// Returns None if the command is not supported by the package manager.
    #[must_use]
    pub fn resolve_audit_command(
        &self,
        options: &AuditCommandOptions,
    ) -> Option<ResolveCommandResult> {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

        match self.client {
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                if options.fix {
                    args.push("audit".into());
                    args.push("fix".into());
                } else {
                    args.push("audit".into());
                }

                if let Some(level) = options.level {
                    args.push("--audit-level".into());
                    args.push(level.to_string());
                }

                if options.production {
                    args.push("--omit=dev".into());
                }

                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("audit".into());

                if options.fix {
                    args.push("--fix".into());
                }

                if let Some(level) = options.level {
                    args.push("--audit-level".into());
                    args.push(level.to_string());
                }

                if options.production {
                    args.push("--prod".into());
                }

                if options.json {
                    args.push("--json".into());
                }
            }
            PackageManagerType::Yarn => {
                let is_yarn1 = self.version.starts_with("1.");

                if is_yarn1 {
                    if options.fix {
                        output::warn("yarn v1 audit does not support --fix");
                        return None;
                    }

                    bin_name = "yarn".into();
                    args.push("audit".into());

                    if let Some(level) = options.level {
                        args.push("--level".into());
                        args.push(level.to_string());
                    }

                    if options.production {
                        args.push("--groups".into());
                        args.push("dependencies".into());
                    }

                    if options.json {
                        args.push("--json".into());
                    }
                } else {
                    if options.fix {
                        output::warn("yarn berry audit does not support --fix");
                        return None;
                    }

                    bin_name = "yarn".into();
                    args.push("npm".into());
                    args.push("audit".into());

                    if let Some(level) = options.level {
                        args.push("--severity".into());
                        args.push(level.to_string());
                    }

                    if options.production {
                        args.push("--environment".into());
                        args.push("production".into());
                    }

                    if options.json {
                        args.push("--json".into());
                    }
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        Some(ResolveCommandResult { bin_path: bin_name, args, envs })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_mock_package_manager(pm_type: PackageManagerType, version: &str) -> PackageManager {
        let _temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(_temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            is_monorepo: false,
            install_dir,
        }
    }

    #[test]
    fn test_npm_audit() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["audit"]);
    }

    #[test]
    fn test_npm_audit_fix() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: true,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["audit", "fix"]);
    }

    #[test]
    fn test_pnpm_audit_fix() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: true,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["audit", "--fix"]);
    }

    #[test]
    fn test_yarn1_audit() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["audit"]);
    }

    #[test]
    fn test_yarn1_audit_fix_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: true,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_none());
    }

    #[test]
    fn test_yarn2_audit() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "audit"]);
    }

    #[test]
    fn test_yarn2_audit_fix_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: true,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_none());
    }

    #[test]
    fn test_audit_with_level_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: Some("high"),
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["audit", "--audit-level", "high"]);
    }

    #[test]
    fn test_audit_with_level_yarn1() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: Some("high"),
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["audit", "--level", "high"]);
    }

    #[test]
    fn test_audit_with_level_yarn2() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: Some("high"),
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "audit", "--severity", "high"]);
    }
}
