//! List installed global packages.

use std::process::ExitStatus;

use chrono::Local;

use super::package_metadata::PackageMetadata;
use crate::error::Error;

/// Execute the packages command.
pub async fn execute(json: bool, pattern: Option<&str>) -> Result<ExitStatus, Error> {
    let all_packages = PackageMetadata::list_all().await?;

    let packages: Vec<_> = if let Some(pat) = pattern {
        let pat_lower = pat.to_lowercase();
        all_packages.into_iter().filter(|p| p.name.to_lowercase().contains(&pat_lower)).collect()
    } else {
        all_packages
    };

    if packages.is_empty() {
        if json {
            println!("[]");
        } else if pattern.is_some() {
            println!("No global packages matching '{}'.", pattern.unwrap());
            println!();
            println!("Run 'vp list -g' to see all installed global packages.");
        } else {
            println!("No global packages installed.");
            println!();
            println!("Install packages with: vp install -g <package>");
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
            let installed_local = pkg.installed_at.with_timezone(&Local);
            let installed_str = installed_local.format("%Y-%m-%d %H:%M:%S").to_string();
            println!("  Package: {}@{}", pkg.name, pkg.version);
            if !pkg.bins.is_empty() {
                println!("  Binaries: {}", pkg.bins.join(", "));
            }
            println!("  Node.js: {}", pkg.platform.node);
            println!("  Installed: {}", installed_str);
            println!();
        }
    }

    Ok(ExitStatus::default())
}
