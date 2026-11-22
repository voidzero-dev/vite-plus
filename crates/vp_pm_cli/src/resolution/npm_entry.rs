//! Find a member's enclosing npm project on releases before automatic workspace roots.
use std::{
    collections::HashMap,
    ffi::OsString,
    fs, io,
    path::{Component, Path, PathBuf},
    process::Stdio,
};

use serde_json::{Map, Value};
use vt_path::AbsolutePath;

use crate::Error;

pub(super) async fn legacy_root(
    cwd: &AbsolutePath,
    npm: &AbsolutePath,
    env: &HashMap<String, String>,
) -> Result<PathBuf, Error> {
    let package = vt_workspace::find_package_root(cwd)?;
    let member = package.path.as_path();
    let parse_manifest = |bytes: &[u8]| {
        serde_json::from_slice::<Value>(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
    };
    let member_manifest = parse_manifest(package.package_json.content())?;
    // npm 8 also treats a path selector as a glob for that directory's children.
    // An empty brace alternative expands to the same glob but changes the exact path.
    let mut children = PathBuf::new();
    let mut add_braces = true;
    for component in member.components() {
        if add_braces && let Component::Normal(name) = component {
            let mut part = OsString::from("{,}");
            part.push(name);
            children.push(part);
            add_braces = false;
        } else {
            children.push(component);
        }
    }
    for ancestor in member.ancestors().skip(1) {
        let bytes = match fs::read(ancestor.join("package.json")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        let Ok(manifest) = parse_manifest(&bytes) else { continue };
        if manifest["workspaces"].is_null() {
            continue;
        }
        // npm keys the response by name. If a member can replace the root's
        // entry, compare the manifests too. An ordinary query needs only names.
        let same_name =
            member_manifest["name"].is_null() || member_manifest["name"] == manifest["name"];
        let selected = workspace_entries(npm, cwd, env, ancestor, member, same_name).await?;
        let descendants = workspace_entries(npm, cwd, env, ancestor, &children, same_name).await?;
        if selected.iter().any(|(name, value)| descendants.get(name) != Some(value)) {
            return Ok(ancestor.to_owned());
        }
        // A literal globstar or brace expansion can match the directory itself. An
        // identical manifest can also hide behind the root's key in npm's JSON.
        let could_match_itself = member.components().any(|part| {
            let segment = part.as_os_str().to_string_lossy();
            segment.contains("**") || segment.contains(['{', '}'])
        });
        let name_was_selected =
            member_manifest["name"].as_str().is_none_or(|name| selected.contains_key(name));
        if name_was_selected && could_match_itself || same_name && member_manifest == manifest {
            return Err(Error::UserMessage(
                "Could not distinguish the workspace path from its matches with this npm version."
                    .into(),
            ));
        }
    }
    Ok(member.to_owned())
}

async fn workspace_entries(
    npm: &AbsolutePath,
    cwd: &AbsolutePath,
    env: &HashMap<String, String>,
    root: &Path,
    filter: &Path,
    compare_manifests: bool,
) -> Result<Map<String, Value>, Error> {
    let mut command = super::npm_workspace::npm_probe(npm, cwd);
    command.args(["pkg", "get"]);
    if !compare_manifests {
        command.arg("name");
    }
    // Including the root gives both queries a common entry when no member matches.
    let output = command
        .args(["--include-workspace-root", "--prefix"])
        .arg(root)
        .arg("--workspace")
        .arg(filter)
        .envs(env)
        .env_remove("NODE_OPTIONS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::UserMessage(vt_str::format!(
            "Could not read npm workspaces in {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
