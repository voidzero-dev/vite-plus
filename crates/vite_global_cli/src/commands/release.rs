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

mod first_publish;
mod planning;
mod presentation;
mod protocols;
mod storage;

use self::{first_publish::*, planning::*, presentation::*, protocols::*, storage::*};

use super::{build_package_manager, prepend_js_runtime_to_path_env};

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

#[derive(Debug, Clone)]
struct CommitInfo {
    hash: String,
    short_hash: String,
    subject: String,
    level: VersionBump,
}

#[derive(Debug, Clone)]
struct WorkspacePackage {
    node: PackageNodeIndex,
    name: String,
    known_names: Vec<String>,
    retired_names: Vec<String>,
    release_paths: Vec<String>,
    selection_order: usize,
    manifest_path: AbsolutePathBuf,
    package_path: AbsolutePathBuf,
    manifest_contents: String,
    manifest: PackageManifest,
}

#[derive(Debug, Clone)]
struct PackageReleasePlan {
    name: String,
    known_names: Vec<String>,
    retired_names: Vec<String>,
    package_path: AbsolutePathBuf,
    manifest_path: AbsolutePathBuf,
    manifest_contents: String,
    manifest: PackageManifest,
    current_version: Version,
    next_version: Version,
    level: VersionBump,
    commits: Vec<CommitInfo>,
    changelog_path: AbsolutePathBuf,
    access: Option<String>,
    publish_tag: Option<String>,
    repository_url: Option<String>,
    protocol_summary: DependencyProtocolSummary,
    tag_name: String,
    scripts: Vec<String>,
    check_scripts: Vec<String>,
}

