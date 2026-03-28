//! Release planning, git-history inspection, and tag naming.
//!
//! This module turns discovered workspace packages into deterministic release plans. Its main
//! jobs are to:
//!
//! - normalize historical package identity across renames and moves
//! - collect conventional commits since the previous release watermark
//! - infer semantic version bumps and prerelease progression
//! - preserve a stable package order that respects both dependencies and user selection order
//!
//! It may read git state, but it intentionally does not mutate the worktree or print summaries.

use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
};

use super::*;

const GIT_LOG_FIELD_SEPARATOR: char = '\u{001f}';
const GIT_LOG_RECORD_SEPARATOR: char = '\u{001e}';
const GIT_LOG_FORMAT: &str = "--format=%H%x1f%s%x1f%b%x1e";
const GIT_LOG_PART_COUNT: usize = 3;
const SHORT_COMMIT_HASH_LEN: usize = 7;

/// Encodes the git tag layout used as the durable release watermark in this repository.
#[derive(Debug, Clone, Copy)]
struct ReleaseTagFormat {
    namespace: &'static str,
    version_prefix: &'static str,
}

impl ReleaseTagFormat {
    fn git_tag_list_args(self, package_names: &[String]) -> Vec<String> {
        let mut args = Vec::with_capacity(package_names.len() + 3);
        args.push(String::from("tag"));
        args.push(String::from("--list"));
        args.push(String::from("--sort=-creatordate"));
        for package_name in package_names {
            args.push(self.package_tag_pattern(package_name));
        }
        args
    }

    fn package_tag_pattern(self, package_name: &str) -> String {
        let sanitized = self.sanitize_package_name(package_name);
        let mut pattern = String::with_capacity(
            self.namespace.len() + sanitized.len() + self.version_prefix.len() + 1,
        );
        pattern.push_str(self.namespace);
        pattern.push_str(sanitized);
        pattern.push_str(self.version_prefix);
        pattern.push('*');
        pattern
    }

    fn all_tags_pattern(self) -> String {
        let mut pattern =
            String::with_capacity(self.namespace.len() + self.version_prefix.len() + 2);
        pattern.push_str(self.namespace);
        pattern.push('*');
        pattern.push_str(self.version_prefix);
        pattern.push('*');
        pattern
    }

    fn format_tag(self, package_name: &str, version: &Version) -> String {
        let sanitized = self.sanitize_package_name(package_name);
        let mut tag = String::with_capacity(
            self.namespace.len() + sanitized.len() + self.version_prefix.len() + 12,
        );
        tag.push_str(self.namespace);
        tag.push_str(sanitized);
        tag.push_str(self.version_prefix);
        push_display(&mut tag, version);
        tag
    }

    fn parse_version(self, tag_name: &str) -> Option<Version> {
        let (_, version) = tag_name.rsplit_once(self.version_prefix)?;
        Version::parse(version).ok()
    }

    fn parse_package_name(self, tag_name: &str) -> Option<String> {
        let package_path =
            tag_name.strip_prefix(self.namespace)?.rsplit_once(self.version_prefix)?.0;
        if package_path.contains('/') {
            let mut package_name = String::with_capacity(package_path.len() + 1);
            package_name.push('@');
            package_name.push_str(package_path);
            Some(package_name)
        } else {
            Some(package_path.to_owned())
        }
    }

    fn sanitize_package_name(self, package_name: &str) -> &str {
        package_name.trim_start_matches('@')
    }
}

const RELEASE_TAG_FORMAT: ReleaseTagFormat =
    ReleaseTagFormat { namespace: "release/", version_prefix: "/v" };

/// Priority-queue entry used to produce a stable topological ordering for selected packages.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ReleaseQueueEntry {
    selection_order: usize,
    name: String,
    node: PackageNodeIndex,
}

impl Ord for ReleaseQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.selection_order
            .cmp(&other.selection_order)
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.node.index().cmp(&other.node.index()))
    }
}

