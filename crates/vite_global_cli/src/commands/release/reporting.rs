//! User-facing release reports, summaries, and prompts.
//!
//! This module is responsible for taking already-computed release state and turning it into
//! operator-facing output:
//!
//! - planned release listings
//! - readiness/security summaries
//! - dry-run action previews
//! - completion summaries and confirmation prompts
//!
//! Keeping reporting here helps the rest of the release flow stay focused on planning,
//! validation, and mutation rather than line-by-line terminal formatting.
//!
//! The tiny macros below act as a purpose-built output DSL. They keep indentation and
//! key/value separators declarative without paying the readability cost of hand-assembling
//! `"  ..."` / `"    - ..."` strings at every callsite.

use super::*;

const ITEM_INDENT: &str = "  ";
const DETAIL_INDENT: &str = "    ";
const BULLET_INDENT: &str = "    - ";
const KEY_VALUE_SEPARATOR: &str = ": ";
const LIST_SEPARATOR: &str = ", ";

macro_rules! info_section {
    ($heading:expr $(,)?) => {{
        output::raw("");
        output::info($heading);
    }};
}

macro_rules! raw_prefixed_line {
    ($prefix:expr, $($part:expr),+ $(,)?) => {{
        let mut line = String::from($prefix);
        $(push_display(&mut line, $part);)+
        output::raw(&line);
    }};
}

macro_rules! raw_item_line {
    ($($part:expr),+ $(,)?) => {
        raw_prefixed_line!(ITEM_INDENT, $($part),+);
    };
}

macro_rules! raw_detail_line {
    ($($part:expr),+ $(,)?) => {
        raw_prefixed_line!(DETAIL_INDENT, $($part),+);
    };
}

macro_rules! raw_bullet_line {
    ($($part:expr),+ $(,)?) => {
        raw_prefixed_line!(BULLET_INDENT, $($part),+);
    };
}

macro_rules! raw_item_kv {
    ($label:expr, $value:expr $(,)?) => {
        raw_item_line!($label, KEY_VALUE_SEPARATOR, $value);
    };
}

macro_rules! raw_detail_kv {
    ($label:expr, $value:expr $(,)?) => {
        raw_detail_line!($label, KEY_VALUE_SEPARATOR, $value);
    };
}

macro_rules! raw_joined_kv_line {
    ($prefix:expr, $label:expr, $values:expr $(,)?) => {{
        let mut line = String::from($prefix);
        line.push_str($label);
        line.push_str(KEY_VALUE_SEPARATOR);
        push_joined(&mut line, $values, LIST_SEPARATOR);
        output::raw(&line);
    }};
}

macro_rules! raw_item_joined_kv {
    ($label:expr, $values:expr $(,)?) => {
        raw_joined_kv_line!(ITEM_INDENT, $label, $values);
    };
}

macro_rules! raw_detail_joined_kv {
    ($label:expr, $values:expr $(,)?) => {
        raw_joined_kv_line!(DETAIL_INDENT, $label, $values);
    };
}

macro_rules! raw_bullet_joined_kv {
    ($label:expr, $values:expr $(,)?) => {
        raw_joined_kv_line!(BULLET_INDENT, $label, $values);
    };
}

/// Prints the high-level version plan for all selected packages.
pub(super) fn print_release_plan(release_plans: &[PackageReleasePlan], options: &ReleaseOptions) {
    output::info("Planned releases:");
    for plan in release_plans {
        raw_item_line!(
            &plan.name,
            ' ',
            &plan.current_version,
            " -> ",
            &plan.next_version,
            " (",
            plan.level.as_str(),
            ')',
        );
        if plan.known_names.len() > 1 {
            raw_detail_joined_kv!(
                "previous names",
                plan.known_names[1..].iter().map(String::as_str),
            );
        }
        if !plan.retired_names.is_empty() {
            raw_detail_joined_kv!(
                "retired package names",
                plan.retired_names.iter().map(String::as_str),
            );
        }
    }

    if options.dry_run {
        output::note("Dry run enabled; no files will be changed.");
    }
    if !options.changelog {
        output::note("Changelog generation is disabled for this run.");
    }
    if let Some(version) = options.version.as_deref() {
        let mut line = String::from("Target version overridden with --version ");
        line.push_str(version);
        line.push('.');
        output::note(&line);
    }
    if !options.dry_run && options.skip_publish {
        output::note("Publishing disabled for this run.");
    }
    if options.run_checks {
        output::note("Release checks will run before publish.");
    } else {
        output::note("Release checks are disabled for this run.");
    }
}

