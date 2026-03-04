//! Enable system-first mode command.
//!
//! Handles `vp env off` to set shim mode to "system_first" -
//! shims prefer system Node.js, fallback to managed if not found.

use std::process::ExitStatus;

use owo_colors::OwoColorize;

use super::config::{ShimMode, load_config, save_config};
use crate::{error::Error, help};

fn accent_command(command: &str) -> String {
    if help::should_style_help() {
        format!("`{}`", command.bright_blue())
    } else {
        format!("`{command}`")
    }
}

/// Execute the `vp env off` command.
pub async fn execute() -> Result<ExitStatus, Error> {
    let mut config = load_config().await?;

    if config.shim_mode == ShimMode::SystemFirst {
        println!("Shim mode is already set to system-first.");
        println!(
            "Shims will prefer system Node.js, falling back to Vite+ managed Node.js if not found."
        );
        return Ok(ExitStatus::default());
    }

    config.shim_mode = ShimMode::SystemFirst;
    save_config(&config).await?;

    println!("\u{2713} Shim mode set to system-first.");
    println!();
    println!(
        "Shims will now prefer system Node.js, falling back to Vite+ managed Node.js if not found."
    );
    println!();
    println!("Run {} to always use the Vite+ managed Node.js.", accent_command("vp env on"));

    Ok(ExitStatus::default())
}
