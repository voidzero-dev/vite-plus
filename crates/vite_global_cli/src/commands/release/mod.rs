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

/// Shared release state and policy helpers.
///
/// This module intentionally keeps the release pipeline's common vocabulary at the module root:
///
/// - CLI/config inputs (`ReleaseOptions`)
/// - release planning records (`WorkspacePackage`, `PackageReleasePlan`, `CommitInfo`)
/// - file mutation descriptors (`ManifestEdit`, `ReleaseArtifactEdit`)
/// - readiness/security state (`ReleaseReadinessReport`, `TrustedPublishContext`)
///
/// Keeping these concepts here makes the higher-level modules easier to read:
///
/// - `planning` computes these values
/// - `reporting` renders them
/// - `storage` persists or rolls them back
/// - `manager` orchestrates the full workflow by moving between these states
///
/// This root module deliberately avoids heavy I/O in the shared helpers below. Expensive work
/// should stay in the specialized sibling modules.

/// User-controlled knobs for `vp release`.
///
/// These values are parsed from the CLI layer and then treated as immutable input for the rest of
/// the release pipeline.
#[derive(Debug, Clone)]
pub struct ReleaseOptions {
    /// When `true`, compute and print the release plan without mutating files or creating tags.
    pub dry_run: bool,
    /// During `--dry-run`, suppress native publish simulation and show only local artifact changes.
    pub skip_publish: bool,
    /// Ignores prior release tags and treats the selected packages as unpublished.
    pub first_release: bool,
    /// Enables root and per-package changelog generation.
    pub changelog: bool,
    /// Exact version to publish instead of auto-computing the next release.
    pub version: Option<String>,
    /// Optional prerelease dist-tag/channel override such as `alpha`, `beta`, or `rc`.
    pub preid: Option<String>,
    /// Legacy TOTP override for package-manager publish commands.
    pub otp: Option<String>,
    /// Optional package-selection filter. The original order is preserved as a tiebreaker.
    pub projects: Option<Vec<String>>,
    /// Whether local git tags should be created after a successful release.
    pub git_tag: bool,
    /// Whether local release artifacts should be committed.
    pub git_commit: bool,
    /// Whether release checks should run before publish.
    pub run_checks: bool,
    /// Skips the interactive confirmation prompt.
    pub yes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PrereleaseTag {
    Standard(VersionBump),
    Custom(String),
}

impl PrereleaseTag {
    fn parse(value: &str) -> Self {
        match value {
            "alpha" => Self::Standard(VersionBump::Alpha),
            "beta" => Self::Standard(VersionBump::Beta),
            "rc" => Self::Standard(VersionBump::Rc),
            _ => Self::Custom(value.to_owned()),
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Standard(level) => {
                debug_assert!(matches!(
                    level,
                    VersionBump::Alpha | VersionBump::Beta | VersionBump::Rc
                ));
                level.as_str()
            }
            Self::Custom(value) => value,
        }
    }
}

/// Conventional-commit information extracted from git history for release planning.
#[derive(Debug, Clone)]
struct CommitInfo {
    /// Full commit SHA used for de-duplication and changelog links.
    pub hash: String,
    /// Short SHA printed in changelogs and summaries.
    pub short_hash: String,
    /// The human-readable commit subject.
    pub subject: String,
    /// Semantic-release level inferred from the commit message/body.
    pub level: VersionBump,
}

/// A publishable workspace package after workspace discovery but before version planning.
#[derive(Debug, Clone)]
struct WorkspacePackage {
    /// Stable graph node id used for dependency ordering.
    pub node: PackageNodeIndex,
    /// Current package name from `package.json`.
    pub name: String,
    /// Current plus historical names that should still match release tags.
    pub known_names: Vec<String>,
    /// Names that used to exist but should no longer map back to this package.
    pub retired_names: Vec<String>,
    /// Paths whose git history should contribute to this package's release notes.
    pub release_paths: Vec<String>,
    /// Original user selection order used to break ties between independent packages.
    pub selection_order: usize,
    /// Absolute path to the package manifest.
    pub manifest_path: AbsolutePathBuf,
    /// Absolute path to the package directory.
    pub package_path: AbsolutePathBuf,
    /// Original raw `package.json` contents used for edit/rollback operations.
    pub manifest_contents: String,
    /// Parsed manifest subset used by release logic.
    pub manifest: PackageManifest,
}

/// Fully planned release information for one package.
///
/// By the time a `PackageReleasePlan` exists, commit classification, next-version calculation,
/// publish metadata selection, and tag naming have already been resolved.
#[derive(Debug, Clone)]
struct PackageReleasePlan {
    pub name: String,
    pub known_names: Vec<String>,
    pub retired_names: Vec<String>,
    pub package_path: AbsolutePathBuf,
    pub manifest_path: AbsolutePathBuf,
    pub manifest_contents: String,
    pub manifest: PackageManifest,
    pub current_version: Version,
    pub next_version: Version,
    pub level: VersionBump,
    pub commits: Vec<CommitInfo>,
    pub changelog_path: AbsolutePathBuf,
    /// Publish-time access level, typically `public` for scoped public packages.
    pub access: Option<String>,
    /// Dist-tag that should be used if the CLI does not override it.
    pub publish_tag: Option<String>,
    /// Explicit provenance preference from `publishConfig`.
    pub publish_provenance: Option<bool>,
    /// Normalized repository URL used for trusted-publishing validation.
    pub repository_url: Option<String>,
    /// Summary of special dependency protocols that may require package-manager rewriting.
    pub protocol_summary: DependencyProtocolSummary,
    /// Final git tag name that will be created on success.
    pub tag_name: String,
    /// Scripts available on the package, used for readiness reporting.
    pub scripts: Vec<String>,
    /// Release-specific script names configured under `vitePlus.release.checkScripts`.
    pub check_scripts: Vec<String>,
}

/// Temporary package-manifest rewrite used during publish preflight and publish execution.
#[derive(Debug, Clone)]
struct ManifestEdit {
    /// Package label used in error messages.
    pub package: String,
    /// Path to the manifest being rewritten.
    pub path: AbsolutePathBuf,
    /// Original contents restored after preflight/publish completes.
    pub original_contents: String,
    /// Publish-time rewritten contents.
    pub updated_contents: String,
}

/// Final release artifact mutation applied after publish succeeds.
///
/// Unlike `ManifestEdit`, this type is used for the durable local changes that should remain in
/// git after the release completes, such as version bumps and changelog entries.
#[derive(Debug, Clone)]
struct ReleaseArtifactEdit {
    /// Human-readable artifact description used in rollback errors.
    pub label: String,
    /// File path to update.
    pub path: AbsolutePathBuf,
    /// Pre-existing file contents, or `None` when the file is generated by the release.
    pub original_contents: Option<String>,
    /// Desired file contents after the release finalizes locally.
    pub updated_contents: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FirstPublishGuidance {
    pub github_repo: Option<String>,
    pub dispatch_ref_hint: Option<String>,
    pub workflow_path: String,
    pub workflow_template_created: bool,
    pub packages_missing_repository: Vec<String>,
    pub packages_mismatched_repository: Vec<String>,
    pub scoped_packages_missing_public_access: Vec<String>,
}

/// Combined readiness summary rendered before a release is confirmed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReleaseReadinessReport {
    /// Release-related scripts found at the workspace root.
    pub workspace_scripts: Vec<String>,
    /// Release-related scripts found on individual packages.
    pub package_scripts: Vec<PackageReadiness>,
    /// Trusted-publishing posture detected for this run.
    pub trusted_publish: TrustedPublishReadiness,
    /// Human-readable warnings that should be reviewed before continuing.
    pub warnings: Vec<String>,
}

/// Release-related scripts detected for one package.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageReadiness {
    pub package: String,
    pub scripts: Vec<String>,
}

