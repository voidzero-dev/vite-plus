//! Enable managed mode command.
//!
//! Handles `vp env on` to set shim mode to "managed" - shims always use vite-plus Node.js.

use std::process::ExitStatus;

use super::config::{ShimMode, load_config, save_config};
use crate::error::Error;

/// Execute the `vp env on` command.
pub async fn execute() -> Result<ExitStatus, Error> {
    let mut config = load_config().await?;

    if config.shim_mode == ShimMode::Managed {
        println!("Shim mode is already set to managed.");
        println!("Shims will always use vite-plus managed Node.js.");
        return Ok(ExitStatus::default());
    }

    config.shim_mode = ShimMode::Managed;
    save_config(&config).await?;

    println!("\u{2713} Shim mode set to managed.");
    println!();
    println!("Shims will now always use vite-plus managed Node.js.");
    println!("Run 'vp env off' to prefer system Node.js instead.");

    Ok(ExitStatus::default())
}