/// Builds the readiness report shown before confirmation or dry-run completion.
///
/// The report intentionally mixes ergonomics and policy:
///
/// - script discovery helps maintainers understand what checks exist
/// - trusted-publishing details explain what environment was detected
/// - warnings surface conditions that would weaken or block a hardened release
pub(super) fn collect_release_readiness_report(
    workspace_manifest: Option<&PackageManifest>,
    release_plans: &[PackageReleasePlan],
    options: &ReleaseOptions,
    trusted_publish_context: &TrustedPublishContext,
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

    report.trusted_publish = TrustedPublishReadiness {
        context: trusted_publish_context.clone(),
        packages_with_provenance_disabled: release_plans
            .iter()
            .filter(|plan| matches!(plan.publish_provenance, Some(false)))
            .map(|plan| plan.name.clone())
            .collect(),
        uses_legacy_otp: options.otp.is_some(),
    };
    if !report.trusted_publish.context.supports_publish_provenance() {
        report.warnings.push(
            "This environment cannot produce the npm provenance attestations required by the hardened release policy. Use `vp release --dry-run` locally and publish from GitHub Actions on a GitHub-hosted runner or from GitLab CI shared runners."
                .into(),
        );
    }
    if !report.trusted_publish.packages_with_provenance_disabled.is_empty() {
        let mut warning = String::from(
            "`publishConfig.provenance = false` is not allowed for hardened releases: ",
        );
        push_joined(
            &mut warning,
            report.trusted_publish.packages_with_provenance_disabled.iter().map(String::as_str),
            ", ",
        );
        report.warnings.push(warning);
    }
    let packages_missing_repository: Vec<&str> = release_plans
        .iter()
        .filter(|plan| plan.repository_url.is_none())
        .map(|plan| plan.name.as_str())
        .collect();
    if !packages_missing_repository.is_empty() {
        let mut warning =
            String::from("Trusted publishing provenance requires `repository` metadata: ");
        push_joined(&mut warning, packages_missing_repository.into_iter(), ", ");
        report.warnings.push(warning);
    }

    report
}

