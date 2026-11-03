//! List installed global packages.

use std::process::ExitStatus;

use console::style;

use crate::{commands::env::package_metadata::PackageMetadata, error::Error};

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
            vp_shared::output::print_stdout_line(format_args!("[]"));
        } else if pattern.is_some() {
            vp_shared::output::print_stdout_line(format_args!(
                "No global packages matching '{}'.",
                pattern.unwrap()
            ));
            vp_shared::output::print_stdout_line(format_args!(""));
            vp_shared::output::print_stdout_line(format_args!(
                "Run 'vp list -g' to see all installed global packages."
            ));
        } else {
            vp_shared::output::print_stdout_line(format_args!("No global packages installed."));
            vp_shared::output::print_stdout_line(format_args!(""));
            vp_shared::output::print_stdout_line(format_args!(
                "Install packages with: vp install -g <package>"
            ));
        }
        return Ok(ExitStatus::default());
    }

    if json {
        let json_output = serde_json::to_string_pretty(&packages).map_err(Error::JsonError)?;
        vp_shared::output::print_stdout_line(format_args!("{json_output}"));
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
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_node$}{:>gap$}{}",
            col_pkg, "", col_node, "", col_bins
        ));
        vp_shared::output::print_stdout_line(format_args!(
            "{:<w_pkg$}{:>gap$}{:<w_node$}{:>gap$}{}",
            "---", "", "---", "", "---"
        ));

        for pkg in &packages {
            let name = format!("{:<w_pkg$}", format!("{}@{}", pkg.name, pkg.version));
            let bins = pkg.bins.join(", ");
            vp_shared::output::print_stdout_line(format_args!(
                "{}{:>gap$}{:<w_node$}{:>gap$}{}",
                style(&name).blue().bright(),
                "",
                pkg.platform.node,
                "",
                bins
            ));
        }
    }

    Ok(ExitStatus::default())
}
