//! Target elicitation for bare app commands at a workspace root.
//!
//! A bare `vp dev`/`build`/`preview`/`pack` at a workspace root has no target
//! and would silently run against the root. Resolution order (rfcs/cwd-flag.md):
//! explicit `-C` and positional targets are handled before this code and skip
//! elicitation entirely; then `defaultPackage` from the config in the
//! invocation directory, then the interactive package picker (a package
//! listing plus exit 1 when the terminal is not interactive).

use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_shared::output;
use vite_task::ExitStatus;
use vite_workspace::WorkspaceFile;

use super::types::SynthesizableSubcommand;

/// Where a bare app command should run.
pub(super) enum AppTarget {
    /// No elicitation applies; run in the invocation directory as today.
    CurrentDir,
    /// Run as if invoked in this directory (implicit `-C`).
    Dir(AbsolutePathBuf),
    /// Elicitation printed its output and decided the exit code.
    Exit(ExitStatus),
}

struct PackageRow {
    name: vite_str::Str,
    path: vite_str::Str,
    absolute: AbsolutePathBuf,
    runnable: bool,
}

/// App commands are the single-target subcommands; everything else never
/// goes through elicitation.
fn app_command_parts(subcommand: &SynthesizableSubcommand) -> Option<(&'static str, &[String])> {
    match subcommand {
        SynthesizableSubcommand::Dev { args } => Some(("dev", args)),
        SynthesizableSubcommand::Build { args } => Some(("build", args)),
        SynthesizableSubcommand::Preview { args } => Some(("preview", args)),
        SynthesizableSubcommand::Pack { args } => Some(("pack", args)),
        _ => None,
    }
}

/// Boolean flags of the Vite CLI (dev/build/preview), from the shipped
/// `vp <command> --help` (snap-tests/command-helper); keep in sync. Under
/// cac/mri parsing every OTHER flag — required-value, optional-value
/// (`--host [host]`), or unknown — consumes a following non-flag token as
/// its value, so only tokens no flag consumes are positional targets.
const VITE_BOOLEAN_FLAGS: &[&str] = &[
    "-w",
    "--watch",
    "--app",
    "--clearScreen",
    "--cors",
    "--emptyOutDir",
    "--experimentalBundle",
    "--force",
    "--profile",
    "--strictPort",
];

/// Boolean flags of the bundled pack CLI (tsdown), from `vp pack --help`.
const PACK_BOOLEAN_FLAGS: &[&str] = &[
    "--attw",
    "--clean",
    "--devtools",
    "--dts",
    "--exe",
    "--exports",
    "--fail-on-warn",
    "--failOnWarn",
    "--minify",
    "--no-write",
    "--publint",
    "--report",
    "--shims",
    "--sourcemap",
    "--treeshake",
    "--unbundle",
    "--unused",
];

/// Bare = no positional target and no help-like flag. Mirrors the tools'
/// own cac/mri parsing: a non-flag token after any non-boolean flag is that
/// flag's value (the tool would never see it as a positional), while a token
/// after a boolean flag is a positional target and disables elicitation.
/// pack's workspace selectors already define their own target set and
/// disable elicitation outright. Help/version requests are answered by the
/// underlying tool and must never be redirected.
fn is_bare(command: &str, args: &[String]) -> bool {
    let is_pack = command == "pack";
    let booleans = if is_pack { PACK_BOOLEAN_FLAGS } else { VITE_BOOLEAN_FLAGS };
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') || super::help::is_app_tool_help_or_version_flag(arg) {
            return false;
        }
        // `--` terminates options: whatever follows is an explicit positional.
        if arg == "--" {
            return iter.next().is_none();
        }
        // An explicit config file (`-c`/`--config`) is explicit build intent:
        // forward it to the tool instead of eliciting a package to override it.
        if ["-c", "--config"]
            .iter()
            .any(|f| arg == f || arg.strip_prefix(f).is_some_and(|r| r.starts_with('=')))
        {
            return false;
        }
        // Workspace selectors and --root already specify pack's target;
        // these previously-valid targeted invocations must keep forwarding.
        if is_pack
            && ["-W", "--workspace", "-F", "--filter", "--root"]
                .iter()
                .any(|f| arg == f || arg.strip_prefix(f).is_some_and(|r| r.starts_with('=')))
        {
            return false;
        }
        let is_boolean = booleans.contains(&arg.as_str()) || arg.starts_with("--no-");
        // An inline `=` already carries the value (`--port=3000`, `--env.FOO=bar`).
        if !is_boolean
            && !arg.contains('=')
            && iter.peek().is_some_and(|next| !next.starts_with('-'))
        {
            iter.next();
        }
    }
    true
}

