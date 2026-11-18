//! Notes for built-in commands that share a name with a `package.json` script.
//!
//! `vp dev` always runs the built-in dev server; the project's `dev` script is
//! a separate thing, reached with `vpr dev`. Users regularly reach for the
//! built-in when they meant the script, so a built-in whose name a script also
//! uses points at `vpr`.

use console::style;
use vp_shared::output;
use vt::MARKER_ENV_NAME;
use vt_path::AbsolutePath;

const NPM_LIFECYCLE_EVENT_ENV_NAME: &str = "npm_lifecycle_event";

/// Point a built-in command at `vpr <command>` when a script of that name
/// exists.
///
/// `command` is the built-in as the user spelled it: the global binary forwards
/// the invoked subcommand, so `vp format` stays `format` here. A direct local
/// invocation takes the first argument before help normalization.
///
/// Returns whether a note was printed, so callers can skip other hints.
pub(super) fn print(command: Option<&str>, cwd: &AbsolutePath, show_directory: bool) -> bool {
    let Some(command) = command else { return false };
    if spawned_from_script() {
        return false;
    }
    let Some(package_dir) = script_package_dir(cwd, command) else { return false };

    let built_in = style(format!("`vp {command}`")).for_stderr().blue().bright().to_string();
    let via_run = style(format!("`vpr {command}`")).for_stderr().blue().bright().to_string();
    let action = if show_directory {
        format!("change to `{}` and use", package_dir.as_path().display())
    } else {
        "use".to_string()
    };
    output::note(&format!(
        "You are running {built_in} as a Vite+ built-in command. \
         If you meant to run the {command} npm script, {action} {via_run} instead."
    ));
    true
}

/// npm-compatible runners set `npm_lifecycle_event`; Vite Task uses its own marker.
pub(super) fn spawned_from_script() -> bool {
    std::env::var_os(MARKER_ENV_NAME).is_some()
        || std::env::var_os(NPM_LIFECYCLE_EVENT_ENV_NAME).is_some_and(|event| !event.is_empty())
}

/// The nearest package directory, if it defines a `<name>` script.
///
/// Walks up to the nearest `package.json`, which is the package `vp run`
/// resolves the task from, so the note holds when a built-in runs from a
/// subdirectory. It stops there rather than climbing to a package that happens
/// to define the script: `vpr <name>` would not reach that one either.
fn script_package_dir<'a>(cwd: &'a AbsolutePath, name: &str) -> Option<&'a AbsolutePath> {
    let package = vt_workspace::find_package_root(cwd).ok()?;
    let content = package.package_json.content();
    let manifest: serde_json::Value =
        serde_json::from_slice(content.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(content)).ok()?;
    manifest
        .get("scripts")
        .and_then(|scripts| scripts.get(name))
        .is_some_and(serde_json::Value::is_string)
        .then_some(package.path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;
    use vt_path::AbsolutePathBuf;

    use super::script_package_dir;

    #[test]
    fn finds_scripts_only_in_the_nearest_package_with_or_without_a_bom() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let app = root.join("app");
        let source = app.join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(root.join("package.json"), r#"{"scripts":{"dev":"root-dev"}}"#).unwrap();
        fs::write(app.join("package.json"), "{}").unwrap();
        assert_eq!(script_package_dir(&source, "dev"), None);

        for prefix in ["", "\u{feff}"] {
            fs::write(
                app.join("package.json"),
                format!("{prefix}{}", r#"{"scripts":{"dev":"nuxt dev"}}"#),
            )
            .unwrap();
            assert_eq!(script_package_dir(&source, "dev"), Some(&*app));
            assert_eq!(script_package_dir(&source, "build"), None);
        }
    }
}
