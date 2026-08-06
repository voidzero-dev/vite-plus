//! Enable system-first mode command.
//!
//! Handles `vp env off` to set shim mode to "system_first" -
//! shims prefer system Node.js, fallback to managed if not found.

use std::process::ExitStatus;

use super::config::{ShimMode, load_config, save_config};
use crate::{error::Error, help};

/// Execute the `vp env off` command.
pub async fn execute() -> Result<ExitStatus, Error> {
    let mut config = load_config().await?;

    if config.shim_mode == ShimMode::SystemFirst {
        println!("Node.js management is already set to system-first.");
        println!(
            "All vp commands and shims will prefer system Node.js, falling back to managed if not found."
        );
        return Ok(ExitStatus::default());
    }

    config.shim_mode = ShimMode::SystemFirst;
    save_config(&config).await?;

    println!("\u{2713} Node.js management set to system-first.");
    println!();
    println!(
        "All vp commands and shims will now prefer system Node.js, falling back to managed if not found."
    );
    println!();
    println!("Run {} to always use Vite+ managed Node.js.", help::accent_command("vp env on"));

    Ok(ExitStatus::default())
}
