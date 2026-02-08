//! List installed global packages.

use std::process::ExitStatus;

use owo_colors::OwoColorize;

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
        let col_pkg = "Package";
        let col_node = "Node version";
        let col_bins = "Binaries";

        let mut w_pkg = col_pkg.len();
        let mut w_node = col_node.len();

        for pkg in &packages {
            let name = format!("{}@{}", pkg.name, pkg.version);
            w_pkg = w_pkg.max(name.len());
            w_node = w_node.max(pkg.platform.node.len());
        }

        let gap = 3;
        println!("{:<w_pkg$}{:>gap$}{:<w_node$}{:>gap$}{}", col_pkg, "", col_node, "", col_bins);
        println!("{:<w_pkg$}{:>gap$}{:<w_node$}{:>gap$}{}", "---", "", "---", "", "---");

        for pkg in &packages {
            let name = format!("{:<w_pkg$}", format!("{}@{}", pkg.name, pkg.version));
            let bins = pkg.bins.join(", ");
            println!(
                "{}{:>gap$}{:<w_node$}{:>gap$}{}",
                name.bright_blue(),
                "",
                pkg.platform.node,
                "",
                bins
            );
        }
    }

    Ok(ExitStatus::default())
}
