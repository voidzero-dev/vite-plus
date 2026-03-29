//! Release persistence, rollback, and side-effect helpers.
//!
//! This module contains the mutating half of the release workflow:
//!
//! - reading manifests
//! - computing final artifact edits
//! - applying and rolling back file mutations
//! - invoking git and native publish commands
//!
//! The orchestration layer calls into these functions only after planning and validation have
//! completed, so the code here can stay focused on careful state transitions and rollback paths.

use super::*;

const DIRECT_PUBLISH_PROTOCOL_PREFIXES: [&str; 6] =
    ["catalog:", "file:", "link:", "portal:", "patch:", "jsr:"];
const PROGRESS_INDENT: &str = "  ";

macro_rules! raw_progress_line {
    ($($part:expr),+ $(,)?) => {{
        let mut line = String::from(PROGRESS_INDENT);
        $(push_display(&mut line, $part);)+
        output::raw(&line);
    }};
}

/// Reads the root workspace manifest when one exists.
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

/// Fails closed when the current worktree is dirty.
pub(super) fn ensure_clean_worktree(cwd: &AbsolutePath) -> Result<(), Error> {
    if is_clean_git_worktree(cwd)? {
        return Ok(());
    }

    Err(Error::UserMessage(
        "Refusing to run `vp release` with a dirty git worktree. Commit, stash, or rerun with `--dry-run` first."
            .into(),
    ))
}

/// Builds one package changelog section for the current release.
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

/// Builds the root changelog section summarizing all packages in the release.
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

/// Computes temporary manifest rewrites used during publish preflight and publish execution.
pub(super) fn build_manifest_edits(
    release_plans: &[PackageReleasePlan],
) -> Result<Vec<ManifestEdit>, Error> {
    let release_versions: HashMap<&str, &Version> =
        release_plans.iter().map(|plan| (plan.name.as_str(), &plan.next_version)).collect();
    let mut edits = Vec::with_capacity(release_plans.len());

    for plan in release_plans {
        let updated_contents = build_updated_manifest_contents(plan, &release_versions)?;
        edits.push(ManifestEdit {
            package: plan.name.clone(),
            path: plan.manifest_path.clone(),
            original_contents: plan.manifest_contents.clone(),
            updated_contents,
        });
    }

    Ok(edits)
}

/// Counts how many durable local artifacts a release will update.
pub(super) fn summarize_release_artifacts(
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    include_changelog: bool,
) -> ReleaseArtifactSummary {
    ReleaseArtifactSummary {
        manifest_file_count: manifest_edits.len(),
        changelog_file_count: if include_changelog { release_plans.len() + 1 } else { 0 },
    }
}

/// Builds the final artifact edits that should remain in git after a successful release.
pub(super) fn build_release_artifact_edits(
    workspace_root_path: &AbsolutePath,
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    root_commits: &[CommitInfo],
    release_date: &str,
    include_changelog: bool,
) -> Result<Vec<ReleaseArtifactEdit>, Error> {
    let mut edits = Vec::with_capacity(
        manifest_edits.len() + if include_changelog { release_plans.len() + 1 } else { 0 },
    );

    if include_changelog {
        let root_changelog_path = workspace_root_path.join("CHANGELOG.md");
        let root_section = build_root_changelog_section(release_date, release_plans, root_commits);
        edits.push(build_changelog_artifact_edit(
            root_changelog_path,
            root_section,
            String::from("workspace changelog"),
        )?);
    }

    for edit in manifest_edits {
        edits.push(ReleaseArtifactEdit {
            label: {
                let mut label = String::from("package manifest for ");
                label.push_str(&edit.package);
                label
            },
            path: edit.path.clone(),
            original_contents: Some(edit.original_contents.clone()),
            updated_contents: edit.updated_contents.clone(),
        });
    }

    if include_changelog {
        for plan in release_plans {
            let section =
                build_package_changelog_section(release_date, &plan.next_version, &plan.commits);
            let mut label = String::from("package changelog for ");
            label.push_str(&plan.name);
            edits.push(build_changelog_artifact_edit(plan.changelog_path.clone(), section, label)?);
        }
    }

    Ok(edits)
}

