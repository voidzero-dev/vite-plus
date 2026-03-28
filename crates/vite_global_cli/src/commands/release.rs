//! Workspace release workflow for versioning, changelog generation, and coordinated publishing.
//!
//! References:
//! - SemVer 2.0.0: https://semver.org/spec/v2.0.0.html
//! - SemVer FAQ for `0.y.z`: https://semver.org/#faq
//! - Conventional Commits 1.0.0: https://www.conventionalcommits.org/en/v1.0.0/#specification
//! - Conventional Commits FAQ: https://www.conventionalcommits.org/en/v1.0.0/#faq

use std::{
    collections::{HashMap, HashSet},
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
    build_prerelease, bump_version, capture_git, is_clean_git_worktree, output,
    parse_conventional_commit, parse_github_repo_slug, prerelease_channel, prerelease_number,
    read_package_manifest, replace_top_level_string_property, run_git, strip_prerelease,
};
use vite_task::ExitStatus;
use vite_workspace::{DependencyType, PackageInfo, PackageNodeIndex};

use crate::error::Error;

use self::{first_publish::*, planning::*, presentation::*, protocols::*, storage::*};

use super::{build_package_manager, prepend_js_runtime_to_path_env};

#[derive(Debug, Clone)]
pub struct ReleaseOptions {
    pub dry_run: bool,
    pub skip_publish: bool,
    pub first_release: bool,
    pub changelog: bool,
    pub preid: Option<String>,
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
    current_version: Version,
    next_version: Version,
    level: VersionBump,
    commits: Vec<CommitInfo>,
    changelog_path: AbsolutePathBuf,
    access: Option<String>,
    repository_url: Option<String>,
    protocol_summary: DependencyProtocolSummary,
    tag_name: String,
    scripts: Vec<String>,
    check_scripts: Vec<String>,
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

pub async fn execute(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Result<ExitStatus, Error> {
    if options.git_tag && !options.git_commit && !options.dry_run {
        return Err(Error::UserMessage(
            "`vp release --no-git-commit --git-tag` is not supported because tags would not point to the release changes."
                .into(),
        ));
    }

    prepend_js_runtime_to_path_env(&cwd).await?;
    let package_manager = build_package_manager(&cwd).await?;

    let (workspace_root, _) = vite_workspace::find_workspace_root(&cwd)?;
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
    let selected = select_workspace_packages(&workspace_packages, options.projects.as_deref())?;
    if selected.is_empty() {
        output::warn("No publishable packages matched the release selection.");
        return Ok(ExitStatus::SUCCESS);
    }

    let ordered = topological_sort_selected_packages(&package_graph, &selected);

    let mut release_plans = Vec::with_capacity(ordered.len());
    let mut root_commits = Vec::new();
    let mut seen_commit_hashes = HashSet::new();

    for package in ordered {
        let previous_tag = if options.first_release {
            None
        } else {
            find_latest_package_tag(&workspace_root_path, &package.known_names)?
        };
        let latest_stable_version = if options.first_release {
            None
        } else {
            find_latest_stable_package_version(&workspace_root_path, &package.known_names)?
        };

        let commits = collect_package_commits(
            &workspace_root_path,
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
            options.preid.as_deref(),
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
            current_version,
            next_version,
            level,
            commits,
            changelog_path: package.package_path.join("CHANGELOG.md"),
            access: package.manifest.publish_config.access.clone(),
            repository_url: package.manifest.repository_url().map(ToOwned::to_owned),
            protocol_summary: package.manifest.dependency_protocol_summary(),
            tag_name: package_tag_name(&package.name, &next_version),
            scripts: package.manifest.scripts.keys().cloned().collect(),
            check_scripts: package.manifest.vite_plus.release.check_scripts.clone(),
        });
    }

    if release_plans.is_empty() {
        output::warn("No releasable package changes were found.");
        return Ok(ExitStatus::SUCCESS);
    }

    print_release_plan(&release_plans, &options);
    if options.first_release {
        let guidance = collect_first_publish_guidance(&workspace_root_path, &release_plans);
        print_first_publish_guidance(&guidance, &options);
    }
    let readiness_report =
        collect_release_readiness_report(workspace_manifest.as_ref(), &release_plans);
    print_release_readiness_report(&readiness_report);

    if !options.skip_publish {
        let mut protocol_issues = String::new();
        for plan in &release_plans {
            let protocols = unsupported_publish_protocols(&package_manager, plan.protocol_summary);
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

        if !protocol_issues.is_empty() {
            let mut message = String::from("Publishing with ");
            push_display(&mut message, package_manager.client);
            message.push_str(" is unsafe because these packages still contain unsupported publish-time dependency protocols: ");
            message.push_str(&protocol_issues);
            message.push_str(". Use a package manager with native publish rewriting support where available, or rerun with `--skip-publish`.");
            if options.dry_run {
                output::warn(&message);
            } else {
                return Err(Error::UserMessage(message.into()));
            }
        }
    }

    if options.dry_run {
        print_dry_run_actions(&release_plans, &package_manager, &options);
        return Ok(ExitStatus::SUCCESS);
    }

    ensure_clean_worktree(&workspace_root_path)?;
    if !options.yes && !confirm_release(&release_plans, &readiness_report, &options)? {
        return Ok(ExitStatus::SUCCESS);
    }

    let mut changed_files =
        Vec::with_capacity(release_plans.len() * (1 + usize::from(options.changelog)) + 1);
    let release_date = Local::now().format("%Y-%m-%d").to_string();

    if options.changelog {
        let root_changelog_path = workspace_root_path.join("CHANGELOG.md");
        let root_section =
            build_root_changelog_section(&release_date, &release_plans, &root_commits);
        write_changelog_section(&root_changelog_path, &root_section)?;
        changed_files.push(root_changelog_path);
    }

    for plan in &release_plans {
        update_manifest_version(
            &plan.manifest_path,
            &plan.manifest_contents,
            &plan.current_version.to_string(),
            &plan.next_version.to_string(),
        )?;
        changed_files.push(plan.manifest_path.clone());

        if options.changelog {
            let package_section =
                build_package_changelog_section(&release_date, &plan.next_version, &plan.commits);
            write_changelog_section(&plan.changelog_path, &package_section)?;
            changed_files.push(plan.changelog_path.clone());
        }
    }

    if options.git_commit {
        git_add_paths(&workspace_root_path, &changed_files)?;
        let commit_message = release_commit_message(&release_plans);
        git_commit(&workspace_root_path, &commit_message)?;
        let mut message = String::from("Created release commit: ");
        message.push_str(&commit_message);
        output::success(&message);
    }

    if !options.skip_publish {
        for plan in &release_plans {
            let mut message = String::from("Publishing ");
            message.push_str(&plan.name);
            message.push('@');
            push_display(&mut message, &plan.next_version);
            output::info(&message);
            let publish_options = PublishCommandOptions {
                dry_run: false,
                tag: options.preid.as_deref(),
                access: plan.access.as_deref(),
                ..Default::default()
            };
            let status =
                package_manager.run_publish_command(&publish_options, &plan.package_path).await?;
            if !status.success() {
                return Ok(status);
            }
        }
    } else {
        output::note("Skipping publish because --skip-publish was provided.");
    }

    if options.git_tag {
        for plan in &release_plans {
            git_tag(&workspace_root_path, &plan.tag_name)?;
            let mut message = String::from("Created git tag ");
            message.push_str(&plan.tag_name);
            output::success(&message);
        }
    }

    let mut message = String::from("Release completed for ");
    push_display(&mut message, release_plans.len());
    message.push_str(" package(s).");
    output::success(&message);
    Ok(ExitStatus::SUCCESS)
}

mod planning {
    use super::*;

    pub(super) fn load_workspace_packages(
        package_graph: &petgraph::graph::DiGraph<
            PackageInfo,
            DependencyType,
            vite_workspace::PackageIx,
        >,
    ) -> Result<Vec<WorkspacePackage>, Error> {
        let mut packages = Vec::new();

        for node in package_graph.node_indices() {
            let package = &package_graph[node];
            if package.path.as_str().is_empty() {
                continue;
            }

            let manifest_path = package.absolute_path.join("package.json");
            let document = read_package_manifest(&manifest_path)?;
            let vite_shared::PackageManifestDocument { contents: manifest_contents, manifest } =
                document;
            if manifest.private {
                continue;
            }

            let known_names = unique_strings(
                std::iter::once(manifest.name.clone())
                    .chain(manifest.vite_plus.release.previous_names.iter().cloned()),
            );
            let retired_names =
                unique_strings(manifest.vite_plus.release.retired_names.iter().cloned());
            let release_paths = unique_strings(
                std::iter::once(package.path.as_str().to_owned())
                    .chain(manifest.vite_plus.release.previous_paths.iter().cloned()),
            );

            packages.push(WorkspacePackage {
                node,
                name: manifest.name.clone(),
                known_names,
                retired_names,
                release_paths,
                selection_order: usize::MAX,
                manifest_path,
                package_path: package.absolute_path.to_absolute_path_buf(),
                manifest_contents,
                manifest,
            });
        }

        Ok(packages)
    }

    pub(super) fn select_workspace_packages(
        packages: &[WorkspacePackage],
        projects: Option<&[String]>,
    ) -> Result<Vec<WorkspacePackage>, Error> {
        let Some(projects) = projects else {
            return Ok(packages.to_vec());
        };

        let patterns: Vec<Pattern> = projects
            .iter()
            .map(|pattern| {
                Pattern::new(pattern).map_err(|e| {
                    let mut message = String::from("Invalid project pattern '");
                    message.push_str(pattern);
                    message.push_str("': ");
                    push_display(&mut message, e);
                    Error::UserMessage(message.into())
                })
            })
            .collect::<Result<_, _>>()?;

        let mut matched_patterns = vec![false; patterns.len()];
        let mut selected = Vec::new();

        for package in packages {
            let mut match_order = None;
            for (index, pattern) in patterns.iter().enumerate() {
                if package.known_names.iter().any(|name| pattern.matches(name)) {
                    matched_patterns[index] = true;
                    match_order = Some(match_order.map_or(index, |current| current.min(index)));
                }
            }
            if let Some(selection_order) = match_order {
                let mut package = package.clone();
                package.selection_order = selection_order;
                selected.push(package);
            }
        }

        for (index, matched) in matched_patterns.into_iter().enumerate() {
            if !matched {
                let mut message = String::from("No publishable packages matched '");
                message.push_str(&projects[index]);
                message.push('\'');
                output::warn(&message);
            }
        }

        Ok(selected)
    }

    pub(super) fn topological_sort_selected_packages(
        package_graph: &petgraph::graph::DiGraph<
            PackageInfo,
            DependencyType,
            vite_workspace::PackageIx,
        >,
        selected: &[WorkspacePackage],
    ) -> Vec<WorkspacePackage> {
        let selected_set: HashSet<PackageNodeIndex> =
            selected.iter().map(|package| package.node).collect();
        let by_node: HashMap<PackageNodeIndex, WorkspacePackage> =
            selected.iter().cloned().map(|package| (package.node, package)).collect();
        let mut pending_dependencies: HashMap<PackageNodeIndex, usize> =
            selected.iter().map(|package| (package.node, 0usize)).collect();
        let mut dependents: HashMap<PackageNodeIndex, Vec<PackageNodeIndex>> =
            selected.iter().map(|package| (package.node, Vec::new())).collect();

        for edge in package_graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            if selected_set.contains(&source) && selected_set.contains(&target) {
                *pending_dependencies
                    .get_mut(&source)
                    .expect("selected package should have dependency counter") += 1;
                dependents
                    .get_mut(&target)
                    .expect("selected package should have dependent list")
                    .push(source);
            }
        }

        let mut available: Vec<PackageNodeIndex> = pending_dependencies
            .iter()
            .filter_map(|(&node, &count)| (count == 0).then_some(node))
            .collect();
        let mut scheduled = HashSet::new();
        let mut ordered_nodes = Vec::with_capacity(selected.len());

        while ordered_nodes.len() < selected.len() {
            sort_nodes_by_release_priority(&mut available, &by_node);

            let next = if let Some(next) = available.first().copied() {
                available.remove(0);
                next
            } else {
                let mut remaining: Vec<PackageNodeIndex> = selected
                    .iter()
                    .map(|package| package.node)
                    .filter(|node| !scheduled.contains(node))
                    .collect();
                sort_nodes_by_release_priority(&mut remaining, &by_node);
                remaining
                    .into_iter()
                    .next()
                    .expect("there should be an unscheduled package when ordering is incomplete")
            };

            if !scheduled.insert(next) {
                continue;
            }
            ordered_nodes.push(next);

            if let Some(node_dependents) = dependents.get(&next) {
                for dependent in node_dependents {
                    if let Some(count) = pending_dependencies.get_mut(dependent) {
                        if *count > 0 {
                            *count -= 1;
                        }
                        if *count == 0 && !scheduled.contains(dependent) {
                            available.push(*dependent);
                        }
                    }
                }
            }
        }

        ordered_nodes.into_iter().filter_map(|node| by_node.get(&node).cloned()).collect()
    }

    fn sort_nodes_by_release_priority(
        nodes: &mut Vec<PackageNodeIndex>,
        by_node: &HashMap<PackageNodeIndex, WorkspacePackage>,
    ) {
        nodes.sort_by(|left, right| {
            let left_package = by_node.get(left).expect("selected package should exist");
            let right_package = by_node.get(right).expect("selected package should exist");
            left_package
                .selection_order
                .cmp(&right_package.selection_order)
                .then_with(|| left_package.name.cmp(&right_package.name))
        });
        nodes.dedup();
    }

    pub(super) fn find_latest_package_tag(
        cwd: &AbsolutePath,
        package_names: &[String],
    ) -> Result<Option<String>, Error> {
        let stdout = capture_git(cwd, release_tag_list_args(package_names))?;
        Ok(stdout.lines().map(str::trim).find(|line| !line.is_empty()).map(ToOwned::to_owned))
    }

    pub(super) fn find_latest_stable_package_version(
        cwd: &AbsolutePath,
        package_names: &[String],
    ) -> Result<Option<Version>, Error> {
        let stdout = capture_git(cwd, release_tag_list_args(package_names))?;
        Ok(stdout
            .lines()
            .map(str::trim)
            .filter_map(parse_package_tag_version)
            .find(|version| !version.has_prerelease()))
    }

    pub(super) fn collect_package_commits(
        cwd: &AbsolutePath,
        package_paths: &[String],
        since_tag: Option<&str>,
    ) -> Result<Vec<CommitInfo>, Error> {
        let mut args = Vec::with_capacity(package_paths.len() + 5);
        args.push(String::from("log"));
        args.push(String::from("--reverse"));
        args.push(String::from("--format=%H%x1f%s%x1f%b%x1e"));
        if let Some(tag) = since_tag {
            let mut range = String::with_capacity(tag.len() + 6);
            range.push_str(tag);
            range.push_str("..HEAD");
            args.push(range);
        }
        args.push(String::from("--"));
        for package_path in package_paths {
            args.push(package_path.clone());
        }

        let stdout = capture_git(cwd, args)?;
        let mut commits = Vec::new();

        for record in stdout.split('\u{001e}') {
            let trimmed = record.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut parts = trimmed.splitn(3, '\u{001f}');
            let hash = parts.next().unwrap_or_default().trim();
            let subject = parts.next().unwrap_or_default().trim();
            let body = parts.next().unwrap_or_default().trim();
            if hash.is_empty() || subject.is_empty() {
                continue;
            }

            if let Some(level) = classify_commit(subject, body) {
                commits.push(CommitInfo {
                    hash: hash.to_owned(),
                    short_hash: hash.get(..7).unwrap_or(hash).to_owned(),
                    subject: subject.to_owned(),
                    level,
                });
            }
        }

        Ok(commits)
    }

    pub(super) fn classify_commit(subject: &str, body: &str) -> Option<VersionBump> {
        let commit = parse_conventional_commit(subject, body)?;
        if commit.breaking {
            return Some(VersionBump::Major);
        }

        match commit.kind {
            "feat" => Some(VersionBump::Minor),
            "fix" | "perf" | "refactor" | "revert" => Some(VersionBump::Patch),
            _ => None,
        }
    }

    pub(super) fn highest_release_level(commits: &[CommitInfo]) -> Option<VersionBump> {
        commits.iter().map(|commit| commit.level).max()
    }

    pub(super) fn effective_release_level(current: &Version, level: VersionBump) -> VersionBump {
        // Conventional Commits marks breaking changes as MAJOR, but SemVer treats `0.y.z` as
        // initial development where API compatibility can still move on the minor line.
        // https://www.conventionalcommits.org/en/v1.0.0/#specification
        // https://semver.org/#faq
        if current.major == 0 && level == VersionBump::Major { VersionBump::Minor } else { level }
    }

    pub(super) fn next_release_version(
        current: &Version,
        level: VersionBump,
        stable_baseline: Option<&Version>,
        prerelease_tag: Option<&str>,
    ) -> Result<Version, Error> {
        if !level.is_version_bump() {
            let mut message = String::from("Invalid release level '");
            message.push_str(level.as_str());
            message.push_str("' for version bump calculation.");
            return Err(Error::UserMessage(message.into()));
        }

        let current_base = strip_prerelease(current);
        let target_base = if !current.has_prerelease() {
            bump_version(&current_base, level)
        } else {
            let baseline = stable_baseline.cloned().unwrap_or_else(|| Version::new(0, 0, 0));
            match release_line_level(&baseline, &current_base) {
                Some(existing_line) if level <= existing_line => current_base.clone(),
                _ => bump_version(&current_base, level),
            }
        };

        let mut next = target_base.clone();
        next.clear_build();

        let prerelease_tag = prerelease_tag.map(PrereleaseTag::parse);

        if let Some(prerelease_tag) = prerelease_tag.as_ref() {
            let prerelease_number = if target_base == current_base
                && prerelease_channel(current) == Some(prerelease_tag.as_str())
            {
                prerelease_number(current).map_or(0, |number| number + 1)
            } else {
                0
            };
            next.set_prerelease(Some(prerelease_with_number(prerelease_tag, prerelease_number)?));
        } else {
            next.set_prerelease(None);
        }

        Ok(next)
    }

    fn release_line_level(stable_baseline: &Version, target_base: &Version) -> Option<VersionBump> {
        if target_base.major > stable_baseline.major {
            Some(VersionBump::Major)
        } else if target_base.minor > stable_baseline.minor {
            Some(VersionBump::Minor)
        } else if target_base.patch > stable_baseline.patch {
            Some(VersionBump::Patch)
        } else {
            None
        }
    }

    fn prerelease_with_number(
        prerelease_tag: &PrereleaseTag,
        number: u64,
    ) -> Result<String, Error> {
        build_prerelease(prerelease_tag.as_str(), number).map_err(|e| {
            let mut message = String::from("Invalid prerelease identifier '");
            message.push_str(prerelease_tag.as_str());
            message.push_str("': ");
            push_display(&mut message, e);
            Error::UserMessage(message.into())
        })
    }

    fn parse_package_tag_version(tag_name: &str) -> Option<Version> {
        let (_, version) = tag_name.rsplit_once("/v")?;
        Version::parse(version).ok()
    }

    pub(super) fn package_tag_name(package_name: &str, version: &Version) -> String {
        let sanitized = sanitize_package_name(package_name);
        let mut tag = String::with_capacity(sanitized.len() + 10);
        tag.push_str("release/");
        tag.push_str(&sanitized);
        tag.push_str("/v");
        push_display(&mut tag, version);
        tag
    }

    fn sanitize_package_name(package_name: &str) -> String {
        package_name.trim_start_matches('@').to_owned()
    }

    fn release_tag_list_args(package_names: &[String]) -> Vec<String> {
        let mut args = Vec::with_capacity(package_names.len() + 3);
        args.push(String::from("tag"));
        args.push(String::from("--list"));
        args.push(String::from("--sort=-creatordate"));
        for package_name in package_names {
            let sanitized = sanitize_package_name(package_name);
            let mut pattern = String::with_capacity(sanitized.len() + 10);
            pattern.push_str("release/");
            pattern.push_str(&sanitized);
            pattern.push_str("/v*");
            args.push(pattern);
        }
        args
    }

    fn parse_package_name_from_tag(tag_name: &str) -> Option<String> {
        let package_path = tag_name.strip_prefix("release/")?.rsplit_once("/v")?.0;
        if package_path.contains('/') {
            let mut package_name = String::with_capacity(package_path.len() + 1);
            package_name.push('@');
            package_name.push_str(package_path);
            Some(package_name)
        } else {
            Some(package_path.to_owned())
        }
    }

    pub(super) fn collect_orphaned_released_packages(
        cwd: &AbsolutePath,
        packages: &[WorkspacePackage],
    ) -> Result<Vec<String>, Error> {
        let stdout = capture_git(cwd, ["tag", "--list", "release/*/v*"])?;
        let known_names: HashSet<&str> = packages
            .iter()
            .flat_map(|package| {
                package
                    .known_names
                    .iter()
                    .map(String::as_str)
                    .chain(package.retired_names.iter().map(String::as_str))
            })
            .collect();

        Ok(collect_orphaned_released_package_names(
            stdout.lines().map(str::trim).filter(|tag| !tag.is_empty()),
            &known_names,
        ))
    }

    pub(super) fn unique_strings<I>(values: I) -> Vec<String>
    where
        I: IntoIterator<Item = String>,
    {
        let iter = values.into_iter();
        let (lower, _) = iter.size_hint();
        let mut seen = HashSet::with_capacity(lower);
        let mut ordered = Vec::with_capacity(lower);
        for value in iter {
            if seen.insert(value.clone()) {
                ordered.push(value);
            }
        }
        ordered
    }

    pub(super) fn collect_orphaned_released_package_names<'a, I>(
        tags: I,
        known_names: &HashSet<&str>,
    ) -> Vec<String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut orphaned = HashSet::new();
        for tag in tags {
            if let Some(package_name) = parse_package_name_from_tag(tag)
                && !known_names.contains(package_name.as_str())
            {
                orphaned.insert(package_name);
            }
        }

