//! Shared CI-environment and interactive-terminal detection.

pub use vite_powershell::is_stdin_terminal;

/// Common CI environment variables.
const CI_ENV_VARS: &[&str] = &[
    "CI",
    "CONTINUOUS_INTEGRATION",
    "GITHUB_ACTIONS",
    "GITLAB_CI",
    "CIRCLECI",
    "TRAVIS",
    "JENKINS_URL",
    "BUILDKITE",
    "DRONE",
    "CODEBUILD_BUILD_ID", // AWS CodeBuild
    "TF_BUILD",           // Azure Pipelines
];

/// Check if running in a CI environment.
pub fn is_ci_environment() -> bool {
    CI_ENV_VARS.iter().any(|key| std::env::var_os(key).is_some())
}

/// Memoized `stdout` terminal check; the fd's TTY-ness cannot change
/// within a process.
#[expect(clippy::disallowed_methods, reason = "the memoizing wrapper itself")]
pub fn is_stdout_terminal() -> bool {
    use std::{io::IsTerminal, sync::LazyLock};
    static IS_TTY: LazyLock<bool> = LazyLock::new(|| std::io::stdout().is_terminal());
    *IS_TTY
}

/// Memoized `stderr` terminal check; the fd's TTY-ness cannot change
/// within a process.
#[expect(clippy::disallowed_methods, reason = "the memoizing wrapper itself")]
pub fn is_stderr_terminal() -> bool {
    use std::{io::IsTerminal, sync::LazyLock};
    static IS_TTY: LazyLock<bool> = LazyLock::new(|| std::io::stderr().is_terminal());
    *IS_TTY
}

/// True when vp can show interactive prompts: stdin and stdout are terminals,
/// the terminal is capable, and this is not a CI environment.
///
/// The CI check is not redundant with the TTY checks: some CI systems run
/// commands in a PTY (Buildkite does by default), where an interactive prompt
/// would hang the job.
pub fn is_interactive_terminal() -> bool {
    is_stdin_terminal()
        && is_stdout_terminal()
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
        && !is_ci_environment()
}
