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
const DEFAULT_PUBLISH_WORKFLOW_PATH: &str = ".github/workflows/publish.yml";
const PUBLISH_WORKFLOW_TEMPLATE: &str = include_str!("templates/publish.yml");
const PACKAGE_MANAGER_SETUP_TOKEN: &str = "__PACKAGE_MANAGER_SETUP__";
const INSTALL_COMMAND_TOKEN: &str = "__INSTALL_COMMAND__";
const RELEASE_COMMAND_TOKEN: &str = "__RELEASE_COMMAND__";
const FIRST_RELEASE_COMMAND_TOKEN: &str = "__FIRST_RELEASE_COMMAND__";
const COREPACK_SETUP_STEP: &str = "      - name: Enable Corepack\n        run: corepack enable\n";
const BUN_SETUP_STEP: &str = "      - name: Setup Bun\n        uses: oven-sh/setup-bun@v2\n";

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
                "Commit a GitHub Actions publish workflow that runs on a GitHub-hosted runner.",
                [
                    kv_borrowed!("Workflow file", &guidance.workflow_path),
                    when_text!(
                        guidance.workflow_template_created,
                        "A starter workflow was scaffolded locally from the built-in template. Review it before committing.",
                    ),
                    kv_static!("Trigger", "`workflow_dispatch`"),
                    maybe_kv_owned!(
                        "Suggested dispatch ref",
                        guidance.dispatch_ref_hint.as_deref(),
                        |dispatch_ref| render_inline_code(dispatch_ref)
                    ),
                    text!(
                        "After review, add any automatic push trigger using the branch or tag rule that matches your release boundary.",
                    ),
                    kv_static!(
                        "Required workflow permissions",
                        "`contents: write` and `id-token: write`",
                    ),
                    text!(
                        "Push the generated release tags back to origin so the release watermark survives CI.",
                    ),
                ],
            ),
            step!(
                "Configure npm Trusted Publishing for each package you are releasing.",
                [
                    Some(match guidance.github_repo.as_deref() {
                        Some(repo) => ChecklistLine::key_value_owned("Repository", render_inline_code(repo)),
                        None => ChecklistLine::key_value_static("Repository", "`<owner>/<repo>`"),
                    }),
                    kv_owned!(
                        "Workflow filename in npm",
                        render_inline_code(workflow_filename(&guidance.workflow_path)),
                    ),
                    text!("npm requires the repository and workflow values to match exactly."),
                    text!("Trusted publishing currently works for public npm packages and scopes."),
                    text!("For any manual maintainer fallback, prefer npm passkeys/security-key 2FA instead of TOTP codes or long-lived publish tokens."),
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
                        "Trusted publish from GitHub Actions",
                        render_release_command(options, false, false),
                    ),
                    text!(
                        "Trusted publishing covers publish itself. If CI also installs private packages, use a separate read-only npm token for install steps.",
                    ),
                    text!(
                        "If you ever need an interactive fallback outside CI, prefer a passkey/security-key prompt over `--otp`.",
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
    let dispatch_ref_hint = detect_dispatch_ref_hint(cwd);
    let workflow_path = find_release_workflow_path(cwd);

    let mut guidance = FirstPublishGuidance {
        github_repo: github_repo.clone(),
        dispatch_ref_hint,
        workflow_path,
        ..Default::default()
    };

    for plan in release_plans {
        if plan.name.starts_with('@') && plan.access.as_deref() != Some("public") {
            guidance.scoped_packages_missing_public_access.push(plan.name.clone());
        }

        match plan.repository_url.as_deref() {
            Some(repository_url) => {
                if let Some(expected_repo) = github_repo.as_deref()
                    && parse_github_repo_slug(repository_url).as_deref() != Some(expected_repo)
                {
                    guidance.packages_mismatched_repository.push(plan.name.clone());
                }
            }
            None => guidance.packages_missing_repository.push(plan.name.clone()),
        }
    }

    guidance
}

/// Scaffolds a trusted-publishing workflow template when the repository has no release workflow.
///
/// Existing `publish.*` / `release.*` workflows always win. The scaffold is only written when the
/// repository has no obvious workflow at all, which avoids surprising users with duplicate jobs.
pub(super) fn ensure_first_publish_workflow_template(
    cwd: &AbsolutePath,
    package_manager: PackageManagerType,
    guidance: &mut FirstPublishGuidance,
) -> Result<(), Error> {
    if find_existing_release_workflow_path(cwd).is_some() {
        return Ok(());
    }

    let workflow_path = default_publish_workflow_path();
    let workflows_dir = cwd.join(".github/workflows");
    fs::create_dir_all(workflows_dir.as_path()).map_err(|error| {
        let mut message = String::from("create .github/workflows directory: ");
        push_display(&mut message, error);
        Error::UserMessage(message.into())
    })?;

    let rendered = render_publish_workflow_template(package_manager);
    let workflow_file_path = cwd.join(workflow_path);
    fs::write(&workflow_file_path, rendered).map_err(|error| {
        let mut message = String::from("write ");
        message.push_str(workflow_path);
        message.push_str(": ");
        push_display(&mut message, error);
        Error::UserMessage(message.into())
    })?;

    guidance.workflow_path = workflow_path.to_owned();
    guidance.workflow_template_created = true;

    let mut message = String::from("Scaffolded ");
    message.push_str(workflow_path);
    message
        .push_str(" from the built-in trusted-publishing template. Review it before committing.");
    output::success(&message);

    Ok(())
}

/// Renders the first-publish checklist using the declarative checklist DSL above.
///
/// Keeping the checklist materialization next to this callsite makes the overall flow easy to
/// inspect, while `print_checklist` keeps the indentation and numbering details centralized.
pub(super) fn print_first_publish_guidance(
    guidance: &FirstPublishGuidance,
    options: &ReleaseOptions,
) {
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
    if let Some(version) = options.version.as_deref() {
        command.push_str(" --version ");
        command.push_str(version);
    }
    if let Some(preid) = options.preid.as_deref() {
        command.push_str(" --preid ");
        command.push_str(preid);
    }
    if let Some(projects) = options.projects.as_ref()
        && !projects.is_empty()
    {
        command.push_str(" --projects ");
        push_joined(&mut command, projects.iter().map(String::as_str), ",");
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
        if !options.run_checks {
            command.push_str(" --no-run-checks");
        }
        command.push_str(" --yes");
    }

    command
}

/// Streams checklist lines to the output layer with a single reusable buffer.
///
/// Building the full output eagerly would be simpler, but reusing one `String` keeps this path
/// cheap and makes allocation behavior very obvious during review.
fn print_checklist(heading: &str, intro: &str, checklist: &[ChecklistStep<'_>]) {
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
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('`');
    rendered.push_str(value);
    rendered.push('`');
    rendered
}

/// Joins a borrowed slice of owned strings with a precomputed output capacity.
fn join_string_slice(values: &[String], separator: &str) -> String {
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

fn render_publish_workflow_template(package_manager: PackageManagerType) -> String {
    let package_manager_setup = package_manager_setup_step(package_manager);
    let install_command = package_manager_install_command(package_manager);
    let mut rendered = String::from(PUBLISH_WORKFLOW_TEMPLATE);
    rendered = rendered.replace(PACKAGE_MANAGER_SETUP_TOKEN, package_manager_setup);
    rendered = rendered.replace(INSTALL_COMMAND_TOKEN, install_command);
    rendered = rendered.replace(RELEASE_COMMAND_TOKEN, "vp release --yes");
    rendered.replace(FIRST_RELEASE_COMMAND_TOKEN, "vp release --first-release --yes")
}

fn package_manager_setup_step(package_manager: PackageManagerType) -> &'static str {
    match package_manager {
        PackageManagerType::Pnpm | PackageManagerType::Yarn => COREPACK_SETUP_STEP,
        PackageManagerType::Bun => BUN_SETUP_STEP,
        PackageManagerType::Npm => "",
    }
}

fn package_manager_install_command(package_manager: PackageManagerType) -> &'static str {
    match package_manager {
        PackageManagerType::Pnpm => "pnpm install --frozen-lockfile",
        PackageManagerType::Yarn => "yarn install --immutable",
        PackageManagerType::Npm => "npm ci",
        PackageManagerType::Bun => "bun install --frozen-lockfile",
    }
}

fn find_release_workflow_path(cwd: &AbsolutePath) -> String {
    find_existing_release_workflow_path(cwd)
        .unwrap_or_else(|| default_publish_workflow_path().to_owned())
}

fn find_existing_release_workflow_path(cwd: &AbsolutePath) -> Option<String> {
    for candidate in [
        ".github/workflows/publish.yml",
        ".github/workflows/publish.yaml",
        ".github/workflows/release.yml",
        ".github/workflows/release.yaml",
    ] {
        if cwd.join(candidate).as_path().exists() {
            return Some(candidate.to_owned());
        }
    }

    let workflows_dir = cwd.join(".github/workflows");
    if let Ok(entries) = fs::read_dir(workflows_dir.as_path()) {
        let mut best_path: Option<String> = None;
        for entry in entries.filter_map(Result::ok) {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            let lowercase = file_name.to_ascii_lowercase();
            let is_workflow_manifest = lowercase.ends_with(".yml") || lowercase.ends_with(".yaml");
            let mentions_publish = lowercase.contains("publish");
            let mentions_release = lowercase.contains("release");
            if !is_workflow_manifest || (!mentions_publish && !mentions_release) {
                continue;
            }

            let mut path = String::with_capacity(file_name.len() + 18);
            path.push_str(".github/workflows/");
            path.push_str(&file_name);
            let should_replace = best_path.as_ref().map_or(true, |best| {
                let best_lowercase = best.to_ascii_lowercase();
                let best_mentions_publish = best_lowercase.contains("publish");
                (mentions_publish && !best_mentions_publish)
                    || (mentions_publish == best_mentions_publish && path < *best)
            });
            if should_replace {
                best_path = Some(path);
            }
        }
        return best_path;
    }

    None
}

const fn default_publish_workflow_path() -> &'static str {
    DEFAULT_PUBLISH_WORKFLOW_PATH
}

