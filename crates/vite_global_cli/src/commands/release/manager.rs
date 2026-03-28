use super::*;

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

pub(super) async fn execute_release(
    cwd: AbsolutePathBuf,
    options: ReleaseOptions,
) -> Result<ExitStatus, Error> {
    ReleaseManager::new(cwd, options).run().await
}