/// Renders the readiness report in a compact terminal-friendly form.
pub(super) fn print_release_readiness_report(report: &ReleaseReadinessReport) {
    info_section!("Pre-release readiness:");

    if !report.workspace_scripts.is_empty() {
        raw_item_joined_kv!(
            "workspace scripts",
            report.workspace_scripts.iter().map(String::as_str)
        );
    }

    if report.package_scripts.is_empty() {
        raw_item_kv!("package scripts", "none detected");
    } else {
        raw_item_line!("package scripts:");
        for package in &report.package_scripts {
            raw_bullet_joined_kv!(&package.package, package.scripts.iter().map(String::as_str));
        }
    }

    raw_item_line!("trusted publishing:");
    raw_detail_kv!("environment", report.trusted_publish.context.environment_summary());
    if let Some(repository) = report.trusted_publish.context.repository.as_deref() {
        raw_detail_kv!("repository", repository);
    }
    if let Some(workflow_name) = report.trusted_publish.context.workflow_name.as_deref() {
        raw_detail_kv!("workflow", workflow_name);
    }
    if let Some(workflow_path) = report.trusted_publish.context.workflow_path() {
        raw_detail_kv!("workflow file", workflow_path);
    }
    if report.trusted_publish.packages_with_provenance_disabled.is_empty() {
        raw_detail_kv!(
            "provenance",
            if report.trusted_publish.context.supports_trusted_publishing() {
                "enabled by default for real releases from trusted-publishing CI"
            } else {
                "will be enabled automatically when the same release runs from trusted-publishing CI"
            },
        );
    } else {
        let mut line = String::from(DETAIL_INDENT);
        line.push_str("provenance");
        line.push_str(KEY_VALUE_SEPARATOR);
        line.push_str("disabled via `publishConfig.provenance = false` for ");
        push_joined(
            &mut line,
            report.trusted_publish.packages_with_provenance_disabled.iter().map(String::as_str),
            LIST_SEPARATOR,
        );
        output::raw(&line);
    }
    raw_detail_kv!(
        "interactive fallback",
        "prefer npm passkey/security-key auth; use `--otp` only for legacy TOTP fallback",
    );

    output::note(if report.workspace_scripts.is_empty() && report.package_scripts.is_empty() {
        "No release checks were detected automatically for this run."
    } else {
        "Release checks can run from these detected scripts; real releases do so by default and dry-runs can opt in with `--run-checks`."
    });
    output::note("Review this summary, then confirm to continue.");
    if !report.trusted_publish.context.supports_trusted_publishing() {
        output::note(
            "Local dry-runs validate packaging and publish command shape, but OIDC auth and trusted-publishing provenance are only exercised from CI.",
        );
    }
    if matches!(
        report.trusted_publish.context.provider,
        Some(TrustedPublishProvider::GitHubActions)
    ) && matches!(
        report.trusted_publish.context.runner_environment,
        TrustedPublishRunnerEnvironment::SelfHosted
    ) {
        output::warn(
            "GitHub Actions self-hosted runners are not supported by npm trusted publishing.",
        );
    }
    if report.trusted_publish.uses_legacy_otp {
        output::warn(
            "`--otp` is a legacy TOTP path. Prefer trusted publishing first, then passkey/security-key auth for any interactive fallback.",
        );
    }

    for warning in &report.warnings {
        output::warn(warning);
    }
}

