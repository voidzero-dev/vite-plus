//! Workspace release workflow for versioning, changelog generation, and coordinated publishing.
//!
//! References:
//! - SemVer 2.0.0: https://semver.org/spec/v2.0.0.html
//! - SemVer FAQ for `0.y.z`: https://semver.org/#faq
//! - Conventional Commits 1.0.0: https://www.conventionalcommits.org/en/v1.0.0/#specification
//! - Conventional Commits FAQ: https://www.conventionalcommits.org/en/v1.0.0/#faq

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::{self, Write as _},
    fs,
    io::{IsTerminal, Write},
};

use chrono::Local;
use glob::Pattern;
use petgraph::visit::EdgeRef;
use vite_install::{
    PackageManager, commands::publish::PublishCommandOptions, package_manager::PackageManagerType,
};
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::{
    DependencyProtocolSummary, PackageJsonError, PackageManifest, Version, VersionBump,
    VersionPattern, build_prerelease, bump_version, capture_git, is_clean_git_worktree, output,
    parse_conventional_commit, parse_github_repo_slug, parse_version_pattern, prerelease_channel,
    prerelease_number, read_package_manifest, replace_dependency_version_ranges,
    replace_top_level_string_property, run_git, strip_prerelease,
};
use vite_task::ExitStatus;
use vite_workspace::{DependencyType, PackageInfo, PackageNodeIndex};

use crate::error::Error;

mod domain;
mod first_publish;
mod manager;
mod planning;
mod presentation;
mod protocols;
mod storage;
#[cfg(test)]
mod tests;

use self::{
    domain::{
        CommitInfo, DEFAULT_RELEASE_CHECK_SCRIPTS, FirstPublishGuidance, ManifestEdit,
        PackageReadiness, PackageReleasePlan, PrereleaseTag, ReleaseReadinessReport,
        WorkspacePackage, WorkspacePackageGraph, push_display, push_joined, resolved_publish_tag,
        validate_release_options,
    },
    first_publish::*,
    manager::execute_release,
    planning::*,
    presentation::*,
    protocols::*,
    storage::*,
};

use super::{build_package_manager, prepend_js_runtime_to_path_env};

pub use self::domain::ReleaseOptions;

pub async fn execute(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Result<ExitStatus, Error> {
    execute_release(cwd, options).await
}
