use super::*;

#[derive(Debug, Clone)]
pub struct ReleaseOptions {
    pub dry_run: bool,
    pub skip_publish: bool,
    pub first_release: bool,
    pub changelog: bool,
    pub preid: Option<String>,
    pub otp: Option<String>,
    pub projects: Option<Vec<String>>,
    pub git_tag: bool,
    pub git_commit: bool,
    pub yes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PrereleaseTag {
    Standard(VersionBump),
    Custom(String),
}

impl PrereleaseTag {
    pub(super) fn parse(value: &str) -> Self {
        match value {
            "alpha" => Self::Standard(VersionBump::Alpha),
            "beta" => Self::Standard(VersionBump::Beta),
            "rc" => Self::Standard(VersionBump::Rc),
            _ => Self::Custom(value.to_owned()),
        }
    }

    pub(super) fn as_str(&self) -> &str {
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

#[derive(Debug, Clone)]
pub(super) struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub level: VersionBump,
}

#[derive(Debug, Clone)]
pub(super) struct WorkspacePackage {
    pub node: PackageNodeIndex,
    pub name: String,
    pub known_names: Vec<String>,
    pub retired_names: Vec<String>,
    pub release_paths: Vec<String>,
    pub selection_order: usize,
    pub manifest_path: AbsolutePathBuf,
    pub package_path: AbsolutePathBuf,
    pub manifest_contents: String,
    pub manifest: PackageManifest,
}

#[derive(Debug, Clone)]
pub(super) struct PackageReleasePlan {
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
    pub access: Option<String>,
    pub publish_tag: Option<String>,
    pub repository_url: Option<String>,
    pub protocol_summary: DependencyProtocolSummary,
    pub tag_name: String,
    pub scripts: Vec<String>,
    pub check_scripts: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct ManifestEdit {
    pub package: String,
    pub path: AbsolutePathBuf,
    pub original_contents: String,
    pub updated_contents: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct FirstPublishGuidance {
    pub github_repo: Option<String>,
    pub release_branch: Option<String>,
    pub workflow_path: String,
    pub packages_missing_repository: Vec<String>,
    pub packages_mismatched_repository: Vec<String>,
    pub scoped_packages_missing_public_access: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ReleaseReadinessReport {
    pub workspace_scripts: Vec<String>,
    pub package_scripts: Vec<PackageReadiness>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PackageReadiness {
    pub package: String,
    pub scripts: Vec<String>,
}

pub(super) const DEFAULT_RELEASE_CHECK_SCRIPTS: [&str; 5] =
    ["build", "pack", "prepack", "prepublishOnly", "prepare"];

pub(super) type WorkspacePackageGraph =
    petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx>;

pub(super) fn push_display<T>(buffer: &mut String, value: T)
where
    T: fmt::Display,
{
    let _ = write!(buffer, "{value}");
}

pub(super) fn push_joined<'a, I>(buffer: &mut String, values: I, separator: &str)
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

pub(super) fn validate_release_options(options: &ReleaseOptions) -> Result<(), Error> {
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

pub(super) fn resolved_publish_tag<'a>(
    plan: &'a PackageReleasePlan,
    options: &'a ReleaseOptions,
) -> Option<&'a str> {
    options.preid.as_deref().or(plan.publish_tag.as_deref())
}
