use super::*;

pub(super) fn print_release_plan(release_plans: &[PackageReleasePlan], options: &ReleaseOptions) {
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
            dry_run: true,
            tag: resolved_publish_tag(plan, options),
            access: plan.access.as_deref(),
            otp: options.otp.as_deref(),
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
