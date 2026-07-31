//! Notes for built-in commands that share a name with a `package.json` script.
//!
//! `vp dev` always runs the built-in dev server; the project's `dev` script is
//! a separate thing, reached with `vpr dev`. Users regularly reach for the
//! built-in when they meant the script, so a built-in whose name a script also
//! uses points at `vpr`.

use owo_colors::OwoColorize;
use vite_path::AbsolutePath;
use vite_shared::output;
use vite_task::MARKER_ENV_NAME;

const NPM_LIFECYCLE_EVENT_ENV_NAME: &str = "npm_lifecycle_event";

/// Point a built-in command at `vpr <command>` when a script of that name
/// exists.
///
/// `command` is the built-in as the user spelled it: the global binary forwards
/// the invoked subcommand, so `vp format` stays `format` here. A direct local
/// invocation takes the first argument before help normalization.
pub(super) fn print(command: Option<&str>, cwd: &AbsolutePath) {
    let Some(command) = command else { return };
    // A task spawned this command, so the user is already on a script-running
    // path. npm-compatible runners set `npm_lifecycle_event`; Vite Task uses
    // its own marker.
    if std::env::var_os(MARKER_ENV_NAME).is_some()
        || std::env::var_os(NPM_LIFECYCLE_EVENT_ENV_NAME).is_some_and(|event| !event.is_empty())
    {
        return;
    }
    if !has_package_json_script(cwd, command) {
        return;
    }

    let built_in = format!("`vp {command}`").bright_blue().to_string();
    let via_run = format!("`vpr {command}`").bright_blue().to_string();
    output::note(&format!(
        "You are running {built_in} as a Vite+ built-in command. \
         If you meant to run the {command} npm script, use {via_run} instead."
    ));
}

/// Whether the package enclosing `cwd` defines a `<name>` script.
///
/// Walks up to the nearest `package.json`, which is the package `vp run`
/// resolves the task from, so the note holds when a built-in runs from a
/// subdirectory. It stops there rather than climbing to a package that happens
/// to define the script: `vpr <name>` would not reach that one either.
fn has_package_json_script(cwd: &AbsolutePath, name: &str) -> bool {
    let Ok(package) = vite_workspace::find_package_root(cwd) else { return false };
    serde_json::from_slice::<serde_json::Value>(package.package_json.content()).is_ok_and(
        |manifest| {
            manifest
                .get("scripts")
                .and_then(|scripts| scripts.get(name))
                .is_some_and(serde_json::Value::is_string)
        },
    )
}