        let mut orphaned = orphaned.into_iter().collect::<Vec<_>>();
        orphaned.sort();
        orphaned
    }
}

mod presentation {
    use super::*;

    pub(super) fn print_release_plan(
        release_plans: &[PackageReleasePlan],
        options: &ReleaseOptions,
    ) {
        output::info("Planned releases:");
        for plan in release_plans {
            let mut line = String::new();
            line.push_str("  ");
            line.push_str(&plan.name);
            line.push(' ');
            push_display(&mut line, &plan.current_version);
            line.push_str(" -> ");
            push_display(&mut line, &plan.next_version);
            line.push_str(" (");
            line.push_str(plan.level.as_str());
            line.push(')');
            output::raw(&line);
            if plan.known_names.len() > 1 {
                let mut line = String::from("    previous names: ");
                push_joined(&mut line, plan.known_names[1..].iter().map(String::as_str), ", ");
                output::raw(&line);
            }
            if !plan.retired_names.is_empty() {
                let mut line = String::from("    retired package names: ");
                push_joined(&mut line, plan.retired_names.iter().map(String::as_str), ", ");
                output::raw(&line);
            }
        }

        if options.dry_run {
            output::note("Dry run enabled; no files will be changed.");
        }
        if !options.changelog {
            output::note("Changelog generation is disabled for this run.");
        }
        if !options.dry_run && options.skip_publish {
            output::note("Publishing disabled for this run.");
        }
    }

