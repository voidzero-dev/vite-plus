//! Capability probe of a downloaded `vp` binary.
//!
//! Installers and `vp upgrade` run the binary with
//! `VP_SELF_SETUP_SUPPORT_CHECK` set before handing over to it. A binary that
//! supports first-start setup prints [`SELF_SETUP_SUPPORT_RESPONSE`]. That line
//! never changes, because released installers compare it exactly. When the
//! check value is `system-only`, a binary that also supports
//! `VP_*_MANAGER=system-only` prints [`SYSTEM_ONLY_SUPPORT_RESPONSE`] on a
//! second line.

use vp_shared::env_vars;
use vt_path::AbsolutePath;

/// First line of the probe response: first-start setup is supported.
pub const SELF_SETUP_SUPPORT_RESPONSE: &str = "vite-plus-self-setup-v1";

/// Second line of the probe response for the `system-only` check value.
pub const SYSTEM_ONLY_SUPPORT_RESPONSE: &str = "vite-plus-self-setup-system-only";

/// What a probed binary supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelfSetupSupport {
    /// No first-start setup: an older binary, or the probe failed.
    Legacy,
    /// First-start setup without `system-only` support.
    SelfSetup,
    /// First-start setup and `VP_*_MANAGER=system-only`.
    SystemOnly,
}

/// Lines a binary prints for the support check value `check`.
#[must_use]
pub fn support_response(check: &std::ffi::OsStr) -> &'static [&'static str] {
    if check == env_vars::SYSTEM_ONLY {
        &[SELF_SETUP_SUPPORT_RESPONSE, SYSTEM_ONLY_SUPPORT_RESPONSE]
    } else {
        &[SELF_SETUP_SUPPORT_RESPONSE]
    }
}

/// Classify the probe output of a successful run.
#[must_use]
pub fn parse_support_response(stdout: &[u8]) -> SelfSetupSupport {
    let v1 = format!("{SELF_SETUP_SUPPORT_RESPONSE}\n");
    let system_only = format!("{SELF_SETUP_SUPPORT_RESPONSE}\n{SYSTEM_ONLY_SUPPORT_RESPONSE}\n");
    if stdout == system_only.as_bytes() {
        SelfSetupSupport::SystemOnly
    } else if stdout == v1.as_bytes() {
        SelfSetupSupport::SelfSetup
    } else {
        SelfSetupSupport::Legacy
    }
}

/// Run `binary --help` with the support check. With `system_only`, the check
/// asks whether the binary also supports `system-only`.
///
/// # Errors
///
/// Returns an error if the binary cannot be started.
pub async fn probe_self_setup(
    binary: &AbsolutePath,
    system_only: bool,
) -> std::io::Result<SelfSetupSupport> {
    let output = tokio::process::Command::new(binary.as_path())
        .arg("--help")
        .env(
            env_vars::VP_SELF_SETUP_SUPPORT_CHECK,
            if system_only { env_vars::SYSTEM_ONLY } else { "1" },
        )
        .output()
        .await?;
    if !output.status.success() {
        return Ok(SelfSetupSupport::Legacy);
    }
    Ok(parse_support_response(&output.stdout))
}

/// Error message for a target that cannot honor the `system-only` variables.
#[must_use]
pub fn system_only_unsupported_message(target: &str, variables: &[&str]) -> String {
    let settings = variables
        .iter()
        .map(|name| format!("{name}={}", env_vars::SYSTEM_ONLY))
        .collect::<Vec<_>>();
    format!(
        "vite-plus {target} does not support {}. Choose a version that supports it, or unset {}.",
        settings.join(", "),
        variables.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_response_adds_system_only_line_only_for_its_check_value() {
        assert_eq!(support_response("1".as_ref()), [SELF_SETUP_SUPPORT_RESPONSE]);
        assert_eq!(support_response("System-Only".as_ref()), [SELF_SETUP_SUPPORT_RESPONSE]);
        assert_eq!(
            support_response("system-only".as_ref()),
            [SELF_SETUP_SUPPORT_RESPONSE, SYSTEM_ONLY_SUPPORT_RESPONSE]
        );
    }

    #[test]
    fn parse_support_response_requires_exact_lines() {
        assert_eq!(
            parse_support_response(b"vite-plus-self-setup-v1\nvite-plus-self-setup-system-only\n"),
            SelfSetupSupport::SystemOnly
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
            r#"echo vite-plus-self-setup-v1; [ "$VP_SELF_SETUP_SUPPORT_CHECK" = system-only ] && echo vite-plus-self-setup-system-only; exit 0"#,
        );
        let v1 = script("v1", "echo vite-plus-self-setup-v1");
        let legacy = script("legacy", "echo 'Usage: vp [COMMAND]'");
        let failing = script("failing", "echo vite-plus-self-setup-v1; exit 1");

        assert_eq!(probe_self_setup(&modern, true).await.unwrap(), SelfSetupSupport::SystemOnly);
        assert_eq!(probe_self_setup(&modern, false).await.unwrap(), SelfSetupSupport::SelfSetup);
        assert_eq!(probe_self_setup(&v1, true).await.unwrap(), SelfSetupSupport::SelfSetup);
        assert_eq!(probe_self_setup(&legacy, true).await.unwrap(), SelfSetupSupport::Legacy);
        assert_eq!(probe_self_setup(&failing, false).await.unwrap(), SelfSetupSupport::Legacy);
    }
}
