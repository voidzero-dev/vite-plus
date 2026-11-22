//! Finds the Yarn version that a launcher actually runs.
//!
//! A Yarn launcher hands each command to the release in `yarnPath` unless
//! `ignorePath` or `YARN_IGNORE_PATH` turns that off. Yarn Classic and Yarn
//! Berry follow different files and variables, so Vite+ does not repeat those
//! rules. It only looks for any sign of delegation and asks the launcher with
//! `yarn --version` when it finds one.

use std::{ffi::OsString, fs};

use semver::Version;
use vp_shared::{PrependOptions, ToolPathEnv};
use vt_path::AbsolutePath;
use vt_str::Str;

use crate::{PackageManager, error::Error};

/// Classic reads `.yarnrc`, Berry reads `.yarnrc.yml` or `YARN_RC_FILENAME`.
const RC_FILE_NAMES: [&str; 2] = [".yarnrc.yml", ".yarnrc"];
/// Classic spells the setting `yarn-path`, Berry spells it `yarnPath`.
const PATH_SETTINGS: [&[u8]; 2] = [b"yarnPath", b"yarn-path"];

/// Returns the version that the Yarn launcher in `manager` runs for `cwd`.
pub(crate) async fn effective_version(
    cwd: &AbsolutePath,
    manager: &PackageManager,
) -> Result<Str, Error> {
    let home = &vp_shared::EnvConfig::get().user_home;
    if may_delegate(cwd, home, std::env::vars_os()) {
        query_version(cwd, manager).await
    } else {
        Ok(manager.version.clone())
    }
}

/// Whether an rc file or variable can make the launcher delegate. A false match
/// only costs one `yarn --version`, so this looks for the setting names in the
/// current, parent, and home directories instead of parsing the files.
fn may_delegate(
    cwd: &AbsolutePath,
    home: &AbsolutePath,
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> bool {
    let mut rc_names: Vec<OsString> = RC_FILE_NAMES.iter().map(OsString::from).collect();
    for (name, value) in vars {
        // Berry matches `YARN_*` names without regard to case.
        let Some(name) = name.to_str() else { continue };
        if name.eq_ignore_ascii_case("YARN_YARN_PATH") {
            return true;
        }
        if name.eq_ignore_ascii_case("YARN_RC_FILENAME") {
            rc_names.push(value);
        }
    }
    std::iter::successors(Some(cwd), |dir| dir.parent()).chain(Some(home)).any(|dir| {
        rc_names.iter().any(|name| {
            fs::read(dir.join(name)).is_ok_and(|content| {
                PATH_SETTINGS
                    .iter()
                    .any(|setting| content.windows(setting.len()).any(|window| window == *setting))
            })
        })
    })
}

async fn query_version(cwd: &AbsolutePath, manager: &PackageManager) -> Result<Str, Error> {
    let mut env = ToolPathEnv::from_env();
    env.prepend(manager.get_bin_prefix(), &manager.bin_names(), PrependOptions::default())?;
    let executable =
        vp_command::resolve_bin("yarn", Some(manager.bin_prefix.as_path().as_os_str()), cwd)?;
    let (program, prefix_args) = vp_command::rewrite_cmd_to_powershell(&executable)
        .unwrap_or_else(|| (executable, Vec::new()));
    let output = vp_command::build_command(&program, cwd)
        .args(prefix_args)
        .arg("--version")
        .envs(env.into_envs())
        // User preloads can print to stdout; only the actual command should run them.
        .env_remove("NODE_OPTIONS")
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::UserMessage(vt_str::format!(
            "Failed to read the Yarn version selected by yarnPath:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim();
    Version::parse(version).map_err(|source| Error::InvalidPackageManagerVersion {
        manager: manager.client,
        version: version.to_owned(),
        source,
    })?;
    Ok(version.into())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vt_path::AbsolutePathBuf;

    use super::*;

    fn temp_dir() -> (TempDir, AbsolutePathBuf) {
        let dir = TempDir::new().unwrap();
        let path = AbsolutePathBuf::new(dir.path().canonicalize().unwrap()).unwrap();
        (dir, path)
    }

    fn check(cwd: &AbsolutePath, home: &AbsolutePath, env: &[(&str, &str)]) -> bool {
        may_delegate(cwd, home, env.iter().map(|(name, value)| (name.into(), value.into())))
    }

    #[test]
    fn no_delegation_needs_no_query() {
        let (_project, project) = temp_dir();
        let (_home, home) = temp_dir();
        fs::write(project.join(".yarnrc.yml"), "nodeLinker: node-modules\n").unwrap();
        assert!(!check(&project, &home, &[("YARN_IGNORE_PATH", "1")]));
    }

    #[test]
    fn every_delegation_source_is_detected() {
        let (_project, project) = temp_dir();
        let (_home, home) = temp_dir();
        let sub = project.join("packages/app");
        fs::create_dir_all(&sub).unwrap();
        assert!(check(&sub, &home, &[("YARN_YARN_PATH", "yarn.cjs")]));
        assert!(check(&sub, &home, &[("yarn_yarn_path", "yarn.cjs")]));

        let rc = |dir: &AbsolutePath, name: &str, content: &[u8]| {
            fs::write(dir.join(name), content).unwrap();
            let found = check(&sub, &home, &[]);
            fs::remove_file(dir.join(name)).unwrap();
            found
        };
        assert!(rc(&home, ".yarnrc.yml", b"yarnPath: /yarn.cjs\n"));
        assert!(rc(&project, ".yarnrc", b"yarn-path: \".yarn/releases/yarn.cjs\"\n"));
        // Forms that a parser could miss still lead to a query.
        assert!(rc(&project, ".yarnrc.yml", b"yarnPath:\n  onConflict: reset\n  value: a.cjs\n"));
        assert!(rc(&project, ".yarnrc.yml", b"# caf\xe9\nyarnPath: ${RELEASE:-a}.cjs\n"));

        fs::write(project.join(".custom.yml"), "yarnPath: yarn.cjs\n").unwrap();
        assert!(!check(&sub, &home, &[]));
        assert!(check(&sub, &home, &[("YARN_RC_FILENAME", ".custom.yml")]));
        assert!(check(&sub, &home, &[("yarn_rc_filename", ".custom.yml")]));
    }
}