    pub(super) fn collect_release_readiness_report(
        workspace_manifest: Option<&PackageManifest>,
        release_plans: &[PackageReleasePlan],
    ) -> ReleaseReadinessReport {
        let mut report = ReleaseReadinessReport::default();

        if let Some(workspace_manifest) = workspace_manifest {
            report.workspace_scripts = collect_available_release_scripts(
                "Workspace root",
                |candidate| workspace_manifest.has_script(candidate),
                &workspace_manifest.vite_plus.release.check_scripts,
                false,
                &mut report.warnings,
            );
        }

        for plan in release_plans {
            let scripts = collect_available_release_scripts(
                &plan.name,
                |candidate| contains_script(&plan.scripts, candidate),
                &plan.check_scripts,
                true,
                &mut report.warnings,
            );
            if !scripts.is_empty() {
                report
                    .package_scripts
                    .push(PackageReadiness { package: plan.name.clone(), scripts });
            }
        }

        if report.workspace_scripts.is_empty() && report.package_scripts.is_empty() {
            report.warnings.push(
                "No explicit build / pack / prepack / prepublishOnly / prepare scripts or `vitePlus.release.checkScripts` were detected for this release.".into(),
            );
        }

        report
    }

    fn collect_available_release_scripts<F>(
        scope: &str,
        mut has_script: F,
        configured_scripts: &[String],
        warn_when_missing: bool,
        warnings: &mut Vec<String>,
    ) -> Vec<String>
    where
        F: FnMut(&str) -> bool,
    {
        let mut scripts =
            Vec::with_capacity(DEFAULT_RELEASE_CHECK_SCRIPTS.len() + configured_scripts.len());

        for candidate in DEFAULT_RELEASE_CHECK_SCRIPTS {
            if has_script(candidate) {
                push_unique_script(&mut scripts, candidate);
            }
        }

        for configured_script in configured_scripts {
            if has_script(configured_script) {
                push_unique_owned_script(&mut scripts, configured_script);
            } else {
                let mut warning = String::new();
                warning.push_str(scope);
                warning.push_str(" declares `vitePlus.release.checkScripts` entry '");
                warning.push_str(configured_script);
                warning.push_str("' but no matching script exists.");
                warnings.push(warning);
            }
        }

        if warn_when_missing && scripts.is_empty() && configured_scripts.is_empty() {
            let mut warning = String::new();
            warning.push_str(scope);
            warning.push_str(" does not expose obvious pre-release checks (`build`, `pack`, `prepack`, `prepublishOnly`, `prepare`, or `vitePlus.release.checkScripts`). Double-check build and pack steps before publishing.");
            warnings.push(warning);
        }

        scripts
    }

