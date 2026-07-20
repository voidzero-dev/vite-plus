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
    /// Returns `ExitStatus` with success (0) if the command is not supported.
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
                let is_berry = self.is_yarn_berry();
                if !is_berry {
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
            PackageManagerType::Bun => {
                bin_name = "bun".into();
                args.push("audit".into());

                if options.fix {
                    output::warn("bun audit does not support --fix");
                    return None;
                }

                if let Some(level) = options.level {
                    args.push("--audit-level".into());
                    args.push(level.to_string());
                }

                if options.production {
                    output::warn("--production not supported by bun audit, ignoring flag");
                }

                if options.json {
                    args.push("--json".into());
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

    use super::*;
    use crate::package_manager::create_mock_package_manager_with_version as create_mock_package_manager;

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
    fn test_bun_audit_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.bin_path, "bun");
        assert!(result.args.contains(&"audit".to_string()), "should contain 'audit'");
        assert!(!result.args.contains(&"pm".to_string()), "should NOT use 'bun pm audit'");
    }

    #[test]
    fn test_bun_audit_level() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: false,
            level: Some("high"),
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(
            result.args.contains(&"--audit-level".to_string()),
            "should use --audit-level not --level"
        );
        assert!(result.args.contains(&"high".to_string()));
    }

    #[test]
    fn test_bun_audit_fix_not_supported() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
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
    fn test_bun_audit_json() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.11");
        let result = pm.resolve_audit_command(&AuditCommandOptions {
            fix: false,
            json: true,
            level: None,
            production: false,
            pass_through_args: None,
        });
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.args, vec!["audit", "--json"]);
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