/// Heuristic ranking signal: does `dir` look runnable for `command`?
/// Used for ordering and single-candidate auto-selection, never for hiding.
/// The rules are documented in rfcs/cwd-flag.md ("The likely-runnable
/// heuristic"); keep both in sync.
///
/// The workspace root needs a stronger signal than member packages: a shared
/// root `vite.config.ts` (lint/fmt/tasks) is the normal monorepo setup and
/// must not make the root look like an app, or auto-select would run the
/// silent root build this feature exists to prevent.
fn looks_runnable(dir: &AbsolutePathBuf, command: &str, is_root: bool) -> bool {
    match command {
        // Bare `vp pack` succeeds when tsdown's default entry exists or the
        // config explicitly declares a `pack` block (a spread that only
        // might contain `pack` does not count: auto-select acts on this
        // signal, so a false positive runs tsdown in a non-packable
        // package). The one-stat entry check runs first: this executes per
        // workspace package, and the config check reads and parses a file.
        "pack" => {
            dir.as_path().join("src/index.ts").is_file()
                || vite_static_config::resolve_static_config(dir).get_declared("pack").is_some()
        }
        // The root needs a stronger signal than a member. A declared `build`
        // block (a library/SSR build with no entry HTML) makes the root a
        // target for `vp build` only: dev/preview serve an app, for which the
        // signal is a root `index.html`. A shared root config for lint/fmt/
        // tasks declares neither, so it never makes the root a target.
        "build" if is_root => {
            dir.as_path().join("index.html").is_file()
                || vite_static_config::resolve_static_config(dir).get_declared("build").is_some()
        }
        _ if is_root => dir.as_path().join("index.html").is_file(),
        _ => vite_static_config::has_config_file(dir) || dir.as_path().join("index.html").is_file(),
    }
}

/// Resolve the `defaultPackage` value [`classify`] extracted from the
/// invocation root's `vite.config.*` (static extraction, so it works at
/// roots without a vite-plus install). The value must be a static string
/// literal naming an existing directory.
fn resolve_default_package(
    command: &str,
    cwd: &AbsolutePathBuf,
    value: vite_static_config::FieldValue,
) -> AppTarget {
    let fail = |msg: &str| {
        output::error(msg);
        AppTarget::Exit(ExitStatus(1))
    };
    match value {
        vite_static_config::FieldValue::Json(serde_json::Value::String(dir)) => {
            let target = cwd.join(&dir).clean();
            if !target.as_path().is_dir() {
                return fail(&format!("defaultPackage points to a missing directory: {dir}"));
            }
            output::note(&format!("vp {command}: using {dir} (defaultPackage)"));
            AppTarget::Dir(target)
        }
        vite_static_config::FieldValue::Json(other) => {
            fail(&format!("defaultPackage must be a string of a directory, got: {other}"))
        }
        vite_static_config::FieldValue::NonStatic => fail(
            "defaultPackage in vite.config.ts must be a static string literal so vp can read it without executing the config",
        ),
    }
}