/// CI provider whose environment variables match npm trusted publishing support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrustedPublishProvider {
    GitHubActions,
    GitLabCi,
    CircleCi,
}

impl TrustedPublishProvider {
    #[must_use]
    const fn as_str(self) -> &'static str {
        match self {
            Self::GitHubActions => "GitHub Actions",
            Self::GitLabCi => "GitLab CI",
            Self::CircleCi => "CircleCI",
        }
    }
}

/// Runner type relevant to provenance-capable trusted publishing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum TrustedPublishRunnerEnvironment {
    GitHubHosted,
    SelfHosted,
    #[default]
    Unknown,
}

impl TrustedPublishRunnerEnvironment {
    #[must_use]
    const fn as_str(self) -> &'static str {
        match self {
            Self::GitHubHosted => "github-hosted",
            Self::SelfHosted => "self-hosted",
            Self::Unknown => "unknown",
        }
    }
}

/// Detected CI/trusted-publishing context for the current process.
///
/// This structure is intentionally small and serializable-by-thought: it contains only the bits of
/// environment metadata the release flow needs for policy checks, user-facing summaries, and
/// provenance defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TrustedPublishContext {
    /// CI provider inferred from process environment.
    pub provider: Option<TrustedPublishProvider>,
    /// Whether the current job is running on infrastructure compatible with trusted publishing.
    pub runner_environment: TrustedPublishRunnerEnvironment,
    /// Repository slug exposed by the CI provider.
    pub repository: Option<String>,
    /// Human-readable workflow/job name.
    pub workflow_name: Option<String>,
    /// Provider-specific workflow reference used to reconstruct the workflow file path.
    pub workflow_ref: Option<String>,
}

