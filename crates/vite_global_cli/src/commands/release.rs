use std::{
    collections::{HashMap, HashSet},
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
        collect_orphaned_released_packages(&workspace_root_path, &workspace_packages).await?;
    if !orphaned_released_packages.is_empty() {
        output::warn(&format!(
            "Previously released packages no longer map to an active workspace package: {}. Add `vitePlus.release.previousNames` / `retiredNames` metadata where appropriate and consider deprecating removed packages on the registry.",
            orphaned_released_packages.join(", ")
        ));
    }
    let selected = select_workspace_packages(&workspace_packages, options.projects.as_deref())?;
    if selected.is_empty() {
        output::warn("No publishable packages matched the release selection.");
        return Ok(ExitStatus::SUCCESS);
    }

    let ordered = topological_sort_selected_packages(&package_graph, &selected);

    let mut release_plans = Vec::new();
    let mut root_commits = Vec::new();
    let mut seen_commit_hashes = HashSet::new();

    for package in ordered {
        let previous_tag = if options.first_release {
            None
        } else {
            find_latest_package_tag(&workspace_root_path, &package.known_names).await?
        };
        let latest_stable_version = if options.first_release {
            None
        } else {
            find_latest_stable_package_version(&workspace_root_path, &package.known_names).await?
        };

        let commits = collect_package_commits(
            &workspace_root_path,
            &package.release_paths,
            previous_tag.as_deref(),
        )
        .await?;

        let Some(level) = highest_release_level(&commits) else {
            output::note(&format!(
                "Skipping {} because no releasable conventional commits were found.",
                package.name
            ));
            continue;
        };

        let current_version = Version::parse(&package.manifest.version).map_err(|e| {
            Error::UserMessage(
                format!(
                    "Package '{}' has an invalid version '{}': {}",
                    package.name, package.manifest.version, e
                )
                .into(),
            )
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
        let guidance = collect_first_publish_guidance(&workspace_root_path, &release_plans).await;
        print_first_publish_guidance(&guidance, &options);
    }
    let readiness_report =
        collect_release_readiness_report(workspace_manifest.as_ref(), &release_plans);
    print_release_readiness_report(&readiness_report);

    if !options.skip_publish {
        let protocol_issues: Vec<String> = release_plans
            .iter()
            .filter_map(|plan| {
                let protocols =
                    unsupported_publish_protocols(&package_manager, plan.protocol_summary);
                (!protocols.is_empty()).then_some(format!(
                    "{} ({})",
                    plan.name,
                    protocols.join(", ")
                ))
            })
            .collect();

        if !protocol_issues.is_empty() {
            let message = format!(
                "Publishing with {} is unsafe because these packages still contain unsupported publish-time dependency protocols: {}. Use a package manager with native publish rewriting support where available, or rerun with `--skip-publish`.",
                package_manager.client,
                protocol_issues.join(", ")
            );
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

    ensure_clean_worktree(&workspace_root_path).await?;
    if !options.yes && !confirm_release(&release_plans, &readiness_report, &options)? {
        return Ok(ExitStatus::SUCCESS);
    }

    let mut changed_files = Vec::new();
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
        git_add_paths(&workspace_root_path, &changed_files).await?;
        let commit_message = release_commit_message(&release_plans);
        git_commit(&workspace_root_path, &commit_message).await?;
        output::success(&format!("Created release commit: {commit_message}"));
    }

    if !options.skip_publish {
        for plan in &release_plans {
            output::info(&format!("Publishing {}@{}", plan.name, plan.next_version));
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
            git_tag(&workspace_root_path, &plan.tag_name).await?;
            output::success(&format!("Created git tag {}", plan.tag_name));
        }
    }

    output::success(&format!("Release completed for {} package(s).", release_plans.len()));
    Ok(ExitStatus::SUCCESS)
}

fn load_workspace_packages(
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

fn select_workspace_packages(
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
                Error::UserMessage(format!("Invalid project pattern '{}': {}", pattern, e).into())
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
            output::warn(&format!("No publishable packages matched '{}'", projects[index]));
        }
    }

    Ok(selected)
}

fn topological_sort_selected_packages(
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

async fn find_latest_package_tag(
    cwd: &AbsolutePath,
    package_names: &[String],
) -> Result<Option<String>, Error> {
    let stdout = capture_git(cwd, release_tag_list_args(package_names)).await?;
    Ok(stdout.lines().map(str::trim).find(|line| !line.is_empty()).map(ToOwned::to_owned))
}

async fn find_latest_stable_package_version(
    cwd: &AbsolutePath,
    package_names: &[String],
) -> Result<Option<Version>, Error> {
    let stdout = capture_git(cwd, release_tag_list_args(package_names)).await?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter_map(parse_package_tag_version)
        .find(|version| !version.has_prerelease()))
}

async fn collect_package_commits(
    cwd: &AbsolutePath,
    package_paths: &[String],
    since_tag: Option<&str>,
) -> Result<Vec<CommitInfo>, Error> {
    let mut args =
        vec!["log".to_string(), "--reverse".to_string(), "--format=%H%x1f%s%x1f%b%x1e".to_string()];
    if let Some(tag) = since_tag {
        args.push(format!("{tag}..HEAD"));
    }
    args.push("--".to_string());
    for package_path in package_paths {
        args.push(package_path.clone());
    }

    let stdout = capture_git(cwd, args).await?;
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
                short_hash: hash.chars().take(7).collect(),
                subject: subject.to_owned(),
                level,
            });
        }
    }

    Ok(commits)
}

