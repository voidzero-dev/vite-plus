//! Managed global package utilities.

use std::{
    collections::HashMap,
    fs::File,
    io::{IsTerminal, Read},
    process::Stdio,
    time::Duration,
};

use flate2::read::GzDecoder;
use futures::{StreamExt, stream::FuturesUnordered};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use tar::Archive;
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
        let output = npm_view(&self.npm_path, &self.node_bin_dir, package_spec, "version").await?;

        parse_npm_view_version(&output)
    }
}

async fn npm_view(
    npm_path: &AbsolutePathBuf,
    node_bin_dir: &AbsolutePathBuf,
    package_spec: &str,
    field: &str,
) -> Result<Vec<u8>, Error> {
    let output = Command::new(npm_path.as_path())
        .args(["view", package_spec, field, "--json"])
        .env("PATH", format_path_prepended(node_bin_dir.as_path()))
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

    Ok(output.stdout)
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

#[derive(Debug)]
pub(crate) struct PackageSpec {
    pub name: String,
    pub local_package_json: Option<serde_json::Value>,
}

/// Parse package spec into package name and local package.json if available.
/// For local packages, read package.json from a directory or package tarball.
///
/// It will never return an `Err()` if it is not a local package
pub(crate) fn parse_package_spec(spec: &str) -> Result<PackageSpec, Error> {
    if is_local_package_spec(spec) {
        let package_json = read_local_package_json(spec)?;
        let Some(package_name) = package_json.get("name").and_then(|name| name.as_str()) else {
            return Err(Error::ConfigError(
                format!("Local package {spec} must have a string name in package.json").into(),
            ));
        };

        Ok(PackageSpec { name: package_name.to_string(), local_package_json: Some(package_json) })
    } else {
        if spec.starts_with('@') {
            if let Some(idx) = spec[1..].find('@') {
                let idx = idx + 1;
                return Ok(PackageSpec { name: spec[..idx].to_string(), local_package_json: None });
            }
            return Ok(PackageSpec { name: spec.to_string(), local_package_json: None });
        }

        if let Some(idx) = spec.find('@') {
            return Ok(PackageSpec { name: spec[..idx].to_string(), local_package_json: None });
        }

        Ok(PackageSpec { name: spec.to_string(), local_package_json: None })
    }
}

fn resolve_local_package_path(spec: &str) -> Result<AbsolutePathBuf, Error> {
    let path_spec = spec.strip_prefix("file:").unwrap_or(spec);
    let path = std::path::Path::new(path_spec);
    if path.is_absolute() {
        AbsolutePathBuf::new(path.to_path_buf())
            .ok_or_else(|| Error::ConfigError(format!("Invalid local package path {spec}").into()))
    } else {
        Ok(current_dir()
            .map_err(|error| {
                Error::ConfigError(format!("Cannot get current directory: {error}").into())
            })?
            .join(path))
    }
}

fn read_local_package_json(spec: &str) -> Result<serde_json::Value, Error> {
    let package_path = resolve_local_package_path(spec)?;
    if package_path.as_path().is_file() && is_package_tarball(package_path.as_path()) {
        return read_package_json_from_tarball(spec, &package_path);
    }

    let package_json_path = package_path.join("package.json");
    let package_json_content =
        std::fs::read_to_string(package_json_path.as_path()).map_err(|error| {
            Error::ConfigError(
                format!(
                    "Failed to read package.json for local package {spec} at {}: {error}",
                    package_json_path.as_path().display()
                )
                .into(),
            )
        })?;
    serde_json::from_str(&package_json_content).map_err(Error::JsonError)
}

fn is_package_tarball(path: &std::path::Path) -> bool {
    let path = path.to_string_lossy();
    path.ends_with(".tgz") || path.ends_with(".tar.gz")
}

fn read_package_json_from_tarball(
    spec: &str,
    package_path: &AbsolutePathBuf,
) -> Result<serde_json::Value, Error> {
    let file = File::open(package_path.as_path()).map_err(|error| {
        Error::ConfigError(
            format!(
                "Failed to read package tarball {spec} at {}: {error}",
                package_path.as_path().display()
            )
            .into(),
        )
    })?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries().map_err(|error| {
        Error::ConfigError(format!("Failed to read package tarball {spec}: {error}").into())
    })? {
        let mut entry = entry.map_err(|error| {
            Error::ConfigError(format!("Failed to read package tarball {spec}: {error}").into())
        })?;
        let path = entry.path().map_err(|error| {
            Error::ConfigError(format!("Failed to read package tarball {spec}: {error}").into())
        })?;
        if path.as_ref() != std::path::Path::new("package/package.json") {
            continue;
        }

        let mut package_json_content = String::new();
        entry.read_to_string(&mut package_json_content).map_err(|error| {
            Error::ConfigError(
                format!("Failed to read package.json from package tarball {spec}: {error}").into(),
            )
        })?;
        return serde_json::from_str(&package_json_content).map_err(Error::JsonError);
    }

    Err(Error::ConfigError(
        format!("Package tarball {spec} must contain package/package.json").into(),
    ))
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
