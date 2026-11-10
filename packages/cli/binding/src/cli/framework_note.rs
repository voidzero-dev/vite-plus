//! A hint for bare `vp dev` in packages that use a framework's own CLI.
//!
//! Existing `dev` scripts already get the script note. Without a script or
//! task, point at the framework CLI without changing command routing.

use console::style;
use vp_shared::output;
use vt_path::AbsolutePath;

use super::{app_target, script_note, types::SynthesizableSubcommand};

/// Returns whether the hint was printed, so the caller can keep it visible.
pub(super) fn print(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePath,
    show_directory: bool,
) -> bool {
    // Explicit Vite arguments can select another root or config. Keep this
    // hint scoped to bare dev, after defaultPackage / package selection.
    if !matches!(subcommand, SynthesizableSubcommand::Dev { args } if args.is_empty())
        || script_note::spawned_from_script()
    {
        return false;
    }
    let Some((name, command, package_dir)) = detect(cwd) else { return false };
    let built_in = style("`vp dev`").for_stderr().blue().bright().to_string();
    let framework_command =
        style(format!("`vp exec {command}`")).for_stderr().blue().bright().to_string();
    // Retargeting changes the child process's directory, not the user's shell.
    // Name the package directory explicitly instead of assuming it is the cwd.
    let action = if show_directory || package_dir != cwd {
        format!("change to `{}` and run", package_dir.as_path().display())
    } else {
        "run".to_string()
    };
    output::note(&format!(
        "This package depends on {name} and has no dev script or task. \
         {built_in} starts the Vite dev server. \
         To start the {name} dev server, {action} {framework_command}."
    ));
    true
}