/// Runs native publisher dry-runs while temporarily rewriting manifests.
pub(super) async fn run_publish_preflight(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    options: &ReleaseOptions,
    trusted_publish_context: &TrustedPublishContext,
) -> Result<ExitStatus, Error> {
    if options.skip_publish {
        output::note("Skipping publish preflight because --skip-publish was provided.");
        return Ok(ExitStatus::default());
    }

    output::raw("");
    output::info("Publish preflight:");
    apply_manifest_edits(manifest_edits, false)?;
    let preflight_result = run_publish_preflight_inner(
        package_manager,
        release_plans,
        options,
        trusted_publish_context,
    )
    .await;
    let restore_result = apply_manifest_edits(manifest_edits, true);

    match (preflight_result, restore_result) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(error), Err(restore_error)) => {
            let mut message = String::from(
                "publish preflight failed and vite-plus could not restore package.json files cleanly: ",
            );
            push_display(&mut message, error);
            message.push_str(" | restore error: ");
            push_display(&mut message, restore_error);
            Err(Error::UserMessage(message.into()))
        }
    }
}

/// Runs configured release checks before publish when the release policy requires them.
pub(super) async fn run_release_checks(
    package_manager: &PackageManager,
    workspace_root_path: &AbsolutePath,
    release_plans: &[PackageReleasePlan],
    readiness_report: &ReleaseReadinessReport,
    options: &ReleaseOptions,
) -> Result<ExitStatus, Error> {
    if !options.run_checks {
        output::note("Release checks are disabled for this run.");
        return Ok(ExitStatus::default());
    }

    if readiness_report.workspace_scripts.is_empty() && readiness_report.package_scripts.is_empty()
    {
        return Err(Error::UserMessage(
            "No release checks were detected for this release. Add `build` / `pack` / `prepack` / `prepublishOnly` / `prepare` scripts or configure `vitePlus.release.checkScripts`, then rerun `vp release`. Use `--no-run-checks` only if you intentionally want to skip this safeguard."
                .into(),
        ));
    }

    output::raw("");
    output::info("Running release checks:");

    let workspace_script_names: HashSet<&str> =
        readiness_report.workspace_scripts.iter().map(String::as_str).collect();
    for script in &readiness_report.workspace_scripts {
        raw_progress_line!("workspace script `", script, '`');
        let status = package_manager
            .run_script_command(std::slice::from_ref(script), workspace_root_path)
            .await?;
        if !status.success() {
            return Ok(status);
        }
    }

    let package_scripts: HashMap<&str, &PackageReadiness> = readiness_report
        .package_scripts
        .iter()
        .map(|readiness| (readiness.package.as_str(), readiness))
        .collect();
    for plan in release_plans {
        let Some(readiness) = package_scripts.get(plan.name.as_str()) else {
            continue;
        };

        for script in readiness
            .scripts
            .iter()
            .filter(|script| !workspace_script_names.contains(script.as_str()))
        {
            raw_progress_line!(&plan.name, " script `", script, '`');
            let status = package_manager
                .run_script_command(std::slice::from_ref(script), &plan.package_path)
                .await?;
            if !status.success() {
                return Ok(status);
            }
        }
    }

    output::success("Release checks succeeded.");
    Ok(ExitStatus::default())
}