fn classify_commit(subject: &str, body: &str) -> Option<VersionBump> {
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

fn highest_release_level(commits: &[CommitInfo]) -> Option<VersionBump> {
    commits.iter().map(|commit| commit.level).max()
}

fn effective_release_level(current: &Version, level: VersionBump) -> VersionBump {
    // Conventional Commits marks breaking changes as MAJOR, but SemVer treats `0.y.z` as initial
    // development where API compatibility can still move on the minor line.
    // https://www.conventionalcommits.org/en/v1.0.0/#specification
    // https://semver.org/#faq
    if current.major == 0 && level == VersionBump::Major { VersionBump::Minor } else { level }
}

fn next_release_version(
    current: &Version,
    level: VersionBump,
    stable_baseline: Option<&Version>,
    prerelease_tag: Option<&str>,
) -> Result<Version, Error> {
    if !level.is_version_bump() {
        return Err(Error::UserMessage(
            format!("Invalid release level '{}' for version bump calculation.", level.as_str())
                .into(),
        ));
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

fn prerelease_with_number(prerelease_tag: &PrereleaseTag, number: u64) -> Result<String, Error> {
    build_prerelease(prerelease_tag.as_str(), number).map_err(|e| {
        Error::UserMessage(
            format!("Invalid prerelease identifier '{}': {}", prerelease_tag.as_str(), e).into(),
        )
    })
}

fn parse_package_tag_version(tag_name: &str) -> Option<Version> {
    let (_, version) = tag_name.rsplit_once("/v")?;
    Version::parse(version).ok()
}

fn print_release_plan(release_plans: &[PackageReleasePlan], options: &ReleaseOptions) {
    output::info("Planned releases:");
    for plan in release_plans {
        output::raw(&format!(
            "  {} {} -> {} ({})",
            plan.name,
            plan.current_version,
            plan.next_version,
            plan.level.as_str()
        ));
        if plan.known_names.len() > 1 {
            let historical_names = plan.known_names[1..].join(", ");
            output::raw(&format!("    previous names: {historical_names}"));
        }
        if !plan.retired_names.is_empty() {
            output::raw(&format!("    retired package names: {}", plan.retired_names.join(", ")));
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

fn collect_release_readiness_report(
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
            report.package_scripts.push(PackageReadiness { package: plan.name.clone(), scripts });
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
            warnings.push(format!(
                "{scope} declares `vitePlus.release.checkScripts` entry '{}' but no matching script exists.",
                configured_script
            ));
        }
    }

    if warn_when_missing && scripts.is_empty() && configured_scripts.is_empty() {
        warnings.push(format!(
            "{scope} does not expose obvious pre-release checks (`build`, `pack`, `prepack`, `prepublishOnly`, `prepare`, or `vitePlus.release.checkScripts`). Double-check build and pack steps before publishing."
        ));
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

fn print_release_readiness_report(report: &ReleaseReadinessReport) {
    output::raw("");
    output::info("Pre-release readiness:");

    if !report.workspace_scripts.is_empty() {
        output::raw(&format!("  workspace scripts: {}", report.workspace_scripts.join(", ")));
    }

    if report.package_scripts.is_empty() {
        output::raw("  package scripts: none detected");
    } else {
        output::raw("  package scripts:");
        for package in &report.package_scripts {
            output::raw(&format!("    - {}: {}", package.package, package.scripts.join(", ")));
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

fn print_dry_run_actions(
    release_plans: &[PackageReleasePlan],
    package_manager: &PackageManager,
    options: &ReleaseOptions,
) {
    let commit_message = release_commit_message(release_plans);
    output::note(&format!("Would update {} package.json file(s).", release_plans.len()));
    if options.changelog {
        output::note("Would update the root and per-package changelog files.");
    } else {
        output::note("Would skip changelog generation because --changelog was not provided.");
    }
    if options.git_commit {
        output::note(&format!("Would create release commit: {commit_message}"));
    }
    if options.git_tag {
        for plan in release_plans {
            output::note(&format!("Would create git tag {}", plan.tag_name));
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
        output::note(&format!(
            "Would publish {}@{} with: {} {}",
            plan.name,
            plan.next_version,
            publish_options.bin_path,
            publish_options.args.join(" ")
        ));
    }
}

async fn collect_first_publish_guidance(
    cwd: &AbsolutePath,
    release_plans: &[PackageReleasePlan],
) -> FirstPublishGuidance {
    let github_repo = detect_github_repo(cwd).await;
    let release_branch = detect_release_branch(cwd).await;
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
                    if parse_github_repo_slug(repository_url).as_deref() != Some(expected_repo) {
                        guidance.packages_mismatched_repository.push(plan.name.clone());
                    }
                }
            }
            None => guidance.packages_missing_repository.push(plan.name.clone()),
        }
    }

    guidance
}

fn print_first_publish_guidance(guidance: &FirstPublishGuidance, options: &ReleaseOptions) {
    output::raw("");
    output::info("First publish checklist:");
    output::raw("  This run uses --first-release, so there are a few one-time setup steps:");
    output::raw(
        "  1. Commit a GitHub Actions release workflow that runs on a GitHub-hosted runner.",
    );
    output::raw(&format!("     - Workflow file: {}", guidance.workflow_path));
    if let Some(branch) = guidance.release_branch.as_deref() {
        output::raw(&format!("     - Trigger from `{branch}` or `workflow_dispatch`"));
    }
    output::raw("     - Required workflow permissions: `contents: write` and `id-token: write`");

    output::raw("  2. Configure npm Trusted Publishing for each package you are releasing.");
    if let Some(repo) = guidance.github_repo.as_deref() {
        output::raw(&format!("     - Repository: `{repo}`"));
    } else {
        output::raw("     - Repository: `<owner>/<repo>`");
    }
    output::raw(&format!(
        "     - Workflow filename in npm: `{}`",
        workflow_filename(&guidance.workflow_path)
    ));
    if let Some(branch) = guidance.release_branch.as_deref() {
        output::raw(&format!("     - Branch / environment: `{branch}`"));
    }
    output::raw("     - npm requires the repository and workflow values to match exactly.");
    output::raw("     - Trusted publishing currently works for public npm packages and scopes.");

    output::raw("  3. Make sure each package.json has a matching `repository` entry.");
    if guidance.packages_missing_repository.is_empty()
        && guidance.packages_mismatched_repository.is_empty()
    {
        output::raw("     - Looks good for the packages in this release.");
    } else {
        if !guidance.packages_missing_repository.is_empty() {
            output::raw(&format!(
                "     - Missing `repository`: {}",
                guidance.packages_missing_repository.join(", ")
            ));
        }
        if !guidance.packages_mismatched_repository.is_empty() {
            output::raw(&format!(
                "     - Repository does not match git remote: {}",
                guidance.packages_mismatched_repository.join(", ")
            ));
        }
    }

    output::raw(
        "  4. For the first public publish of scoped packages, set `publishConfig.access` to `public`.",
    );
    if guidance.scoped_packages_missing_public_access.is_empty() {
        output::raw("     - No obvious access issues detected.");
    } else {
        output::raw(&format!(
            "     - Missing `publishConfig.access = \"public\"`: {}",
            guidance.scoped_packages_missing_public_access.join(", ")
        ));
    }

    output::raw("  5. Validate the release flow from CI before the first real publish.");
    output::raw(&format!("     - Dry run: {}", render_release_command(options, true, true)));
    output::raw(&format!(
        "     - Real publish from GitHub Actions: {}",
        render_release_command(options, false, false)
    ));
    output::raw(
        "     - Trusted publishing covers publish itself. If CI also installs private packages, use a separate read-only npm token for install steps.",
    );
}

fn render_release_command(
    options: &ReleaseOptions,
    dry_run: bool,
    include_skip_publish: bool,
) -> String {
    let mut args = vec!["vp".to_string(), "release".to_string()];

    if options.first_release {
        args.push("--first-release".to_string());
    }
    if options.changelog {
        args.push("--changelog".to_string());
    }
    if let Some(preid) = options.preid.as_deref() {
        args.push("--preid".to_string());
        args.push(preid.to_string());
    }
    if let Some(projects) = options.projects.as_ref() {
        if !projects.is_empty() {
            args.push("--projects".to_string());
            args.push(projects.join(","));
        }
    }
    if !options.git_tag {
        args.push("--no-git-tag".to_string());
    }
    if !options.git_commit {
        args.push("--no-git-commit".to_string());
    }
    if include_skip_publish && options.skip_publish {
        args.push("--skip-publish".to_string());
    }
    if dry_run {
        args.push("--dry-run".to_string());
    } else {
        args.push("--yes".to_string());
    }

    args.join(" ")
}

fn read_workspace_manifest(cwd: &AbsolutePath) -> Result<Option<PackageManifest>, Error> {
    let manifest_path = cwd.join("package.json");
    match read_package_manifest(&manifest_path) {
        Ok(document) => Ok(Some(document.manifest)),
        Err(PackageJsonError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn confirm_release(
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
    output::raw(&format!("  packages: {}", release_plans.len()));
    output::raw(&format!("  publish: {}", if options.skip_publish { "no" } else { "yes" }));
    output::raw(&format!("  changelog: {}", if options.changelog { "yes" } else { "no" }));
    output::raw(&format!("  git commit: {}", if options.git_commit { "yes" } else { "no" }));
    output::raw(&format!("  git tags: {}", if options.git_tag { "yes" } else { "no" }));
    output::raw(&format!("  prerelease tag: {}", options.preid.as_deref().unwrap_or("stable")));
    if !readiness_report.warnings.is_empty() {
        output::warn(&format!(
            "{} pre-release warning(s) need review before continuing.",
            readiness_report.warnings.len()
        ));
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

async fn ensure_clean_worktree(cwd: &AbsolutePath) -> Result<(), Error> {
    if is_clean_git_worktree(cwd).await? {
        return Ok(());
    }

    Err(Error::UserMessage(
        "Refusing to run `vp release` with a dirty git worktree. Commit, stash, or rerun with `--dry-run` first.".into(),
    ))
}

fn build_package_changelog_section(
    release_date: &str,
    version: &Version,
    commits: &[CommitInfo],
) -> String {
    let mut section = format!("## {} - {}\n\n", version, release_date);
    for commit in commits {
        section.push_str(&format!("- {} ({})\n", commit.subject, commit.short_hash));
    }
    section.push('\n');
    section
}

fn build_root_changelog_section(
    release_date: &str,
    release_plans: &[PackageReleasePlan],
    commits: &[CommitInfo],
) -> String {
    let mut section = format!("## Release {}\n\n", release_date);
    section.push_str("### Packages\n\n");
    for plan in release_plans {
        section.push_str(&format!("- {}@{}\n", plan.name, plan.next_version));
    }
    section.push('\n');
    section.push_str("### Changes\n\n");
    for commit in commits {
        section.push_str(&format!("- {} ({})\n", commit.subject, commit.short_hash));
    }
    section.push('\n');
    section
}

fn write_changelog_section(path: &AbsolutePath, section: &str) -> Result<(), Error> {
    let new_contents = match fs::read_to_string(path) {
        Ok(existing) => prepend_changelog_section(&existing, section),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            format!("# Changelog\n\n{section}")
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
            return format!("# Changelog\n\n{section}");
        }
        return format!("# Changelog\n\n{section}{rest}");
    }

    format!("# Changelog\n\n{section}{}", existing.trim_start())
}

fn update_manifest_version(
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

fn release_commit_message(release_plans: &[PackageReleasePlan]) -> String {
    if release_plans.len() <= 3 {
        let packages = release_plans
            .iter()
            .map(|plan| format!("{}@{}", plan.name, plan.next_version))
            .collect::<Vec<_>>()
            .join(", ");
        format!("chore(release): publish {packages}")
    } else {
        format!("chore(release): publish {} packages", release_plans.len())
    }
}

fn package_tag_name(package_name: &str, version: &Version) -> String {
    format!("release/{}/v{}", sanitize_package_name(package_name), version)
}

fn sanitize_package_name(package_name: &str) -> String {
    package_name.trim_start_matches('@').to_owned()
}

fn release_tag_list_args(package_names: &[String]) -> Vec<String> {
    let mut args = vec!["tag".to_string(), "--list".to_string(), "--sort=-creatordate".to_string()];
    for package_name in package_names {
        args.push(format!("release/{}/v*", sanitize_package_name(package_name)));
    }
    args
}

fn parse_package_name_from_tag(tag_name: &str) -> Option<String> {
    let package_path = tag_name.strip_prefix("release/")?.rsplit_once("/v")?.0;
    if package_path.contains('/') {
        Some(format!("@{package_path}"))
    } else {
        Some(package_path.to_owned())
    }
}

async fn collect_orphaned_released_packages(
    cwd: &AbsolutePath,
    packages: &[WorkspacePackage],
) -> Result<Vec<String>, Error> {
    let stdout = capture_git(cwd, ["tag", "--list", "release/*/v*"]).await?;
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

fn unique_strings<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            ordered.push(value);
        }
    }
    ordered
}

fn collect_orphaned_released_package_names<'a, I>(
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

fn find_release_workflow_path(cwd: &AbsolutePath) -> String {
    for candidate in [".github/workflows/release.yml", ".github/workflows/release.yaml"] {
        if cwd.join(candidate).as_path().exists() {
            return candidate.to_string();
        }
    }

    let workflows_dir = cwd.join(".github/workflows");
    if let Ok(entries) = fs::read_dir(workflows_dir.as_path()) {
        let mut release_workflows = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let lowercase = file_name.to_ascii_lowercase();
                (lowercase.contains("release")
                    && (lowercase.ends_with(".yml") || lowercase.ends_with(".yaml")))
                .then_some(format!(".github/workflows/{file_name}"))
            })
            .collect::<Vec<_>>();
        release_workflows.sort();
        if let Some(path) = release_workflows.into_iter().next() {
            return path;
        }
    }

    ".github/workflows/release.yml".to_string()
}

fn workflow_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

async fn detect_github_repo(cwd: &AbsolutePath) -> Option<String> {
    let remote = capture_git(cwd, ["config", "--get", "remote.origin.url"]).await.ok()?;
    parse_github_repo_slug(&remote)
}

async fn detect_release_branch(cwd: &AbsolutePath) -> Option<String> {
    if let Ok(default_head) =
        capture_git(cwd, ["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).await
    {
        let branch = default_head.strip_prefix("origin/").unwrap_or(&default_head).trim();
        if !branch.is_empty() {
            return Some(branch.to_string());
        }
    }

    let branch = capture_git(cwd, ["branch", "--show-current"]).await.ok()?;
    let branch = branch.trim();
    (!branch.is_empty()).then_some(branch.to_string())
}

async fn git_add_paths(cwd: &AbsolutePath, paths: &[AbsolutePathBuf]) -> Result<(), Error> {
    let mut args = vec!["add".to_string()];
    for path in paths {
        args.push(path.as_path().to_string_lossy().to_string());
    }
    run_git(cwd, args)
        .await
        .map_err(|err| Error::UserMessage(format!("stage release changes: {err}").into()))
}

async fn git_commit(cwd: &AbsolutePath, message: &str) -> Result<(), Error> {
    run_git(cwd, ["commit", "-m", message])
        .await
        .map_err(|err| Error::UserMessage(format!("create release commit: {err}").into()))
}

async fn git_tag(cwd: &AbsolutePath, tag_name: &str) -> Result<(), Error> {
    run_git(cwd, ["tag", tag_name])
        .await
        .map_err(|err| Error::UserMessage(format!("create release tag: {err}").into()))
}

fn unsupported_publish_protocols(
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
