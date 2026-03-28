//! Release workflow orchestration.
//!
//! This module is the top-level state machine for `vp release`. It intentionally owns the
//! sequence of high-level phases:
//!
//! 1. detect workspace/package-manager context
//! 2. build a release plan
//! 3. present readiness/security information
//! 4. run dry-run preflight or the real publish flow
//! 5. finalize local git state
//!
//! Lower-level details such as git history inspection, file rewrites, and rendering live in
//! neighboring modules. Keeping the phase transitions here makes it easier to audit the control
//! flow and the fail-closed security checks in one place.

use super::*;

/// Workspace-scoped data loaded once before package selection and release planning.
struct ReleaseWorkspace {
    package_manager: PackageManager,
    workspace_root_path: AbsolutePathBuf,
    workspace_manifest: Option<PackageManifest>,
    package_graph: WorkspacePackageGraph,
    workspace_packages: Vec<WorkspacePackage>,
}

/// Immutable release plan plus the data needed to execute it.
///
/// This type sits at the boundary between planning and execution. After it is constructed, the
/// remaining workflow should not need to rediscover workspace state.
struct PreparedRelease {
    package_manager: PackageManager,
    workspace_root_path: AbsolutePathBuf,
    workspace_manifest: Option<PackageManifest>,
    release_plans: Vec<PackageReleasePlan>,
    manifest_edits: Vec<ManifestEdit>,
    root_commits: Vec<CommitInfo>,
}

/// Coordinator for one `vp release` invocation.
///
/// The manager owns runtime-only context such as the current working directory and detected
/// trusted-publishing environment, then delegates specialized work to sibling modules.
struct ReleaseManager {
    cwd: AbsolutePathBuf,
    options: ReleaseOptions,
    trusted_publish_context: TrustedPublishContext,
}

impl ReleaseManager {
    /// Constructs a manager and snapshots the current trusted-publishing environment.
    fn new(cwd: AbsolutePathBuf, options: ReleaseOptions) -> Self {
        Self { cwd, options, trusted_publish_context: TrustedPublishContext::detect() }
    }