/// Executes the real publish flow while guaranteeing manifest restoration afterward.
pub(super) async fn publish_packages(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    options: &ReleaseOptions,
    trusted_publish_context: &TrustedPublishContext,
) -> Result<ExitStatus, Error> {
    debug_assert!(
        !options.skip_publish,
        "validate_release_options should reject real releases with --skip-publish"
    );

    output::raw("");
    output::info("Publishing packages:");
    apply_manifest_edits(manifest_edits, false)?;
    let publish_result =
        publish_packages_inner(package_manager, release_plans, options, trusted_publish_context)
            .await;
    let restore_result = apply_manifest_edits(manifest_edits, true);

    match (publish_result, restore_result) {
        (Ok((published_count, status)), Ok(())) => {
            if !status.success() && published_count > 0 {
                let mut message = String::from("Publish stopped after ");
                push_display(&mut message, published_count);
                message.push_str(" package(s) were already published. Local package.json files were restored, so inspect the registry state before retrying.");
                let remaining_packages: Vec<&str> = release_plans[published_count..]
                    .iter()
                    .map(|plan| plan.name.as_str())
                    .collect();
                if !remaining_packages.is_empty() {
                    message.push_str(" Remaining selection: ");
                    push_joined(&mut message, remaining_packages.iter().copied(), ", ");
                    message.push('.');
                    message.push_str(" Rerun with `--projects` narrowed to the remaining packages");
                    let mut unique_versions = release_plans[published_count..]
                        .iter()
                        .map(|plan| plan.next_version.to_string())
                        .collect::<Vec<_>>();
                    unique_versions.sort();
                    unique_versions.dedup();
                    if unique_versions.len() == 1 {
                        message.push_str(" and `--version ");
                        message.push_str(&unique_versions[0]);
                        message.push('`');
                    }
                    message.push('.');
                }
                output::warn(&message);
            }
            Ok(status)
        }
        (Err(error), Ok(())) => Err(error),
        (Ok((_published_count, _status)), Err(restore_error)) => Err(restore_error),
        (Err(error), Err(restore_error)) => {
            let mut message = String::from(
                "publish failed and vite-plus could not restore package.json files cleanly: ",
            );
            push_display(&mut message, error);
            message.push_str(" | restore error: ");
            push_display(&mut message, restore_error);
            Err(Error::UserMessage(message.into()))
        }
    }
}

/// Applies final release artifacts transactionally, rolling back already-written files on error.
pub(super) fn apply_release_artifact_edits(edits: &[ReleaseArtifactEdit]) -> Result<(), Error> {
    let mut applied_count = 0usize;
    for edit in edits {
        if let Err(error) = write_release_artifact_edit(edit) {
            let rollback_result = rollback_applied_release_artifact_edits(&edits[..applied_count]);
            return Err(release_artifact_error("write release artifacts", error, rollback_result));
        }
        applied_count += 1;
    }

    Ok(())
}

/// Rolls back final release artifacts in reverse application order.
pub(super) fn rollback_release_artifact_edits(edits: &[ReleaseArtifactEdit]) -> Result<(), Error> {
    rollback_applied_release_artifact_edits(edits)
}

/// Formats the local release commit message from the package set.
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

