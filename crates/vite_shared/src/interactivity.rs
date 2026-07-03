//! Shared CI-environment and interactive-terminal detection.

use std::io::IsTerminal;

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

/// True when vp can show interactive prompts: stdin and stdout are terminals,
/// the terminal is capable, and this is not a CI environment.
pub fn is_interactive_terminal() -> bool {
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && std::env::var("TERM").map_or(true, |term| term != "dumb")
        && !is_ci_environment()
}
