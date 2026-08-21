//! Enable system-first mode command.
//!
//! Handles `vp env off` to set shim mode to "system_first" -
//! shims prefer system Node.js, fallback to managed if not found.

use std::process::ExitStatus;

use super::{
    config::{ShimMode, load_config, save_config},
    spec::EnvScope,
};
use crate::{error::Error, help};

/// Execute the `vp env off` command.
pub async fn execute(scope: Option<String>) -> Result<ExitStatus, Error> {
    let scope = EnvScope::parse(scope.as_deref())?;
    if matches!(scope, EnvScope::PackageManager(_)) {
        return Err(Error::Other("off accepts only node or pm as a scope".into()));
    }
    let mut config = load_config().await?;
    config.set_shim_modes(
        scope.includes_node(),
        scope.includes_package_managers(),
        ShimMode::SystemFirst,
    );
    save_config(&config).await?;

    let component = match scope {
        EnvScope::All => "Node.js and package-manager management",
        EnvScope::Node => "Node.js management",
        EnvScope::PackageManagers => "Package-manager management",
        EnvScope::PackageManager(_) => unreachable!(),
    };
    println!("\u{2713} {component} set to system-first.");
    println!();
    println!(
        "Selected commands and shims will now prefer system tools, falling back to managed tools."
    );
    println!();
    println!("Run {} to always use Vite+ managed tools.", help::accent_command("vp env on"));

    Ok(ExitStatus::default())
}