/// Stages the final release artifacts.
pub(super) fn git_add_paths(cwd: &AbsolutePath, paths: &[AbsolutePathBuf]) -> Result<(), Error> {
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

/// Creates the local release commit.
pub(super) fn git_commit(cwd: &AbsolutePath, message: &str) -> Result<(), Error> {
    run_git(cwd, ["commit", "-m", message]).map_err(|err| {
        let mut error_message = String::from("create release commit: ");
        push_display(&mut error_message, err);
        Error::UserMessage(error_message.into())
    })
}

/// Creates one local release watermark tag.
pub(super) fn git_tag(cwd: &AbsolutePath, tag_name: &str) -> Result<(), Error> {
    run_git(cwd, ["tag", tag_name]).map_err(|err| {
        let mut message = String::from("create release tag: ");
        push_display(&mut message, err);
        Error::UserMessage(message.into())
    })
}

/// Deletes one local release watermark tag.
pub(super) fn git_delete_tag(cwd: &AbsolutePath, tag_name: &str) -> Result<(), Error> {
    run_git(cwd, ["tag", "-d", tag_name]).map_err(|err| {
        let mut message = String::from("delete release tag: ");
        push_display(&mut message, err);
        Error::UserMessage(message.into())
    })
}

/// Removes tags created by the current release attempt in reverse order.
pub(super) fn rollback_created_git_tags(
    cwd: &AbsolutePath,
    tag_names: &[String],
) -> Result<(), Error> {
    for tag_name in tag_names.iter().rev() {
        git_delete_tag(cwd, tag_name)?;
    }
    Ok(())
}

/// Prepends a generated changelog section while preserving the conventional heading structure.
pub(super) fn prepend_changelog_section(existing: &str, section: &str) -> String {
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

fn build_changelog_artifact_edit(
    path: AbsolutePathBuf,
    section: String,
    label: String,
) -> Result<ReleaseArtifactEdit, Error> {
    let (original_contents, updated_contents) = match fs::read_to_string(&path) {
        Ok(existing) => {
            let updated = prepend_changelog_section(&existing, &section);
            (Some(existing), updated)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            (None, prepend_changelog_heading(&section))
        }
        Err(err) => return Err(err.into()),
    };

    Ok(ReleaseArtifactEdit { label, path, original_contents, updated_contents })
}

async fn run_publish_preflight_inner(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    options: &ReleaseOptions,
    trusted_publish_context: &TrustedPublishContext,
) -> Result<ExitStatus, Error> {
    for plan in release_plans {
        raw_progress_line!("checking ", &plan.name, '@', &plan.next_version);

        let publish_options = PublishCommandOptions {
            dry_run: true,
            tag: resolved_publish_tag(plan, options),
            access: plan.access.as_deref(),
            otp: options.otp.as_deref(),
            provenance: resolved_publish_provenance(plan, trusted_publish_context),
            ..Default::default()
        };
        let status =
            package_manager.run_publish_command(&publish_options, &plan.package_path).await?;
        if !status.success() {
            return Ok(status);
        }
    }

    output::success("Publish preflight succeeded.");
    Ok(ExitStatus::default())
}

async fn publish_packages_inner(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    options: &ReleaseOptions,
    trusted_publish_context: &TrustedPublishContext,
) -> Result<(usize, ExitStatus), Error> {
    let mut published_count = 0usize;

    for plan in release_plans {
        let mut message = String::from("Publishing ");
        message.push_str(&plan.name);
        message.push('@');
        push_display(&mut message, &plan.next_version);
        output::info(&message);

        let publish_options = PublishCommandOptions {
            dry_run: false,
            tag: resolved_publish_tag(plan, options),
            access: plan.access.as_deref(),
            otp: options.otp.as_deref(),
            provenance: resolved_publish_provenance(plan, trusted_publish_context),
            ..Default::default()
        };
        let status =
            package_manager.run_publish_command(&publish_options, &plan.package_path).await?;
        if !status.success() {
            return Ok((published_count, status));
        }

        published_count += 1;
    }

    Ok((published_count, ExitStatus::default()))
}

fn apply_manifest_edits(
    manifest_edits: &[ManifestEdit],
    restore_original: bool,
) -> Result<(), Error> {
    for edit in manifest_edits {
        let contents =
            if restore_original { &edit.original_contents } else { &edit.updated_contents };
        fs::write(&edit.path, contents).map_err(|error| {
            let mut message = String::from("write release manifest for ");
            message.push_str(&edit.package);
            message.push_str(": ");
            push_display(&mut message, error);
            Error::UserMessage(message.into())
        })?;
    }

    Ok(())
}

fn write_release_artifact_edit(edit: &ReleaseArtifactEdit) -> Result<(), Error> {
    fs::write(&edit.path, &edit.updated_contents).map_err(|error| {
        let mut message = String::from("write ");
        message.push_str(&edit.label);
        message.push_str(": ");
        push_display(&mut message, error);
        Error::UserMessage(message.into())
    })
}

fn rollback_applied_release_artifact_edits(edits: &[ReleaseArtifactEdit]) -> Result<(), Error> {
    for edit in edits.iter().rev() {
        restore_release_artifact_edit(edit)?;
    }

    Ok(())
}

fn restore_release_artifact_edit(edit: &ReleaseArtifactEdit) -> Result<(), Error> {
    match &edit.original_contents {
        Some(contents) => fs::write(&edit.path, contents).map_err(|error| {
            let mut message = String::from("restore ");
            message.push_str(&edit.label);
            message.push_str(": ");
            push_display(&mut message, error);
            Error::UserMessage(message.into())
        }),
        None => match fs::remove_file(&edit.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => {
                let mut message = String::from("remove generated ");
                message.push_str(&edit.label);
                message.push_str(": ");
                push_display(&mut message, error);
                Err(Error::UserMessage(message.into()))
            }
        },
    }
}

fn release_artifact_error(
    context: &str,
    error: Error,
    rollback_result: Result<(), Error>,
) -> Error {
    let mut message = String::from(context);
    message.push_str(": ");
    push_display(&mut message, error);
    match rollback_result {
        Ok(()) => {
            message.push_str(". Local release files were rolled back.");
        }
        Err(rollback_error) => {
            message.push_str(" | rollback error: ");
            push_display(&mut message, rollback_error);
        }
    }
    Error::UserMessage(message.into())
}

fn build_updated_manifest_contents(
    plan: &PackageReleasePlan,
    release_versions: &HashMap<&str, &Version>,
) -> Result<String, Error> {
    let current_version = plan.current_version.to_string();
    let next_version = plan.next_version.to_string();
    let mut updated = replace_top_level_string_property(
        &plan.manifest_contents,
        "version",
        &current_version,
        &next_version,
    )?;

    let dependency_updates = collect_dependency_version_updates(plan, release_versions)?;
    if !dependency_updates.is_empty() {
        updated =
            replace_dependency_version_ranges(&updated, &dependency_updates).map_err(|error| {
                let mut message = String::from("update dependency ranges for ");
                message.push_str(&plan.name);
                message.push_str(": ");
                push_display(&mut message, error);
                Error::UserMessage(message.into())
            })?;
    }

    Ok(updated)
}

fn collect_dependency_version_updates(
    plan: &PackageReleasePlan,
    release_versions: &HashMap<&str, &Version>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let mut updates = BTreeMap::new();
    for (section_name, dependencies) in [
        ("dependencies", &plan.manifest.dependencies),
        ("devDependencies", &plan.manifest.dev_dependencies),
        ("peerDependencies", &plan.manifest.peer_dependencies),
        ("optionalDependencies", &plan.manifest.optional_dependencies),
    ] {
        collect_dependency_section_updates(
            &plan.name,
            section_name,
            dependencies,
            release_versions,
            &mut updates,
        )?;
    }
    Ok(updates)
}

fn collect_dependency_section_updates(
    package_name: &str,
    section_name: &str,
    dependencies: &BTreeMap<String, String>,
    release_versions: &HashMap<&str, &Version>,
    updates: &mut BTreeMap<String, BTreeMap<String, String>>,
) -> Result<(), Error> {
    for (dependency_name, spec) in dependencies {
        let Some(next_version) = release_versions.get(dependency_name.as_str()) else {
            continue;
        };
        let Some(updated_spec) = rewrite_released_dependency_spec(
            package_name,
            section_name,
            dependency_name,
            spec,
            next_version,
        )?
        else {
            continue;
        };
        updates
            .entry(section_name.to_string())
            .or_default()
            .insert(dependency_name.clone(), updated_spec);
    }

    Ok(())
}

fn rewrite_released_dependency_spec(
    package_name: &str,
    section_name: &str,
    dependency_name: &str,
    spec: &str,
    next_version: &Version,
) -> Result<Option<String>, Error> {
    if is_publish_protocol_reference(spec) {
        return Ok(None);
    }

    match parse_version_pattern(spec) {
        Ok(VersionPattern::Any) => Ok(None),
        Ok(VersionPattern::Version { prefix, .. }) => {
            let mut updated = String::from(prefix.as_str());
            push_display(&mut updated, next_version);
            Ok(Some(updated))
        }
        Ok(VersionPattern::Token(prefix)) => {
            let mut message = String::from("Package '");
            message.push_str(package_name);
            message.push_str("' depends on released workspace package '");
            message.push_str(dependency_name);
            message.push_str("' via unsupported ");
            message.push_str(section_name);
            message.push_str(" range '");
            message.push_str(spec);
            message.push_str("'. Bare '");
            message.push_str(prefix.as_str());
            message.push_str("' tokens are not supported for publishable internal dependencies.");
            Err(Error::UserMessage(message.into()))
        }
        Err(_) => {
            let mut message = String::from("Package '");
            message.push_str(package_name);
            message.push_str("' depends on released workspace package '");
            message.push_str(dependency_name);
            message.push_str("' via unsupported ");
            message.push_str(section_name);
            message.push_str(" range '");
            message.push_str(spec);
            message.push_str("'. Use `workspace:` or a simple exact/^/~ version.");
            Err(Error::UserMessage(message.into()))
        }
    }
}

fn is_publish_protocol_reference(spec: &str) -> bool {
    spec.contains("workspace:")
        || DIRECT_PUBLISH_PROTOCOL_PREFIXES.iter().any(|prefix| spec.starts_with(prefix))
}