    fn contains_script(available_scripts: &[String], candidate: &str) -> bool {
        available_scripts.iter().any(|script| script == candidate)
    }

    fn push_unique_script(scripts: &mut Vec<String>, candidate: &str) {
        if !scripts.iter().any(|script| script == candidate) {
            scripts.push(candidate.to_string());
        }
    }

    fn push_unique_owned_script(scripts: &mut Vec<String>, candidate: &str) {
        push_unique_script(scripts, candidate);
    }

    pub(super) fn print_release_readiness_report(report: &ReleaseReadinessReport) {
        output::raw("");
        output::info("Pre-release readiness:");

        if !report.workspace_scripts.is_empty() {
            let mut line = String::from("  workspace scripts: ");
            push_joined(&mut line, report.workspace_scripts.iter().map(String::as_str), ", ");
            output::raw(&line);
        }

        if report.package_scripts.is_empty() {
            output::raw("  package scripts: none detected");
        } else {
            output::raw("  package scripts:");
            for package in &report.package_scripts {
                let mut line = String::from("    - ");
                line.push_str(&package.package);
                line.push_str(": ");
                push_joined(&mut line, package.scripts.iter().map(String::as_str), ", ");
                output::raw(&line);
            }
        }

        output::note(
            "`vp release` does not run build, pack, or custom pre-release scripts implicitly.",
        );
        output::note(
            "Review this summary, run any checks you need manually, then confirm to continue.",
        );

        for warning in &report.warnings {
            output::warn(warning);
        }
    }

    pub(super) fn print_dry_run_actions(
        release_plans: &[PackageReleasePlan],
        package_manager: &PackageManager,
        options: &ReleaseOptions,
    ) {
        let commit_message = release_commit_message(release_plans);
        let mut line = String::from("Would update ");
        push_display(&mut line, release_plans.len());
        line.push_str(" package.json file(s).");
        output::note(&line);
        if options.changelog {
            output::note("Would update the root and per-package changelog files.");
        } else {
            output::note("Would skip changelog generation because --changelog was not provided.");
        }
        if options.git_commit {
            let mut line = String::from("Would create release commit: ");
            line.push_str(&commit_message);
            output::note(&line);
        }
        if options.git_tag {
            for plan in release_plans {
                let mut line = String::from("Would create git tag ");
                line.push_str(&plan.tag_name);
                output::note(&line);
            }
        }
        if options.yes {
            output::note("Would skip the final confirmation because --yes was provided.");
        } else {
            output::note(
                "Would print the release summary and ask for confirmation before changing files.",
            );
        }
        if options.skip_publish {
            output::note("Would skip publishing because --skip-publish was provided.");
            return;
        }

        for plan in release_plans {
            let publish_options = package_manager.resolve_publish_command(&PublishCommandOptions {
                tag: options.preid.as_deref(),
                access: plan.access.as_deref(),
                ..Default::default()
            });
            let mut line = String::from("Would publish ");
            line.push_str(&plan.name);
            line.push('@');
            push_display(&mut line, &plan.next_version);
            line.push_str(" with: ");
            line.push_str(&publish_options.bin_path);
            if !publish_options.args.is_empty() {
                line.push(' ');
                push_joined(&mut line, publish_options.args.iter().map(String::as_str), " ");
            }
            output::note(&line);
        }
    }

    pub(super) fn confirm_release(
        release_plans: &[PackageReleasePlan],
        readiness_report: &ReleaseReadinessReport,
        options: &ReleaseOptions,
    ) -> Result<bool, Error> {
        if !std::io::stdin().is_terminal() {
            return Err(Error::UserMessage(
                "Cannot prompt for confirmation: stdin is not a TTY. Use --yes to skip confirmation."
                    .into(),
            ));
        }

        output::raw("");
        output::info("Release summary:");
        let mut line = String::from("  packages: ");
        push_display(&mut line, release_plans.len());
        output::raw(&line);

        let mut line = String::from("  publish: ");
        line.push_str(if options.skip_publish { "no" } else { "yes" });
        output::raw(&line);

        let mut line = String::from("  changelog: ");
        line.push_str(if options.changelog { "yes" } else { "no" });
        output::raw(&line);

        let mut line = String::from("  git commit: ");
        line.push_str(if options.git_commit { "yes" } else { "no" });
        output::raw(&line);

        let mut line = String::from("  git tags: ");
        line.push_str(if options.git_tag { "yes" } else { "no" });
        output::raw(&line);

        let mut line = String::from("  prerelease tag: ");
        line.push_str(options.preid.as_deref().unwrap_or("stable"));
        output::raw(&line);
        if !readiness_report.warnings.is_empty() {
            let mut warning = String::new();
            push_display(&mut warning, readiness_report.warnings.len());
            warning.push_str(" pre-release warning(s) need review before continuing.");
            output::warn(&warning);
        }

        output::raw_inline("Continue with this release? [Y/n] ");
        #[expect(clippy::disallowed_types)]
        let mut input = String::new();
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input)?;