impl TrustedPublishContext {
    /// Detects the current trusted-publishing context from process environment variables.
    #[must_use]
    fn detect() -> Self {
        Self::from_env(|key| std::env::var(key).ok())
    }

    /// Environment-backed constructor used by production code and unit tests alike.
    #[must_use]
    fn from_env<F>(mut get: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let github_actions = get("GITHUB_ACTIONS");
        if github_actions.as_deref() == Some("true") {
            return Self {
                provider: Some(TrustedPublishProvider::GitHubActions),
                runner_environment: match get("RUNNER_ENVIRONMENT").as_deref() {
                    Some("github-hosted") => TrustedPublishRunnerEnvironment::GitHubHosted,
                    Some("self-hosted") => TrustedPublishRunnerEnvironment::SelfHosted,
                    _ => TrustedPublishRunnerEnvironment::Unknown,
                },
                repository: get("GITHUB_REPOSITORY"),
                workflow_name: get("GITHUB_WORKFLOW"),
                workflow_ref: get("GITHUB_WORKFLOW_REF"),
            };
        }

        if get("GITLAB_CI").as_deref() == Some("true") {
            return Self { provider: Some(TrustedPublishProvider::GitLabCi), ..Self::default() };
        }

        if get("CIRCLECI").as_deref() == Some("true") {
            return Self { provider: Some(TrustedPublishProvider::CircleCi), ..Self::default() };
        }

        Self::default()
    }

    /// Returns whether the current environment is eligible for npm trusted publishing at all.
    #[must_use]
    const fn supports_trusted_publishing(&self) -> bool {
        match self.provider {
            Some(TrustedPublishProvider::GitHubActions) => {
                !matches!(self.runner_environment, TrustedPublishRunnerEnvironment::SelfHosted)
            }
            Some(TrustedPublishProvider::GitLabCi | TrustedPublishProvider::CircleCi) => true,
            None => false,
        }
    }

    /// Returns whether the current environment can emit npm provenance attestations.
    ///
    /// The hardened release policy is stricter than baseline trusted-publishing support: it
    /// requires provenance-capable infrastructure, not just OIDC-based auth.
    #[must_use]
    const fn supports_publish_provenance(&self) -> bool {
        match self.provider {
            Some(TrustedPublishProvider::GitHubActions) => {
                matches!(self.runner_environment, TrustedPublishRunnerEnvironment::GitHubHosted)
            }
            Some(TrustedPublishProvider::GitLabCi) => true,
            Some(TrustedPublishProvider::CircleCi) | None => false,
        }
    }

    /// Human-readable environment label used in release summaries and validation errors.
    #[must_use]
    fn environment_summary(&self) -> String {
        match self.provider {
            Some(TrustedPublishProvider::GitHubActions) => {
                let mut summary = String::from("GitHub Actions");
                match self.runner_environment {
                    TrustedPublishRunnerEnvironment::GitHubHosted => {
                        summary.push_str(" on a GitHub-hosted runner");
                    }
                    TrustedPublishRunnerEnvironment::SelfHosted => {
                        summary.push_str(" on a self-hosted runner");
                    }
                    TrustedPublishRunnerEnvironment::Unknown => {
                        summary.push_str(" (runner environment unavailable)");
                    }
                }
                summary
            }
            Some(provider) => {
                let mut summary = String::from(provider.as_str());
                summary.push_str(" (trusted publish eligible environment detected)");
                summary
            }
            None => String::from("not detected (local shell or unsupported CI provider)"),
        }
    }

    /// Best-effort reconstruction of the workflow file path from provider metadata.
    #[must_use]
    fn workflow_path(&self) -> Option<String> {
        let workflow_ref = self.workflow_ref.as_deref()?;
        let (_, suffix) = workflow_ref.split_once("/.github/workflows/")?;
        let (filename, _) = suffix.split_once('@')?;
        let mut path = String::from(".github/workflows/");
        path.push_str(filename);
        Some(path)
    }
}

/// Trusted-publishing-specific readiness details folded into the broader release report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TrustedPublishReadiness {
    /// Detected execution context.
    pub context: TrustedPublishContext,
    /// Packages that explicitly disable provenance and therefore violate hardened release policy.
    pub packages_with_provenance_disabled: Vec<String>,
    /// Whether the operator supplied a legacy TOTP code.
    pub uses_legacy_otp: bool,
}

/// Compact count of local release artifacts touched by the current plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReleaseArtifactSummary {
    pub manifest_file_count: usize,
    pub changelog_file_count: usize,
}

impl ReleaseArtifactSummary {
    #[must_use]
    const fn total_file_count(self) -> usize {
        self.manifest_file_count + self.changelog_file_count
    }
}

