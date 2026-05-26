//! Managed global package utilities.

use std::{collections::HashMap, io::IsTerminal, process::Stdio, time::Duration};

use futures::{StreamExt, stream::FuturesUnordered};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tokio::process::Command;
use vite_path::{AbsolutePathBuf, current_dir};
use vite_shared::format_path_prepended;

use crate::{commands::env::config::resolve_version, error::Error};

pub mod install;
pub mod outdated;
pub mod packages;

/// Core shims that should not be overwritten by package binaries.
pub(crate) const CORE_SHIMS: &[&str] = &["node", "npm", "npx", "vp"];

#[derive(Debug)]
struct PackageVersion {
    package_spec: String,
    version: Result<String, Error>,
}

struct NpmRegistry {
    npm_path: AbsolutePathBuf,
    node_bin_dir: AbsolutePathBuf,
}

impl NpmRegistry {
    async fn resolve() -> Result<Self, Error> {
        let cwd = current_dir().map_err(|error| {
            Error::ConfigError(format!("Cannot get current directory: {error}").into())
        })?;
        let resolution = resolve_version(&cwd).await?;
        let runtime = vite_js_runtime::download_runtime(
            vite_js_runtime::JsRuntimeType::Node,
            &resolution.version,
        )
        .await?;

        let node_bin_dir = runtime.get_bin_prefix();
        let npm_path =
            if cfg!(windows) { node_bin_dir.join("npm.cmd") } else { node_bin_dir.join("npm") };

        Ok(Self { npm_path, node_bin_dir })
    }

    async fn latest_package_version(&self, package_spec: &str) -> Result<String, Error> {
        let output = Command::new(self.npm_path.as_path())
            .args(["view", package_spec, "version", "--json"])
            .env("PATH", format_path_prepended(self.node_bin_dir.as_path()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(Error::ConfigError(
                format!("npm view failed for {package_spec}: {stderr}").into(),
            ));
        }

        parse_npm_view_version(&output.stdout)
    }
}

pub(crate) async fn latest_package_versions(
    specs: &[String],
    concurrency: usize,
) -> Result<HashMap<String, Result<String, Error>>, Error> {
    if specs.is_empty() {
        return Ok(HashMap::new());
    }

    let registry = NpmRegistry::resolve().await?;
    let concurrency = concurrency.max(1);
    let mut package_specs = specs.iter();
    let mut versions = HashMap::with_capacity(specs.len());

    let progress = ProgressBar::new(specs.len() as u64);
    if std::io::stderr().is_terminal() && std::env::var_os("CI").is_none() {
        let style = ProgressStyle::with_template("{spinner:.cyan} {msg} ({pos}/{len})")
            .unwrap_or_else(|_| ProgressStyle::default_spinner())
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]);
        progress.set_style(style);
        progress.set_message("Checking latest package versions");
        progress.enable_steady_tick(Duration::from_millis(80));
    } else {
        progress.set_draw_target(ProgressDrawTarget::hidden());
    }

    let mut queries = FuturesUnordered::new();

    loop {
        while queries.len() < concurrency {
            let Some(package_spec) = package_specs.next() else { break };
            queries.push(async {
                let package_spec = package_spec.clone();
                let version = registry.latest_package_version(&package_spec).await;
                PackageVersion { package_spec, version }
            });
        }

        if queries.is_empty() {
            break;
        }

        if let Some(version) = queries.next().await {
            progress.inc(1);
            versions.insert(version.package_spec, version.version);
        }
    }
    progress.finish_and_clear();

    Ok(versions)
}

/// Return true for package specs that refer to local filesystem content.
pub(crate) fn is_local_package_spec(spec: &str) -> bool {
    spec == "."
        || spec == ".."
        || spec.starts_with("./")
        || spec.starts_with("../")
        || spec.starts_with('/')
        || spec.starts_with("file:")
        || (cfg!(windows)
            && spec.len() >= 3
            && spec.as_bytes()[1] == b':'
            && (spec.as_bytes()[2] == b'\\' || spec.as_bytes()[2] == b'/'))
}

/// Parse package spec into name and optional version.
pub(crate) fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    if spec.starts_with('@') {
        if let Some(idx) = spec[1..].find('@') {
            let idx = idx + 1;
            return (spec[..idx].to_string(), Some(spec[idx + 1..].to_string()));
        }
        return (spec.to_string(), None);
    }

    if let Some(idx) = spec.find('@') {
        return (spec[..idx].to_string(), Some(spec[idx + 1..].to_string()));
    }

    (spec.to_string(), None)
}

fn parse_npm_view_version(stdout: &[u8]) -> Result<String, Error> {
    let raw = String::from_utf8_lossy(stdout);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::ConfigError("npm view returned an empty version".into()));
    }

    match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(serde_json::Value::String(version)) => Ok(version),
        Ok(serde_json::Value::Array(versions)) => {
            let Some(version) = versions.iter().rev().find_map(|version| version.as_str()) else {
                return Err(Error::ConfigError("npm view returned an empty version list".into()));
            };
            Ok(version.to_string())
        }
        _ => Ok(trimmed.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_string_version() {
        let version = parse_npm_view_version(br#""5.0.0""#).unwrap();
        assert_eq!(version, "5.0.0");
    }

    #[test]
    fn parses_json_array_version() {
        let version = parse_npm_view_version(br#"["4.9.5","5.0.0"]"#).unwrap();
        assert_eq!(version, "5.0.0");
    }

    #[test]
    fn parses_plain_version() {
        let version = parse_npm_view_version(b"5.0.0").unwrap();
        assert_eq!(version, "5.0.0");
    }

    #[test]
    fn rejects_empty_output() {
        let error = parse_npm_view_version(b"\n").unwrap_err();
        assert!(error.to_string().contains("empty version"));
    }
}
