//! Prepare a downloaded release before the running CLI activates it.

use std::process::Stdio;

use tokio::process::Command;
use vp_setup::upgrade::{PREPARE_CAPABILITY, PREPARE_ENV, PREPARE_PROTOCOL};
use vp_shared::{EnvConfig, VpDirsLayout, env_vars, output};
use vt_path::AbsolutePath;

use crate::error::Error;

/// Return false only when the target does not advertise preparation support.
pub(super) async fn prepare(
    binary: &AbsolutePath,
    silent: bool,
    registry: Option<&str>,
) -> Result<bool, Error> {
    // Existing self-setup binaries recognize this variable before any setup.
    // A new variable alone would let their ordinary --help invocation install them.
    let probe = command(binary, registry)
        .arg("--help")
        .env(env_vars::VP_SELF_SETUP_SUPPORT_CHECK, PREPARE_PROTOCOL)
        .output()
        .await?;
    if !probe.status.success() {
        output::print_and_flush(
            &mut std::io::stderr().lock(),
            &String::from_utf8_lossy(&probe.stderr),
        );
        return Err(Error::Upgrade(
            format!(
                "Failed to probe the downloaded Vite+ binary (exit code {}).",
                vp_shared::exit_code_from_status(probe.status)
            )
            .into(),
        ));
    }
    if probe.stdout != PREPARE_CAPABILITY.as_bytes() {
        return Ok(false);
    }

    let mut child = command(binary, registry);
    child.env(PREPARE_ENV, PREPARE_PROTOCOL);
    let status = if silent {
        let result = child.arg("--silent").output().await?;
        if !result.status.success() {
            output::print_and_flush(
                &mut std::io::stderr().lock(),
                &String::from_utf8_lossy(&result.stdout),
            );
            output::print_and_flush(
                &mut std::io::stderr().lock(),
                &String::from_utf8_lossy(&result.stderr),
            );
        }
        result.status
    } else {
        child.stdout(Stdio::inherit()).stderr(Stdio::inherit()).status().await?
    };
    if !status.success() {
        return Err(Error::Upgrade(
            format!(
                "The downloaded Vite+ binary could not prepare its dependencies (exit code {}).",
                vp_shared::exit_code_from_status(status)
            )
            .into(),
        ));
    }
    Ok(true)
}

fn command(binary: &AbsolutePath, registry: Option<&str>) -> Command {
    let mut command = Command::new(binary.as_path());
    command.stdin(Stdio::null());
    for name in [
        PREPARE_ENV,
        env_vars::VP_SELF_SETUP_SUPPORT_CHECK,
        env_vars::VP_SELF_SETUP_SHELL,
        env_vars::VP_SELF_SETUP_REPLACE_EXISTING,
        env_vars::VP_SETUP_NODE_VERSION,
        env_vars::VP_SETUP_NODE_VERSION_FILE,
        env_vars::VP_DUMP_DIRS,
        "VP_SKIP_DEPS_INSTALL",
        "VP_VERSION",
        "VP_COMPLETE",
    ] {
        command.env_remove(name);
    }

    // Pin the resolved installation roots for this handoff, including an
    // existing single-root installation selected without an explicit VP_HOME.
    let dirs = &EnvConfig::get().dirs;
    match dirs.layout() {
        VpDirsLayout::SingleRoot => {
            command
                .env(env_vars::VP_HOME, dirs.data.as_path())
                .env_remove(env_vars::VP_BIN_DIR)
                .env_remove(env_vars::VP_DATA_DIR)
                .env_remove(env_vars::VP_CACHE_DIR);
        }
        VpDirsLayout::Split => {
            command
                .env_remove(env_vars::VP_HOME)
                .env(env_vars::VP_BIN_DIR, dirs.bin.as_path())
                .env(env_vars::VP_DATA_DIR, dirs.data.as_path())
                .env(env_vars::VP_CACHE_DIR, dirs.cache.as_path());
        }
    }
    if let Some(registry) = registry {
        command
            .env(env_vars::NPM_CONFIG_REGISTRY, registry)
            .env(env_vars::NPM_CONFIG_REGISTRY_UPPER, registry);
    }
    command
}

