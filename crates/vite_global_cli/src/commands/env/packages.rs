//! List installed global packages.

use std::process::ExitStatus;

use super::package_metadata::PackageMetadata;
use crate::error::Error;

/// Execute the packages command.
pub async fn execute(json: bool) -> Result<ExitStatus, Error> {
    let packages = PackageMetadata::list_all().await?;

    if packages.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No global packages installed.");
            println!();
            println!("Install packages with: npm install -g <package>");
        }
        return Ok(ExitStatus::default());
    }

    if json {
        let json_output = serde_json::to_string_pretty(&packages)
            .map_err(|e| Error::ConfigError(format!("Failed to serialize: {e}").into()))?;
        println!("{json_output}");
    } else {
        println!("Installed global packages:");
        println!();

        for pkg in &packages {
            println!("  {} v{} (Node {})", pkg.name, pkg.version, pkg.platform.node);
            if !pkg.bins.is_empty() {
                println!("    Binaries: {}", pkg.bins.join(", "));
            }
            println!();
        }
    }

    Ok(ExitStatus::default())
}
