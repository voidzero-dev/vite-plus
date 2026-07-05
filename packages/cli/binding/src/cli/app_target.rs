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

/// Required-value flags of the Vite CLI (dev/build/preview). Space-separated
/// values of these flags are not positional targets. Optional-value flags
/// (`--host`, `--open`, `--debug`, `--ssr`, `--sourcemap`, `--minify`, ...)
/// are deliberately absent: a token after them stays ambiguous and keeps the
/// conservative fallback.
const VITE_VALUE_FLAGS: &[&str] = &[
    "-c",
    "--config",
    "--config-loader",
    "--log-level",
    "--out-dir",
    "--assets-dir",
    "--assets-inline-limit",
    "--base",
    "-l",
    "--logLevel",
    "-m",
    "--mode",
    "--configLoader",
    "-f",
    "--filter",
    "--port",
    "--outDir",
    "--assetsDir",
    "--assetsInlineLimit",
    "--target",
];

/// Required-value flags of the bundled pack CLI (see pack-bin.ts; cac accepts
/// both camelCase and kebab-case spellings). Optional-value flags (`--debug`,
/// `--watch`, `--from-vite`) are deliberately absent, as above.
const PACK_VALUE_FLAGS: &[&str] = &[
    "--root",
    "-F",
    "--filter",
    "-c",
    "--config",
    "--config-loader",
    "--configLoader",
    "-f",
    "--format",
    "--deps.never-bundle",
    "--target",
    "-l",
    "--logLevel",
    "--log-level",
    "-d",
    "--out-dir",
    "--outDir",
    "--platform",
    "--tsconfig",
    "--ignore-watch",
    "--ignoreWatch",
    "--env-file",
    "--envFile",
    "--env-prefix",
    "--envPrefix",
    "--on-success",
    "--onSuccess",
    "--copy",
    "--public-dir",
    "--publicDir",
];

