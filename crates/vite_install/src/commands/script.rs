use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

/// Options for running package manager scripts.
#[derive(Debug, Default)]
pub struct ScriptCommandOptions<'a> {
    pub scripts: &'a [String],
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run one or more scripts using the current package manager.
    #[must_use]
    pub async fn run_script_command(
        &self,
        options: &ScriptCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_script_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve a package-manager script invocation into an executable, args, and env map.
    #[must_use]
    pub fn resolve_script_command(&self, options: &ScriptCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args = Vec::with_capacity(4 + options.scripts.len());

        args.push("run".into());
        args.extend(options.scripts.iter().cloned());

        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: self.client.to_string(), args, envs }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use vite_path::AbsolutePathBuf;

    use super::*;
    use crate::package_manager::PackageManagerType;

    fn temp_absolute_path() -> AbsolutePathBuf {
        AbsolutePathBuf::new(env::temp_dir()).unwrap()
    }

    fn make_pm(client: PackageManagerType) -> PackageManager {
        PackageManager {
            client,
            package_name: "pm".into(),
            version: "1.0.0".into(),
            hash: None,
            bin_name: client.to_string().into(),
            workspace_root: temp_absolute_path(),
            is_monorepo: false,
            install_dir: temp_absolute_path(),
        }
    }

    #[test]
    fn resolve_script_command_uses_run_subcommand() {
        let pm = make_pm(PackageManagerType::Pnpm);
        let options = ScriptCommandOptions {
            scripts: &["build".to_string()],
            pass_through_args: Some(&["--if-present".to_string()]),
        };

        let result = pm.resolve_script_command(&options);

        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["run", "build", "--if-present"]);
        assert!(result.envs.contains_key("PATH"));
    }
}