/// Fuzzy package picker on `vite_select`, the same component behind the
/// `vp run` task selector. Returns the selected row index, or `None` on
/// Ctrl+C. When the PTY snapshot harness sets `VP_EMIT_MILESTONES=1`, every
/// render emits a `package-select:<query>:<index>` milestone (invisible
/// OSC 8 hyperlinks) for the tests to synchronize on — same gate as
/// packages/prompts/src/milestone.ts; real terminals never see the bytes.
fn run_package_picker(command: &str, rows: &[PackageRow]) -> Result<Option<usize>, Error> {
    let emit_milestones = std::env::var_os("VP_EMIT_MILESTONES").is_some_and(|value| value == "1");
    let items: Vec<vite_select::SelectItem> = rows
        .iter()
        .map(|row| vite_select::SelectItem {
            label: vite_str::format!("{} {}", row.name, row.path),
            display_name: row.name.clone(),
            description: row.path.clone(),
            group: None,
        })
        .collect();
    let prompt =
        format!("Select a package to {command} (\u{2191}/\u{2193}, Enter to run, type to search):");
    let params = vite_select::SelectParams {
        items: &items,
        query: None,
        header: None,
        prompt: &prompt,
        page_size: 12,
    };
    let mut selected_index = 0usize;
    let mut stdout = std::io::stdout();
    let result = vite_select::select_list(
        &mut stdout,
        &params,
        vite_select::Mode::Interactive { selected_index: &mut selected_index },
        |state| {
            use std::io::Write as _;
            if !emit_milestones {
                return;
            }
            let milestone =
                vite_str::format!("package-select:{}:{}", state.query, state.selected_index);
            let bytes = pty_terminal_test_client::encoded_milestone(&milestone);
            let mut out = std::io::stdout();
            let _ = out.write_all(&bytes);
            let _ = out.flush();
        },
    )
    .map_err(Error::Anyhow)?;
    Ok(match result {
        vite_select::SelectResult::Selected => Some(selected_index),
        vite_select::SelectResult::Cancelled => None,
    })
}

/// Pure predicate for the vp-script interception: would `resolve_app_target`
/// do anything other than run in `cwd`? Never prints and never runs the
/// picker. Slightly over-approximates (an empty workspace reports true), in
/// which case the script merely spawns the real binary, which then behaves
/// identically to a direct invocation.
pub(super) fn needs_elicitation(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
) -> bool {
    matches!(classify(subcommand, cwd), Classification::Elicit(..))
}