/// Result of dry-run publish simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DryRunPublishStatus {
    SkippedByOption,
    SkippedDirtyWorktree,
    Failed,
    Succeeded,
}

const DEFAULT_RELEASE_CHECK_SCRIPTS: [&str; 5] =
    ["build", "pack", "prepack", "prepublishOnly", "prepare"];

type WorkspacePackageGraph =
    petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx>;

fn push_display<T>(buffer: &mut String, value: T)
where
    T: fmt::Display,
{
    let _ = write!(buffer, "{value}");
}

fn push_joined<'a, I>(buffer: &mut String, values: I, separator: &str)
where
    I: IntoIterator<Item = &'a str>,
{
    let mut first = true;
    for value in values {
        if !first {
            buffer.push_str(separator);
        }
        first = false;
        buffer.push_str(value);
    }
}

/// Rejects option combinations that would produce ambiguous or unsafe release boundaries.
fn validate_release_options(options: &ReleaseOptions) -> Result<(), Error> {
    if let Some(version) = options.version.as_deref() {
        Version::parse(version).map_err(|error| {
            let mut message = String::from("Invalid `--version` value '");
            message.push_str(version);
            message.push_str("': ");
            push_display(&mut message, error);
            Error::UserMessage(message.into())
        })?;
    }
    if options.version.is_some() && options.preid.is_some() {
        return Err(Error::UserMessage(
            "`vp release --version` cannot be combined with `--preid` because the exact target version already determines the prerelease channel."
                .into(),
        ));
    }

    if options.git_tag && !options.git_commit && !options.dry_run {
        return Err(Error::UserMessage(
            "`vp release --no-git-commit --git-tag` is not supported because tags would not point to the release changes."
                .into(),
        ));
    }

    if !options.git_tag && !options.dry_run {
        return Err(Error::UserMessage(
            "`vp release --no-git-tag` is only supported with `--dry-run` because git tags are the release watermark used to avoid re-releasing the same commits."
                .into(),
        ));
    }

    if options.skip_publish && !options.dry_run {
        return Err(Error::UserMessage(
            "`vp release --skip-publish` is only supported with `--dry-run` because vite-plus treats a successful publish plus git tags as the release boundary."
                .into(),
        ));
    }

    Ok(())
}

/// Rejects real publishes that do not meet the hardened trusted-publishing requirements.
fn validate_trusted_publish_context(
    options: &ReleaseOptions,
    context: &TrustedPublishContext,
) -> Result<(), Error> {
    if options.dry_run {
        return Ok(());
    }

    if context.supports_publish_provenance() {
        return Ok(());
    }

    let mut message = String::from(
        "Real `vp release` publishes are intended to run from npm trusted-publishing CI, but this environment is ",
    );
    message.push_str(&context.environment_summary());
    message.push_str(
        ". Run `vp release --dry-run` locally to verify the plan, then rerun the real publish from CI with OIDC (`id-token: write`) and provenance enabled.",
    );
    message.push_str(
        " If you need an interactive maintainer fallback outside CI, prefer npm passkey/security-key auth over long-lived tokens or `--otp`.",
    );

    if matches!(context.provider, Some(TrustedPublishProvider::GitHubActions))
        && matches!(context.runner_environment, TrustedPublishRunnerEnvironment::SelfHosted)
    {
        message.push_str(
            " npm trusted publishing does not support GitHub Actions self-hosted runners.",
        );
    }
    if matches!(context.provider, Some(TrustedPublishProvider::CircleCi)) {
        message.push_str(
            " CircleCI trusted publishing does not currently generate npm provenance attestations, so this hardened release flow rejects it.",
        );
    }

    Err(Error::UserMessage(message.into()))
}

/// Resolves the effective dist-tag for a package release.
fn resolved_publish_tag<'a>(
    plan: &'a PackageReleasePlan,
    options: &'a ReleaseOptions,
) -> Option<&'a str> {
    options.preid.as_deref().or(plan.publish_tag.as_deref())
}

/// Resolves the effective provenance preference for a package release.
///
/// Package-level opt-outs win, otherwise provenance defaults to `true` when the runtime context
/// is capable of generating trustworthy attestations.
fn resolved_publish_provenance(
    plan: &PackageReleasePlan,
    context: &TrustedPublishContext,
) -> Option<bool> {
    plan.publish_provenance.or_else(|| context.supports_publish_provenance().then_some(true))
}

mod first_publish;
mod manager;
mod planning;
mod protocols;
mod reporting;
mod storage;
#[cfg(test)]
mod tests;

use self::{
    first_publish::*, manager::execute_release, planning::*, protocols::*, reporting::*, storage::*,
};
use super::{build_package_manager, prepend_js_runtime_to_path_env};

pub async fn execute(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Result<ExitStatus, Error> {
    execute_release(cwd, options).await
}