#[derive(Debug, Clone)]
struct ManifestEdit {
    package: String,
    path: AbsolutePathBuf,
    original_contents: String,
    updated_contents: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct FirstPublishGuidance {
    github_repo: Option<String>,
    release_branch: Option<String>,
    workflow_path: String,
    packages_missing_repository: Vec<String>,
    packages_mismatched_repository: Vec<String>,
    scoped_packages_missing_public_access: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ReleaseReadinessReport {
    workspace_scripts: Vec<String>,
    package_scripts: Vec<PackageReadiness>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageReadiness {
    package: String,
    scripts: Vec<String>,
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

fn validate_release_options(options: &ReleaseOptions) -> Result<(), Error> {
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

fn resolved_publish_tag<'a>(
    plan: &'a PackageReleasePlan,
    options: &'a ReleaseOptions,
) -> Option<&'a str> {
    options.preid.as_deref().or(plan.publish_tag.as_deref())
}

struct ReleaseWorkspace {
    package_manager: PackageManager,
    workspace_root_path: AbsolutePathBuf,
    workspace_manifest: Option<PackageManifest>,
    package_graph: WorkspacePackageGraph,
    workspace_packages: Vec<WorkspacePackage>,
}

struct PreparedRelease {
    package_manager: PackageManager,
    workspace_root_path: AbsolutePathBuf,
    workspace_manifest: Option<PackageManifest>,
    release_plans: Vec<PackageReleasePlan>,
    manifest_edits: Vec<ManifestEdit>,
    root_commits: Vec<CommitInfo>,
}

struct ReleaseManager {
    cwd: AbsolutePathBuf,
    options: ReleaseOptions,
}

impl ReleaseManager {
    fn new(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Self {
        Self { cwd, options }
    }

    async fn run(self) -> Result<ExitStatus, Error> {
        validate_release_options(&self.options)?;

        let workspace = self.load_workspace().await?;
        let Some(release) = self.prepare_release(workspace)? else {
            return Ok(ExitStatus::SUCCESS);
        };

        let readiness_report = self.present_release(&release);
        self.validate_publish_protocol_safety(&release)?;

        if self.options.dry_run {
            self.run_dry_run(&release).await
        } else {
            self.run_release(&release, &readiness_report).await
        }
    }

    async fn load_workspace(&self) -> Result<ReleaseWorkspace, Error> {
        prepend_js_runtime_to_path_env(&self.cwd).await?;
        let package_manager = build_package_manager(&self.cwd).await?;

        let (workspace_root, _) = vite_workspace::find_workspace_root(&self.cwd)?;
        let workspace_root_path = workspace_root.path.to_absolute_path_buf();
        let package_graph = vite_workspace::load_package_graph(&workspace_root)?;
        let workspace_manifest = read_workspace_manifest(&workspace_root_path)?;
        let workspace_packages = load_workspace_packages(&package_graph)?;
        let orphaned_released_packages =
            collect_orphaned_released_packages(&workspace_root_path, &workspace_packages)?;
        if !orphaned_released_packages.is_empty() {
            let mut message = String::from(
                "Previously released packages no longer map to an active workspace package: ",
            );
            push_joined(&mut message, orphaned_released_packages.iter().map(String::as_str), ", ");
            message.push_str(". Add `vitePlus.release.previousNames` / `retiredNames` metadata where appropriate and consider deprecating removed packages on the registry.");
            output::warn(&message);
        }

        Ok(ReleaseWorkspace {
            package_manager,
            workspace_root_path,
            workspace_manifest,
            package_graph,
            workspace_packages,
        })
    }

    fn prepare_release(
        &self,
        workspace: ReleaseWorkspace,
    ) -> Result<Option<PreparedRelease>, Error> {
        let selected = select_workspace_packages(
            &workspace.workspace_packages,
            self.options.projects.as_deref(),
        )?;
        if selected.is_empty() {
            output::warn("No publishable packages matched the release selection.");
            return Ok(None);
        }

        let ordered = topological_sort_selected_packages(&workspace.package_graph, &selected);
        let (release_plans, root_commits) =
            self.build_release_plans(&workspace.workspace_root_path, ordered)?;
        if release_plans.is_empty() {
            output::warn("No releasable package changes were found.");
            return Ok(None);
        }

        let manifest_edits = build_manifest_edits(&release_plans)?;
        Ok(Some(PreparedRelease {
            package_manager: workspace.package_manager,
            workspace_root_path: workspace.workspace_root_path,
            workspace_manifest: workspace.workspace_manifest,
            release_plans,
            manifest_edits,
            root_commits,
        }))
    }

    fn build_release_plans(
        &self,
        workspace_root_path: &AbsolutePath,
        ordered: Vec<WorkspacePackage>,
    ) -> Result<(Vec<PackageReleasePlan>, Vec<CommitInfo>), Error> {
        let mut release_plans = Vec::with_capacity(ordered.len());
        let mut root_commits = Vec::new();
        let mut seen_commit_hashes = HashSet::new();

        for package in ordered {
            let previous_tag = if self.options.first_release {
                None
            } else {
                find_latest_package_tag(workspace_root_path, &package.known_names)?
            };
            let latest_stable_version = if self.options.first_release {
                None
            } else {
                find_latest_stable_package_version(workspace_root_path, &package.known_names)?
            };

            let commits = collect_package_commits(
                workspace_root_path,
                &package.release_paths,
                previous_tag.as_deref(),
            )?;

            let Some(level) = highest_release_level(&commits) else {
                let mut message = String::from("Skipping ");
                message.push_str(&package.name);
                message.push_str(" because no releasable conventional commits were found.");
                output::note(&message);
                continue;
            };

            let current_version = Version::parse(&package.manifest.version).map_err(|e| {
                let mut message = String::from("Package '");
                message.push_str(&package.name);
                message.push_str("' has an invalid version '");
                message.push_str(&package.manifest.version);
                message.push_str("': ");
                push_display(&mut message, e);
                Error::UserMessage(message.into())
            })?;
            let level = effective_release_level(&current_version, level);
            let next_version = next_release_version(
                &current_version,
                level,
                latest_stable_version.as_ref(),
                self.options.preid.as_deref(),
            )?;

            for commit in &commits {
                if seen_commit_hashes.insert(commit.hash.clone()) {
                    root_commits.push(commit.clone());
                }
            }

            release_plans.push(PackageReleasePlan {
                name: package.name.clone(),
                known_names: package.known_names.clone(),
                retired_names: package.retired_names.clone(),
                package_path: package.package_path.clone(),
                manifest_path: package.manifest_path.clone(),
                manifest_contents: package.manifest_contents,
                manifest: package.manifest.clone(),
                current_version,
                next_version,
                level,
                commits,
                changelog_path: package.package_path.join("CHANGELOG.md"),
                access: package.manifest.publish_config.access.clone(),
                publish_tag: package.manifest.publish_config.tag.clone(),
                repository_url: package.manifest.repository_url().map(ToOwned::to_owned),
                protocol_summary: package.manifest.dependency_protocol_summary(),
                tag_name: package_tag_name(&package.name, &next_version),
                scripts: package.manifest.scripts.keys().cloned().collect(),
                check_scripts: package.manifest.vite_plus.release.check_scripts.clone(),
            });
        }

        Ok((release_plans, root_commits))
    }

    fn present_release(&self, release: &PreparedRelease) -> ReleaseReadinessReport {
        print_release_plan(&release.release_plans, &self.options);
        if self.options.first_release {
            let guidance = collect_first_publish_guidance(
                &release.workspace_root_path,
                &release.release_plans,
            );
            print_first_publish_guidance(&guidance, &self.options);
        }
        let readiness_report = collect_release_readiness_report(
            release.workspace_manifest.as_ref(),
            &release.release_plans,
        );
        print_release_readiness_report(&readiness_report);
        readiness_report
    }

    fn validate_publish_protocol_safety(&self, release: &PreparedRelease) -> Result<(), Error> {
        if self.options.skip_publish {
            return Ok(());
        }

        let mut protocol_issues = String::new();
        for plan in &release.release_plans {
            let protocols =
                unsupported_publish_protocols(&release.package_manager, plan.protocol_summary);
            if protocols.is_empty() {
                continue;
            }
            if !protocol_issues.is_empty() {
                protocol_issues.push_str(", ");
            }
            protocol_issues.push_str(&plan.name);
            protocol_issues.push_str(" (");
            push_joined(&mut protocol_issues, protocols.into_iter(), ", ");
            protocol_issues.push(')');
        }

        if protocol_issues.is_empty() {
            return Ok(());
        }

        let mut message = String::from("Publishing with ");
        push_display(&mut message, release.package_manager.client);
        message.push_str(" is unsafe because these packages still contain unsupported publish-time dependency protocols: ");
        message.push_str(&protocol_issues);
        message.push_str(". Use a package manager with native publish rewriting support where available, or rerun with `--dry-run --skip-publish` to preview versioning without publishing.");
        if self.options.dry_run {
            output::warn(&message);
            Ok(())
        } else {
            Err(Error::UserMessage(message.into()))
        }
    }

    async fn run_dry_run(&self, release: &PreparedRelease) -> Result<ExitStatus, Error> {
        print_dry_run_actions(&release.release_plans, &release.package_manager, &self.options);
        if !self.options.skip_publish {
            if is_clean_git_worktree(&release.workspace_root_path)? {
                let status = run_publish_preflight(
                    &release.package_manager,
                    &release.release_plans,
                    &release.manifest_edits,
                    &self.options,
                )
                .await?;
                if !status.success() {
                    return Ok(status);
                }
            } else {
                output::warn(
                    "Skipping native publish dry-run because the git worktree is dirty. Rerun from a clean tree for full publish preflight.",
                );
            }
        }
        Ok(ExitStatus::SUCCESS)
    }

    async fn run_release(
        &self,
        release: &PreparedRelease,
        readiness_report: &ReleaseReadinessReport,
    ) -> Result<ExitStatus, Error> {
        ensure_clean_worktree(&release.workspace_root_path)?;
        if !self.options.yes
            && !confirm_release(&release.release_plans, readiness_report, &self.options)?
        {
            return Ok(ExitStatus::SUCCESS);
        }

        let preflight_status = run_publish_preflight(
            &release.package_manager,
            &release.release_plans,
            &release.manifest_edits,
            &self.options,
        )
        .await?;
        if !preflight_status.success() {
            return Ok(preflight_status);
        }

        let publish_status = publish_packages(
            &release.package_manager,
            &release.release_plans,
            &release.manifest_edits,
            &self.options,
        )
        .await?;
        if !publish_status.success() {
            return Ok(publish_status);
        }

        let mut changed_files = Vec::with_capacity(
            release.release_plans.len() * (1 + usize::from(self.options.changelog)) + 1,
        );
        let release_date = Local::now().format("%Y-%m-%d").to_string();

        if self.options.changelog {
            let root_changelog_path = release.workspace_root_path.join("CHANGELOG.md");
            let root_section = build_root_changelog_section(
                &release_date,
                &release.release_plans,
                &release.root_commits,
            );
            write_changelog_section(&root_changelog_path, &root_section)?;
            changed_files.push(root_changelog_path);
        }

        for edit in &release.manifest_edits {
            write_manifest_contents(&edit.path, &edit.updated_contents)?;
            changed_files.push(edit.path.clone());
        }

        for plan in &release.release_plans {
            if self.options.changelog {
                let package_section = build_package_changelog_section(
                    &release_date,
                    &plan.next_version,
                    &plan.commits,
                );
                write_changelog_section(&plan.changelog_path, &package_section)?;
                changed_files.push(plan.changelog_path.clone());
            }
        }

        if self.options.git_commit {
            git_add_paths(&release.workspace_root_path, &changed_files)?;
            let commit_message = release_commit_message(&release.release_plans);
            git_commit(&release.workspace_root_path, &commit_message)?;
            let mut message = String::from("Created release commit: ");
            message.push_str(&commit_message);
            output::success(&message);
        }

        if self.options.git_tag {
            for plan in &release.release_plans {
                git_tag(&release.workspace_root_path, &plan.tag_name)?;
                let mut message = String::from("Created git tag ");
                message.push_str(&plan.tag_name);
                output::success(&message);
            }
        }

        let mut message = String::from("Release completed for ");
        push_display(&mut message, release.release_plans.len());
        message.push_str(" package(s).");
        output::success(&message);
        Ok(ExitStatus::SUCCESS)
    }
}

pub async fn execute(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Result<ExitStatus, Error> {
    ReleaseManager::new(cwd, options).run().await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vite_path::{AbsolutePathBuf, RelativePathBuf};
    use vite_str::Str;
    use vite_workspace::{PackageInfo, PackageJson, PackageNodeIndex};

    use super::*;

    fn test_absolute_path(suffix: &str) -> AbsolutePathBuf {
        #[cfg(windows)]
        let base = PathBuf::from(format!("C:\\workspace{}", suffix.replace('/', "\\")));
        #[cfg(not(windows))]
        let base = PathBuf::from(format!("/workspace{suffix}"));
        AbsolutePathBuf::new(base).unwrap()
    }

    fn build_test_package_graph()
    -> petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx> {
        let mut graph = petgraph::graph::DiGraph::default();

        let _root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: test_absolute_path("").into(),
        });
        let pkg_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-a").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-a").into(),
        });
        let pkg_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-b").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-b").into(),
        });
        let pkg_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-c").unwrap(),
            absolute_path: test_absolute_path("/packages/pkg-c").into(),
        });

        graph.add_edge(pkg_a, pkg_b, DependencyType::Normal);

        let _ = pkg_c;
        graph
    }

    fn test_package_manager(client: PackageManagerType, version: &str) -> PackageManager {
        PackageManager {
            client,
            package_name: client.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: client.to_string().into(),
            workspace_root: test_absolute_path(""),
            is_monorepo: false,
            install_dir: test_absolute_path("/install"),
        }
    }

    fn make_workspace_package(
        graph: &petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx>,
        node: PackageNodeIndex,
        selection_order: usize,
    ) -> WorkspacePackage {
        let package = &graph[node];
        WorkspacePackage {
            node,
            name: package.package_json.name.to_string(),
            known_names: vec![package.package_json.name.to_string()],
            retired_names: Vec::new(),
            release_paths: vec![package.path.as_str().to_owned()],
            selection_order,
            manifest_path: package.absolute_path.join("package.json"),
            package_path: package.absolute_path.to_absolute_path_buf(),
            manifest_contents: r#"{"name":"pkg","version":"1.0.0"}"#.into(),
            manifest: PackageManifest {
                name: package.package_json.name.to_string(),
                version: "1.0.0".into(),
                ..Default::default()
            },
        }
    }

    fn make_release_plan(
        name: &str,
        scripts: &[&str],
        check_scripts: &[&str],
    ) -> PackageReleasePlan {
        PackageReleasePlan {
            name: name.to_string(),
            known_names: vec![name.to_string()],
            retired_names: Vec::new(),
            package_path: test_absolute_path(&format!("/packages/{name}")),
            manifest_path: test_absolute_path(&format!("/packages/{name}/package.json")),
            manifest_contents: format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
            manifest: PackageManifest {
                name: name.to_string(),
                version: "1.0.0".into(),
                ..Default::default()
            },
            current_version: Version::parse("1.0.0").unwrap(),
            next_version: Version::parse("1.0.1").unwrap(),
            level: VersionBump::Patch,
            commits: Vec::new(),
            changelog_path: test_absolute_path(&format!("/packages/{name}/CHANGELOG.md")),
            access: None,
            publish_tag: None,
            repository_url: None,
            protocol_summary: DependencyProtocolSummary::default(),
            tag_name: format!("release/{name}/v1.0.1"),
            scripts: scripts.iter().map(|script| (*script).to_string()).collect(),
            check_scripts: check_scripts.iter().map(|script| (*script).to_string()).collect(),
        }
    }

    #[test]
    fn parse_github_repo_slug_supports_common_remote_formats() {
        assert_eq!(
            parse_github_repo_slug("git@github.com:voidzero-dev/vite-plus.git"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(
            parse_github_repo_slug("https://github.com/voidzero-dev/vite-plus.git"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(
            parse_github_repo_slug("github:voidzero-dev/vite-plus"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(parse_github_repo_slug("https://example.com/acme/repo.git"), None);
    }

    #[test]
    fn repository_url_reads_string_and_object_forms() {
        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "name": "@scope/pkg-a",
                "version": "1.0.0",
                "repository": "https://github.com/voidzero-dev/vite-plus.git"
            }"#,
        )
        .unwrap();
        assert_eq!(
            manifest.repository_url(),
            Some("https://github.com/voidzero-dev/vite-plus.git")
        );

        let manifest: PackageManifest = serde_json::from_str(
            r#"{
                "name": "@scope/pkg-b",
                "version": "1.0.0",
                "repository": {
                    "type": "git",
                    "url": "git@github.com:voidzero-dev/vite-plus.git"
                }
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.repository_url(), Some("git@github.com:voidzero-dev/vite-plus.git"));
    }

    #[test]
    fn render_release_command_keeps_relevant_flags() {
        let command = render_release_command(
            &ReleaseOptions {
                dry_run: false,
                skip_publish: true,
                first_release: true,
                changelog: true,
                preid: Some("alpha".into()),
                otp: None,
                projects: Some(vec!["@scope/pkg-a".into(), "@scope/pkg-b".into()]),
                git_tag: false,
                git_commit: true,
                yes: false,
            },
            true,
            true,
        );

        assert_eq!(
            command,
            "vp release --first-release --changelog --preid alpha --projects @scope/pkg-a,@scope/pkg-b --no-git-tag --skip-publish --dry-run"
        );
    }

    #[test]
    fn render_release_command_uses_yes_for_non_interactive_runs() {
        let command = render_release_command(
            &ReleaseOptions {
                dry_run: false,
                skip_publish: false,
                first_release: false,
                changelog: false,
                preid: None,
                otp: None,
                projects: None,
                git_tag: true,
                git_commit: true,
                yes: false,
            },
            false,
            true,
        );

        assert_eq!(command, "vp release --yes");
    }

    #[test]
    fn validate_release_options_rejects_real_skip_publish() {
        let error = validate_release_options(&ReleaseOptions {
            dry_run: false,
            skip_publish: true,
            first_release: false,
            changelog: false,
            preid: None,
            otp: None,
            projects: None,
            git_tag: true,
            git_commit: true,
            yes: false,
        })
        .unwrap_err();

        assert!(error.to_string().contains("--skip-publish"));
    }

    #[test]
    fn validate_release_options_rejects_real_no_git_tag() {
        let error = validate_release_options(&ReleaseOptions {
            dry_run: false,
            skip_publish: false,
            first_release: false,
            changelog: false,
            preid: None,
            otp: None,
            projects: None,
            git_tag: false,
            git_commit: true,
            yes: false,
        })
        .unwrap_err();

        assert!(error.to_string().contains("--no-git-tag"));
    }

    #[test]
    fn resolved_publish_tag_prefers_cli_preid_over_manifest_tag() {
        let mut plan = make_release_plan("pkg-a", &[], &[]);
        plan.publish_tag = Some("next".into());

        let options = ReleaseOptions {
            dry_run: false,
            skip_publish: false,
            first_release: false,
            changelog: false,
            preid: Some("beta".into()),
            otp: None,
            projects: None,
            git_tag: true,
            git_commit: true,
            yes: false,
        };

        assert_eq!(resolved_publish_tag(&plan, &options), Some("beta"));
    }

    #[test]
    fn build_manifest_edits_updates_simple_internal_dependency_ranges() {
        let mut pkg_a = make_release_plan("pkg-a", &[], &[]);
        pkg_a.current_version = Version::parse("1.0.0").unwrap();
        pkg_a.next_version = Version::parse("1.1.0").unwrap();
        pkg_a.tag_name = "release/pkg-a/v1.1.0".into();
        pkg_a.manifest.version = "1.0.0".into();
        pkg_a.manifest_contents = r#"{
  "name": "pkg-a",
  "version": "1.0.0"
}
"#
        .into();

        let mut pkg_b = make_release_plan("pkg-b", &[], &[]);
        pkg_b.current_version = Version::parse("1.0.0").unwrap();
        pkg_b.next_version = Version::parse("1.0.1").unwrap();
        pkg_b.tag_name = "release/pkg-b/v1.0.1".into();
        pkg_b.manifest.version = "1.0.0".into();
        pkg_b.manifest.dependencies.insert("pkg-a".into(), "^1.0.0".into());
        pkg_b.manifest_contents = r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "pkg-a": "^1.0.0"
  }
}
"#
        .into();

        let edits = build_manifest_edits(&[pkg_a, pkg_b]).unwrap();
        assert_eq!(edits.len(), 2);
        assert!(edits[1].updated_contents.contains(r#""pkg-a": "^1.1.0""#));
        assert!(edits[1].updated_contents.contains(r#""version": "1.0.1""#));
    }

    #[test]
    fn build_manifest_edits_rejects_complex_internal_dependency_ranges() {
        let mut pkg_a = make_release_plan("pkg-a", &[], &[]);
        pkg_a.current_version = Version::parse("1.0.0").unwrap();
        pkg_a.next_version = Version::parse("1.1.0").unwrap();
        pkg_a.tag_name = "release/pkg-a/v1.1.0".into();
        pkg_a.manifest.version = "1.0.0".into();

        let mut pkg_b = make_release_plan("pkg-b", &[], &[]);
        pkg_b.manifest.dependencies.insert("pkg-a".into(), ">=1.0.0 <2.0.0".into());

        let error = build_manifest_edits(&[pkg_a, pkg_b]).unwrap_err();
        assert!(error.to_string().contains("Use `workspace:` or a simple exact/^/~ version"));
    }

    #[test]
    fn readiness_report_collects_workspace_and_package_scripts() {
        let workspace_manifest: PackageManifest = serde_json::from_str(
            r#"{
                "scripts": {
                    "build": "pnpm -r build",
                    "release:verify": "pnpm test"
                },
                "vitePlus": {
                    "release": {
                        "checkScripts": ["release:verify"]
                    }
                }
            }"#,
        )
        .unwrap();
        let plans = vec![make_release_plan("pkg-a", &["build", "prepack"], &[])];

        let report = collect_release_readiness_report(Some(&workspace_manifest), &plans);

        assert_eq!(report.workspace_scripts, vec!["build", "release:verify"]);
        assert_eq!(report.package_scripts.len(), 1);
        assert_eq!(report.package_scripts[0].package, "pkg-a");
        assert_eq!(report.package_scripts[0].scripts, vec!["build", "prepack"]);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn readiness_report_warns_for_missing_custom_scripts() {
        let plans = vec![make_release_plan("pkg-a", &["build"], &["release:verify"])];

        let report = collect_release_readiness_report(None, &plans);

        assert_eq!(report.package_scripts[0].scripts, vec!["build"]);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("release:verify"));
    }

    #[test]
    fn readiness_report_warns_when_no_obvious_checks_exist() {
        let plans = vec![make_release_plan("pkg-a", &[], &[])];

        let report = collect_release_readiness_report(None, &plans);

        assert!(report.package_scripts.is_empty());
        assert_eq!(report.warnings.len(), 2);
        assert!(report.warnings[0].contains("pkg-a"));
        assert!(report.warnings[1].contains("No explicit build / pack / prepack"));
    }

    #[test]
    fn classify_conventional_commits() {
        assert_eq!(classify_commit("feat(cli): add release", ""), Some(VersionBump::Minor));
        assert_eq!(classify_commit("fix(core): handle null", ""), Some(VersionBump::Patch));
        assert_eq!(classify_commit("fix!: break behavior", ""), Some(VersionBump::Major));
        assert_eq!(
            classify_commit("chore: cleanup", "BREAKING CHANGE: changed the API"),
            Some(VersionBump::Major)
        );
        assert_eq!(classify_commit("docs: update readme", ""), None);
    }

    #[test]
    fn bump_version_supports_prerelease() {
        let current = Version::parse("1.2.3").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, None, None).unwrap().to_string(),
            "1.2.4"
        );
        assert_eq!(
            next_release_version(&current, VersionBump::Minor, None, Some("beta"))
                .unwrap()
                .to_string(),
            "1.3.0-beta.0"
        );
    }

    #[test]
    fn zero_major_breaking_changes_are_downgraded_to_minor_bumps() {
        let current = Version::parse("0.4.2").unwrap();
        assert_eq!(effective_release_level(&current, VersionBump::Major), VersionBump::Minor);
        assert_eq!(
            next_release_version(
                &current,
                effective_release_level(&current, VersionBump::Major),
                None,
                None
            )
            .unwrap()
            .to_string(),
            "0.5.0"
        );
    }

    #[test]
    fn stable_major_versions_keep_breaking_changes_as_major_bumps() {
        let current = Version::parse("1.4.2").unwrap();
        assert_eq!(effective_release_level(&current, VersionBump::Major), VersionBump::Major);
    }

    #[test]
    fn prerelease_on_same_channel_increments_sequence() {
        let stable = Version::parse("1.0.0").unwrap();
        let current = Version::parse("1.1.0-alpha.0").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, Some(&stable), Some("alpha"))
                .unwrap()
                .to_string(),
            "1.1.0-alpha.1"
        );
    }

    #[test]
    fn prerelease_can_escalate_to_higher_release_line() {
        let stable = Version::parse("1.0.0").unwrap();
        let current = Version::parse("1.0.1-alpha.0").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Minor, Some(&stable), Some("alpha"))
                .unwrap()
                .to_string(),
            "1.1.0-alpha.0"
        );
    }

    #[test]
    fn prerelease_can_switch_channels_without_bumping_base_again() {
        let stable = Version::parse("1.0.0").unwrap();
        let current = Version::parse("1.1.0-alpha.2").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, Some(&stable), Some("beta"))
                .unwrap()
                .to_string(),
            "1.1.0-beta.0"
        );
    }

    #[test]
    fn prerelease_supports_rc_channel() {
        let stable = Version::parse("1.0.0").unwrap();
        let current = Version::parse("1.1.0-beta.2").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, Some(&stable), Some("rc"))
                .unwrap()
                .to_string(),
            "1.1.0-rc.0"
        );
        let current = Version::parse("1.1.0-rc.0").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, Some(&stable), Some("rc"))
                .unwrap()
                .to_string(),
            "1.1.0-rc.1"
        );
    }

    #[test]
    fn stable_release_from_prerelease_keeps_current_target_version() {
        let stable = Version::parse("1.0.0").unwrap();
        let current = Version::parse("1.1.0-beta.2").unwrap();
        assert_eq!(
            next_release_version(&current, VersionBump::Patch, Some(&stable), None)
                .unwrap()
                .to_string(),
            "1.1.0"
        );
    }

    #[test]
    fn zero_major_prerelease_breaking_changes_stay_on_minor_line() {
        let stable = Version::parse("0.1.0").unwrap();
        let current = Version::parse("0.2.0-alpha.1").unwrap();
        let level = effective_release_level(&current, VersionBump::Major);
        assert_eq!(level, VersionBump::Minor);
        assert_eq!(
            next_release_version(&current, level, Some(&stable), Some("alpha"))
                .unwrap()
                .to_string(),
            "0.2.0-alpha.2"
        );
    }

    #[test]
    fn prerelease_levels_cannot_be_used_as_bump_levels() {
        let current = Version::parse("1.0.0").unwrap();
        let error = next_release_version(&current, VersionBump::Alpha, None, None).unwrap_err();
        assert!(error.to_string().contains("Invalid release level 'alpha'"));
    }

    #[test]
    fn replace_top_level_string_property_only_updates_top_level_field() {
        let contents = r#"{
  "version": "1.0.0",
  "nested": {
    "version": "should-stay"
  }
}
"#;

        let updated =
            replace_top_level_string_property(contents, "version", "1.0.0", "2.0.0").unwrap();
        assert!(updated.contains(r#""version": "2.0.0""#));
        assert!(updated.contains(r#""version": "should-stay""#));
    }

    #[test]
    fn prepend_changelog_section_reuses_existing_heading() {
        let existing = "# Changelog\n\n## 0.1.0 - 2026-01-01\n\n- existing\n";
        let prepended = prepend_changelog_section(existing, "## 0.2.0 - 2026-02-01\n\n- new\n\n");
        assert!(prepended.starts_with("# Changelog\n\n## 0.2.0 - 2026-02-01"));
        assert!(prepended.contains("## 0.1.0 - 2026-01-01"));
    }

    #[test]
    fn package_tags_are_scoped_and_safe() {
        let version = Version::parse("1.0.0").unwrap();
        assert_eq!(package_tag_name("@scope/pkg-a", &version), "release/scope/pkg-a/v1.0.0");
    }

    #[test]
    fn release_tags_roundtrip_scoped_and_unscoped_package_names() {
        assert_eq!(
            parse_package_name_from_tag("release/scope/pkg-a/v1.0.0"),
            Some("@scope/pkg-a".into())
        );
        assert_eq!(parse_package_name_from_tag("release/pkg-b/v2.0.0"), Some("pkg-b".into()));
    }

    #[test]
    fn publish_protocol_matrix_prefers_native_workspace_and_catalog_rewrites() {
        let pnpm = test_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let summary =
            DependencyProtocolSummary { workspace: true, catalog: true, ..Default::default() };

        assert!(unsupported_publish_protocols(&pnpm, summary).is_empty());

        let npm = test_package_manager(PackageManagerType::Npm, "11.0.0");
        assert_eq!(unsupported_publish_protocols(&npm, summary), vec!["workspace:", "catalog:"]);
    }

    #[test]
    fn file_protocol_remains_blocked_even_with_native_publishers() {
        let pnpm = test_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let summary = DependencyProtocolSummary { file: true, ..Default::default() };
        assert_eq!(unsupported_publish_protocols(&pnpm, summary), vec!["file:"]);
    }

    #[test]
    fn project_selection_matches_previous_package_names() {
        let graph = build_test_package_graph();
        let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
        let pkg_a = nodes.next().unwrap();

        let mut package = make_workspace_package(&graph, pkg_a, usize::MAX);
        package.known_names.push("@scope/old-pkg-a".into());

        let selected =
            select_workspace_packages(&[package], Some(&["@scope/old-pkg-*".into()])).unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "pkg-a");
    }

    #[test]
    fn unique_strings_preserves_order() {
        let values = unique_strings(vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
            "c".to_string(),
        ]);
        assert_eq!(values, vec!["a", "b", "c"]);
    }

    #[test]
    fn orphaned_released_packages_ignore_known_and_retired_names() {
        let known = HashSet::from(["pkg-a", "@scope/pkg-b", "@scope/old-pkg-c"]);
        let orphaned = collect_orphaned_released_package_names(
            [
                "release/pkg-a/v1.0.0",
                "release/scope/pkg-b/v1.0.0",
                "release/scope/old-pkg-c/v0.9.0",
                "release/scope/pkg-d/v2.0.0",
            ],
            &known,
        );
        assert_eq!(orphaned, vec!["@scope/pkg-d"]);
    }

    #[test]
    fn project_order_breaks_ties_between_independent_packages() {
        let graph = build_test_package_graph();
        let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
        let pkg_a = nodes.next().unwrap();
        let pkg_b = nodes.next().unwrap();
        let pkg_c = nodes.next().unwrap();

        let selected = vec![
            make_workspace_package(&graph, pkg_a, 2),
            make_workspace_package(&graph, pkg_b, 1),
            make_workspace_package(&graph, pkg_c, 0),
        ];

        let ordered = topological_sort_selected_packages(&graph, &selected);
        let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

        assert_eq!(names, vec!["pkg-b", "pkg-c", "pkg-a"]);
    }

    #[test]
    fn dependency_order_wins_over_requested_project_order() {
        let graph = build_test_package_graph();
        let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
        let pkg_a = nodes.next().unwrap();
        let pkg_b = nodes.next().unwrap();

        let selected = vec![
            make_workspace_package(&graph, pkg_a, 0),
            make_workspace_package(&graph, pkg_b, 1),
        ];

        let ordered = topological_sort_selected_packages(&graph, &selected);
        let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

        assert_eq!(names, vec!["pkg-b", "pkg-a"]);
    }
}
