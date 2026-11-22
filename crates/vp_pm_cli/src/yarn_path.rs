//! Finds the Yarn version that a launcher actually runs.
//!
//! A Yarn launcher hands each command to the release in `yarnPath` unless
//! `ignorePath` or `YARN_IGNORE_PATH` turns that off. Yarn Classic and Yarn
//! Berry follow different files and variables, so Vite+ does not repeat those
//! rules. It only looks for any sign of delegation, asks the launcher with
//! `yarn --version` when it finds one, and caches the answer under a key built
//! from the rc files, the release metadata, the Yarn path variables, and the
//! launcher version. The key does not see what a `yarnPath` script runs.

use std::{ffi::OsString, fs, io, time::UNIX_EPOCH};

use semver::Version;
use sha2::{Digest, Sha256};
use vp_shared::{PrependOptions, ToolPathEnv};
use vt_path::AbsolutePath;
use vt_str::Str;

use crate::{PackageManager, error::Error};

/// Classic reads `.yarnrc`, Berry reads `.yarnrc.yml` or `YARN_RC_FILENAME`.
const RC_FILE_NAMES: [&str; 2] = [".yarnrc.yml", ".yarnrc"];
/// Classic spells the setting `yarn-path`, Berry spells it `yarnPath`.
const PATH_SETTINGS: [&str; 2] = ["yarnPath", "yarn-path"];
const ENV_VARS: [&str; 3] = ["YARN_YARN_PATH", "YARN_IGNORE_PATH", "YARN_RC_FILENAME"];

/// Returns the version that the Yarn launcher in `manager` runs for `cwd`.
pub(crate) async fn effective_version(
    cwd: &AbsolutePath,
    manager: &PackageManager,
) -> Result<Str, Error> {
    let config = vp_shared::EnvConfig::get();
    let env = yarn_env(std::env::vars_os());
    let Some(key) = delegation_key(cwd, &config.user_home, &manager.version, &env) else {
        return Ok(manager.version.clone());
    };
    let cache_path =
        config.dirs.cache.join("package_manager_effective_version").join("yarn").join(&*key);
    if let Ok(version) = fs::read_to_string(&cache_path)
        && Version::parse(&version).is_ok()
    {
        return Ok(version.into());
    }
    let version = query_version(cwd, manager).await?;
    // A failed write only means the next command asks Yarn again.
    let _ = write_cache(&cache_path, &version);
    Ok(version)
}

/// Returns a cache key built from the rc files, the release metadata, the Yarn
/// path variables, and the launcher version, or `None` when nothing can make
/// the launcher delegate.
fn delegation_key(
    cwd: &AbsolutePath,
    home: &AbsolutePath,
    launcher_version: &str,
    env: &[(&str, OsString, OsString)],
) -> Option<Str> {
    let mut hasher = Sha256::new();
    let mut delegates = false;
    hasher.update(launcher_version);

    let mut rc_names: Vec<OsString> = RC_FILE_NAMES.iter().map(OsString::from).collect();
    for (setting, name, value) in env {
        hasher.update([0]);
        hasher.update(name.as_encoded_bytes());
        hasher.update([0]);
        hasher.update(value.as_encoded_bytes());
        match *setting {
            "YARN_YARN_PATH" => {
                delegates = true;
                hash_target(&mut hasher, &cwd.join(value));
            }
            "YARN_RC_FILENAME" => rc_names.push(value.clone()),
            _ => {}
        }
    }

    let dirs = std::iter::successors(Some(cwd), |dir| dir.parent()).chain(Some(home));
    for dir in dirs {
        for name in &rc_names {
            let rc_path = dir.join(name);
            let Ok(content) = fs::read_to_string(&rc_path) else { continue };
            hasher.update([0]);
            hasher.update(rc_path.as_path().as_os_str().as_encoded_bytes());
            hasher.update([0]);
            hasher.update(&content);
            for target in path_settings(&content) {
                delegates = true;
                hash_target(&mut hasher, &dir.join(target));
            }
        }
    }

    delegates.then(|| hex::encode(hasher.finalize()).into())
}

/// The Yarn variables that can change the effective version, with the setting
/// each one names. Berry matches `YARN_*` names without regard to case and lets
/// a later spelling override an earlier one, so the original names and their
/// order stay in the key.
fn yarn_env(
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(&'static str, OsString, OsString)> {
    vars.into_iter()
        .filter_map(|(name, value)| {
            let text = name.to_str()?;
            let setting = ENV_VARS.iter().find(|var| var.eq_ignore_ascii_case(text))?;
            Some((*setting, name, value))
        })
        .collect()
}

/// Values of `yarnPath` and `yarn-path` in an rc file. Both formats put the
/// value after the setting name, separated by `:` or spaces, possibly quoted,
/// and possibly followed by a `#` comment.
fn path_settings(content: &str) -> impl Iterator<Item = &str> {
    content.lines().filter_map(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix(['"', '\'']).unwrap_or(line);
        let rest = PATH_SETTINGS.iter().find_map(|setting| line.strip_prefix(setting))?;
        let rest = rest.strip_prefix(['"', '\'']).unwrap_or(rest);
        // A longer setting name, such as `yarnPathX`, is not this setting.
        if !rest.starts_with([':', ' ', '\t']) {
            return None;
        }
        scalar_value(rest.trim_start_matches([':', ' ', '\t']))
    })
}

