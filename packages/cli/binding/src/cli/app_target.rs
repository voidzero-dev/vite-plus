//! Target elicitation for bare app commands at a workspace root.
//!
//! A bare `vp dev`/`build`/`preview`/`pack` at a workspace root has no target
//! and would silently run against the root. Explicit `-C` and any subcommand
//! arguments skip target selection. An exact bare command uses
//! `defaultPackage`, a root intent signal, or the package picker. See
//! rfcs/cwd-flag.md.

use vp_error::Error;
use vp_shared::{env_vars, output};
use vt::ExitStatus;
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_workspace::WorkspaceFile;

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
    name: vt_str::Str,
    path: vt_str::Str,
    absolute: AbsolutePathBuf,
    likely_runnable: bool,
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

/// Does the workspace root have an intent signal for this app command?
/// This signal selects the root. It does not prove that the command succeeds.
/// The `defaultPackage` lookup passes the config that [`classify`] already
/// resolved, so Vite+ reads and parses the file only once for each command.
///
/// A declared field is sufficient. Vite reports invalid values after the
/// command starts. Dev, build, and preview use the same union of configuration
/// and source `index.html` signals.
fn root_has_intent_signal(
    config: &vp_static_config::FieldMap,
    dir: &AbsolutePath,
    command: &str,
) -> bool {
    // Bare `vp pack` accepts tsdown's default entry or a declared `pack` block.
    // A spread that might contain `pack` does not declare the block. A false
    // positive would run tsdown in a package that it cannot pack.
    if command == "pack" {
        return dir.as_path().join("src/index.ts").is_file()
            || config.get_declared("pack").is_some();
    }

    let config_fields = ["root", "build", "input", "environments", "appType"];
    if config_fields.iter().any(|field| config.get_declared(field).is_some()) {
        return true;
    }

    dir.as_path().join("index.html").is_file()
}

/// Member ranking heuristic. It orders picker rows and can auto-select one
/// member, but it never hides a member. This check differs from the root intent
/// signal. A shared root `vite.config.ts` is normal monorepo configuration and
/// does not by itself select the root. Resolve each member config lazily. For
/// `pack`, check the default entry first because config extraction reads and
/// parses a file.
fn member_is_likely_runnable(dir: &AbsolutePath, command: &str) -> bool {
    match command {
        "pack" => {
            dir.as_path().join("src/index.ts").is_file()
                || vp_static_config::resolve_static_config(dir).get_declared("pack").is_some()
        }
        _ => vp_static_config::has_config_file(dir) || dir.as_path().join("index.html").is_file(),
    }
}

/// Resolve the `defaultPackage` value [`classify`] extracted from the
/// invocation root's `vite.config.*` (static extraction, so it works at
/// roots without a vite-plus install). The value must be a static string
/// literal naming an existing directory.
fn resolve_default_package(
    command: &str,
    cwd: &AbsolutePath,
    value: vp_static_config::FieldValue,
) -> AppTarget {
    let fail = |msg: &str| {
        output::error(msg);
        AppTarget::Exit(ExitStatus(1))
    };
    match value {
        vp_static_config::FieldValue::Json(serde_json::Value::String(dir)) => {
            let target = cwd.join(&dir).clean();
            if !target.as_path().is_dir() {
                return fail(&format!("defaultPackage points to a missing directory: {dir}"));
            }
            output::note(&format!("vp {command}: using {dir} (defaultPackage in vite.config.ts)"));
            AppTarget::Dir(target)
        }
        vp_static_config::FieldValue::Json(other) => {
            fail(&format!("defaultPackage must be a string of a directory, got: {other}"))
        }
        vp_static_config::FieldValue::NonStatic => fail(
            "defaultPackage in vite.config.ts must be a static string literal so vp can read it without executing the config",
        ),
    }
}