#[cfg(test)]
mod environment_tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn child_preserves_resolved_roots_for_both_layouts() {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("commands");
        let data = root.path().join("data");
        let cache = root.path().join("cache");
        for home in [None, Some(root.path().as_os_str())] {
            EnvConfig::with_vars(
                [
                    (env_vars::VP_HOME, home),
                    (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                    (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                    (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                    ("HOME", Some(root.path().as_os_str())),
                    ("USERPROFILE", Some(root.path().as_os_str())),
                ],
                |config| {
                    let child = command(&config.dirs.data.join("target/bin/vp"), None);
                    let envs = child.as_std().get_envs().collect::<Vec<_>>();
                    let value = |key| {
                        envs.iter()
                            .find(|(name, _)| *name == OsStr::new(key))
                            .map(|(_, value)| *value)
                    };
                    if home.is_some() {
                        assert_eq!(value(env_vars::VP_HOME), Some(home));
                        for name in
                            [env_vars::VP_BIN_DIR, env_vars::VP_DATA_DIR, env_vars::VP_CACHE_DIR]
                        {
                            assert_eq!(value(name), Some(None));
                        }
                    } else {
                        assert_eq!(value(env_vars::VP_HOME), Some(None));
                        for (name, directory) in [
                            (env_vars::VP_BIN_DIR, &config.dirs.bin),
                            (env_vars::VP_DATA_DIR, &config.dirs.data),
                            (env_vars::VP_CACHE_DIR, &config.dirs.cache),
                        ] {
                            assert_eq!(value(name), Some(Some(directory.as_path().as_os_str())));
                        }
                    }
                },
            );
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use flate2::{Compression, write::GzEncoder};
    use vp_setup::install;
    use vt_path::AbsolutePathBuf;

    use super::*;

    const HELPER: &str = r#"#!/bin/sh
if [ -n "$VP_SELF_SETUP_SUPPORT_CHECK" ]; then
  test "$VP_SELF_SETUP_SUPPORT_CHECK" = upgrade-prepare-v1 || exit 91
  test "$1" = --help || exit 92
  printf 'vite-plus-upgrade-prepare-v1\n'
  exit 0
fi
if [ -n "$VP_UPGRADE_PREPARE" ]; then
  test "$VP_UPGRADE_PREPARE" = upgrade-prepare-v1 || exit 93
  test "$1" = --silent || exit 94
  test "$NPM_CONFIG_REGISTRY" = "$npm_config_registry" || exit 95
  test -z "$VP_SKIP_DEPS_INSTALL$VP_SETUP_NODE_VERSION$VP_SETUP_NODE_VERSION_FILE$VP_SELF_SETUP_SHELL$VP_VERSION$VP_COMPLETE$VP_DUMP_DIRS" || exit 96
  test "$(cat "$VP_HOME/current/version")" = old || exit 97
  test "$(cat "$VP_HOME/.previous-version")" = previous || exit 98
  target=$(dirname "$(dirname "$0")")
  printf '%s' "$NPM_CONFIG_REGISTRY" > "$target/prepared"
  exit "${TEST_PREPARE_EXIT:-0}"
fi
test "$1 $2 $3" = 'env setup --refresh' || exit 99
printf refreshed > "$VP_HOME/refreshed"
"#;

    fn archive(script: &str) -> Vec<u8> {
        let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
        let mut header = tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive.append_data(&mut header, "package/vp", script.as_bytes()).unwrap();
        archive.into_inner().unwrap().finish().unwrap()
    }

    fn executable(directory: &AbsolutePath, script: &str) -> AbsolutePathBuf {
        let binary = directory.join("vp");
        std::fs::write(&binary, script).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        binary
    }

    #[tokio::test]
    async fn unsupported_target_uses_only_the_safe_probe() {
        EnvConfig::scoped_async(|config| async move {
            let binary = executable(
                &config.dirs.data,
                r#"#!/bin/sh
test "$VP_SELF_SETUP_SUPPORT_CHECK" = upgrade-prepare-v1 || exit 91
test "$1" = --help || exit 92
printf 'vite-plus-self-setup-v1\n'
"#,
            );
            assert!(!prepare(&binary, true, None).await.unwrap());
        })
        .await;
    }

    #[tokio::test]
    async fn failed_probe_does_not_fall_back() {
        EnvConfig::scoped_async(|config| async move {
            let binary = executable(&config.dirs.data, "#!/bin/sh\nexit 42\n");
            let error = prepare(&binary, true, None).await.unwrap_err();
            assert!(error.to_string().contains("Failed to probe"));
            assert!(error.to_string().contains("exit code 42"));
        })
        .await;
    }

    async fn install_with_helper(exit: &str) {
        let registry = httpmock::MockServer::start_async().await;
        Box::pin(EnvConfig::scoped_async(|config| async move {
            let data = &config.dirs.data;
            let old = data.join("old");
            let new = data.join("new");
            tokio::fs::create_dir_all(&old).await.unwrap();
            tokio::fs::create_dir_all(&new).await.unwrap();
            tokio::fs::write(old.join("version"), "old").await.unwrap();
            tokio::fs::write(data.join(".previous-version"), "previous").await.unwrap();
            install::swap_current_link(data, "old").await.unwrap();

            let result = Box::pin(EnvConfig::with_vars_async(
                [
                    ("TEST_PREPARE_EXIT", Some(exit)),
                    (env_vars::NPM_CONFIG_REGISTRY, Some("https://invalid.example")),
                    (env_vars::NPM_CONFIG_REGISTRY_UPPER, Some("https://invalid.example")),
                    (env_vars::VP_SETUP_NODE_VERSION, Some("1.0.0")),
                    (env_vars::VP_SETUP_NODE_VERSION_FILE, Some("must-not-write")),
                    (env_vars::VP_SELF_SETUP_SHELL, Some("sh")),
                    (env_vars::VP_DUMP_DIRS, Some("1")),
                    ("VP_SKIP_DEPS_INSTALL", Some("1")),
                    ("VP_VERSION", Some("wrong-version")),
                    ("VP_COMPLETE", Some("bash")),
                ],
                |_| async {
                    super::super::install_platform_and_main(
                        &archive(HELPER),
                        &new,
                        data,
                        "new",
                        "2.0.0",
                        "1.0.0",
                        true,
                        Some(&registry.base_url()),
                    )
                    .await
                },
            ))
            .await;

            assert_eq!(
                tokio::fs::read_to_string(new.join("prepared")).await.unwrap(),
                registry.base_url()
            );
            assert!(
                !new.join("package.json").as_path().exists(),
                "parent must not generate the target's wrapper"
            );
            if exit == "0" {
                result.unwrap();
                assert_eq!(install::read_current_version(data).await.as_deref(), Some("new"));
                assert_eq!(
                    tokio::fs::read_to_string(data.join(".previous-version")).await.unwrap(),
                    "old"
                );
                assert!(data.join("refreshed").as_path().is_file());
            } else {
                let error = result.unwrap_err();
                assert!(
                    error.to_string().contains("could not prepare its dependencies (exit code 42)")
                );
                assert_eq!(install::read_current_version(data).await.as_deref(), Some("old"));
                assert_eq!(
                    tokio::fs::read_to_string(data.join(".previous-version")).await.unwrap(),
                    "previous"
                );
                assert!(!data.join("refreshed").as_path().exists());
            }
        }))
        .await;
    }

    #[tokio::test]
    async fn successful_target_preparation_precedes_activation() {
        Box::pin(install_with_helper("0")).await;
    }

    #[tokio::test]
    async fn failed_target_preparation_preserves_activation_and_rollback() {
        Box::pin(install_with_helper("42")).await;
    }
}
