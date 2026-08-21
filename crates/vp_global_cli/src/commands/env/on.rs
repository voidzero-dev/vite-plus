//! Enable managed mode command.
//!
//! Handles `vp env on` to set shim mode to "managed" - shims always use vite-plus Node.js.

use std::process::ExitStatus;

use super::{
    config::{ShimMode, load_config, save_config},
    spec::EnvScope,
};
use crate::{error::Error, help};

/// Execute the `vp env on` command.
pub async fn execute(scope: Option<String>) -> Result<ExitStatus, Error> {
    let scope = EnvScope::parse(scope.as_deref())?;
    if matches!(scope, EnvScope::PackageManager(_)) {
        return Err(Error::Other("on accepts only node or pm as a scope".into()));
    }
    let mut config = load_config().await?;
    config.set_shim_modes(
        scope.includes_node(),
        scope.includes_package_managers(),
        ShimMode::Managed,
    );
    save_config(&config).await?;

    let component = match scope {
        EnvScope::All => "Node.js and package-manager management",
        EnvScope::Node => "Node.js management",
        EnvScope::PackageManagers => "Package-manager management",
        EnvScope::PackageManager(_) => unreachable!(),
    };
    println!("\u{2713} {component} set to managed.");
    println!();
    println!("Selected commands and shims will now use Vite+ managed tools.");
    println!();
    println!("Run {} to prefer system tools instead.", help::accent_command("vp env off"));

    Ok(ExitStatus::default())
}
