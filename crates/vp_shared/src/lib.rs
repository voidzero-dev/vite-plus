//! Shared utilities for vite-plus crates

#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_types,
    clippy::print_stdout
)]

pub mod conventional_commit;
mod env_config;
pub mod env_vars;
mod error;
pub mod git;
pub mod header;
mod home;
mod http;
mod interactivity;
mod json_edit;
pub mod output;
mod package_json;
mod path_env;
mod process;
mod release_manifest;
mod stdio;
pub mod string_similarity;
mod tls;
mod tracing;
pub mod versioning;

pub use conventional_commit::{ConventionalCommit, parse_conventional_commit};
pub use env_config::{EnvConfig, TestEnvGuard};
pub use error::format_error_chain;
pub use git::{GitError, capture_git, is_clean_git_worktree, parse_github_repo_slug, run_git};
pub use home::{VP_BINARY_NAME, get_vp_home};
pub use http::{HttpClientError, download_timeout, shared_http_client};
pub use interactivity::{
    is_ci_environment, is_interactive_terminal, is_stderr_terminal, is_stdin_terminal,
    is_stdout_terminal,
};
pub use json_edit::{JsonStyle, edit_json_object, insert_after};
pub use package_json::{
    DevEngineDependency, DevEngineField, DevEngines, Engines, OnFail, PackageJson, dev_engine_entry,
};
pub use path_env::{
    PrependOptions, PrependResult, format_path_prepended, format_path_with_prepend,
    prepend_to_path_env,
};
pub use process::exit_code_from_status;
pub use release_manifest::{
    DependencyProtocolSummary, PackageJsonError, PackageManifest, PackageManifestDocument,
    PublishConfig, ReleaseLifecycle, VitePlusMetadata, WorkspaceReference, WorkspaceVersionSpec,
    parse_workspace_reference, read_package_manifest, replace_dependency_version_ranges,
    replace_top_level_string_property,
};
pub use stdio::ensure_blocking_stdio;
pub use tls::ensure_tls_provider;
pub use tracing::init_tracing;
pub use versioning::{
    Version, VersionBump, VersionError, VersionPattern, VersionPrefix, build_prerelease,
    bump_version, parse_version_pattern, prerelease_channel, prerelease_number, strip_prerelease,
};