        match input.trim().to_ascii_lowercase().as_str() {
            "" | "y" | "yes" => Ok(true),
            _ => {
                output::info("Aborted.");
                Ok(false)
            }
        }
    }
}

mod first_publish {
    //! First-release guidance is intentionally modeled as a tiny inline DSL.
    //!
    //! The goal here is twofold:
    //!
    //! 1. Keep the full checklist visible in one place so maintainers can scan the entire
    //!    first-publish experience without jumping through a long chain of helper functions.
    //! 2. Stay extremely conservative on runtime cost even though this code is primarily
    //!    user-facing text generation.
    //!
    //! The macros below expand directly into a fixed checklist structure rather than building an
    //! intermediate template language at runtime. That keeps the declaration readable while also
    //! avoiding:
    //!
    //! - `format!`-heavy string construction
    //! - repeated temporary `Vec<String>` / `String` creation for static content
    //! - fragmented step builder functions that make the overall checklist harder to audit
    //!
    //! The resulting flow is:
    //!
    //! - `first_publish_checklist!` declares the entire checklist as a fixed array of steps
    //! - `ChecklistStep` / `ChecklistLine` store only the minimal renderable structure
    //! - `print_checklist` reuses a single `String` buffer while streaming output line by line
    //!
    //! This is intentionally not a generic templating system. It is a small, purpose-built,
    //! allocation-aware representation tailored to the handful of first-publish messages we need
    //! to render.

    use super::*;

    const CHECKLIST_STEP_PREFIX: &str = "  ";
    const CHECKLIST_ITEM_PREFIX: &str = "     - ";

    /// Declares a checklist step in a compact, template-like form.
    ///
    /// This macro exists so that the first-publish checklist can be read top-to-bottom as a
    /// single declarative block. It expands straight into `ChecklistStep::new`, so there is no
    /// runtime template parsing or second-pass interpretation cost.
    macro_rules! step {
        ($title:expr, [$( $line:expr ),* $(,)?] $(,)?) => {
            ChecklistStep::new($title, [$( $line ),*])
        };
    }

    /// Emits a static text line.
    ///
    /// Static strings stay borrowed all the way through rendering, which lets the checklist carry
    /// explanatory text without allocating per line.
    macro_rules! text {
        ($text:expr $(,)?) => {
            Some(ChecklistLine::static_text($text))
        };
    }

    /// Emits a key/value line where both sides are static.
    ///
    /// This is the cheapest path through the checklist DSL because both key and value can remain
    /// borrowed until the final buffered write.
    macro_rules! kv_static {
        ($key:expr, $value:expr $(,)?) => {
            Some(ChecklistLine::key_value_static($key, $value))
        };
    }

    /// Emits a key/value line whose value is borrowed from existing guidance state.
    ///
    /// Borrowing here matters because several values, such as the workflow path, already live in
    /// `FirstPublishGuidance`; cloning them just to print one checklist would be unnecessary work.
    macro_rules! kv_borrowed {
        ($key:expr, $value:expr $(,)?) => {
            Some(ChecklistLine::key_value_borrowed($key, $value))
        };
    }

    /// Emits a key/value line that owns its rendered value.
    ///
    /// This is reserved for lines that genuinely need a synthesized `String`, such as inline-code
    /// wrappers or comma-joined package lists. Keeping this explicit makes it easier to audit
    /// where allocations still happen.
    macro_rules! kv_owned {
        ($key:expr, $value:expr $(,)?) => {
            Some(ChecklistLine::key_value_owned($key, $value))
        };
    }

    /// Emits an owned key/value line only when an optional source value exists.
    ///
    /// The render closure runs only on the populated path, which keeps optional checklist lines
    /// concise without forcing the surrounding step to split into multiple helper functions.
    macro_rules! maybe_kv_owned {
        ($key:expr, $value:expr, |$binding:ident| $render:expr $(,)?) => {
            $value.map(|$binding| ChecklistLine::key_value_owned($key, $render))
        };
    }

    /// Emits a static text line behind a boolean gate.
    ///
    /// This keeps conditional checklist entries inline with their neighboring lines, which is
    /// useful for preserving the “entire template in one screen” property of this module.
    macro_rules! when_text {
        ($condition:expr, $text:expr $(,)?) => {
            ($condition).then_some(ChecklistLine::static_text($text))
        };
    }

    /// Emits an owned key/value line behind a boolean gate.
    ///
    /// The checklist uses this for diagnostics such as missing repository metadata, where we only
    /// want to pay the join/allocation cost when there is something actionable to show.
    macro_rules! when_kv_owned {
        ($condition:expr, $key:expr, $value:expr $(,)?) => {
            ($condition).then(|| ChecklistLine::key_value_owned($key, $value))
        };
    }

