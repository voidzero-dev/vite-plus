//! Shared utilities for vite-plus crates

pub mod conventional_commit;
mod env_config;
pub mod env_vars;
pub mod git;
pub mod header;
mod home;
pub mod output;
mod package_json;
mod path_env;
pub mod string_similarity;
mod tls;
mod tracing;
pub mod versioning;

pub use conventional_commit::{ConventionalCommit, parse_conventional_commit};
pub use env_config::{EnvConfig, TestEnvGuard};
pub use home::get_vp_home;
pub use git::{GitError, capture_git, is_clean_git_worktree, parse_github_repo_slug, run_git};
pub use package_json::{
    DependencyProtocolSummary, DevEngines, Engines, PackageJson, PackageJsonError, PackageManifest,
    PackageManifestDocument, PublishConfig, ReleaseLifecycle, RuntimeEngine, RuntimeEngineConfig,
    VitePlusMetadata, WorkspaceReference, WorkspaceVersionSpec, parse_workspace_reference,
    read_package_manifest, replace_top_level_string_property,
};
pub use path_env::{
    PrependOptions, PrependResult, format_path_prepended, format_path_with_prepend,
    prepend_to_path_env,
};
pub use tls::ensure_tls_provider;
pub use tracing::init_tracing;
pub use versioning::{
    Version, VersionBump, VersionError, VersionPattern, VersionPrefix, build_prerelease,
    bump_version, parse_version_pattern, prerelease_channel, prerelease_number, strip_prerelease,
};