impl PartialOrd for ReleaseQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn load_workspace_packages(
    package_graph: &WorkspacePackageGraph,
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
    package_graph: &WorkspacePackageGraph,
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

    let mut available = BinaryHeap::new();
    for (&node, &count) in &pending_dependencies {
        if count == 0 {
            let package = by_node.get(&node).expect("selected package should exist");
            available.push(Reverse(release_queue_entry(package)));
        }
    }

    let mut scheduled = HashSet::with_capacity(selected.len());
    let mut ordered_nodes = Vec::with_capacity(selected.len());

    while ordered_nodes.len() < selected.len() {
        let next = available
            .pop()
            .map(|Reverse(entry)| entry.node)
            .unwrap_or_else(|| select_cycle_breaker(selected, &scheduled));

        if !scheduled.insert(next) {
            continue;
        }
        ordered_nodes.push(next);

        if let Some(node_dependents) = dependents.get(&next) {
            for dependent in node_dependents {
                if let Some(count) = pending_dependencies.get_mut(dependent) {
                    if *count == 0 {
                        continue;
                    }

                    *count -= 1;
                    if *count == 0 && !scheduled.contains(dependent) {
                        let package =
                            by_node.get(dependent).expect("selected package should exist");
                        available.push(Reverse(release_queue_entry(package)));
                    }
                }
            }
        }
    }

    ordered_nodes.into_iter().filter_map(|node| by_node.get(&node).cloned()).collect()
}

pub(super) fn find_latest_package_tag(
    cwd: &AbsolutePath,
    package_names: &[String],
) -> Result<Option<String>, Error> {
    let stdout = capture_git(cwd, RELEASE_TAG_FORMAT.git_tag_list_args(package_names))?;
    Ok(stdout.lines().map(str::trim).find(|line| !line.is_empty()).map(ToOwned::to_owned))
}

pub(super) fn find_latest_stable_package_version(
    cwd: &AbsolutePath,
    package_names: &[String],
) -> Result<Option<Version>, Error> {
    let stdout = capture_git(cwd, RELEASE_TAG_FORMAT.git_tag_list_args(package_names))?;
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter_map(|tag_name| RELEASE_TAG_FORMAT.parse_version(tag_name))
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
    args.push(String::from(GIT_LOG_FORMAT));
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

    for record in stdout.split(GIT_LOG_RECORD_SEPARATOR) {
        let trimmed = record.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(GIT_LOG_PART_COUNT, GIT_LOG_FIELD_SEPARATOR);
        let hash = parts.next().unwrap_or_default().trim();
        let subject = parts.next().unwrap_or_default().trim();
        let body = parts.next().unwrap_or_default().trim();
        if hash.is_empty() || subject.is_empty() {
            continue;
        }

        if let Some(level) = classify_commit(subject, body) {
            commits.push(CommitInfo {
                hash: hash.to_owned(),
                short_hash: hash.get(..SHORT_COMMIT_HASH_LEN).unwrap_or(hash).to_owned(),
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

pub(super) fn package_tag_name(package_name: &str, version: &Version) -> String {
    RELEASE_TAG_FORMAT.format_tag(package_name, version)
}

pub(super) fn collect_orphaned_released_packages(
    cwd: &AbsolutePath,
    packages: &[WorkspacePackage],
) -> Result<Vec<String>, Error> {
    let stdout = capture_git(cwd, ["tag", "--list", &RELEASE_TAG_FORMAT.all_tags_pattern()])?;
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
        if let Some(package_name) = RELEASE_TAG_FORMAT.parse_package_name(tag)
            && !known_names.contains(package_name.as_str())
        {
            orphaned.insert(package_name);
        }
    }

    let mut orphaned = orphaned.into_iter().collect::<Vec<_>>();
    orphaned.sort();
    orphaned
}

#[cfg(test)]
pub(super) fn parse_package_name_from_release_tag(tag_name: &str) -> Option<String> {
    RELEASE_TAG_FORMAT.parse_package_name(tag_name)
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
        let mut message = String::from("Invalid prerelease identifier '");
        message.push_str(prerelease_tag.as_str());
        message.push_str("': ");
        push_display(&mut message, e);
        Error::UserMessage(message.into())
    })
}

fn release_queue_entry(package: &WorkspacePackage) -> ReleaseQueueEntry {
    ReleaseQueueEntry {
        selection_order: package.selection_order,
        name: package.name.clone(),
        node: package.node,
    }
}

fn select_cycle_breaker(
    selected: &[WorkspacePackage],
    scheduled: &HashSet<PackageNodeIndex>,
) -> PackageNodeIndex {
    selected
        .iter()
        .filter(|package| !scheduled.contains(&package.node))
        .min_by(|left, right| compare_release_priority(left, right))
        .map(|package| package.node)
        .expect("there should be an unscheduled package when ordering is incomplete")
}

fn compare_release_priority(left: &WorkspacePackage, right: &WorkspacePackage) -> Ordering {
    left.selection_order.cmp(&right.selection_order).then_with(|| left.name.cmp(&right.name))
}