    /// Declares the full first-publish checklist as a single fixed array.
    ///
    /// This macro is the main readability/performance tradeoff point for the module:
    ///
    /// - Readability: every step is visible in one contiguous block, so reviewers can understand
    ///   the entire checklist without chasing helper functions.
    /// - Performance: the macro expands to a fixed `[ChecklistStep; 5]`, avoiding a top-level
    ///   dynamic `Vec` allocation for the checklist itself.
    ///
    /// The helpers used inside the block (`kv_*`, `when_*`, `text!`) are intentionally tiny so the
    /// callsite still reads like a declarative template rather than imperative push-based code.
    macro_rules! first_publish_checklist {
        ($guidance:expr, $options:expr $(,)?) => {{
            let guidance = $guidance;
            let options = $options;
            let has_repository_issues = !guidance.packages_missing_repository.is_empty()
                || !guidance.packages_mismatched_repository.is_empty();

            [
                step!(
                    "Commit a GitHub Actions release workflow that runs on a GitHub-hosted runner.",
                    [
                        kv_borrowed!("Workflow file", &guidance.workflow_path),
                        maybe_kv_owned!(
                            "Trigger",
                            guidance.release_branch.as_deref(),
                            |branch| render_branch_or_dispatch(branch)
                        ),
                        kv_static!(
                            "Required workflow permissions",
                            "`contents: write` and `id-token: write`",
                        ),
                    ],
                ),
                step!(
                    "Configure npm Trusted Publishing for each package you are releasing.",
                    [
                        Some(match guidance.github_repo.as_deref() {
                            Some(repo) => {
                                ChecklistLine::key_value_owned("Repository", render_inline_code(repo))
                            }
                            None => ChecklistLine::key_value_static("Repository", "`<owner>/<repo>`"),
                        }),
                        kv_owned!(
                            "Workflow filename in npm",
                            render_inline_code(workflow_filename(&guidance.workflow_path)),
                        ),
                        maybe_kv_owned!(
                            "Branch / environment",
                            guidance.release_branch.as_deref(),
                            |branch| render_inline_code(branch)
                        ),
                        text!("npm requires the repository and workflow values to match exactly."),
                        text!(
                            "Trusted publishing currently works for public npm packages and scopes.",
                        ),
                    ],
                ),
                step!(
                    "Make sure each package.json has a matching `repository` entry.",
                    [
                        when_text!(
                            !has_repository_issues,
                            "Looks good for the packages in this release.",
                        ),
                        when_kv_owned!(
                            !guidance.packages_missing_repository.is_empty(),
                            "Missing `repository`",
                            join_string_slice(&guidance.packages_missing_repository, ", "),
                        ),
                        when_kv_owned!(
                            !guidance.packages_mismatched_repository.is_empty(),
                            "Repository does not match git remote",
                            join_string_slice(&guidance.packages_mismatched_repository, ", "),
                        ),
                    ],
                ),
                step!(
                    "For the first public publish of scoped packages, set `publishConfig.access` to `public`.",
                    [
                        when_text!(
                            guidance.scoped_packages_missing_public_access.is_empty(),
                            "No obvious access issues detected."
                        ),
                        when_kv_owned!(
                            !guidance.scoped_packages_missing_public_access.is_empty(),
                            "Missing `publishConfig.access = \"public\"`",
                            join_string_slice(&guidance.scoped_packages_missing_public_access, ", "),
                        ),
                    ],
                ),
                step!(
                    "Validate the release flow from CI before the first real publish.",
                    [
                        kv_owned!("Dry run", render_release_command(options, true, true)),
                        kv_owned!(
                            "Real publish from GitHub Actions",
                            render_release_command(options, false, false),
                        ),
                        text!(
                            "Trusted publishing covers publish itself. If CI also installs private packages, use a separate read-only npm token for install steps.",
                        ),
                    ],
                ),
            ]
        }};
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ChecklistText<'a> {
        Static(&'static str),
        Borrowed(&'a str),
        Owned(String),
    }

    impl ChecklistText<'_> {
        /// Writes a previously classified text fragment into the shared render buffer.
        ///
        /// The checklist renderer deliberately reuses a single `String`, so each line component
        /// writes directly into that buffer instead of allocating a brand new line string.
        fn write_into(&self, buffer: &mut String) {
            match self {
                Self::Static(value) | Self::Borrowed(value) => buffer.push_str(value),
                Self::Owned(value) => buffer.push_str(value),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum ChecklistLine<'a> {
        Text(ChecklistText<'a>),
        KeyValue { key: &'static str, value: ChecklistText<'a> },
    }

    impl ChecklistLine<'_> {
        /// Serializes a single line into the shared render buffer.
        ///
        /// This stays intentionally tiny because the hot path is simple: append the line prefix,
        /// then stream the already-prepared content into the same buffer.
        fn write_into(&self, buffer: &mut String) {
            match self {
                Self::Text(text) => text.write_into(buffer),
                Self::KeyValue { key, value } => {
                    buffer.push_str(key);
                    buffer.push_str(": ");
                    value.write_into(buffer);
                }
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ChecklistStep<'a> {
        title: &'static str,
        lines: Vec<ChecklistLine<'a>>,
    }

    impl<'a> ChecklistStep<'a> {
        /// Builds a step from an iterator of optional lines.
        ///
        /// Accepting `Option<ChecklistLine>` lets the declarative macros keep conditional lines
        /// inline without falling back to imperative `push` code. The constructor uses the
        /// iterator's lower-bound size hint to preallocate just once for the common case.
        fn new<I>(title: &'static str, lines: I) -> Self
        where
            I: IntoIterator<Item = Option<ChecklistLine<'a>>>,
        {
            let iter = lines.into_iter();
            let (lower, _) = iter.size_hint();
            let mut collected = Vec::with_capacity(lower);
            for line in iter {
                if let Some(line) = line {
                    collected.push(line);
                }
            }
            Self { title, lines: collected }
        }
    }

    impl<'a> ChecklistLine<'a> {
        fn static_text(text: &'static str) -> Self {
            Self::Text(ChecklistText::Static(text))
        }

        fn key_value_static(key: &'static str, value: &'static str) -> Self {
            Self::KeyValue { key, value: ChecklistText::Static(value) }
        }

        fn key_value_borrowed(key: &'static str, value: &'a str) -> Self {
            Self::KeyValue { key, value: ChecklistText::Borrowed(value) }
        }

        fn key_value_owned(key: &'static str, value: String) -> Self {
            Self::KeyValue { key, value: ChecklistText::Owned(value) }
        }
    }

    /// Collects repository/workflow/package metadata needed by the first-publish checklist.
    ///
    /// The checklist rendering path is intentionally pure and declarative, so any filesystem or
    /// git-derived facts are gathered ahead of time into `FirstPublishGuidance`.
    pub(super) fn collect_first_publish_guidance(
        cwd: &AbsolutePath,
        release_plans: &[PackageReleasePlan],
    ) -> FirstPublishGuidance {
        let github_repo = detect_github_repo(cwd);
        let release_branch = detect_release_branch(cwd);
        let workflow_path = find_release_workflow_path(cwd);

        let mut guidance = FirstPublishGuidance {
            github_repo: github_repo.clone(),
            release_branch,
            workflow_path,
            ..Default::default()
        };

        for plan in release_plans {
            if plan.name.starts_with('@') && plan.access.as_deref() != Some("public") {
                guidance.scoped_packages_missing_public_access.push(plan.name.clone());
            }

            match plan.repository_url.as_deref() {
                Some(repository_url) => {
                    if let Some(expected_repo) = github_repo.as_deref() {
                        if parse_github_repo_slug(repository_url).as_deref() != Some(expected_repo)
                        {
                            guidance.packages_mismatched_repository.push(plan.name.clone());
                        }
                    }
                }
                None => guidance.packages_missing_repository.push(plan.name.clone()),
            }
        }

        guidance
    }

    /// Renders the first-publish checklist using the declarative checklist DSL above.
    ///
    /// Keeping the checklist materialization next to this callsite makes the overall flow easy to
    /// inspect, while `print_checklist` keeps the indentation and numbering details centralized.
    pub(super) fn print_first_publish_guidance(
        guidance: &FirstPublishGuidance,
        options: &ReleaseOptions,
    ) {
        // Keep the checklist declaration itself adjacent to the callsite so the full first-publish
        // story remains easy to audit in one place.
        let checklist = first_publish_checklist!(guidance, options);
        print_checklist(
            "First publish checklist:",
            "This run uses --first-release, so there are a few one-time setup steps:",
            &checklist,
        );
    }

    /// Renders a concrete `vp release` example command for checklist output.
    ///
    /// This path deliberately avoids `format!` so the user-facing examples follow the same
    /// allocation discipline as the rest of the release command.
    pub(super) fn render_release_command(
        options: &ReleaseOptions,
        dry_run: bool,
        include_skip_publish: bool,
    ) -> String {
        let mut command = String::from("vp release");
        if options.first_release {
            command.push_str(" --first-release");
        }
        if options.changelog {
            command.push_str(" --changelog");
        }
        if let Some(preid) = options.preid.as_deref() {
            command.push_str(" --preid ");
            command.push_str(preid);
        }
        if let Some(projects) = options.projects.as_ref() {
            if !projects.is_empty() {
                command.push_str(" --projects ");
                push_joined(&mut command, projects.iter().map(String::as_str), ",");
            }
        }
        if !options.git_tag {
            command.push_str(" --no-git-tag");
        }
        if !options.git_commit {
            command.push_str(" --no-git-commit");
        }
        if include_skip_publish && options.skip_publish {
            command.push_str(" --skip-publish");
        }
        if dry_run {
            command.push_str(" --dry-run");
        } else {
            command.push_str(" --yes");
        }

        command
    }

    /// Streams checklist lines to the output layer with a single reusable buffer.
    ///
    /// Building the full output eagerly would be simpler, but reusing one `String` keeps this path
    /// cheap and makes allocation behavior very obvious during review.
    fn print_checklist(heading: &str, intro: &str, checklist: &[ChecklistStep<'_>]) {
        // A single reusable buffer keeps line rendering simple and cheap. This is intentionally
        // more verbose than collecting all output first because we want predictable, low-overhead
        // streaming writes without building one large intermediate string.
        output::raw("");
        output::info(heading);

        let mut line = String::with_capacity(256);
        line.push_str(CHECKLIST_STEP_PREFIX);
        line.push_str(intro);
        output::raw(&line);

        for (index, step) in checklist.iter().enumerate() {
            line.clear();
            line.push_str(CHECKLIST_STEP_PREFIX);
            push_display(&mut line, index + 1);
            line.push_str(". ");
            line.push_str(step.title);
            output::raw(&line);

            for item in &step.lines {
                line.clear();
                line.push_str(CHECKLIST_ITEM_PREFIX);
                item.write_into(&mut line);
                output::raw(&line);
            }
        }
    }

    /// Wraps a value in backticks using one tightly-sized owned buffer.
    fn render_inline_code(value: &str) -> String {
        // These helpers precompute a tight-enough capacity because they are hit from the checklist
        // macros and are among the few places where we intentionally materialize owned strings.
        let mut rendered = String::with_capacity(value.len() + 2);
        rendered.push('`');
        rendered.push_str(value);
        rendered.push('`');
        rendered
    }

    /// Formats the branch trigger hint shown in the workflow step.
    fn render_branch_or_dispatch(branch: &str) -> String {
        // Keep the branch rendering centralized so the declarative checklist stays focused on
        // content, while string sizing remains easy to audit in one helper.
        let mut rendered = String::with_capacity(branch.len() + 26);
        rendered.push('`');
        rendered.push_str(branch);
        rendered.push_str("` or `workflow_dispatch`");
        rendered
    }

    /// Joins a borrowed slice of owned strings with a precomputed output capacity.
    fn join_string_slice(values: &[String], separator: &str) -> String {
        // Joining package names is one of the few dynamic list operations in the checklist. The
        // capacity calculation avoids repeated growth when surfacing multiple package issues.
        if values.is_empty() {
            return String::new();
        }

        let separator_bytes = separator.len();
        let total_len = values.iter().map(String::len).sum::<usize>()
            + separator_bytes * values.len().saturating_sub(1);
        let mut joined = String::with_capacity(total_len);
        push_joined(&mut joined, values.iter().map(String::as_str), separator);
        joined
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn render_checklist_lines(checklist: &[ChecklistStep<'_>]) -> Vec<String> {
            let mut rendered = Vec::new();
            let mut line = String::with_capacity(256);

            for (index, step) in checklist.iter().enumerate() {
                line.clear();
                line.push_str(CHECKLIST_STEP_PREFIX);
                push_display(&mut line, index + 1);
                line.push_str(". ");
                line.push_str(step.title);
                rendered.push(line.clone());

                for item in &step.lines {
                    line.clear();
                    line.push_str(CHECKLIST_ITEM_PREFIX);
                    item.write_into(&mut line);
                    rendered.push(line.clone());
                }
            }

            rendered
        }

        #[test]
        fn first_publish_checklist_is_declared_in_stable_step_order() {
            let guidance = FirstPublishGuidance {
                github_repo: Some("voidzero-dev/vite-plus".into()),
                release_branch: Some("main".into()),
                workflow_path: ".github/workflows/release.yml".into(),
                ..Default::default()
            };

            let checklist = first_publish_checklist!(
                &guidance,
                &ReleaseOptions {
                    dry_run: false,
                    skip_publish: false,
                    first_release: true,
                    changelog: false,
                    preid: None,
                    projects: None,
                    git_tag: true,
                    git_commit: true,
                    yes: false,
                },
            );

            let lines = render_checklist_lines(&checklist);
            assert_eq!(
                lines[0],
                "  1. Commit a GitHub Actions release workflow that runs on a GitHub-hosted runner."
            );
            assert!(lines.iter().any(|line| line.contains("Repository: `voidzero-dev/vite-plus`")));
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("Dry run: vp release --first-release --dry-run"))
            );
            assert!(lines.iter().any(|line| {
                line.contains("Real publish from GitHub Actions: vp release --first-release --yes")
            }));
        }

        #[test]
        fn first_publish_checklist_surfaces_package_issues_compactly() {
            let guidance = FirstPublishGuidance {
                workflow_path: ".github/workflows/release.yml".into(),
                packages_missing_repository: vec!["@scope/pkg-a".into(), "@scope/pkg-b".into()],
                packages_mismatched_repository: vec!["@scope/pkg-c".into()],
                scoped_packages_missing_public_access: vec!["@scope/pkg-a".into()],
                ..Default::default()
            };

            let checklist = first_publish_checklist!(
                &guidance,
                &ReleaseOptions {
                    dry_run: false,
                    skip_publish: false,
                    first_release: true,
                    changelog: true,
                    preid: Some("beta".into()),
                    projects: Some(vec!["@scope/pkg-a".into()]),
                    git_tag: false,
                    git_commit: false,
                    yes: false,
                },
            );

            let lines = render_checklist_lines(&checklist);
            assert!(
                lines.iter().any(|line| {
                    line.contains("Missing `repository`: @scope/pkg-a, @scope/pkg-b")
                })
            );
            assert!(lines.iter().any(|line| {
                line.contains("Repository does not match git remote: @scope/pkg-c")
            }));
            assert!(lines.iter().any(|line| {
                line.contains("Missing `publishConfig.access = \"public\"`: @scope/pkg-a")
            }));
            assert!(lines.iter().any(|line| {
                line.contains(
                    "Dry run: vp release --first-release --changelog --preid beta --projects @scope/pkg-a --no-git-tag --no-git-commit --dry-run",
                )
            }));
        }
    }

    fn find_release_workflow_path(cwd: &AbsolutePath) -> String {
        for candidate in [".github/workflows/release.yml", ".github/workflows/release.yaml"] {
            if cwd.join(candidate).as_path().exists() {
                return candidate.to_owned();
            }
        }

        let workflows_dir = cwd.join(".github/workflows");
        if let Ok(entries) = fs::read_dir(workflows_dir.as_path()) {
            let mut best_path: Option<String> = None;
            for entry in entries.filter_map(Result::ok) {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();
                let lowercase = file_name.to_ascii_lowercase();
                if !(lowercase.contains("release")
                    && (lowercase.ends_with(".yml") || lowercase.ends_with(".yaml")))
                {
                    continue;
                }

                let mut path = String::with_capacity(file_name.len() + 18);
                path.push_str(".github/workflows/");
                path.push_str(&file_name);
                if best_path.as_ref().map_or(true, |best| path < *best) {
                    best_path = Some(path);
                }
            }
            if let Some(path) = best_path {
                return path;
            }
        }

        String::from(".github/workflows/release.yml")
    }

    fn workflow_filename(path: &str) -> &str {
        path.rsplit('/').next().unwrap_or(path)
    }

    fn detect_github_repo(cwd: &AbsolutePath) -> Option<String> {
        let remote = capture_git(cwd, ["config", "--get", "remote.origin.url"]).ok()?;
        parse_github_repo_slug(&remote)
    }

    fn detect_release_branch(cwd: &AbsolutePath) -> Option<String> {
        if let Ok(default_head) =
            capture_git(cwd, ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
        {
            let branch = default_head.strip_prefix("origin/").unwrap_or(&default_head).trim();
            if !branch.is_empty() {
                return Some(branch.to_owned());
            }
        }

        let branch = capture_git(cwd, ["branch", "--show-current"]).ok()?;
        let branch = branch.trim();
        (!branch.is_empty()).then(|| branch.to_owned())
    }
}

mod storage {
    use super::*;

    pub(super) fn read_workspace_manifest(
        cwd: &AbsolutePath,
    ) -> Result<Option<PackageManifest>, Error> {
        let manifest_path = cwd.join("package.json");
        match read_package_manifest(&manifest_path) {
            Ok(document) => Ok(Some(document.manifest)),
            Err(PackageJsonError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn ensure_clean_worktree(cwd: &AbsolutePath) -> Result<(), Error> {
        if is_clean_git_worktree(cwd)? {
            return Ok(());
        }

        Err(Error::UserMessage(
            "Refusing to run `vp release` with a dirty git worktree. Commit, stash, or rerun with `--dry-run` first.".into(),
        ))
    }

    pub(super) fn build_package_changelog_section(
        release_date: &str,
        version: &Version,
        commits: &[CommitInfo],
    ) -> String {
        let mut section = String::new();
        section.push_str("## ");
        push_display(&mut section, version);
        section.push_str(" - ");
        section.push_str(release_date);
        section.push_str("\n\n");
        for commit in commits {
            section.push_str("- ");
            section.push_str(&commit.subject);
            section.push_str(" (");
            section.push_str(&commit.short_hash);
            section.push_str(")\n");
        }
        section.push('\n');
        section
    }

    pub(super) fn build_root_changelog_section(
        release_date: &str,
        release_plans: &[PackageReleasePlan],
        commits: &[CommitInfo],
    ) -> String {
        let mut section = String::from("## Release ");
        section.push_str(release_date);
        section.push_str("\n\n");
        section.push_str("### Packages\n\n");
        for plan in release_plans {
            section.push_str("- ");
            section.push_str(&plan.name);
            section.push('@');
            push_display(&mut section, &plan.next_version);
            section.push('\n');
        }
        section.push('\n');
        section.push_str("### Changes\n\n");
        for commit in commits {
            section.push_str("- ");
            section.push_str(&commit.subject);
            section.push_str(" (");
            section.push_str(&commit.short_hash);
            section.push_str(")\n");
        }
        section.push('\n');
        section
    }

    pub(super) fn write_changelog_section(path: &AbsolutePath, section: &str) -> Result<(), Error> {
        let new_contents = match fs::read_to_string(path) {
            Ok(existing) => prepend_changelog_section(&existing, section),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                prepend_changelog_heading(section)
            }
            Err(err) => return Err(err.into()),
        };

        fs::write(path, new_contents)?;
        Ok(())
    }

    fn prepend_changelog_section(existing: &str, section: &str) -> String {
        if let Some(rest) = existing.strip_prefix("# Changelog") {
            let rest = rest.trim_start_matches('\n');
            if rest.is_empty() {
                return prepend_changelog_heading(section);
            }
            let mut updated = prepend_changelog_heading(section);
            updated.push_str(rest);
            return updated;
        }

        let mut updated = prepend_changelog_heading(section);
        updated.push_str(existing.trim_start());
        updated
    }

    fn prepend_changelog_heading(section: &str) -> String {
        let mut updated = String::from("# Changelog\n\n");
        updated.push_str(section);
        updated
    }

    pub(super) fn update_manifest_version(
        path: &AbsolutePath,
        contents: &str,
        current_version: &str,
        next_version: &str,
    ) -> Result<(), Error> {
        let updated =
            replace_top_level_string_property(contents, "version", current_version, next_version)?;
        fs::write(path, updated)?;
        Ok(())
    }

    pub(super) fn release_commit_message(release_plans: &[PackageReleasePlan]) -> String {
        if release_plans.len() <= 3 {
            let mut message = String::from("chore(release): publish ");
            let mut first = true;
            for plan in release_plans {
                if !first {
                    message.push_str(", ");
                }
                first = false;
                message.push_str(&plan.name);
                message.push('@');
                push_display(&mut message, &plan.next_version);
            }
            message
        } else {
            let mut message = String::from("chore(release): publish ");
            push_display(&mut message, release_plans.len());
            message.push_str(" packages");
            message
        }
    }

    pub(super) fn git_add_paths(
        cwd: &AbsolutePath,
        paths: &[AbsolutePathBuf],
    ) -> Result<(), Error> {
        let mut args = Vec::with_capacity(paths.len() + 1);
        args.push(String::from("add"));
        for path in paths {
            args.push(path.as_path().to_string_lossy().to_string());
        }
        run_git(cwd, args).map_err(|err| {
            let mut message = String::from("stage release changes: ");
            push_display(&mut message, err);
            Error::UserMessage(message.into())
        })
    }

    pub(super) fn git_commit(cwd: &AbsolutePath, message: &str) -> Result<(), Error> {
        run_git(cwd, ["commit", "-m", message]).map_err(|err| {
            let mut error_message = String::from("create release commit: ");
            push_display(&mut error_message, err);
            Error::UserMessage(error_message.into())
        })
    }

    pub(super) fn git_tag(cwd: &AbsolutePath, tag_name: &str) -> Result<(), Error> {
        run_git(cwd, ["tag", tag_name]).map_err(|err| {
            let mut message = String::from("create release tag: ");
            push_display(&mut message, err);
            Error::UserMessage(message.into())
        })
    }
}

mod protocols {
    use super::*;

    pub(super) fn unsupported_publish_protocols(
        package_manager: &PackageManager,
        summary: DependencyProtocolSummary,
    ) -> Vec<&'static str> {
        // Publish-time protocol rewriting differs across package managers, so release stays
        // conservative and only allows protocols that the selected native publisher documents.
        // npm workspaces: https://docs.npmjs.com/cli/v11/using-npm/workspaces/
        // pnpm workspaces/catalogs: https://pnpm.io/workspaces / https://pnpm.io/catalogs
        // Yarn workspace protocol: https://yarnpkg.com/protocol/workspace
        // Bun workspaces/catalogs: https://bun.sh/docs/pm/workspaces / https://bun.sh/docs/pm/catalogs
        let mut protocols = Vec::new();

        if summary.workspace && !supports_workspace_publish_rewrite(package_manager) {
            protocols.push("workspace:");
        }
        if summary.catalog && !supports_catalog_publish_rewrite(package_manager) {
            protocols.push("catalog:");
        }
        if summary.file {
            protocols.push("file:");
        }
        if summary.link {
            protocols.push("link:");
        }
        if summary.portal {
            protocols.push("portal:");
        }
        if summary.patch {
            protocols.push("patch:");
        }
        if summary.jsr {
            protocols.push("jsr:");
        }

        protocols
    }

    fn supports_workspace_publish_rewrite(package_manager: &PackageManager) -> bool {
        match package_manager.client {
            PackageManagerType::Pnpm | PackageManagerType::Bun => true,
            PackageManagerType::Yarn => !package_manager.version.starts_with("1."),
            PackageManagerType::Npm => false,
        }
    }

    fn supports_catalog_publish_rewrite(package_manager: &PackageManager) -> bool {
        match package_manager.client {
            PackageManagerType::Pnpm | PackageManagerType::Bun => true,
            PackageManagerType::Yarn => !package_manager.version.starts_with("1."),
            PackageManagerType::Npm => false,
        }
    }
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
            current_version: Version::parse("1.0.0").unwrap(),
            next_version: Version::parse("1.0.1").unwrap(),
            level: VersionBump::Patch,
            commits: Vec::new(),
            changelog_path: test_absolute_path(&format!("/packages/{name}/CHANGELOG.md")),
            access: None,
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