    /// Executes the full release workflow, returning early when no packages need a release.
    async fn run(self) -> Result<ExitStatus, Error> {
        validate_release_options(&self.options)?;
        validate_trusted_publish_context(&self.options, &self.trusted_publish_context)?;

        let workspace = self.load_workspace().await?;
        let Some(release) = self.prepare_release(workspace)? else {
            return Ok(ExitStatus::SUCCESS);
        };

        let readiness_report = self.present_release(&release)?;
        self.validate_release_security_posture(&release)?;
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

    /// Builds per-package release plans and the root changelog commit set.
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
                publish_provenance: package.manifest.publish_config.provenance,
                repository_url: package.manifest.repository_url().map(ToOwned::to_owned),
                protocol_summary: package.manifest.dependency_protocol_summary(),
                tag_name: package_tag_name(&package.name, &next_version),
                scripts: package.manifest.scripts.keys().cloned().collect(),
                check_scripts: package.manifest.vite_plus.release.check_scripts.clone(),
            });
        }

        Ok((release_plans, root_commits))
    }

    /// Renders the user-facing release/readiness summaries and returns the computed report.
    fn present_release(&self, release: &PreparedRelease) -> Result<ReleaseReadinessReport, Error> {
        print_release_plan(&release.release_plans, &self.options);
        if self.options.first_release {
            let mut guidance = collect_first_publish_guidance(
                &release.workspace_root_path,
                &release.release_plans,
            );
            ensure_first_publish_workflow_template(
                &release.workspace_root_path,
                release.package_manager.client,
                &mut guidance,
            )?;
            print_first_publish_guidance(&guidance, &self.options);
        }
        let readiness_report = collect_release_readiness_report(
            release.workspace_manifest.as_ref(),
            &release.release_plans,
            &self.options,
            &self.trusted_publish_context,
        );
        print_release_readiness_report(&readiness_report);
        Ok(readiness_report)
    }

    /// Rejects package-manager/protocol combinations that are known to publish unsafe manifests.
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

    /// Enforces hardened release policy before any real publish happens.
    ///
    /// This gate is intentionally fail-closed. If a real release cannot satisfy provenance and
    /// repository metadata expectations, the workflow should stop before touching the registry.
    fn validate_release_security_posture(&self, release: &PreparedRelease) -> Result<(), Error> {
        if self.options.dry_run {
            return Ok(());
        }

        let mut issues = Vec::new();
        let provenance_disabled: Vec<&str> = release
            .release_plans
            .iter()
            .filter(|plan| matches!(plan.publish_provenance, Some(false)))
            .map(|plan| plan.name.as_str())
            .collect();
        if !provenance_disabled.is_empty() {
            let mut message = String::from(
                "`publishConfig.provenance = false` is not allowed for hardened releases: ",
            );
            push_joined(&mut message, provenance_disabled.into_iter(), ", ");
            issues.push(message);
        }

        let missing_repository: Vec<&str> = release
            .release_plans
            .iter()
            .filter(|plan| plan.repository_url.is_none())
            .map(|plan| plan.name.as_str())
            .collect();
        if !missing_repository.is_empty() {
            let mut message =
                String::from("Repository metadata is required for trusted publishing provenance: ");
            push_joined(&mut message, missing_repository.into_iter(), ", ");
            issues.push(message);
        }

        if let Some(expected_repository) = self.trusted_publish_context.repository.as_deref() {
            let mismatched_repository: Vec<&str> = release
                .release_plans
                .iter()
                .filter(|plan| {
                    plan.repository_url
                        .as_deref()
                        .and_then(parse_github_repo_slug)
                        .map_or(true, |slug| slug != expected_repository)
                })
                .map(|plan| plan.name.as_str())
                .collect();
            if !mismatched_repository.is_empty() {
                let mut message = String::from(
                    "Package `repository` metadata must match the trusted publishing repository `",
                );
                message.push_str(expected_repository);
                message.push_str("`: ");
                push_joined(&mut message, mismatched_repository.into_iter(), ", ");
                issues.push(message);
            }
        }

        if issues.is_empty() {
            return Ok(());
        }

        let mut message = String::from("Security policy rejected this release: ");
        push_joined(&mut message, issues.iter().map(String::as_str), " | ");
        Err(Error::UserMessage(message.into()))
    }

    /// Executes a no-write preview of the release, including publish preflight when possible.
    async fn run_dry_run(&self, release: &PreparedRelease) -> Result<ExitStatus, Error> {
        let artifact_summary = summarize_release_artifacts(
            &release.release_plans,
            &release.manifest_edits,
            self.options.changelog,
        );
        print_dry_run_actions(
            &release.release_plans,
            &release.package_manager,
            &self.options,
            artifact_summary,
            &self.trusted_publish_context,
        );

        let publish_status = if self.options.skip_publish {
            DryRunPublishStatus::SkippedByOption
        } else if is_clean_git_worktree(&release.workspace_root_path)? {
            let status = run_publish_preflight(
                &release.package_manager,
                &release.release_plans,
                &release.manifest_edits,
                &self.options,
                &self.trusted_publish_context,
            )
            .await?;
            if !status.success() {
                print_dry_run_summary(
                    artifact_summary,
                    DryRunPublishStatus::Failed,
                    &self.options,
                    release.release_plans.len(),
                    &self.trusted_publish_context,
                );
                return Ok(status);
            }
            DryRunPublishStatus::Succeeded
        } else {
            output::warn(
                "Skipping native publish dry-run because the git worktree is dirty. Rerun from a clean tree for full publish preflight.",
            );
            DryRunPublishStatus::SkippedDirtyWorktree
        };

        print_dry_run_summary(
            artifact_summary,
            publish_status,
            &self.options,
            release.release_plans.len(),
            &self.trusted_publish_context,
        );

        Ok(ExitStatus::SUCCESS)
    }

    /// Executes the real publish flow and finalizes local release artifacts on success.
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
            &self.trusted_publish_context,
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
            &self.trusted_publish_context,
        )
        .await?;
        if !publish_status.success() {
            return Ok(publish_status);
        }

        let artifact_summary = summarize_release_artifacts(
            &release.release_plans,
            &release.manifest_edits,
            self.options.changelog,
        );
        let release_date = Local::now().format("%Y-%m-%d").to_string();
        let artifact_edits = build_release_artifact_edits(
            &release.workspace_root_path,
            &release.release_plans,
            &release.manifest_edits,
            &release.root_commits,
            &release_date,
            self.options.changelog,
        )?;
        let changed_files: Vec<AbsolutePathBuf> =
            artifact_edits.iter().map(|edit| edit.path.clone()).collect();
        apply_release_artifact_edits(&artifact_edits)?;

        let commit_message =
            self.options.git_commit.then(|| release_commit_message(&release.release_plans));
        if self.options.git_commit {
            if let Err(error) = git_add_paths(&release.workspace_root_path, &changed_files) {
                return Err(artifact_rollback_error(
                    "stage release changes",
                    error,
                    &artifact_edits,
                    true,
                ));
            }
            let commit_message = commit_message.as_deref().expect("commit message should exist");
            if let Err(error) = git_commit(&release.workspace_root_path, commit_message) {
                return Err(artifact_rollback_error(
                    "create release commit",
                    error,
                    &artifact_edits,
                    true,
                ));
            }
            let mut message = String::from("Created release commit: ");
            message.push_str(commit_message);
            output::success(&message);
        }

        let mut created_tag_names = Vec::new();
        if self.options.git_tag {
            for plan in &release.release_plans {
                if let Err(error) = git_tag(&release.workspace_root_path, &plan.tag_name) {
                    return Err(tag_rollback_error(
                        &release.workspace_root_path,
                        error,
                        &created_tag_names,
                    ));
                }
                created_tag_names.push(plan.tag_name.clone());
                let mut message = String::from("Created git tag ");
                message.push_str(&plan.tag_name);
                output::success(&message);
            }
        }

        print_release_completion_summary(
            artifact_summary,
            release.release_plans.len(),
            commit_message.as_deref(),
            created_tag_names.len(),
            &self.trusted_publish_context,
        );
        Ok(ExitStatus::SUCCESS)
    }
}