fn detect(cwd: &AbsolutePath) -> Option<(&'static str, &'static str, &AbsolutePath)> {
    // Stop at the nearest package, just as `vp run` does. A dependency in a
    // workspace root or sibling must not cause a hint for this package.
    let package = vt_workspace::find_package_root(cwd).ok()?;
    let manifest: serde_json::Value =
        serde_json::from_slice(package.package_json.content()).ok()?;
    if manifest
        .get("scripts")
        .and_then(|scripts| scripts.get("dev"))
        .is_some_and(serde_json::Value::is_string)
    {
        return None;
    }
    // These frameworks use their own dev commands. Do not include frameworks
    // such as SvelteKit that start their dev server through the Vite CLI.
    let (name, dependency, mut command) = [
        ("Astro", "astro", "astro dev"),
        ("Nuxt", "nuxt", "nuxt dev"),
        ("VitePress", "vitepress", "vitepress dev"),
    ]
    .into_iter()
    .find(|(_, dependency, _)| {
        ["dependencies", "devDependencies"].iter().any(|field| {
            manifest
                .get(field)
                .and_then(|deps| deps.get(dependency))
                .is_some_and(serde_json::Value::is_string)
        })
    })?;
    let config = vp_static_config::resolve_static_config(package.path);
    match config.get("run") {
        Some(vp_static_config::FieldValue::Json(run)) => {
            if run.get("tasks").and_then(|tasks| tasks.get("dev")).is_some() {
                return None;
            }
        }
        // A dynamic config could define a dev task. Do not suggest a command
        // that bypasses it, or execute the config just to produce a hint.
        Some(vp_static_config::FieldValue::NonStatic) => return None,
        None => {}
    }
    if dependency == "vitepress" {
        // A Vite app can also depend on VitePress for its documentation. Avoid
        // a docs hint for that app, and only suggest a known documentation root.
        if app_target::root_has_intent_signal(&config, package.path, "dev") {
            return None;
        }
        command = if package.path.join(".vitepress").as_path().is_dir() {
            "vitepress dev"
        } else if package.path.join("docs/.vitepress").as_path().is_dir() {
            "vitepress dev docs"
        } else {
            return None;
        };
    }
    Some((name, command, package.path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::TempDir;
    use vt_path::AbsolutePathBuf;

    use super::detect;

    #[test]
    fn detects_direct_framework_dependencies_without_a_dev_script() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        fs::create_dir(root.join(".vitepress")).unwrap();
        for field in ["dependencies", "devDependencies"] {
            for (name, dependency, command) in [
                ("Astro", "astro", "astro dev"),
                ("Nuxt", "nuxt", "nuxt dev"),
                ("VitePress", "vitepress", "vitepress dev"),
            ] {
                fs::write(
                    root.join("package.json"),
                    json!({ field: { dependency: "*" } }).to_string(),
                )
                .unwrap();
                assert_eq!(detect(&root), Some((name, command, &*root)));
            }
        }
    }

    #[test]
    fn skips_existing_scripts_and_unrelated_dependencies() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        fs::create_dir(root.join(".vitepress")).unwrap();
        for manifest in [
            json!({ "dependencies": { "astro": "*" }, "scripts": { "dev": "astro dev" } }),
            json!({ "dependencies": { "nuxt": "*" }, "scripts": { "dev": "vp dev" } }),
            json!({ "devDependencies": { "vitepress": "*" }, "scripts": { "dev": "vitepress dev" } }),
            json!({ "peerDependencies": { "astro": "*" } }),
            json!({ "peerDependencies": { "vitepress": "*" } }),
            json!({ "optionalDependencies": { "astro": "*", "nuxt": "*", "vitepress": "*" } }),
            json!({ "dependencies": { "@solidjs/start": "*", "vinxi": "*" } }),
            json!({ "dependencies": { "@astrojs/react": "*", "@nuxt/kit": "*", "@sveltejs/kit": "*", "vue": "*", "react": "*" } }),
            json!({}),
        ] {
            fs::write(root.join("package.json"), manifest.to_string()).unwrap();
            assert_eq!(detect(&root), None, "{manifest}");
        }
    }

    #[test]
    fn uses_the_nearest_package_from_a_subdirectory() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let child = root.join("child");
        let source = child.join("src");
        fs::create_dir_all(&source).unwrap();
        fs::write(root.join("package.json"), r#"{"dependencies":{"astro":"*"}}"#).unwrap();
        fs::write(child.join("package.json"), "{}").unwrap();
        assert_eq!(detect(&source), None);

        fs::write(child.join("package.json"), r#"{"devDependencies":{"nuxt":"*"}}"#).unwrap();
        assert_eq!(detect(&source), Some(("Nuxt", "nuxt dev", &*child)));

        fs::write(
            child.join("vite.config.ts"),
            "export default { run: { tasks: { dev: { command: 'nuxt dev' } } } };",
        )
        .unwrap();
        assert_eq!(detect(&source), None);
    }

    #[test]
    fn skips_existing_tasks_and_configs_that_could_define_them() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        fs::write(root.join("package.json"), r#"{"dependencies":{"astro":"*"}}"#).unwrap();
        for config in [
            "export default { run: { tasks: { dev: 'astro dev' } } };",
            "export default { run: { tasks: { dev: ['astro', 'dev'] } } };",
            "export default { run: { tasks: { dev: { command: 'astro dev' } } } };",
            "import run from './run'; export default { run };",
            "import tasks from './tasks'; export default { run: { tasks } };",
            "import base from './base'; export default { ...base };",
            "export { default } from './base';",
            "export default () => ({ run: { tasks: { dev: { command: 'astro dev' } } } });",
        ] {
            fs::write(root.join("vite.config.ts"), config).unwrap();
            assert_eq!(detect(&root), None, "{config}");
        }
    }

    #[test]
    fn allows_configs_without_a_dev_task() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        fs::write(root.join("package.json"), r#"{"dependencies":{"astro":"*"}}"#).unwrap();
        for config in [
            "export default {};",
            "import plugin from './plugin'; export default { plugins: [plugin()] };",
            "export default { run: { tasks: { build: { command: 'astro build' } } } };",
            "import base from './base'; export default { ...base, run: {} };",
        ] {
            fs::write(root.join("vite.config.ts"), config).unwrap();
            assert_eq!(detect(&root), Some(("Astro", "astro dev", &*root)), "{config}");
        }
    }

    #[test]
    fn detects_vitepress_roots_without_overriding_vite_app_intent() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        fs::write(root.join("package.json"), r#"{"devDependencies":{"vitepress":"*"}}"#).unwrap();
        // A dependency alone does not identify which directory contains the docs.
        assert_eq!(detect(&root), None);
        fs::create_dir_all(root.join("docs/.vitepress")).unwrap();
        assert_eq!(detect(&root), Some(("VitePress", "vitepress dev docs", &*root)));
        assert_eq!(detect(&root.join("docs")), Some(("VitePress", "vitepress dev docs", &*root)));

        fs::create_dir(root.join(".vitepress")).unwrap();
        assert_eq!(detect(&root), Some(("VitePress", "vitepress dev", &*root)));
        fs::write(root.join("index.html"), "<div id='app'></div>").unwrap();
        assert_eq!(detect(&root), None);
        fs::remove_file(root.join("index.html")).unwrap();
        for config in [
            "export default { root: 'app' };",
            "export default { build: { lib: { entry: 'src/index.ts' } } };",
            "export default { run: { tasks: { dev: { command: 'vitepress dev docs' } } } };",
            "export default () => ({});",
        ] {
            fs::write(root.join("vite.config.ts"), config).unwrap();
            assert_eq!(detect(&root), None, "{config}");
        }
    }

    #[test]
    fn ignores_missing_or_invalid_manifests() {
        let temp = TempDir::new().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        assert_eq!(detect(&root), None);
        fs::write(root.join("package.json"), "invalid json").unwrap();
        assert_eq!(detect(&root), None);
    }
}
