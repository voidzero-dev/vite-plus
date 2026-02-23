use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

/// Options for the search command.
#[derive(Debug, Default)]
pub struct SearchCommandOptions<'a> {
    pub terms: &'a [String],
    pub json: bool,
    pub long: bool,
    pub registry: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the search command with the package manager.
    #[must_use]
    pub async fn run_search_command(
        &self,
        options: &SearchCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_search_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the search command.
    /// All package managers delegate to npm search.
    #[must_use]
    pub fn resolve_search_command(&self, options: &SearchCommandOptions) -> ResolveCommandResult {
        let bin_name: String = "npm".to_string();
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        args.push("search".into());

        for term in options.terms {
            args.push(term.clone());
        }

        if options.json {
            args.push("--json".into());
        }

        if options.long {
            args.push("--long".into());
        }

        if let Some(registry_value) = options.registry {
            args.push("--registry".into());
            args.push(registry_value.to_string());
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;
    use crate::package_manager::PackageManagerType;

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
            is_monorepo: false,
            install_dir,
        }
    }

    #[test]
    fn test_search_basic() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let terms = vec!["react".to_string()];
        let result = pm.resolve_search_command(&SearchCommandOptions {
            terms: &terms,
            json: false,
            long: false,
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["search", "react"]);
    }

    #[test]
    fn test_search_with_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let terms = vec!["lodash".to_string()];
        let result = pm.resolve_search_command(&SearchCommandOptions {
            terms: &terms,
            json: true,
            long: false,
            registry: None,
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["search", "lodash", "--json"]);
    }

    #[test]
    fn test_search_multiple_terms() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let terms = vec!["react".to_string(), "hooks".to_string(), "state".to_string()];
        let result = pm.resolve_search_command(&SearchCommandOptions {
            terms: &terms,
            json: false,
            long: true,
            registry: Some("https://registry.npmjs.org"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(
            result.args,
            vec![
                "search",
                "react",
                "hooks",
                "state",
                "--long",
                "--registry",
                "https://registry.npmjs.org",
            ]
        );
    }
}