/// Formats an execution error while rolling back final local release artifacts when possible.
fn artifact_rollback_error(
    context: &str,
    error: Error,
    artifact_edits: &[ReleaseArtifactEdit],
    mention_git_index: bool,
) -> Error {
    let rollback_result = rollback_release_artifact_edits(artifact_edits);
    let mut message = String::from(context);
    message.push_str(": ");
    push_display(&mut message, error);
    match rollback_result {
        Ok(()) => {
            message.push_str(". Local release files were rolled back.");
            if mention_git_index {
                message
                    .push_str(" If git staged any paths before the failure, inspect `git status`.");
            }
        }
        Err(rollback_error) => {
            message.push_str(" | rollback error: ");
            push_display(&mut message, rollback_error);
        }
    }
    Error::UserMessage(message.into())
}

/// Formats a tag-creation error and removes any tags created by the current run.
fn tag_rollback_error(
    workspace_root_path: &AbsolutePath,
    error: Error,
    created_tag_names: &[String],
) -> Error {
    let rollback_result = rollback_created_git_tags(workspace_root_path, created_tag_names);
    let mut message = String::from("create release tag: ");
    push_display(&mut message, error);
    match rollback_result {
        Ok(()) => {
            if !created_tag_names.is_empty() {
                message.push_str(". Previously created release tags from this run were removed.");
            }
            message.push_str(
                " The release commit remains in git; inspect it before retrying tag creation.",
            );
        }
        Err(rollback_error) => {
            message.push_str(" | tag rollback error: ");
            push_display(&mut message, rollback_error);
        }
    }
    Error::UserMessage(message.into())
}

/// Public entrypoint used by the parent `release` command module.
pub(super) async fn execute_release(
    cwd: AbsolutePathBuf,
    options: ReleaseOptions,
) -> Result<ExitStatus, Error> {
    ReleaseManager::new(cwd, options).run().await
}