/// Fuzzy package picker on `vt_select`, the same component behind the
/// `vp run` task selector. Returns the selected row index, or `None` on
/// Ctrl+C. When the PTY snapshot runner sets `VP_EMIT_MILESTONES=1`, every
/// render emits a `package-select:<query>:<index>` milestone (an invisible
/// window-title update) for the tests to synchronize on, same gate and
/// protocol as packages/prompts/src/milestone.ts; real terminals never see
/// the marker as content.
fn run_package_picker(command: &str, rows: &[PackageRow]) -> Result<Option<usize>, Error> {
    let emit_milestones =
        std::env::var_os(env_vars::VP_EMIT_MILESTONES).is_some_and(|value| value == "1");
    let items: Vec<vt_select::SelectItem> = rows
        .iter()
        .map(|row| vt_select::SelectItem {
            label: vt_str::format!("{} {}", row.name, row.path),
            display_name: row.name.clone(),
            description: row.path.clone(),
            group: None,
        })
        .collect();
    let prompt =
        format!("Select a package to {command} (\u{2191}/\u{2193}, Enter to run, type to search):");
    let params = vt_select::SelectParams {
        items: &items,
        query: None,
        header: None,
        prompt: &prompt,
        page_size: 12,
    };
    let mut selected_index = 0usize;
    let mut stdout = std::io::stdout();
    let result = vt_select::select_list(
        &mut stdout,
        &params,
        vt_select::Mode::Interactive { selected_index: &mut selected_index },
        |state| {
            if !emit_milestones {
                return;
            }
            let milestone =
                vt_str::format!("package-select:{}:{}", state.query, state.selected_index);
            emit_milestone_title(&milestone);
        },
    )
    .map_err(Error::Anyhow)?;
    Ok(match result {
        vt_select::SelectResult::Selected => Some(selected_index),
        vt_select::SelectResult::Cancelled => None,
    })
}

/// Emits a render-milestone as a window-title update for the PTY snapshot
/// runner, mirroring packages/prompts/src/milestone.ts:
/// `OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)> ST`. The protocol
/// is shared with vite-task's `pty_terminal_test_client`, whose emitting API
/// compiles to a no-op outside its `testing` feature (enabling that feature
/// here would also un-gate vt's own task-picker milestones in
/// production), so the sequence is written by hand. A fresh random id per
/// emission keeps repeated milestones with the same name observable as
/// distinct title changes through Windows ConPTY.
fn emit_milestone_title(name: &str) {
    use std::io::Write as _;
    let id = uuid::Uuid::new_v4();
    let encoded_name = base64_simd::URL_SAFE_NO_PAD.encode_to_string(name.as_bytes());
    let mut out = std::io::stdout().lock();
    let _ = write!(out, "\x1b]2;pty-terminal-test:{}:{encoded_name}\x1b\\", id.simple());
    let _ = out.flush();
}

/// Pure predicate for the vp-script interception: would `resolve_app_target`
/// do anything other than run in `cwd`? Never prints and never runs the
/// picker. Slightly over-approximates (an empty workspace reports true), in
/// which case the script merely spawns the real binary, which then behaves
/// identically to a direct invocation.
pub(super) fn needs_elicitation(subcommand: &SynthesizableSubcommand, cwd: &AbsolutePath) -> bool {
    matches!(classify(subcommand, cwd, false), Classification::Elicit(..))
}