/// Prints the concrete actions a dry-run would perform.
pub(super) fn print_dry_run_actions(
    release_plans: &[PackageReleasePlan],
    package_manager: &PackageManager,
    options: &ReleaseOptions,
    artifact_summary: ReleaseArtifactSummary,
    trusted_publish_context: &TrustedPublishContext,
) {
    let commit_message = release_commit_message(release_plans);
    let mut line = String::from("Would update ");
    push_display(&mut line, artifact_summary.total_file_count());
    line.push_str(" release file(s) (");
    push_display(&mut line, artifact_summary.manifest_file_count);
    line.push_str(" manifests");
    if artifact_summary.changelog_file_count > 0 {
        line.push_str(", ");
        push_display(&mut line, artifact_summary.changelog_file_count);
        line.push_str(" changelogs");
    }
    line.push(')');
    output::note(&line);
    if !options.changelog {
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
    if options.run_checks {
        output::note("Would run detected release checks before publish.");
    } else {
        output::note("Would skip release checks for this run.");
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
            provenance: resolved_publish_provenance(plan, trusted_publish_context),
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

/// Prints the final dry-run summary after optional publish preflight completes.
pub(super) fn print_dry_run_summary(
    artifact_summary: ReleaseArtifactSummary,
    publish_status: DryRunPublishStatus,
    options: &ReleaseOptions,
    package_count: usize,
    trusted_publish_context: &TrustedPublishContext,
) {
    info_section!("Dry-run summary:");

    raw_item_kv!("packages planned", package_count);

    let mut line = String::from(ITEM_INDENT);
    line.push_str("files covered");
    line.push_str(KEY_VALUE_SEPARATOR);
    push_display(&mut line, artifact_summary.total_file_count());
    line.push_str(" total (");
    push_display(&mut line, artifact_summary.manifest_file_count);
    line.push_str(" manifests");
    if artifact_summary.changelog_file_count > 0 {
        line.push_str(", ");
        push_display(&mut line, artifact_summary.changelog_file_count);
        line.push_str(" changelogs");
    }
    line.push(')');
    output::raw(&line);

    let mut line = String::from(ITEM_INDENT);
    line.push_str("native publish check");
    line.push_str(KEY_VALUE_SEPARATOR);
    line.push_str(match publish_status {
        DryRunPublishStatus::SkippedByOption => "skipped by --skip-publish",
        DryRunPublishStatus::SkippedDirtyWorktree => "skipped because the git worktree was dirty",
        DryRunPublishStatus::Failed => "failed",
        DryRunPublishStatus::Succeeded => "succeeded",
    });
    output::raw(&line);

    raw_item_kv!("trusted publish target", trusted_publish_context.environment_summary(),);
    raw_item_kv!(
        "interactive fallback auth",
        "passkey/security-key preferred; `--otp` remains legacy-only",
    );

    raw_item_line!("rollback coverage:");
    raw_bullet_line!(
        "temporary package.json rewrites are restored after publish checks and publish"
    );
    raw_bullet_line!("final release files are applied transactionally before the release commit");
    if options.git_tag {
        raw_bullet_line!("partially created local release tags are removed if tag creation fails");
    }
    if !trusted_publish_context.supports_trusted_publishing() {
        raw_bullet_line!(
            "OIDC auth and provenance are not exercised in this local preview; rerun from CI for the full trusted-publish path"
        );
    }
}

/// Prints the final summary after a real release succeeds locally.
pub(super) fn print_release_completion_summary(
    artifact_summary: ReleaseArtifactSummary,
    package_count: usize,
    commit_message: Option<&str>,
    created_tag_count: usize,
    trusted_publish_context: &TrustedPublishContext,
) {
    info_section!("Release completion summary:");

    raw_item_kv!("published packages", package_count);

    let mut line = String::from(ITEM_INDENT);
    line.push_str("files updated");
    line.push_str(KEY_VALUE_SEPARATOR);
    push_display(&mut line, artifact_summary.total_file_count());
    line.push_str(" total (");
    push_display(&mut line, artifact_summary.manifest_file_count);
    line.push_str(" manifests");
    if artifact_summary.changelog_file_count > 0 {
        line.push_str(", ");
        push_display(&mut line, artifact_summary.changelog_file_count);
        line.push_str(" changelogs");
    }
    line.push(')');
    output::raw(&line);

    raw_item_kv!("release commit", commit_message.unwrap_or("skipped"));
    raw_item_kv!("git tags created", created_tag_count);
    raw_item_kv!("trusted publishing", trusted_publish_context.environment_summary());
    raw_item_kv!(
        "manual fallback auth",
        "passkey/security-key preferred; legacy `--otp` should rarely be needed",
    );

    raw_item_line!("rollback coverage:");
    raw_bullet_line!(
        "publish-time package.json rewrites were restored before finalizing local files"
    );
    raw_bullet_line!("final release files were written transactionally before git operations");
    if created_tag_count > 0 {
        raw_bullet_line!(
            "local release tags can be rolled back if a later tag creation step fails"
        );
        output::note(
            "Push the new release tags to origin so future `vp release` runs can reuse the published watermark.",
        );
    }
}

/// Prompts the operator to confirm the release plan.
///
/// The confirmation step lives in reporting rather than orchestration so that the entire
/// terminal interaction remains in one place.
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

    info_section!("Release summary:");
    raw_item_kv!("packages", release_plans.len());
    raw_item_kv!("publish", if options.skip_publish { "no" } else { "yes" });
    raw_item_kv!("changelog", if options.changelog { "yes" } else { "no" });
    raw_item_kv!("git commit", if options.git_commit { "yes" } else { "no" });
    raw_item_kv!("git tags", if options.git_tag { "yes" } else { "no" });
    raw_item_kv!("release checks", if options.run_checks { "yes" } else { "no" });
    raw_item_kv!(
        "trusted publishing",
        readiness_report.trusted_publish.context.environment_summary(),
    );
    raw_item_kv!("prerelease tag", options.preid.as_deref().unwrap_or("stable"));
    if let Some(version) = options.version.as_deref() {
        raw_item_kv!("version override", version);
    }
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

/// Collects scripts worth surfacing in the readiness summary for one scope.
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