fn workflow_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn detect_github_repo(cwd: &AbsolutePath) -> Option<String> {
    let remote = capture_git(cwd, ["config", "--get", "remote.origin.url"]).ok()?;
    parse_github_repo_slug(&remote)
}

fn detect_dispatch_ref_hint(cwd: &AbsolutePath) -> Option<String> {
    if let Ok(tag) = capture_git(cwd, ["describe", "--tags", "--exact-match"]) {
        let tag = tag.trim();
        if !tag.is_empty() {
            return Some(tag.to_owned());
        }
    }

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

#[cfg(test)]
mod tests {
    use std::fs;

    use vite_path::AbsolutePathBuf;

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
            dispatch_ref_hint: Some("main".into()),
            workflow_path: ".github/workflows/publish.yml".into(),
            ..Default::default()
        };

        let checklist = first_publish_checklist!(
            &guidance,
            &ReleaseOptions {
                dry_run: false,
                skip_publish: false,
                first_release: true,
                changelog: false,
                version: None,
                preid: None,
                otp: None,
                projects: None,
                git_tag: true,
                git_commit: true,
                run_checks: true,
                yes: false,
            },
        );

        let lines = render_checklist_lines(&checklist);
        assert_eq!(
            lines[0],
            "  1. Commit a GitHub Actions publish workflow that runs on a GitHub-hosted runner."
        );
        assert!(lines.iter().any(|line| line.contains("Repository: `voidzero-dev/vite-plus`")));
        assert!(
            lines.iter().any(|line| line.contains("Dry run: vp release --first-release --dry-run"))
        );
        assert!(lines.iter().any(|line| {
            line.contains("Trusted publish from GitHub Actions: vp release --first-release --yes")
        }));
    }

    #[test]
    fn first_publish_checklist_surfaces_package_issues_compactly() {
        let guidance = FirstPublishGuidance {
            workflow_path: ".github/workflows/publish.yml".into(),
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
                version: None,
                preid: Some("beta".into()),
                otp: None,
                projects: Some(vec!["@scope/pkg-a".into()]),
                git_tag: false,
                git_commit: false,
                run_checks: true,
                yes: false,
            },
        );

        let lines = render_checklist_lines(&checklist);
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Missing `repository`: @scope/pkg-a, @scope/pkg-b"))
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Repository does not match git remote: @scope/pkg-c"))
        );
        assert!(lines.iter().any(|line| {
            line.contains("Missing `publishConfig.access = \"public\"`: @scope/pkg-a")
        }));
        assert!(lines.iter().any(|line| {
            line.contains(
                "Dry run: vp release --first-release --changelog --preid beta --projects @scope/pkg-a --no-git-tag --no-git-commit --dry-run",
            )
        }));
    }

    #[test]
    fn first_publish_checklist_surfaces_scaffolded_workflow_note() {
        let guidance = FirstPublishGuidance {
            workflow_path: ".github/workflows/publish.yml".into(),
            workflow_template_created: true,
            ..Default::default()
        };

        let checklist = first_publish_checklist!(&guidance, &make_release_options_for_tests());
        let lines = render_checklist_lines(&checklist);

        assert!(lines.iter().any(|line| {
            line.contains("A starter workflow was scaffolded locally from the built-in template")
        }));
    }

    #[test]
    fn render_release_command_omits_skip_publish_for_real_publish_examples() {
        let command = render_release_command(
            &ReleaseOptions {
                dry_run: false,
                skip_publish: true,
                first_release: false,
                changelog: false,
                version: None,
                preid: None,
                otp: Some("123456".into()),
                projects: Some(Vec::new()),
                git_tag: true,
                git_commit: true,
                run_checks: true,
                yes: false,
            },
            false,
            false,
        );

        assert_eq!(command, "vp release --yes");
    }

    #[test]
    fn render_release_command_preserves_no_git_commit_for_checklist_examples() {
        let command = render_release_command(
            &ReleaseOptions {
                dry_run: false,
                skip_publish: false,
                first_release: true,
                changelog: false,
                version: None,
                preid: None,
                otp: None,
                projects: None,
                git_tag: true,
                git_commit: false,
                run_checks: true,
                yes: false,
            },
            true,
            true,
        );

        assert_eq!(command, "vp release --first-release --no-git-commit --dry-run");
    }

    #[test]
    fn render_release_command_includes_version_override() {
        let command = render_release_command(
            &ReleaseOptions {
                dry_run: false,
                skip_publish: false,
                first_release: false,
                changelog: false,
                version: Some("1.2.3".into()),
                preid: None,
                otp: None,
                projects: None,
                git_tag: true,
                git_commit: true,
                run_checks: true,
                yes: false,
            },
            false,
            false,
        );

        assert_eq!(command, "vp release --version 1.2.3 --yes");
    }

    #[test]
    fn workflow_detection_prefers_publish_filename_when_both_exist() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workflows_dir = cwd.join(".github/workflows");
        fs::create_dir_all(workflows_dir.as_path()).unwrap();
        fs::write(workflows_dir.join("release.yml").as_path(), "name: Release\n").unwrap();
        fs::write(workflows_dir.join("publish.yml").as_path(), "name: Publish\n").unwrap();

        assert_eq!(find_release_workflow_path(&cwd), ".github/workflows/publish.yml");
    }

    #[test]
    fn workflow_detection_uses_release_filename_when_publish_is_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workflows_dir = cwd.join(".github/workflows");
        fs::create_dir_all(workflows_dir.as_path()).unwrap();
        fs::write(workflows_dir.join("release.yaml").as_path(), "name: Release\n").unwrap();

        assert_eq!(find_release_workflow_path(&cwd), ".github/workflows/release.yaml");
    }

    #[test]
    fn workflow_detection_defaults_to_publish_filename_when_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        assert_eq!(find_release_workflow_path(&cwd), ".github/workflows/publish.yml");
    }

    #[test]
    fn workflow_template_creation_writes_publish_workflow_when_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let mut guidance = FirstPublishGuidance {
            dispatch_ref_hint: Some("release-main".into()),
            workflow_path: ".github/workflows/publish.yml".into(),
            ..Default::default()
        };

        ensure_first_publish_workflow_template(&cwd, PackageManagerType::Pnpm, &mut guidance)
            .unwrap();

        let created = std::fs::read_to_string(cwd.join(".github/workflows/publish.yml")).unwrap();
        assert!(guidance.workflow_template_created);
        assert!(created.contains("workflow_dispatch:"));
        assert!(created.contains("<default-branch>"));
        assert!(created.contains("<release-tag-pattern>"));
        assert!(created.contains("run: corepack enable"));
        assert!(created.contains("run: pnpm install --frozen-lockfile"));
        assert!(created.contains("git config user.name \"github-actions[bot]\""));
        assert!(created.contains("vp release --first-release --yes"));
        assert!(created.contains("vp release --yes"));
        assert!(created.contains("run: git push origin --tags"));
        assert!(!created.contains(PACKAGE_MANAGER_SETUP_TOKEN));
        assert!(!created.contains(INSTALL_COMMAND_TOKEN));
    }

    #[test]
    fn workflow_template_creation_skips_existing_release_workflow() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let workflows_dir = cwd.join(".github/workflows");
        fs::create_dir_all(workflows_dir.as_path()).unwrap();
        fs::write(workflows_dir.join("release.yml").as_path(), "name: Release\n").unwrap();
        let mut guidance = FirstPublishGuidance {
            workflow_path: ".github/workflows/release.yml".into(),
            ..Default::default()
        };

        ensure_first_publish_workflow_template(&cwd, PackageManagerType::Pnpm, &mut guidance)
            .unwrap();

        assert!(!guidance.workflow_template_created);
        assert!(!cwd.join(".github/workflows/publish.yml").as_path().exists());
        assert_eq!(
            std::fs::read_to_string(cwd.join(".github/workflows/release.yml")).unwrap(),
            "name: Release\n"
        );
    }

    #[test]
    fn workflow_template_creation_uses_bun_specific_setup() {
        let rendered = render_publish_workflow_template(PackageManagerType::Bun);

        assert!(rendered.contains("uses: oven-sh/setup-bun@v2"));
        assert!(rendered.contains("run: bun install --frozen-lockfile"));
        assert!(!rendered.contains("run: corepack enable"));
    }

    fn make_release_options_for_tests() -> ReleaseOptions {
        ReleaseOptions {
            dry_run: false,
            skip_publish: false,
            first_release: true,
            changelog: false,
            version: None,
            preid: None,
            otp: None,
            projects: None,
            git_tag: true,
            git_commit: true,
            run_checks: true,
            yes: false,
        }
    }
}