/// Outcome of classifying a bare app command.
enum Classification {
    /// Run in `cwd` unchanged. Carries the workspace root found for `cwd`
    /// (when the lookup succeeded) so the caller can reuse it instead of
    /// walking the tree a second time — the hot path for a bare command deep
    /// inside a large monorepo, where the walk is the only per-invocation
    /// cost this feature adds.
    RunInPlace(Option<vite_workspace::WorkspaceRoot>),
    /// Elicit a target: `defaultPackage`, or the picker/listing at a
    /// workspace root.
    Elicit(&'static str, Elicitation),
}

/// Why a bare app command needs target elicitation.
enum Elicitation {
    /// The invocation root's config explicitly declares `defaultPackage`
    /// (with this value — possibly invalid, which the resolver reports).
    DefaultPackage(vite_static_config::FieldValue),
    /// Bare app command at a real workspace root: picker/listing territory.
    WorkspaceRoot(vite_workspace::WorkspaceRoot),
}

/// The RFC's resolution order, written once for both entry points: bare app
/// command, then `defaultPackage` at the invocation root, then the workspace
/// root itself. `defaultPackage` is a root-pointer concept: it applies where
/// the invocation directory is its own root (a workspace root, a standalone
/// package, or a framework directory with no package.json ancestry); below a
/// workspace root the current directory already identifies the target, so a
/// member's own config must not redirect.
///
/// The one `find_workspace_root` walk here rides back out on
/// [`Classification::RunInPlace`] whenever the command ends up running in
/// `cwd`, so a bare command in a sub-package walks the tree once, not twice.
fn classify(subcommand: &SynthesizableSubcommand, cwd: &AbsolutePathBuf) -> Classification {
    let Some((command, args)) = app_command_parts(subcommand) else {
        return Classification::RunInPlace(None);
    };
    if !is_bare(command, args) {
        return Classification::RunInPlace(None);
    }
    let workspace = vite_workspace::find_workspace_root(cwd);
    let at_invocation_root =
        workspace.as_ref().map_or(true, |(_, rel_from_root)| rel_from_root.as_str().is_empty());
    if at_invocation_root
        && let Some(value) =
            vite_static_config::resolve_static_config(cwd).get_declared("defaultPackage")
    {
        return Classification::Elicit(command, Elicitation::DefaultPackage(value));
    }
    // The picker/listing needs workspace metadata; anything unresolvable
    // keeps today's behavior (the caller surfaces its own workspace errors).
    let Ok((workspace_root, rel_from_root)) = workspace else {
        return Classification::RunInPlace(None);
    };
    if !rel_from_root.as_str().is_empty()
        || matches!(workspace_root.workspace_file, WorkspaceFile::NonWorkspacePackage(_))
    {
        return Classification::RunInPlace(Some(workspace_root));
    }
    // A runnable workspace root runs in place, TTY or not: the invocation
    // already has its configured target, and repos whose root is the app or
    // library (e.g. a single package with a settings-only pnpm-workspace.yaml)
    // ran this way before elicitation existed. Eliciting only when the root
    // is not a plausible target is what keeps this feature purely additive.
    if looks_runnable(&workspace_root.path.to_absolute_path_buf(), command, true) {
        return Classification::RunInPlace(Some(workspace_root));
    }
    Classification::Elicit(command, Elicitation::WorkspaceRoot(workspace_root))
}

/// Resolve a bare app command's target. The second tuple element is the
/// workspace root already found for `cwd`, present only when the command runs
/// in the unchanged `cwd` (so it always matches a fresh lookup there); the
/// caller reuses it to skip a second `find_workspace_root` walk.
pub(super) fn resolve_app_target(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
) -> Result<(AppTarget, Option<vite_workspace::WorkspaceRoot>), Error> {
    let (command, elicitation) = match classify(subcommand, cwd) {
        Classification::RunInPlace(workspace_root) => {
            return Ok((AppTarget::CurrentDir, workspace_root));
        }
        Classification::Elicit(command, elicitation) => (command, elicitation),
    };
    let workspace_root = match elicitation {
        Elicitation::DefaultPackage(value) => {
            return Ok((resolve_default_package(command, cwd, value), None));
        }
        Elicitation::WorkspaceRoot(workspace_root) => workspace_root,
    };

    let graph =
        vite_workspace::load_package_graph(&workspace_root).map_err(|e| Error::Anyhow(e.into()))?;
    let mut rows: Vec<PackageRow> = graph
        .node_weights()
        .filter(|info| {
            // The root is never a row: when it looks runnable, classify
            // already ran the command in place instead of eliciting.
            !info.path.as_str().is_empty()
        })
        .map(|info| {
            let absolute = info.absolute_path.to_absolute_path_buf();
            PackageRow {
                name: info.package_json.name.clone(),
                path: vite_str::Str::from(info.path.as_str()),
                runnable: looks_runnable(&absolute, command, false),
                absolute,
            }
        })
        .collect();
    if rows.is_empty() {
        // Root excluded and no members: runs in place, and the root we found
        // is still valid for the unchanged cwd.
        return Ok((AppTarget::CurrentDir, Some(workspace_root)));
    }
    rows.sort_by(|a, b| (!a.runnable, a.path.as_str()).cmp(&(!b.runnable, b.path.as_str())));

    // In an interactive terminal, pick the target: exactly one likely-runnable
    // package (rows are sorted runnable first) auto-selects without a menu;
    // otherwise the fuzzy picker runs.
    if vite_shared::is_interactive_terminal() {
        let single_runnable = rows[0].runnable && rows.get(1).is_none_or(|row| !row.runnable);
        let picked = if single_runnable { Some(0) } else { run_package_picker(command, &rows)? };
        let Some(index) = picked else {
            return Ok((AppTarget::Exit(ExitStatus(130)), None));
        };
        let row = &rows[index];
        // Deliberately stdout via println!: these lines belong to the
        // command's own output stream, like the tool output that follows.
        println!("Selected package: {} ({})", row.name, row.path);
        println!("Tip: run this directly with `vp -C {} {command}`", row.path);
        return Ok((AppTarget::Dir(row.absolute.clone()), None));
    }

    output::error(&format!("`vp {command}` at the workspace root needs a target package."));
    output::raw_stderr("");
    output::raw_stderr("  Packages in this workspace:");
    let name_width = rows.iter().map(|row| row.name.len()).max().unwrap_or(0);
    for row in &rows {
        output::raw_stderr(&format!("    {:<name_width$}  {}", row.name, row.path));
    }
    output::raw_stderr("");
    let example = &rows[0].path;
    output::raw_stderr(&format!("  Pass a directory:  vp -C {example} {command}"));
    output::raw_stderr(&format!("  Or run every package's {command} script:  vp run -r {command}"));
    Ok((AppTarget::Exit(ExitStatus(1)), None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_means_no_positional_target_and_no_help() {
        let to_args = |args: &[&str]| args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert!(is_bare("dev", &to_args(&[])));
        assert!(is_bare("dev", &to_args(&["--watch"])));
        assert!(is_bare("build", &to_args(&["-w", "--minify"])));
        // A positional target disables elicitation.
        assert!(!is_bare("dev", &to_args(&["apps/web"])));
        // Like cac, any non-boolean flag consumes a following non-flag token
        // as its value — required and optional values alike.
        assert!(is_bare("dev", &to_args(&["--port", "3000"])));
        assert!(is_bare("dev", &to_args(&["--host", "0.0.0.0"])));
        assert!(is_bare("dev", &to_args(&["--open", "/foo"])));
        assert!(is_bare("build", &to_args(&["--mode", "production", "--minify"])));
        assert!(is_bare("build", &to_args(&["--port=3000"])));
        assert!(is_bare("pack", &to_args(&["--env-file", ".env"])));
        assert!(is_bare("pack", &to_args(&["--env.FOO=bar", "--minify"])));
        // A token after a boolean flag is a positional; the tables are
        // command-specific (--minify is optional-value for Vite build,
        // boolean for pack).
        assert!(!is_bare("build", &to_args(&["--watch", "apps/web"])));
        assert!(!is_bare("pack", &to_args(&["--minify", "src/index.ts"])));
        assert!(!is_bare("pack", &to_args(&["--env.FOO", "bar", "src/cli.ts"])));
        assert!(is_bare("build", &to_args(&["--minify", "esbuild"])));
        // pack workspace selectors define their own target set, in both the
        // spaced and inline-value forms.
        assert!(!is_bare("pack", &to_args(&["-W"])));
        assert!(!is_bare("pack", &to_args(&["--workspace", "packages/a"])));
        assert!(!is_bare("pack", &to_args(&["-F", "ui"])));
        assert!(!is_bare("pack", &to_args(&["--filter=ui"])));
        assert!(!is_bare("pack", &to_args(&["--workspace=packages/a"])));
        assert!(!is_bare("pack", &to_args(&["--root", "packages/lib"])));
        assert!(!is_bare("pack", &to_args(&["--root=packages/lib"])));
        // An explicit config file is an explicit target (build and pack).
        assert!(!is_bare("build", &to_args(&["-c", "apps/web/vite.config.ts"])));
        assert!(!is_bare("build", &to_args(&["--config", "apps/web/vite.config.ts"])));
        assert!(!is_bare("build", &to_args(&["--config=apps/web/vite.config.ts"])));
        assert!(!is_bare("preview", &to_args(&["-c", "x.ts"])));
        assert!(!is_bare("pack", &to_args(&["-c", "tsdown.config.ts"])));
        // `--` terminates options; a token after it is an explicit positional.
        assert!(!is_bare("build", &to_args(&["--", "apps/web"])));
        assert!(!is_bare("pack", &to_args(&["--minify", "--", "src/index.ts"])));
        assert!(is_bare("build", &to_args(&["--"])));
        // Help/version requests go to the underlying tool, never elicitation.
        assert!(!is_bare("dev", &to_args(&["--help"])));
        assert!(!is_bare("dev", &to_args(&["-h"])));
        assert!(!is_bare("build", &to_args(&["--watch", "--version"])));
        // Vite and tsdown are cac-based and use `-v` for version.
        assert!(!is_bare("build", &to_args(&["-v"])));
    }

    #[test]
    fn only_app_commands_elicit() {
        for (subcommand, expected) in [
            (SynthesizableSubcommand::Dev { args: vec![] }, Some("dev")),
            (SynthesizableSubcommand::Build { args: vec![] }, Some("build")),
            (SynthesizableSubcommand::Preview { args: vec![] }, Some("preview")),
            (SynthesizableSubcommand::Pack { args: vec![] }, Some("pack")),
            (SynthesizableSubcommand::Lint { args: vec![] }, None),
            (SynthesizableSubcommand::Test { args: vec![] }, None),
        ] {
            assert_eq!(app_command_parts(&subcommand).map(|(name, _)| name), expected);
        }
    }
}
