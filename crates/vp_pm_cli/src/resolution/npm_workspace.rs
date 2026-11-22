//! Select the owner of a workspace dependency edit without changing the caller's directory.
use std::{
    collections::HashMap,
    ffi::OsStr,
    io,
    path::{Path, PathBuf},
    process::Stdio,
};

use cow_utils::CowUtils;
use vt_path::AbsolutePath;

use super::{command::NpmWorkspaceEdit, npm_entry::legacy_root};
use crate::Error;

pub(super) async fn edit_args(
    program: &str,
    cwd: &AbsolutePath,
    edit: NpmWorkspaceEdit,
    mut args: Vec<String>,
    env: &HashMap<String, String>,
) -> Result<Vec<String>, Error> {
    let flags = args.iter().take_while(|arg| arg.as_str() != "--");
    let mut has_selector = std::env::vars_os().any(|(key, value)| {
        !value.is_empty()
            && key.to_str().is_some_and(|key| key.eq_ignore_ascii_case("npm_config_workspace"))
    });
    let mut has_globalconfig = false;
    for arg in flags {
        let (option, value) = arg.split_once('=').unwrap_or((arg, ""));
        match option {
            "--global" | "-g" if value != "false" => return Ok(args),
            "--include-workspace-root" if value == "false" => return Ok(args),
            "--prefix" | "--no-workspaces" | "--no-include-workspace-root" => {
                return Ok(args);
            }
            "--workspace" | "-w" | "--workspaces" | "-ws" => has_selector = true,
            "--globalconfig" => has_globalconfig = true,
            _ => {}
        }
    }
    if edit == NpmWorkspaceEdit::AutomaticRoot && has_selector {
        return Ok(args);
    }

    let Ok(package) = vt_workspace::find_package_root(cwd) else { return Ok(args) };
    if !package.path.as_path().ancestors().skip(1).any(|p| p.join("package.json").is_file()) {
        return Ok(args);
    }
    let npm = vp_command::resolve_bin(program, env.get("PATH").map(OsStr::new), cwd)?;
    let root = match edit {
        NpmWorkspaceEdit::AutomaticRoot => npm_path(&npm, cwd, &["prefix"], env).await?,
        NpmWorkspaceEdit::LegacyRoot { .. } => legacy_root(cwd, &npm, env).await?,
    };
    if root == package.path.as_path() {
        return Ok(args);
    }
    let root = AbsolutePath::new(&root)
        .ok_or_else(|| io::Error::other("npm returned a relative workspace root"))?;

    if matches!(edit, NpmWorkspaceEdit::LegacyRoot { .. }) {
        // Early npm 8 interprets even ./ and ../ workspace paths from its prefix.
        args = rebase_workspace_selectors(args, cwd);
    }
    // PowerShell -File splits an attached --option=C:\path at the drive colon.
    // Keep each generated path in its own argument when forwarding it to npm.
    if !has_globalconfig {
        let globalconfig = original_globalconfig(&npm, root, cwd, &args, env).await?;
        args.splice(1..1, ["--globalconfig".to_owned(), globalconfig.display().to_string()]);
    }
    args.splice(1..1, ["--prefix".to_owned(), root.as_path().display().to_string()]);
    if matches!(edit, NpmWorkspaceEdit::LegacyRoot { local_dependency: true })
        && (has_selector || legacy_config_selects_members(&npm, cwd, &args, env).await?)
    {
        // Early Arborist writes the same root-relative file path into every selected manifest.
        return Err(Error::UserMessage(
            "Adding a local dependency to both the root and a workspace requires npm 8.19.0 or later.".into(),
        ));
    }
    Ok(args)
}

// A selection can come from any npmrc. A non-workspace `npm pkg get name` returns
// the root's name (or an empty object). A workspace query returns a map of packages.
async fn legacy_config_selects_members(
    npm: &AbsolutePath,
    cwd: &AbsolutePath,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<bool, Error> {
    let mut probe = npm_probe(npm, cwd);
    probe.args(["pkg", "get", "name", "--include-workspace-root"]);
    let mut flags = args.iter().take_while(|arg| arg.as_str() != "--");
    while let Some(arg) = flags.next() {
        let key = arg.split('=').next().unwrap_or(arg);
        if matches!(key, "--prefix" | "--globalconfig" | "--userconfig") {
            probe.arg(arg);
            if arg == key
                && let Some(value) = flags.next()
            {
                probe.arg(value);
            }
        }
    }
    let output = probe
        .envs(env)
        .env_remove("NODE_OPTIONS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::UserMessage(
            "Could not check npm's configured workspace selection.".into(),
        ));
    }
    let selection: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(selection.as_object().is_some_and(|members| !members.is_empty()))
}

