use std::{collections::HashMap, iter};

use crate::package_manager::{PackageManager, ResolveCommandResult, format_path_env};

impl PackageManager {
    /// Resolve the install command.
    pub fn resolve_install_command(&self, args: &Vec<String>) -> ResolveCommandResult {
        ResolveCommandResult {
            bin_path: self.bin_name.to_string(),
            args: iter::once("install")
                .chain(args.iter().map(String::as_str))
                .map(String::from)
                .collect(),
            envs: HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]),
        }
    }
}