/// Reads a quoted value up to its closing quote, or a plain value up to a `#`
/// that starts a comment, which YAML requires to follow whitespace.
fn scalar_value(text: &str) -> Option<&str> {
    let value = if let Some(quote) = text.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        let inner = &text[1..];
        inner.split_once(quote)?.0
    } else {
        let comment = text
            .match_indices('#')
            .map(|(index, _)| index)
            .find(|&index| index == 0 || text[..index].ends_with([' ', '\t']))
            .unwrap_or(text.len());
        text[..comment].trim_end()
    };
    (!value.is_empty()).then_some(value)
}

/// A release can be replaced in place under the same name, so its metadata is
/// part of the key. Hashing the whole bundle would cost more than the cache saves.
fn hash_target(hasher: &mut Sha256, target: &AbsolutePath) {
    hasher.update([0]);
    hasher.update(target.as_path().as_os_str().as_encoded_bytes());
    let Ok(metadata) = fs::metadata(target) else {
        hasher.update(b"missing");
        return;
    };
    hasher.update(metadata.len().to_le_bytes());
    let modified = metadata.modified().ok().and_then(|time| time.duration_since(UNIX_EPOCH).ok());
    hasher.update(modified.unwrap_or_default().as_nanos().to_le_bytes());
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

fn write_cache(path: &AbsolutePath, version: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, version)
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

    fn key(cwd: &AbsolutePath, home: &AbsolutePath, env: &[(&str, &str)]) -> Option<Str> {
        let env = yarn_env(env.iter().map(|(name, value)| (name.into(), value.into())));
        delegation_key(cwd, home, "2.4.2", &env)
    }

    #[test]
    fn no_delegation_needs_no_query() {
        let (_project, project) = temp_dir();
        let (_home, home) = temp_dir();
        fs::write(project.join(".yarnrc.yml"), "nodeLinker: node-modules\n").unwrap();
        assert_eq!(key(&project, &home, &[("YARN_IGNORE_PATH", "1")]), None);
    }

    #[test]
    fn every_delegation_source_is_detected() {
        let (_project, project) = temp_dir();
        let (_home, home) = temp_dir();
        let sub = project.join("packages/app");
        fs::create_dir_all(&sub).unwrap();
        assert!(key(&sub, &home, &[("YARN_YARN_PATH", "yarn.cjs")]).is_some());
        fs::write(home.join(".yarnrc.yml"), "yarnPath: /yarn.cjs\n").unwrap();
        assert!(key(&sub, &home, &[]).is_some());
        fs::remove_file(home.join(".yarnrc.yml")).unwrap();
        fs::write(project.join(".yarnrc"), "yarn-path \".yarn/releases/yarn.cjs\"\n").unwrap();
        assert!(key(&sub, &home, &[]).is_some());
        fs::remove_file(project.join(".yarnrc")).unwrap();
        fs::write(project.join(".custom.yml"), "yarnPath: yarn.cjs\n").unwrap();
        assert!(key(&sub, &home, &[]).is_none());
        assert!(key(&sub, &home, &[("YARN_RC_FILENAME", ".custom.yml")]).is_some());
        assert!(key(&sub, &home, &[("yarn_rc_filename", ".custom.yml")]).is_some());
        fs::remove_file(project.join(".custom.yml")).unwrap();
        assert!(key(&sub, &home, &[("yarn_yarn_path", "yarn.cjs")]).is_some());
    }

    #[test]
    fn key_changes_with_each_input() {
        let (_project, project) = temp_dir();
        let (_home, home) = temp_dir();
        let release = project.join(".yarn/releases/yarn-berry.cjs");
        fs::create_dir_all(release.parent().unwrap()).unwrap();
        fs::write(&release, "a").unwrap();
        fs::write(
            project.join(".yarnrc.yml"),
            "yarnPath: .yarn/releases/yarn-berry.cjs # pinned\n",
        )
        .unwrap();
        let base = key(&project, &home, &[]).unwrap();
        assert_eq!(key(&project, &home, &[]).unwrap(), base);

        fs::write(&release, "ab").unwrap();
        let replaced = key(&project, &home, &[]).unwrap();
        assert_ne!(replaced, base, "a release replaced in place");

        let ignored = key(&project, &home, &[("YARN_IGNORE_PATH", "1")]).unwrap();
        assert_ne!(ignored, replaced);
        assert_ne!(key(&project, &home, &[("yarn_ignore_path", "1")]).unwrap(), replaced);
        // Yarn applies whichever spelling comes last.
        let delegated_env = [("YARN_IGNORE_PATH", "1"), ("yarn_ignore_path", "0")];
        let ignored_env = [("YARN_IGNORE_PATH", "0"), ("yarn_ignore_path", "1")];
        assert_ne!(key(&project, &home, &delegated_env), key(&project, &home, &ignored_env));
        let launcher = delegation_key(&project, &home, "3.0.0", &[]).unwrap();
        assert_ne!(launcher, replaced, "another launcher version");

        fs::write(home.join(".yarnrc.yml"), "ignorePath: true\n").unwrap();
        assert_ne!(key(&project, &home, &[]).unwrap(), replaced, "an rc file without yarnPath");
    }

    #[test]
    fn path_settings_read_both_formats() {
        let content = [
            "yarnPath: \".yarn/releases/a.cjs\"",
            "\"yarn-path\" 'b.js'",
            "  yarnPath: c.cjs  ",
            "yarnPath: d.cjs # pinned",
            "yarnPath: \"e # f.cjs\" # pinned",
            "yarn-path g#h.js",
            "yarnPathX: ignored.cjs",
            "# yarnPath: ignored.cjs",
            "yarnPath: # empty",
        ]
        .join("\n");
        assert_eq!(
            path_settings(&content).collect::<Vec<_>>(),
            [".yarn/releases/a.cjs", "b.js", "c.cjs", "d.cjs", "e # f.cjs", "g#h.js"]
        );
    }
}
