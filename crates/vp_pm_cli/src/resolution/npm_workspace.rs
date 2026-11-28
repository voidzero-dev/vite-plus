//! Select npm's local project while preserving its launcher, cwd and global configuration.
use std::{collections::HashMap, ffi::OsString, io::Write};

use tempfile::NamedTempFile;
use vt_path::AbsolutePath;

use crate::Error;

pub(super) fn prepare(
    cwd: &AbsolutePath,
    env: &mut HashMap<String, String>,
) -> Result<Option<NamedTempFile>, Error> {
    let Ok(member) = vt_workspace::find_package_root(cwd) else { return Ok(None) };
    let has_parent_project =
        member.path.as_path().ancestors().skip(1).any(|p| p.join("package.json").is_file());
    if !has_parent_project
        && serde_json::from_slice::<serde_json::Value>(member.package_json.content())
            .is_ok_and(|pkg| pkg.get("workspaces").is_none())
    {
        return Ok(None);
    }
    // npm has no CLI flag for its local prefix alone. Load the adapter in the
    // selected npm, including when the launcher chooses a runtime for this cwd.
    let mut script = tempfile::Builder::new().prefix("vp-npm-").suffix(".cjs").tempfile()?;
    let preload =
        vt_str::format!("--require {}", serde_json::to_string(script.path())?).to_string();
    writeln!(
        script,
        "{}\nmodule.exports({}, {});",
        include_str!("npm_workspace.cjs"),
        serde_json::to_string(member.path.as_path())?,
        serde_json::to_string(&preload)?,
    )?;
    script.flush()?;
    let original = env
        .get("NODE_OPTIONS")
        .map(OsString::from)
        .or_else(|| std::env::var_os("NODE_OPTIONS"))
        .unwrap_or_default();
    let options = if original.is_empty() {
        preload
    } else {
        vt_str::format!("{preload} {}", original.to_string_lossy()).to_string()
    };
    env.insert("NODE_OPTIONS".into(), options);
    Ok(Some(script))
}
