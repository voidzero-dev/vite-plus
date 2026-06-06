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
struct PackageInfoResult {
    package_spec: String,
    info: Result<PackageInfo, Error>,
}

#[derive(Debug)]
pub(crate) struct PackageInfoQuery {
    pub package_spec: String,
    pub package_name: String,
}

#[derive(Debug)]
pub(crate) struct PackageInfo {
    pub version: String,
    pub bins: Vec<String>,
}

pub(crate) struct NpmRegistry {
    npm_path: AbsolutePathBuf,
    node_bin_dir: AbsolutePathBuf,
}

impl NpmRegistry {
    pub(crate) fn from_paths(npm_path: AbsolutePathBuf, node_bin_dir: AbsolutePathBuf) -> Self {
        Self { npm_path, node_bin_dir }
    }

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

        Ok(Self::from_paths(npm_path, node_bin_dir))
    }

    async fn latest_package_info(&self, package_spec: &str) -> Result<PackageInfo, Error> {
        let view_spec = npm_view_spec_from_package_spec(package_spec);
        let output = npm_view(&self.npm_path, &self.node_bin_dir, view_spec, "").await?;

        parse_npm_view_package_info(&output)
    }
}

async fn npm_view(
    npm_path: &AbsolutePathBuf,
    node_bin_dir: &AbsolutePathBuf,
    package_spec: &str,
    field: &str,
) -> Result<Vec<u8>, Error> {
    let mut command = Command::new(npm_path.as_path());
    command.args(["view", package_spec]);
    if !field.is_empty() {
        command.arg(field);
    }
    let output = command
        .arg("--json")
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

pub(crate) async fn latest_package_infos(
    specs: &[String],
    concurrency: usize,
) -> Result<HashMap<String, Result<PackageInfo, Error>>, Error> {
    if specs.is_empty() {
        return Ok(HashMap::new());
    }

    let registry = NpmRegistry::resolve().await?;
    let mut infos = HashMap::with_capacity(specs.len());
    let mut queries = Vec::new();
    for package_spec in specs {
        match parse_package_spec(package_spec) {
            Ok((package_name, _, _)) => {
                queries.push(PackageInfoQuery { package_spec: package_spec.clone(), package_name });
            }
            Err(error) => {
                infos.insert(package_spec.clone(), Err(error));
            }
        }
    }

    infos.extend(latest_package_infos_with_registry(&registry, &queries, concurrency).await?);
    Ok(infos)
}

pub(crate) async fn latest_package_infos_with_registry(
    registry: &NpmRegistry,
    queries: &[PackageInfoQuery],
    concurrency: usize,
) -> Result<HashMap<String, Result<PackageInfo, Error>>, Error> {
    if queries.is_empty() {
        return Ok(HashMap::new());
    }

    let concurrency = concurrency.max(1);
    let mut package_queries = queries.iter();
    let mut infos = HashMap::with_capacity(queries.len());

    let progress = ProgressBar::new(queries.len() as u64);
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

    let mut pending = FuturesUnordered::new();

    loop {
        while pending.len() < concurrency {
            let Some(package_query) = package_queries.next() else { break };
            pending.push(async {
                let package_spec = package_query.package_spec.clone();
                let info = registry.latest_package_info(&package_spec).await;
                PackageInfoResult { package_spec, info }
            });
        }

        if pending.is_empty() {
            break;
        }

        if let Some(info) = pending.next().await {
            progress.inc(1);
            infos.insert(info.package_spec, info.info);
        }
    }
    progress.finish_and_clear();

    Ok(infos)
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

/// Parse package spec into package name, version, and bin names if available.
/// For local packages, read package.json from a directory or package tarball.
///
/// It will never return an `Err()` if it is not a local package
pub(crate) fn parse_package_spec(
    spec: &str,
) -> Result<(String, Option<String>, Option<Vec<String>>), Error> {
    if is_local_package_spec(spec) {
        let package_json = read_local_package_json(spec)?;
        let Some(package_name) = package_json.get("name").and_then(|name| name.as_str()) else {
            return Err(Error::ConfigError(
                format!("Local package {spec} must have a string name in package.json").into(),
            ));
        };

        Ok((package_name.to_string(), None, Some(extract_bin_names(&package_json))))
    } else {
        if spec.starts_with('@') {
            if let Some(idx) = spec[1..].find('@') {
                let idx = idx + 1;
                return Ok((spec[..idx].to_string(), Some(spec[idx + 1..].to_string()), None));
            }
            return Ok((spec.to_string(), None, None));
        }

        if let Some(idx) = spec.find('@') {
            return Ok((spec[..idx].to_string(), Some(spec[idx + 1..].to_string()), None));
        }

        Ok((spec.to_string(), None, None))
    }
}

fn extract_bin_names(package_json: &serde_json::Value) -> Vec<String> {
    let mut bins = Vec::new();

    if let Some(bin) = package_json.get("bin") {
        match bin {
            serde_json::Value::String(_) => {
                if let Some(name) = package_json["name"].as_str() {
                    bins.push(name.split('/').last().unwrap_or(name).to_string());
                }
            }
            serde_json::Value::Object(map) => {
                bins.extend(map.keys().cloned());
            }
            _ => {}
        }
    }

    bins
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

fn npm_view_spec_from_package_spec(spec: &str) -> &str {
    if let Some((_, target)) = parse_npm_alias_spec(spec) { target } else { spec }
}

fn parse_npm_alias_spec(spec: &str) -> Option<(&str, &str)> {
    let (alias, target) = spec.split_once("@npm:")?;
    if alias.is_empty() || target.is_empty() {
        return None;
    }

    Some((alias, target))
}

fn parse_npm_view_package_info(stdout: &[u8]) -> Result<PackageInfo, Error> {
    let raw = String::from_utf8_lossy(stdout);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::ConfigError("npm view returned empty package metadata".into()));
    }

    let value = serde_json::from_str::<serde_json::Value>(trimmed)?;
    let package = match value {
        serde_json::Value::Array(values) => values
            .iter()
            .rev()
            .find(|value| value.get("version").and_then(|version| version.as_str()).is_some())
            .cloned(),
        value if value.get("version").and_then(|version| version.as_str()).is_some() => Some(value),
        _ => None,
    }
    .ok_or_else(|| {
        Error::ConfigError("npm view returned package metadata without a version".into())
    })?;

    let Some(version) = package["version"].as_str() else {
        return Err(Error::ConfigError(
            "npm view returned package metadata without a version".into(),
        ));
    };
    let version = version.to_string();
    let bins = match package.get("bin") {
        Some(bin) => {
            let Some(name) = package["name"].as_str() else {
                return Err(Error::ConfigError(
                    "npm view returned package metadata with bin but without a name".into(),
                ));
            };
            bin_names_from_value(name, bin)
        }
        None => Vec::new(),
    };

    Ok(PackageInfo { version, bins })
}

fn bin_names_from_value(package_name: &str, bin: &serde_json::Value) -> Vec<String> {
    let default_bin_name = package_name.split('/').last().unwrap_or(package_name).to_string();
    match bin {
        serde_json::Value::String(_) => vec![default_bin_name],
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_info() {
        let info = parse_npm_view_package_info(
            br#"{"name":"typescript","version":"5.0.0","bin":{"tsc":"bin/tsc","tsserver":"bin/tsserver"}}"#,
        )
        .unwrap();
        assert_eq!(info.version, "5.0.0");
        assert_eq!(info.bins, vec!["tsc", "tsserver"]);
    }

    #[test]
    fn parses_package_info_array() {
        let info = parse_npm_view_package_info(
            br#"[{"name":"testnpm2","version":"1.0.0","bin":{"old":"old.js"}},{"name":"testnpm2","version":"2.0.0","bin":"cli.js"}]"#,
        )
        .unwrap();
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.bins, vec!["testnpm2"]);
    }

    #[test]
    fn parses_string_bin_with_manifest_name() {
        let info = parse_npm_view_package_info(
            br#"{"name":"real-package","version":"1.0.0","bin":"cli.js"}"#,
        )
        .unwrap();
        assert_eq!(info.bins, vec!["real-package"]);
    }

    #[test]
    fn parses_package_info_without_bins() {
        let info = parse_npm_view_package_info(br#"{"version":"1.3.0"}"#).unwrap();
        assert_eq!(info.version, "1.3.0");
        assert!(info.bins.is_empty());
    }

    #[test]
    fn rejects_bin_without_name() {
        let error =
            parse_npm_view_package_info(br#"{"version":"1.0.0","bin":"cli.js"}"#).unwrap_err();
        assert!(error.to_string().contains("without a name"));
    }

    #[test]
    fn parses_npm_alias_view_target() {
        assert_eq!(npm_view_spec_from_package_spec("alias@npm:real"), "real");
        assert_eq!(npm_view_spec_from_package_spec("alias@npm:real@1.0.0"), "real@1.0.0");
        assert_eq!(npm_view_spec_from_package_spec("@scope/alias@npm:real"), "real");
        assert_eq!(
            npm_view_spec_from_package_spec("@scope/alias@npm:@scope/real@1.0.0"),
            "@scope/real@1.0.0"
        );
        assert_eq!(npm_view_spec_from_package_spec("@scope/pkg@1.0.0"), "@scope/pkg@1.0.0");
    }

    #[test]
    fn rejects_empty_output() {
        let error = parse_npm_view_package_info(b"\n").unwrap_err();
        assert!(error.to_string().contains("empty package metadata"));
    }
}
