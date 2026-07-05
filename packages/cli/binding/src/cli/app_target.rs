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

/// Bare = no positional target and no help-like flag. A non-flag token may be
/// a flag value (`--port 3000`), so any non-flag argument conservatively
/// disables elicitation; help/version requests are answered by the underlying
/// tool and must never be redirected.
fn is_bare(args: &[String]) -> bool {
    args.iter()
        .all(|arg| arg.starts_with('-') && !super::help::is_app_tool_help_or_version_flag(arg))
}

/// Heuristic ranking signal: does `dir` look runnable for `command`?
/// Used for ordering and single-candidate auto-selection, never for hiding.
/// The rules are documented in rfcs/cwd-flag.md ("The likely-runnable
/// heuristic"); keep both in sync.
fn looks_runnable(dir: &AbsolutePathBuf, command: &str) -> bool {
    match command {
        // Bare `vp pack` succeeds when the config declares a `pack` block or
        // tsdown's default entry exists; a Vite config without `pack` does
        // not make a package packable. `contains` deliberately counts configs
        // with spreads (`pack` may exist behind them): the right direction
        // for a never-hide ranking signal.
        "pack" => {
            vite_static_config::resolve_static_config(dir).contains("pack")
                || dir.as_path().join("src/index.ts").is_file()
        }
        _ => vite_static_config::has_config_file(dir) || dir.as_path().join("index.html").is_file(),
    }
}

/// `defaultPackage` from the `vite.config.*` in `cwd`, read via static
/// extraction so it works at roots without a vite-plus install (non-workspace
/// framework repos). The value must be a static string literal.
///
/// `get_declared` keeps this to explicitly written fields: a config that is
/// unanalyzable or hides fields behind a spread simply falls through to the
/// picker/current-dir resolution instead of failing every bare app command.
fn resolve_default_package(command: &str, cwd: &AbsolutePathBuf) -> Option<AppTarget> {
    let fail = |msg: &str| {
        output::error(msg);
        Some(AppTarget::Exit(ExitStatus(1)))
    };
    match vite_static_config::resolve_static_config(cwd).get_declared("defaultPackage") {
        Some(vite_static_config::FieldValue::Json(serde_json::Value::String(dir))) => {
            let target = cwd.join(&dir).clean();
            if !target.as_path().is_dir() {
                return fail(&format!("defaultPackage points to a missing directory: {dir}"));
            }
            output::note(&format!("vp {command}: using {dir} (defaultPackage)"));
            Some(AppTarget::Dir(target))
        }
        Some(vite_static_config::FieldValue::Json(other)) => {
            fail(&format!("defaultPackage must be a string of a directory, got: {other}"))
        }
        Some(vite_static_config::FieldValue::NonStatic) => fail(
            "defaultPackage in vite.config.ts must be a static string literal so vp can read it without executing the config",
        ),
        None => None,
    }
}

/// Fuzzy package picker on `vite_select`, the same component behind the
/// `vp run` task selector. Returns the selected row index, or `None` on
/// Ctrl+C. Every render emits a `package-select:<query>:<index>` milestone
/// (invisible OSC 8 hyperlinks) so PTY snapshot tests can synchronize.
fn run_package_picker(command: &str, rows: &[PackageRow]) -> Result<Option<usize>, Error> {
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

pub(super) fn resolve_app_target(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
) -> Result<AppTarget, Error> {
    let Some((command, args)) = app_command_parts(subcommand) else {
        return Ok(AppTarget::CurrentDir);
    };
    if !is_bare(args) {
        return Ok(AppTarget::CurrentDir);
    }

    // `defaultPackage` comes before any workspace lookup: the non-workspace
    // framework shape (a Laravel-style root with a vite.config.ts pointer and
    // no package.json up-tree) has no workspace metadata at all.
    if let Some(target) = resolve_default_package(command, cwd) {
        return Ok(target);
    }

    // The package listing needs workspace metadata; anything unresolvable
    // keeps today's behavior (the caller surfaces its own workspace errors).
    let Ok((workspace_root, rel_from_root)) = vite_workspace::find_workspace_root(cwd) else {
        return Ok(AppTarget::CurrentDir);
    };
    if !rel_from_root.as_str().is_empty()
        || matches!(workspace_root.workspace_file, WorkspaceFile::NonWorkspacePackage(_))
    {
        return Ok(AppTarget::CurrentDir);
    }

    let graph =
        vite_workspace::load_package_graph(&workspace_root).map_err(|e| Error::Anyhow(e.into()))?;
    let mut rows: Vec<PackageRow> = graph
        .node_weights()
        .filter(|info| !info.path.as_str().is_empty())
        .map(|info| {
            let absolute = info.absolute_path.to_absolute_path_buf();
            PackageRow {
                name: info.package_json.name.clone(),
                path: vite_str::Str::from(info.path.as_str()),
                runnable: looks_runnable(&absolute, command),
                absolute,
            }
        })
        .collect();
    if rows.is_empty() {
        return Ok(AppTarget::CurrentDir);
    }
    rows.sort_by(|a, b| (!a.runnable, a.path.as_str()).cmp(&(!b.runnable, b.path.as_str())));

    // In an interactive terminal, pick the target: exactly one likely-runnable
    // package (rows are sorted runnable first) auto-selects without a menu;
    // otherwise the fuzzy picker runs.
    if vite_shared::is_interactive_terminal() {
        let single_runnable = rows[0].runnable && rows.get(1).is_none_or(|row| !row.runnable);
        let picked = if single_runnable { Some(0) } else { run_package_picker(command, &rows)? };
        let Some(index) = picked else {
            return Ok(AppTarget::Exit(ExitStatus(130)));
        };
        let row = &rows[index];
        // Deliberately stdout via println!: these lines belong to the
        // command's own output stream, like the tool output that follows.
        println!("Selected package: {} ({})", row.name, row.path);
        println!("Tip: run this directly with `vp -C {} {command}`", row.path);
        return Ok(AppTarget::Dir(row.absolute.clone()));
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
    Ok(AppTarget::Exit(ExitStatus(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_means_no_positional_target_and_no_help() {
        let to_args = |args: &[&str]| args.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert!(is_bare(&to_args(&[])));
        assert!(is_bare(&to_args(&["--watch"])));
        assert!(is_bare(&to_args(&["-w", "--minify"])));
        // A positional target disables elicitation.
        assert!(!is_bare(&to_args(&["apps/web"])));
        // A flag value is indistinguishable from a positional without knowing
        // the tool's flag arity, so it conservatively counts as non-bare.
        assert!(!is_bare(&to_args(&["--port", "3000"])));
        // Help/version requests go to the underlying tool, never elicitation.
        assert!(!is_bare(&to_args(&["--help"])));
        assert!(!is_bare(&to_args(&["-h"])));
        assert!(!is_bare(&to_args(&["--watch", "--version"])));
        // Vite and tsdown are cac-based and use `-v` for version.
        assert!(!is_bare(&to_args(&["-v"])));
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