async fn original_globalconfig(
    npm: &AbsolutePath,
    root: &AbsolutePath,
    cwd: &AbsolutePath,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<PathBuf, Error> {
    let mut userconfig = None;
    let mut flags = args.iter().take_while(|arg| arg.as_str() != "--");
    while let Some(arg) = flags.next() {
        if arg == "--userconfig" {
            userconfig = flags.next().map(String::as_str);
        } else if let Some(path) = arg.strip_prefix("--userconfig=") {
            userconfig = Some(path);
        }
    }
    if let Some(path) = crate::config::workspace_globalconfig(root, cwd, userconfig, env) {
        return Ok(path);
    }

    // Ask this npm installation for its default. Its global mode rejects workspace
    // selectors, and the caller's selected user npmrc has already been checked.
    let mut probe_env = env.clone();
    for key in
        std::env::vars_os().filter_map(|(key, _)| key.into_string().ok()).chain(env.keys().cloned())
    {
        if key.eq_ignore_ascii_case("npm_config_workspace")
            || key.eq_ignore_ascii_case("npm_config_workspaces")
        {
            probe_env.insert(key, String::new());
        }
    }
    let userconfig = if cfg!(windows) { "--userconfig=NUL" } else { "--userconfig=/dev/null" };
    npm_path(npm, cwd, &["config", "get", "globalconfig", "--global", userconfig], &probe_env).await
}

// Early npm saves local paths relative to the root even in a member's manifest.
pub(super) fn is_local_dependency(package: &str) -> bool {
    fn is_path(spec: &str) -> bool {
        spec.starts_with('.')
            || spec.starts_with("~/")
            || Path::new(spec).is_absolute()
            || spec.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file:"))
            || spec.as_bytes().get(1) == Some(&b':')
            || (cfg!(windows) && spec.contains('\\'))
    }
    if is_path(package) {
        return true;
    }
    let unscoped = package.strip_prefix('@').unwrap_or(package);
    let spec = unscoped.split_once('@').map_or(package, |(_, spec)| spec);
    if is_path(spec) {
        return true;
    }
    // Remote URLs, hosted Git references and registry aliases can also contain slashes.
    let path = spec.split('#').next().unwrap_or(spec);
    if path.contains(':') || package.contains("://") {
        return false;
    }
    if let Some((owner, repo)) = path.split_once('/') {
        if owner.starts_with('@') {
            return repo.contains('/');
        }
        return repo.is_empty() || repo.contains('/') || path.contains(char::is_whitespace);
    }
    let lower = path.cow_to_ascii_lowercase();
    lower.ends_with(".tgz") || lower.ends_with(".tar") || lower.ends_with(".tar.gz")
}

fn rebase_workspace_selectors(args: Vec<String>, cwd: &AbsolutePath) -> Vec<String> {
    let mut result = Vec::with_capacity(args.len());
    let mut args = args.into_iter();
    let mut next_selector = false;
    while let Some(arg) = args.next() {
        if arg == "--" {
            result.push(arg);
            result.extend(args);
            break;
        }
        if next_selector {
            result.push(caller_relative_selector(&arg, cwd).unwrap_or(arg));
            next_selector = false;
        } else if let Some((option @ ("--workspace" | "-w"), value)) = arg.split_once('=')
            && let Some(path) = caller_relative_selector(value, cwd)
        {
            result.push(option.to_owned());
            result.push(path);
        } else {
            next_selector = matches!(arg.as_str(), "--workspace" | "-w");
            result.push(arg);
        }
    }
    result
}

fn caller_relative_selector(value: &str, cwd: &AbsolutePath) -> Option<String> {
    let path = Path::new(value);
    (path.starts_with(".") || path.starts_with(".."))
        .then(|| cwd.as_path().join(path).to_string_lossy().into_owned())
}

// Use the same Windows shim as the final install or uninstall command.
pub(super) fn npm_probe(npm: &AbsolutePath, cwd: &AbsolutePath) -> tokio::process::Command {
    if let Some((program, args)) = vp_command::rewrite_cmd_to_powershell(npm) {
        let mut command = vp_command::build_command(&program, cwd);
        command.args(args);
        command
    } else {
        vp_command::build_command(npm, cwd)
    }
}

async fn npm_path(
    npm: &AbsolutePath,
    cwd: &AbsolutePath,
    args: &[&str],
    env: &HashMap<String, String>,
) -> Result<PathBuf, Error> {
    let output = npm_probe(npm, cwd)
        .args(args)
        .arg("--json=false")
        .envs(env)
        .env_remove("NODE_OPTIONS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::UserMessage(vt_str::format!(
            "Could not read npm {}: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let path = std::str::from_utf8(&output.stdout)
        .map_err(io::Error::other)?
        .trim_end_matches(['\r', '\n']);
    if !Path::new(path).is_absolute() {
        return Err(Error::UserMessage(
            "npm returned an invalid configuration or prefix path".into(),
        ));
    }
    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::is_local_dependency;

    #[test]
    fn early_npm_local_dependencies() {
        for package in [
            "file:./a@b",
            "./pkg",
            "/tmp/a@b.tgz",
            "a.tgz",
            "pkg@a.tar.gz",
            "@scope/pkg@file:./a.tgz",
            "nested/path/to/package",
            "with spaces/pkg",
        ] {
            assert!(is_local_dependency(package), "{package}");
        }
        for package in [
            "pkg@~1",
            "@scope/pkg",
            "pkg@npm:@scope/other@1",
            "owner/repo",
            "owner/repo#feature/branch",
            "https://example.com/a.tgz",
            "git+ssh://git@example.com/a/b",
        ] {
            assert!(!is_local_dependency(package), "{package}");
        }
    }
}
