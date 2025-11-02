use std::process::ExitStatus, str::FromStr};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, run_command,
};

/// Output format for the outdated command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Table format (default)
    Table,
    /// List format (parseable)
    List,
    /// JSON format
    Json,
}

impl Format {
    /// Convert format to string representation
    pub const fn as_str(self) -> &'static str {
        match self {
            Format::Table => "table",
            Format::List => "list",
            Format::Json => "json",
        }
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(Format::Table),
            "list" => Ok(Format::List),
            "json" => Ok(Format::Json),
            _ => Err(format!("Invalid format '{}'. Valid formats: table, list, json", s)),
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Options for the outdated command.
#[derive(Debug, Default)]
pub struct OutdatedCommandOptions<'a> {
    pub packages: &'a [String],
    pub long: bool,
    pub format: Option<Format>,
    pub recursive: bool,
    pub filters: Option<&'a [String]>,
    pub workspace_root: bool,
    pub prod: bool,
    pub dev: bool,
    pub no_optional: bool,
    pub compatible: bool,
    pub sort_by: Option<&'a str>,
    pub global: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the outdated command with the package manager.
    /// Return the exit status of the command.
    #[must_use]
    pub async fn run_outdated_command(
        &self,
        options: &OutdatedCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_outdated_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the outdated command.
    #[must_use]
    pub fn resolve_outdated_command(
        &self,
        options: &OutdatedCommandOptions,
    ) -> ResolveCommandResult {
        let bin_path = self.get_bin_path();
        let envs = self.get_envs();
        let mut args: Vec<String> = Vec::new();

        // Global packages should use npm cli only
        if options.global {
            
            Self::format_npm_outdated_args(&mut args, options);
            args.push("-g".into());
        } else {
            match self.client {
                PackageManagerType::Pnpm => {
                    

                    // pnpm: --filter must come before command
                    if let Some(filters) = options.filters {
                        for filter in filters {
                            args.push("--filter".into());
                            args.push(filter.clone());
                        }
                    }

                    args.push("outdated".into());

                    // Handle format option
                    if let Some(format) = options.format {
                        args.push("--format".into());
                        args.push(format.as_str().into());
                    }

                    if options.long {
                        args.push("--long".into());
                    }

                    if options.workspace_root {
                        args.push("--workspace-root".into());
                    }

                    if options.recursive {
                        args.push("--recursive".into());
                    }

                    if options.prod {
                        args.push("--prod".into());
                    }

                    if options.dev {
                        args.push("--dev".into());
                    }

                    if options.no_optional {
                        args.push("--no-optional".into());
                    }

                    if options.compatible {
                        args.push("--compatible".into());
                    }

                    if let Some(sort_by) = options.sort_by {
                        args.push("--sort-by".into());
                        args.push(sort_by.into());
                    }

                    // Add packages (pnpm supports glob patterns)
                    args.extend_from_slice(options.packages);
                }
                PackageManagerType::Yarn => {
                    

                    // Check if yarn@2+ (uses upgrade-interactive)
                    if !self.version.starts_with("1.") {
                        println!(
                            "Note: yarn@2+ uses 'yarn upgrade-interactive' for checking outdated packages"
                        );
                        args.push("upgrade-interactive".into());

                        // Warn about unsupported flags
                        if options.format.is_some() {
                            println!("Warning: --format not supported by yarn@2+");
                        }
                    } else {
                        // yarn@1
                        args.push("outdated".into());

                        // Add packages (yarn@1 supports package names)
                        args.extend_from_slice(options.packages);

                        // yarn@1 supports --json format
                        if let Some(format) = options.format {
                            match format {
                                Format::Json => args.push("--json".into()),
                                Format::List => {
                                    println!("Warning: yarn@1 not support list format");
                                }
                                Format::Table => {} // Default, no flag needed
                            }
                        }
                    }

                    // Common warnings
                    if options.long {
                        println!("Warning: --long not supported by yarn");
                    }
                    if options.workspace_root {
                        println!("Warning: --workspace-root not supported by yarn");
                    }
                    if options.recursive {
                        println!("Warning: --recursive not supported by yarn");
                    }
                    if let Some(filters) = options.filters {
                        if !filters.is_empty() {
                            println!("Warning: --filter not supported by yarn");
                        }
                    }
                    if options.prod || options.dev {
                        println!("Warning: --prod/--dev not supported by yarn");
                    }
                    if options.no_optional {
                        println!("Warning: --no-optional not supported by yarn");
                    }
                    if options.compatible {
                        println!("Warning: --compatible not supported by yarn");
                    }
                    if options.sort_by.is_some() {
                        println!("Warning: --sort-by not supported by yarn");
                    }
                }
                PackageManagerType::Npm => {
                    
                    Self::format_npm_outdated_args(&mut args, options);
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path, args, envs }
    }

    fn format_npm_outdated_args(args: &mut Vec<String>, options: &OutdatedCommandOptions) {
        args.push("outdated".into());

        // npm format flags - translate from --format
        if let Some(format) = options.format {
            match format {
                Format::Json => args.push("--json".into()),
                Format::List => args.push("--parseable".into()),
                Format::Table => {} // Default, no flag needed
            }
        }

        if options.long {
            args.push("--long".into());
        }

        // npm workspace flags - translate from --filter
        if let Some(filters) = options.filters {
            for filter in filters {
                args.push("--workspace".into());
                args.push(filter.clone());
            }
        }

        // npm uses --include-workspace-root when workspace_root is set
        if options.workspace_root {
            args.push("--include-workspace-root".into());
        }

        // npm --all translates from -r/--recursive
        if options.recursive {
            args.push("--all".into());
        }

        // Add packages (npm supports package names)
        args.extend_from_slice(options.packages);

        // Warn about pnpm-specific flags
        if options.prod || options.dev {
            println!("Warning: --prod/--dev not supported by npm");
        }
        if options.no_optional {
            println!("Warning: --no-optional not supported by npm");
        }
        if options.compatible {
            println!("Warning: --compatible not supported by npm");
        }
        if options.sort_by.is_some() {
            println!("Warning: --sort-by not supported by npm");
        }
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
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_outdated_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions { ..Default::default() });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated"]);
    }

    #[test]
    fn test_pnpm_outdated_with_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let packages = vec!["*babel*".to_string(), "eslint-*".to_string()];
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            packages: &packages,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "*babel*", "eslint-*"]);
    }

    #[test]
    fn test_pnpm_outdated_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            format: Some(Format::Json),
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--format", "json"]);
    }

    #[test]
    fn test_npm_outdated_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["outdated"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_outdated_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            format: Some(Format::Json),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["outdated", "--json"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_yarn_outdated_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.19");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions { ..Default::default() });
        assert_eq!(result.args, vec!["outdated"]);
        assert!(result.bin_path.ends_with("/yarn") || result.bin_path.ends_with("\\yarn"), "Expected path to end with yarn, got: {}", result.bin_path);
    }

    #[test]
    fn test_pnpm_outdated_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            filters: Some(&filters),
            recursive: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["--filter", "app", "outdated", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_prod_only() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm
            .resolve_outdated_command(&OutdatedCommandOptions { prod: true, ..Default::default() });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--prod"]);
    }

    #[test]
    fn test_npm_outdated_list_format() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            format: Some(Format::List),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["outdated", "--parseable"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_outdated_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.args, vec!["outdated", "--all"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_npm_outdated_with_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["outdated", "--workspace", "app"]);
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
    }

    #[test]
    fn test_global_outdated() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            global: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "-g"]);
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            workspace_root: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--workspace-root"]);
    }

    #[test]
    fn test_pnpm_outdated_with_workspace_root_and_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            workspace_root: true,
            recursive: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--workspace-root", "--recursive"]);
    }

    #[test]
    fn test_pnpm_outdated_with_all_flags() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let packages = vec!["react".to_string()];
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            packages: &packages,
            long: true,
            format: Some(Format::Json),
            recursive: true,
            filters: Some(&filters),
            workspace_root: true,
            prod: true,
            compatible: true,
            sort_by: Some("name"),
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/pnpm") || result.bin_path.ends_with("\\pnpm"), "Expected path to end with pnpm, got: {}", result.bin_path);
        assert_eq!(
            result.args,
            vec![
                "--filter",
                "app",
                "outdated",
                "--format",
                "json",
                "--long",
                "--workspace-root",
                "--recursive",
                "--prod",
                "--compatible",
                "--sort-by",
                "name",
                "react"
            ]
        );
    }

    #[test]
    fn test_npm_outdated_with_workspace_root() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            workspace_root: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--include-workspace-root"]);
    }

    #[test]
    fn test_npm_outdated_with_workspace_root_and_workspace() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_outdated_command(&OutdatedCommandOptions {
            filters: Some(&filters),
            workspace_root: true,
            ..Default::default()
        });
        assert!(result.bin_path.ends_with("/npm") || result.bin_path.ends_with("\\npm") || result.bin_path == "npm", "Expected path to end with npm or be npm, got: {}", result.bin_path);
        assert_eq!(result.args, vec!["outdated", "--workspace", "app", "--include-workspace-root"]);
    }
}