/// Bare = no positional target and no help-like flag. Values of the
/// forwarded tool's known required-value flags (`--port 3000`) are skipped;
/// any other non-flag token may be a positional target and conservatively
/// disables elicitation. Help/version requests are answered by the
/// underlying tool and must never be redirected.
fn is_bare(command: &str, args: &[String]) -> bool {
    let value_flags = if command == "pack" { PACK_VALUE_FLAGS } else { VITE_VALUE_FLAGS };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if !arg.starts_with('-') || super::help::is_app_tool_help_or_version_flag(arg) {
            return false;
        }
        // `--env.NAME <value>` defines a compile-time env variable in pack;
        // the inline `--env.NAME=value` form already carries its value.
        if value_flags.contains(&arg.as_str())
            || (command == "pack" && arg.starts_with("--env.") && !arg.contains('='))
        {
            // Consume the flag's value; a missing value is the tool's error.
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
        // Bare `vp pack` succeeds when the config explicitly declares a
        // `pack` block or tsdown's default entry exists. A spread that only
        // might contain `pack` does not count: auto-select acts on this
        // signal, so a false positive runs tsdown in a non-packable package.
        "pack" => {
            vite_static_config::resolve_static_config(dir).get_declared("pack").is_some()
                || dir.as_path().join("src/index.ts").is_file()
        }
        _ if is_root => dir.as_path().join("index.html").is_file(),
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

/// Pure predicate for the vp-script interception: would `resolve_app_target`
/// do anything other than run in `cwd`? Never prints and never runs the
/// picker. Slightly over-approximates (an empty workspace reports true), in
/// which case the script merely spawns the real binary, which then behaves
/// identically to a direct invocation.
pub(super) fn needs_elicitation(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
) -> bool {
    let Some((command, args)) = app_command_parts(subcommand) else {
        return false;
    };
    if !is_bare(command, args) {
        return false;
    }
    let workspace = vite_workspace::find_workspace_root(cwd);
    if at_invocation_root(workspace.as_ref().ok().map(|(_, rel)| rel.as_str()))
        && vite_static_config::resolve_static_config(cwd).get_declared("defaultPackage").is_some()
    {
        return true;
    }
    let Ok((workspace_root, rel_from_root)) = workspace else {
        return false;
    };
    rel_from_root.as_str().is_empty()
        && !matches!(workspace_root.workspace_file, WorkspaceFile::NonWorkspacePackage(_))
}

/// `defaultPackage` is a root-pointer concept: it applies where the
/// invocation directory is its own root (a workspace root, a standalone
/// package, or a framework directory with no package.json ancestry — pass
/// the workspace lookup's `rel_from_root`, or `None` when the lookup
/// failed). Below a workspace root the current directory already identifies
/// the target package, so a member's own config must not redirect.
fn at_invocation_root(rel_from_root: Option<&str>) -> bool {
    rel_from_root.is_none_or(str::is_empty)
}

pub(super) fn resolve_app_target(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePathBuf,
) -> Result<AppTarget, Error> {
    let Some((command, args)) = app_command_parts(subcommand) else {
        return Ok(AppTarget::CurrentDir);
    };
    if !is_bare(command, args) {
        return Ok(AppTarget::CurrentDir);
    }

    // `defaultPackage` is consulted before the workspace-shape dispatch (the
    // non-workspace framework shape has no workspace metadata at all), but
    // only at the invocation root: a member package's config must not
    // redirect a command already running in that member.
    let workspace = vite_workspace::find_workspace_root(cwd);
    if at_invocation_root(workspace.as_ref().ok().map(|(_, rel)| rel.as_str()))
        && let Some(target) = resolve_default_package(command, cwd)
    {
        return Ok(target);
    }

    // The package listing needs workspace metadata; anything unresolvable
    // keeps today's behavior (the caller surfaces its own workspace errors).
    let Ok((workspace_root, rel_from_root)) = workspace else {
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
        .filter_map(|info| {
            let absolute = info.absolute_path.to_absolute_path_buf();
            let is_root = info.path.as_str().is_empty();
            let runnable = looks_runnable(&absolute, command, is_root);
            // The root itself is a valid target only when it looks runnable;
            // `.` keeps the -C hint and the selection working there.
            if is_root && !runnable {
                return None;
            }
            let path = if is_root { "." } else { info.path.as_str() };
            Some(PackageRow {
                name: info.package_json.name.clone(),
                path: vite_str::Str::from(path),
                runnable,
                absolute,
            })
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
        assert!(is_bare("dev", &to_args(&[])));
        assert!(is_bare("dev", &to_args(&["--watch"])));
        assert!(is_bare("build", &to_args(&["-w", "--minify"])));
        // A positional target disables elicitation.
        assert!(!is_bare("dev", &to_args(&["apps/web"])));
        // Known required-value flags consume their value, per command.
        assert!(is_bare("dev", &to_args(&["--port", "3000"])));
        assert!(is_bare("build", &to_args(&["--mode", "production", "--minify"])));
        assert!(is_bare("build", &to_args(&["--assetsDir", "assets"])));
        assert!(is_bare("build", &to_args(&["--port=3000"])));
        assert!(is_bare("pack", &to_args(&["--env-file", ".env"])));
        assert!(is_bare("pack", &to_args(&["-d", "out", "--env.FOO", "1"])));
        // The tables are command-specific: pack's flags mean nothing to Vite,
        // and Vite's optional-value `-d, --debug [feat]` must not consume.
        assert!(!is_bare("dev", &to_args(&["--env-file", ".env"])));
        assert!(!is_bare("dev", &to_args(&["-d", "apps/web"])));
        // A token after an unknown or optional-value flag is ambiguous with a
        // positional target, so it conservatively counts as non-bare.
        assert!(!is_bare("build", &to_args(&["--watch", "apps/web"])));
        assert!(!is_bare("dev", &to_args(&["--host", "0.0.0.0"])));
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
