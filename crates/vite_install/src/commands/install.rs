use std::iter;

use crate::package_manager::{PackageManager, ResolveCommandResult};

impl PackageManager {
    /// Resolve the install command.
    pub fn resolve_install_command(&self, args: &Vec<String>) -> ResolveCommandResult {
        ResolveCommandResult {
            bin_path: self.get_bin_path(),
            args: iter::once("install")
                .chain(args.iter().map(String::as_str))
                .map(String::from)
                .collect(),
            envs: self.get_envs(),
        }
    }
}
