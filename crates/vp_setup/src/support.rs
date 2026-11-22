//! Capability probe of a downloaded `vp` binary.
//!
//! Installers and `vp upgrade` run the binary with
//! `VP_SELF_SETUP_SUPPORT_CHECK` set before handing over to it. A binary that
//! supports first-start setup prints [`SELF_SETUP_SUPPORT_RESPONSE`]. That line
//! never changes, because released installers compare it exactly. When the
//! check value is [`ENV_FORBIDDEN_SUPPORT_CHECK`], a binary that also supports
//! `VP_ENV_FORBIDDEN` prints [`ENV_FORBIDDEN_SUPPORT_RESPONSE`] on a second
//! line.

use vp_shared::env_vars;
use vt_path::AbsolutePath;

/// First line of the probe response: first-start setup is supported.
pub const SELF_SETUP_SUPPORT_RESPONSE: &str = "vite-plus-self-setup-v1";

/// Check value that asks whether the binary supports `VP_ENV_FORBIDDEN`.
pub const ENV_FORBIDDEN_SUPPORT_CHECK: &str = "env-forbidden";

/// Second line of the probe response for [`ENV_FORBIDDEN_SUPPORT_CHECK`].
pub const ENV_FORBIDDEN_SUPPORT_RESPONSE: &str = "vite-plus-self-setup-env-forbidden";

/// What a probed binary supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfSetupSupport {
    /// No first-start setup: an older binary, or the probe failed.
    Legacy,
    /// First-start setup without `VP_ENV_FORBIDDEN` support.
    SelfSetup,
    /// First-start setup and `VP_ENV_FORBIDDEN`.
    EnvForbidden,
}

/// Lines a binary prints for the support check value `check`.
#[must_use]
pub fn support_response(check: &std::ffi::OsStr) -> &'static [&'static str] {
    if check == ENV_FORBIDDEN_SUPPORT_CHECK {
        &[SELF_SETUP_SUPPORT_RESPONSE, ENV_FORBIDDEN_SUPPORT_RESPONSE]
    } else {
        &[SELF_SETUP_SUPPORT_RESPONSE]
    }
}

/// Classify the probe output of a successful run.
#[must_use]
pub fn parse_support_response(stdout: &[u8]) -> SelfSetupSupport {
    let v1 = format!("{SELF_SETUP_SUPPORT_RESPONSE}\n");
    let env_forbidden =
        format!("{SELF_SETUP_SUPPORT_RESPONSE}\n{ENV_FORBIDDEN_SUPPORT_RESPONSE}\n");
    if stdout == env_forbidden.as_bytes() {
        SelfSetupSupport::EnvForbidden
    } else if stdout == v1.as_bytes() {
        SelfSetupSupport::SelfSetup
    } else {
        SelfSetupSupport::Legacy
    }
}

/// Run `binary --help` with the support check. With `env_forbidden`, the
/// check asks whether the binary also supports `VP_ENV_FORBIDDEN`.
///
/// # Errors
///
/// Returns an error if the binary cannot be started.
pub async fn probe_self_setup(
    binary: &AbsolutePath,
    env_forbidden: bool,
) -> std::io::Result<SelfSetupSupport> {
    let output = tokio::process::Command::new(binary.as_path())
        .arg("--help")
        .env(
            env_vars::VP_SELF_SETUP_SUPPORT_CHECK,
            if env_forbidden { ENV_FORBIDDEN_SUPPORT_CHECK } else { "1" },
        )
        .output()
        .await?;
    if !output.status.success() {
        return Ok(SelfSetupSupport::Legacy);
    }
    Ok(parse_support_response(&output.stdout))
}

/// Error message for a target that cannot honor `VP_ENV_FORBIDDEN`.
#[must_use]
pub fn env_forbidden_unsupported_message(target: &str) -> String {
    let variable = env_vars::VP_ENV_FORBIDDEN;
    format!(
        "vite-plus {target} does not support {variable}. Choose a version that supports it, or unset {variable}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_response_adds_env_forbidden_line_only_for_its_check_value() {
        assert_eq!(support_response("1".as_ref()), [SELF_SETUP_SUPPORT_RESPONSE]);
        assert_eq!(support_response("Env-Forbidden".as_ref()), [SELF_SETUP_SUPPORT_RESPONSE]);
        assert_eq!(
            support_response(ENV_FORBIDDEN_SUPPORT_CHECK.as_ref()),
            [SELF_SETUP_SUPPORT_RESPONSE, ENV_FORBIDDEN_SUPPORT_RESPONSE]
        );
    }

    #[test]
    fn parse_support_response_requires_exact_lines() {
        assert_eq!(
            parse_support_response(
                b"vite-plus-self-setup-v1\nvite-plus-self-setup-env-forbidden\n"
            ),
            SelfSetupSupport::EnvForbidden
        );
        assert_eq!(
            parse_support_response(b"vite-plus-self-setup-v1\n"),
            SelfSetupSupport::SelfSetup
        );
        assert_eq!(parse_support_response(b"Usage: vp [COMMAND]\n"), SelfSetupSupport::Legacy);
        assert_eq!(parse_support_response(b""), SelfSetupSupport::Legacy);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn probe_self_setup_passes_the_check_value() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let script = |name: &str, body: &str| {
            let path = temp.path().join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            vt_path::AbsolutePathBuf::new(path).unwrap()
        };
        let modern = script(
            "modern",
            &format!(
                r#"echo {SELF_SETUP_SUPPORT_RESPONSE}; [ "$VP_SELF_SETUP_SUPPORT_CHECK" = {ENV_FORBIDDEN_SUPPORT_CHECK} ] && echo {ENV_FORBIDDEN_SUPPORT_RESPONSE}; exit 0"#
            ),
        );
        let v1 = script("v1", &format!("echo {SELF_SETUP_SUPPORT_RESPONSE}"));
        let legacy = script("legacy", "echo 'Usage: vp [COMMAND]'");
        let failing = script("failing", &format!("echo {SELF_SETUP_SUPPORT_RESPONSE}; exit 1"));

        assert_eq!(probe_self_setup(&modern, true).await.unwrap(), SelfSetupSupport::EnvForbidden);
        assert_eq!(probe_self_setup(&modern, false).await.unwrap(), SelfSetupSupport::SelfSetup);
        assert_eq!(probe_self_setup(&v1, true).await.unwrap(), SelfSetupSupport::SelfSetup);
        assert_eq!(probe_self_setup(&legacy, true).await.unwrap(), SelfSetupSupport::Legacy);
        assert_eq!(probe_self_setup(&failing, false).await.unwrap(), SelfSetupSupport::Legacy);
    }
}
