use super::*;

const DIRECT_PUBLISH_PROTOCOL_PREFIXES: [&str; 6] =
    ["catalog:", "file:", "link:", "portal:", "patch:", "jsr:"];

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
        "Refusing to run `vp release` with a dirty git worktree. Commit, stash, or rerun with `--dry-run` first."
            .into(),
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

pub(super) async fn run_publish_preflight(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    options: &ReleaseOptions,
) -> Result<ExitStatus, Error> {
    if options.skip_publish {
        output::note("Skipping publish preflight because --skip-publish was provided.");
        return Ok(ExitStatus::SUCCESS);
    }

    output::raw("");
    output::info("Publish preflight:");
    apply_manifest_edits(manifest_edits, false)?;
    let preflight_result =
        run_publish_preflight_inner(package_manager, release_plans, options).await;
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

pub(super) async fn publish_packages(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    manifest_edits: &[ManifestEdit],
    options: &ReleaseOptions,
) -> Result<ExitStatus, Error> {
    debug_assert!(
        !options.skip_publish,
        "validate_release_options should reject real releases with --skip-publish"
    );

    output::raw("");
    output::info("Publishing packages:");
    apply_manifest_edits(manifest_edits, false)?;
    let publish_result = publish_packages_inner(package_manager, release_plans, options).await;
    let restore_result = apply_manifest_edits(manifest_edits, true);

    match (publish_result, restore_result) {
        (Ok((published_count, status)), Ok(())) => {
            if !status.success() && published_count > 0 {
                let mut message = String::from("Publish stopped after ");
                push_display(&mut message, published_count);
                message.push_str(" package(s) were already published. Local package.json files were restored, so inspect the registry state before retrying.");
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

pub(super) fn write_manifest_contents(path: &AbsolutePath, contents: &str) -> Result<(), Error> {
    fs::write(path, contents)?;
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

async fn run_publish_preflight_inner(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    options: &ReleaseOptions,
) -> Result<ExitStatus, Error> {
    for plan in release_plans {
        let mut message = String::from("  checking ");
        message.push_str(&plan.name);
        message.push('@');
        push_display(&mut message, &plan.next_version);
        output::raw(&message);

        let publish_options = PublishCommandOptions {
            dry_run: true,
            tag: resolved_publish_tag(plan, options),
            access: plan.access.as_deref(),
            otp: options.otp.as_deref(),
            ..Default::default()
        };
        let status =
            package_manager.run_publish_command(&publish_options, &plan.package_path).await?;
        if !status.success() {
            return Ok(status);
        }
    }

    output::success("Publish preflight succeeded.");
    Ok(ExitStatus::SUCCESS)
}

async fn publish_packages_inner(
    package_manager: &PackageManager,
    release_plans: &[PackageReleasePlan],
    options: &ReleaseOptions,
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
            ..Default::default()
        };
        let status =
            package_manager.run_publish_command(&publish_options, &plan.package_path).await?;
        if !status.success() {
            return Ok((published_count, status));
        }

        published_count += 1;
    }

    Ok((published_count, ExitStatus::SUCCESS))
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