/// Outcome of classifying a bare app command.
enum Classification {
    /// Run in `cwd` unchanged. Carries the workspace root found for `cwd`
    /// (when the lookup succeeded) so the caller can reuse it instead of
    /// walking the tree a second time — the hot path for a bare command deep
    /// inside a large monorepo, where the walk is the only per-invocation
    /// cost this feature adds.
    RunInPlace(Option<vt_workspace::WorkspaceRoot>),
    /// Elicit a target: `defaultPackage`, or the picker/listing at a
    /// workspace root.
    Elicit(&'static str, Elicitation),
}

/// Why a bare app command needs target elicitation.
enum Elicitation {
    /// The invocation root's config explicitly declares `defaultPackage`
    /// (with this value — possibly invalid, which the resolver reports).
    DefaultPackage(vp_static_config::FieldValue),
    /// Bare app command at a real workspace root: picker/listing territory.
    WorkspaceRoot(vt_workspace::WorkspaceRoot),
}

/// Applies a `defaultPackage` declaration to one command. A string covers
/// all four app commands; an object maps commands individually
/// (`{ dev: './apps/web', pack: './packages/ui' }`), and a command absent
/// from the object falls through to the picker/listing resolution. Every
/// other shape (a non-string, a non-static value) passes through for
/// [`resolve_default_package`] to report.
fn default_package_for_command(
    command: &str,
    value: vp_static_config::FieldValue,
) -> Option<vp_static_config::FieldValue> {
    match value {
        vp_static_config::FieldValue::Json(serde_json::Value::Object(map)) => {
            map.get(command).cloned().map(vp_static_config::FieldValue::Json)
        }
        other => Some(other),
    }
}

/// The RFC's resolution order, written once for both entry points. An explicit
/// `-C` or any subcommand argument runs in place. An exact bare app command
/// then uses `defaultPackage` at the invocation root or checks the workspace
/// root itself. `defaultPackage` is a root-pointer concept: it applies where
/// the invocation directory is its own root (a workspace root, a standalone
/// package, or a framework directory with no package.json ancestry). Below a
/// workspace root, the current directory identifies the target. A member's
/// own config must not redirect.
///
/// The one `find_workspace_root` walk here rides back out on
/// [`Classification::RunInPlace`] whenever the command ends up running in
/// `cwd`, so a bare command in a sub-package walks the tree once, not twice.
fn classify(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePath,
    explicit_chdir: bool,
) -> Classification {
    let Some((command, args)) = app_command_parts(subcommand) else {
        return Classification::RunInPlace(None);
    };
    if explicit_chdir || !args.is_empty() {
        return Classification::RunInPlace(None);
    }
    let workspace = vt_workspace::find_workspace_root(cwd);
    let at_invocation_root =
        workspace.as_ref().map_or(true, |(_, rel_from_root)| rel_from_root.as_str().is_empty());
    // Resolved once and reused by `root_has_intent_signal` below, so a bare
    // command at a root reads and parses the config a single time.
    let root_config = at_invocation_root.then(|| vp_static_config::resolve_static_config(cwd));
    if let Some(value) = root_config
        .as_ref()
        .and_then(|config| config.get_declared("defaultPackage"))
        .and_then(|value| default_package_for_command(command, value))
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
    // A workspace root with an intent signal runs in place, TTY or not. The
    // invocation already identifies its configured target. This keeps the
    // existing behavior for repos whose root is the app or library, including
    // a single package with a settings-only pnpm-workspace.yaml.
    // An empty `rel_from_root` means the invocation is at the root, so the
    // config resolved above is present; degrade to running in place rather
    // than panic if that invariant ever breaks.
    let Some(root_config) = root_config else {
        return Classification::RunInPlace(Some(workspace_root));
    };
    if root_has_intent_signal(&root_config, &workspace_root.path, command) {
        return Classification::RunInPlace(Some(workspace_root));
    }
    Classification::Elicit(command, Elicitation::WorkspaceRoot(workspace_root))
}

/// Resolve an app command's target. The second tuple element is the workspace
/// root already found for `cwd`. It is present only when the command runs in
/// the unchanged `cwd`, so it matches a fresh lookup there. The caller reuses
/// it to skip a second `find_workspace_root` walk.
pub(super) fn resolve_app_target(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePath,
    explicit_chdir: bool,
) -> Result<(AppTarget, Option<vt_workspace::WorkspaceRoot>), Error> {
    let (command, elicitation) = match classify(subcommand, cwd, explicit_chdir) {
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
        vt_workspace::load_package_graph(&workspace_root).map_err(|e| Error::Anyhow(e.into()))?;
    let mut root_row = None;
    let mut rows: Vec<PackageRow> = graph
        .node_weights()
        .filter_map(|info| {
            let absolute = info.absolute_path.to_absolute_path_buf();
            if info.path.as_str().is_empty() {
                root_row = Some(PackageRow {
                    name: info.package_json.name.clone(),
                    path: vt_str::Str::from("."),
                    absolute,
                    likely_runnable: false,
                });
                None
            } else {
                Some(PackageRow {
                    name: info.package_json.name.clone(),
                    path: vt_str::Str::from(info.path.as_str()),
                    likely_runnable: member_is_likely_runnable(&absolute, command),
                    absolute,
                })
            }
        })
        .collect();
    rows.sort_by(|a, b| {
        (!a.likely_runnable, a.path.as_str()).cmp(&(!b.likely_runnable, b.path.as_str()))
    });
    let single_likely_runnable_member = rows.first().is_some_and(|row| row.likely_runnable)
        && rows.get(1).is_none_or(|row| !row.likely_runnable);
    rows.push(root_row.unwrap_or_else(|| PackageRow {
        name: vt_str::Str::from("workspace root"),
        path: vt_str::Str::from("."),
        absolute: workspace_root.path.to_absolute_path_buf(),
        likely_runnable: false,
    }));

    // In an interactive terminal, pick the target: exactly one likely-runnable
    // member auto-selects without a menu. The root fallback does not count.
    if vp_shared::is_interactive_terminal() {
        let picked = if single_likely_runnable_member {
            Some(0)
        } else {
            run_package_picker(command, &rows)?
        };
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
    use std::fs;

    use super::*;

    #[test]
    fn vite_commands_share_root_intent_signals() {
        let temp = tempfile::tempdir().expect("temporary directory should exist");
        let dir = AbsolutePathBuf::new(temp.path().to_path_buf()).expect("path should be absolute");

        for field in ["root", "build", "input", "environments", "appType"] {
            fs::write(
                temp.path().join("vite.config.ts"),
                format!("const value = {{}}; export default {{ {field}: value }};"),
            )
            .expect("config should be written");
            let config = vp_static_config::resolve_static_config(&dir);

            for command in ["dev", "build", "preview"] {
                assert!(
                    root_has_intent_signal(&config, &dir, command),
                    "{field} should express root intent for {command}"
                );
            }
        }
    }

    #[test]
    fn vite_commands_share_the_source_index_signal() {
        let temp = tempfile::tempdir().expect("temporary directory should exist");
        let dir = AbsolutePathBuf::new(temp.path().to_path_buf()).expect("path should be absolute");
        fs::write(temp.path().join("index.html"), "").expect("index should be written");
        let config = vp_static_config::resolve_static_config(&dir);

        for command in ["dev", "build", "preview"] {
            assert!(root_has_intent_signal(&config, &dir, command));
        }

        fs::remove_file(temp.path().join("index.html")).expect("source index should be removed");
        fs::create_dir(temp.path().join("dist")).expect("output directory should be created");
        fs::write(temp.path().join("dist/index.html"), "").expect("output index should be written");

        for command in ["dev", "build", "preview"] {
            assert!(!root_has_intent_signal(&config, &dir, command));
        }
    }

    #[test]
    fn arguments_and_explicit_chdir_run_in_place() {
        let cwd = AbsolutePathBuf::new(std::env::current_dir().expect("cwd should exist"))
            .expect("cwd should be absolute");
        let commands = [
            SynthesizableSubcommand::Build { args: vec!["--watch".into()] },
            SynthesizableSubcommand::Build { args: vec!["--ssr".into()] },
            SynthesizableSubcommand::Build { args: vec!["apps/web".into()] },
            SynthesizableSubcommand::Dev { args: vec!["--host".into()] },
            SynthesizableSubcommand::Preview { args: vec!["--help".into()] },
            SynthesizableSubcommand::Pack { args: vec!["--root".into(), "src".into()] },
        ];

        for command in &commands {
            assert!(matches!(classify(command, &cwd, false), Classification::RunInPlace(None)));
        }
        assert!(matches!(
            classify(&SynthesizableSubcommand::Build { args: vec![] }, &cwd, true),
            Classification::RunInPlace(None)
        ));
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
