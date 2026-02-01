//! Enable system-first mode command.
//!
//! Handles `vp env off` to set shim mode to "system_first" -
//! shims prefer system Node.js, fallback to managed if not found.

use std::process::ExitStatus;

use super::config::{ShimMode, load_config, save_config};
use crate::error::Error;

/// Execute the `vp env off` command.
pub async fn execute() -> Result<ExitStatus, Error> {
    let mut config = load_config().await?;

    if config.shim_mode == ShimMode::SystemFirst {
        println!("Shim mode is already set to system-first.");
        println!("Shims will prefer system Node.js, falling back to managed if not found.");
        return Ok(ExitStatus::default());
    }

    config.shim_mode = ShimMode::SystemFirst;
    save_config(&config).await?;

    println!("\u{2713} Shim mode set to system-first.");
    println!();
    println!("Shims will now prefer system Node.js, falling back to managed if not found.");
    println!("Run 'vp env on' to always use vite-plus managed Node.js.");

    Ok(ExitStatus::default())
}
