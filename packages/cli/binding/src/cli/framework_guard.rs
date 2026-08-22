//! Refusal for framework projects whose own CLI wraps Vite.
//!
//! `vp dev` and `vp build` run the bundled Vite CLI. Nuxt and Astro run Vite
//! only through their own CLIs. The bundled Vite CLI cannot serve or build
//! these projects: dev answers every URL with 404, and build stops on the
//! missing `index.html` entry. When a framework config file is next to the
//! nearest `package.json`, the two commands stop with an error and a hint
//! (voidzero-dev/vite-plus#1506). The hint points at the `package.json`
//! script that runs the framework command, or at the framework CLI through
//! `vp exec` when no script matches. The guard checks direct invocations
//! only: a command spawned from a task or package script, for example a
//! `"dev": "vp dev"` script, runs as invoked. The guarded directory is the
//! one Vite would use: the invocation directory, a positional root, or the
//! target that `defaultPackage`/elicitation resolved. Help and version
//! requests reach the tool, and an explicit `--config`/`-c` flag selects a
//! Vite config on purpose, so both skip the refusal.

use owo_colors::OwoColorize;
use vp_shared::output;
use vt::ExitStatus;
use vt_path::AbsolutePath;

use super::types::SynthesizableSubcommand;

/// Frameworks that wrap Vite behind their own CLI. Each entry lists the
/// config files that the framework's loader resolves, in its resolution
/// order.
const FRAMEWORKS: &[Framework] = &[
    // Nuxt resolves `nuxt.config` through c12. `loadNuxtConfig` passes
    // `configFile: "nuxt.config"`
    // (https://github.com/nuxt/nuxt/blob/v4.5.2/packages/kit/src/loader/config.ts),
    // and c12 tries the script extensions in `SUPPORTED_EXTENSIONS`
    // (https://github.com/unjs/c12/blob/v3.3.4/src/loader.ts). c12 also
    // accepts data configs (`.json`, `.jsonc`, `.json5`, `.yaml`, `.yml`,
    // `.toml`) and rc files. Those are rare, so the guard does not check
    // them.
    Framework {
        name: "Nuxt",
        config_files: &[
            "nuxt.config.js",
            "nuxt.config.ts",
            "nuxt.config.mjs",
            "nuxt.config.cjs",
            "nuxt.config.mts",
            "nuxt.config.cts",
        ],
        // The nuxt package ships the `nuxt` and `nuxi` bins (its `bin`
        // field, verified against nuxt 4.5.2).
        bins: &["nuxt", "nuxi"],
    },
    // Astro searches only these four names: `configPaths` in
    // https://github.com/withastro/astro/blob/astro@7.2.2/packages/astro/src/core/config/config.ts.
    // Astro does not load a `.cjs` or `.cts` config.
    Framework {
        name: "Astro",
        config_files: &[
            "astro.config.mjs",
            "astro.config.js",
            "astro.config.ts",
            "astro.config.mts",
        ],
        // The astro package ships the `astro` bin (its `bin` field,
        // verified against astro 7.2.2).
        bins: &["astro"],
    },
];

struct Framework {
    name: &'static str,
    config_files: &'static [&'static str],
    /// Executable names of the framework CLI. The first one is the name the
    /// `vp exec` hint shows.
    bins: &'static [&'static str],
}

/// Refuse `vp dev` / `vp build` in a Nuxt or Astro project.
///
/// `retarget` is the path that elicitation or `defaultPackage` resolved for
/// a bare command, relative to the invocation directory; hints then carry
/// the matching `-C <target>` so they run in the refused package. Returns
/// the exit status after it prints the refusal, or `None` when the command
/// can proceed.
pub(super) fn check(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePath,
    retarget: Option<&str>,
) -> Option<ExitStatus> {
    let refusal = find_refusal(subcommand, cwd, retarget)?;
    let built_in = format!("`vp {}`", refusal.command).bright_blue().to_string();
    output::error(&format!(
        "this project uses {name} ({config_file}). {built_in} runs the bundled Vite CLI, \
         not the {name} CLI.",
        name = refusal.framework.name,
        config_file = refusal.config_file,
    ));
    output::raw_stderr(&format!("hint: {}", run_hint(&refusal)));
    Some(ExitStatus(1))
}

/// Whether `check` would refuse, without output. The script note and
/// workspace elicitation ask this before they act: a note must not
/// recommend `vpr` right before a refusal, and a package listing must not
/// hide one.
pub(super) fn applies(subcommand: &SynthesizableSubcommand, cwd: &AbsolutePath) -> bool {
    find_refusal(subcommand, cwd, None).is_some()
}

struct Refusal {
    command: &'static str,
    framework: &'static Framework,
    config_file: &'static str,
    /// The enclosing `package.json` of the refused directory, for the hint.
    manifest: serde_json::Value,
    /// `-C` target for the hint when the refused directory is not the
    /// invocation directory (a positional root or a resolved retarget).
    target: Option<String>,
}

/// The refusal for this invocation, or `None` when the command can proceed.
fn find_refusal(
    subcommand: &SynthesizableSubcommand,
    cwd: &AbsolutePath,
    retarget: Option<&str>,
) -> Option<Refusal> {
    let (command, args) = match subcommand {
        SynthesizableSubcommand::Dev { args } => ("dev", args),
        SynthesizableSubcommand::Build { args } => ("build", args),
        _ => return None,
    };
    // A task or package script can spawn `vp dev` itself, for example a
    // `"dev": "vp dev"` script. The nested command runs as invoked.
    if super::script_note::spawned_from_script() {
        return None;
    }
    // The tool's own arg walk decides what the guard inspects: a positional
    // is the Vite root, and an explicit invocation (a help/version request,
    // a `-c`/`--config` file) must reach the tool untouched.
    let positional_root;
    let (dir, target): (&AbsolutePath, Option<&str>) =
        match super::app_target::classify_args(command, args) {
            super::app_target::ArgTarget::Explicit => return None,
            super::app_target::ArgTarget::Positional(root) => {
                positional_root = cwd.join(root).clean();
                (&positional_root, Some(root))
            }
            super::app_target::ArgTarget::Bare => (cwd, retarget),
        };
    // `vp run` resolves the task from the nearest `package.json`. The same
    // walk here keeps the hint correct from a subdirectory.
    let package = vt_workspace::find_package_root(dir).ok()?;
    let (framework, config_file) = detect(package.path)?;
    let manifest = serde_json::from_slice::<serde_json::Value>(package.package_json.content())
        .unwrap_or(serde_json::Value::Null);
    Some(Refusal { command, framework, config_file, manifest, target: target.map(str::to_string) })
}

/// The hint that follows the refusal. It points at the first path that works
/// in the refused package:
///
/// 1. the `package.json` script with the command's name, when its command
///    runs the framework (a `"dev": "storybook dev"` must not become the
///    hint),
/// 2. another script that runs the framework command,
/// 3. the framework CLI through `vp exec`.
///
/// The check reads `package.json` scripts only. A `run.tasks` entry in
/// `vite.config.ts` with the command's name also works with `vp run`, but
/// the guard does not load that config.
fn run_hint(refusal: &Refusal) -> String {
    let Refusal { command, framework, manifest, target, .. } = refusal;
    let vp = match target {
        Some(target) => format!("vp -C {target}"),
        None => "vp".to_string(),
    };
    if let Some(scripts) = manifest.get("scripts").and_then(serde_json::Value::as_object) {
        let named = scripts.get(*command).and_then(serde_json::Value::as_str);
        if named.is_some_and(|script| framework_invocation(script, framework, command).is_some()) {
            let via_run = format!("`{vp} run {command}`").bright_blue().to_string();
            return format!("did you mean {via_run}?");
        }
        for (name, script) in scripts {
            if name == command {
                continue;
            }
            let Some(script) = script.as_str() else { continue };
            if let Some(invocation) = framework_invocation(script, framework, command) {
                let via_run = format!("`{vp} run {name}`").bright_blue().to_string();
                return format!("did you mean {via_run}? The {name} script runs `{invocation}`.");
            }
        }
    }
    let via_exec = format!("`{vp} exec {} {command}`", framework.bins[0]).bright_blue().to_string();
    format!("run the {} CLI with {via_exec}.", framework.name)
}

/// The `<bin> <command>` invocation inside `script`, when the script runs
/// the framework command.
fn framework_invocation(script: &str, framework: &Framework, command: &str) -> Option<String> {
    framework
        .bins
        .iter()
        .map(|bin| format!("{bin} {command}"))
        .find(|invocation| contains_word(script, invocation))
}

/// Whether `text` contains `pattern` between whitespace boundaries, so
/// `nuxt dev` does not match inside `nuxt devtools`.
fn contains_word(text: &str, pattern: &str) -> bool {
    let mut search_from = 0;
    while let Some(found) = text[search_from..].find(pattern) {
        let start = search_from + found;
        let end = start + pattern.len();
        let boundary_before = start == 0 || text[..start].ends_with(char::is_whitespace);
        let boundary_after = end == text.len() || text[end..].starts_with(char::is_whitespace);
        if boundary_before && boundary_after {
            return true;
        }
        search_from = start + 1;
    }
    false
}

/// The first framework config file present in `dir`.
fn detect(dir: &AbsolutePath) -> Option<(&'static Framework, &'static str)> {
    for framework in FRAMEWORKS {
        for &config_file in framework.config_files {
            if dir.join(config_file).as_path().is_file() {
                return Some((framework, config_file));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use vt_path::AbsolutePathBuf;

    use super::{
        super::app_target::{ArgTarget, classify_args},
        FRAMEWORKS, Refusal, contains_word, detect, run_hint,
    };

    fn framework(name: &str) -> &'static super::Framework {
        FRAMEWORKS.iter().find(|framework| framework.name == name).expect("known framework")
    }

    fn temp_dir(label: &str) -> AbsolutePathBuf {
        let unique = format!(
            "vp-framework-guard-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        AbsolutePathBuf::new(dir).expect("temp dir is absolute")
    }

    #[test]
    fn detects_nuxt_before_astro_across_extensions() {
        let dir = temp_dir("detect");
        std::fs::write(dir.as_path().join("astro.config.mjs"), "export default {}")
            .expect("write astro config");
        std::fs::write(dir.as_path().join("nuxt.config.mts"), "export default {}")
            .expect("write nuxt config");

        let (framework, config_file) = detect(&dir).expect("framework detected");
        assert_eq!(framework.name, "Nuxt");
        assert_eq!(config_file, "nuxt.config.mts");

        std::fs::remove_dir_all(dir.as_path()).expect("remove temp dir");
    }

    #[test]
    fn ignores_directories_and_unrelated_files() {
        let dir = temp_dir("ignore");
        std::fs::create_dir_all(dir.as_path().join("nuxt.config.ts")).expect("create dir");
        std::fs::write(dir.as_path().join("vite.config.ts"), "export default {}")
            .expect("write vite config");

        assert!(detect(&dir).is_none());

        std::fs::remove_dir_all(dir.as_path()).expect("remove temp dir");
    }

    #[test]
    fn ignores_config_names_astro_does_not_load() {
        let dir = temp_dir("astro-cjs");
        std::fs::write(dir.as_path().join("astro.config.cjs"), "module.exports = {}")
            .expect("write cjs config");
        std::fs::write(dir.as_path().join("astro.config.cts"), "module.exports = {}")
            .expect("write cts config");

        assert!(detect(&dir).is_none());

        std::fs::remove_dir_all(dir.as_path()).expect("remove temp dir");
    }

    fn refusal(
        name: &str,
        command: &'static str,
        manifest: serde_json::Value,
        target: Option<&str>,
    ) -> Refusal {
        Refusal {
            command,
            framework: framework(name),
            config_file: "nuxt.config.ts",
            manifest,
            target: target.map(str::to_string),
        }
    }

    #[test]
    fn hint_prefers_the_script_with_the_command_name() {
        let manifest = serde_json::json!({ "scripts": { "dev": "nuxt dev" } });
        let hint = run_hint(&refusal("Nuxt", "dev", manifest, None));
        assert!(hint.contains("vp run dev"), "hint was: {hint}");
    }

    #[test]
    fn hint_skips_a_same_named_script_that_runs_something_else() {
        let manifest = serde_json::json!({ "scripts": {
            "dev": "storybook dev",
            "serve": "nuxt dev --host",
        } });
        let hint = run_hint(&refusal("Nuxt", "dev", manifest, None));
        assert!(hint.contains("vp run serve"), "hint was: {hint}");
        assert!(!hint.contains("vp run dev"), "hint was: {hint}");

        let only_unrelated = serde_json::json!({ "scripts": { "dev": "storybook dev" } });
        let hint = run_hint(&refusal("Nuxt", "dev", only_unrelated, None));
        assert!(hint.contains("vp exec nuxt dev"), "hint was: {hint}");
    }

    #[test]
    fn hint_finds_a_renamed_script_that_runs_the_framework_command() {
        let manifest = serde_json::json!({ "scripts": {
            "devtools": "nuxt devtools enable",
            "start": "NODE_OPTIONS=--inspect nuxi dev --host",
        } });
        let hint = run_hint(&refusal("Nuxt", "dev", manifest, None));
        assert!(hint.contains("vp run start"), "hint was: {hint}");
        assert!(hint.contains("nuxi dev"), "hint was: {hint}");
    }

    #[test]
    fn hint_falls_back_to_vp_exec_without_a_matching_script() {
        let hint = run_hint(&refusal("Nuxt", "dev", serde_json::json!({}), None));
        assert!(hint.contains("vp exec nuxt dev"), "hint was: {hint}");

        let unrelated = serde_json::json!({ "scripts": { "lint": "oxlint ." } });
        let hint = run_hint(&refusal("Astro", "build", unrelated, None));
        assert!(hint.contains("vp exec astro build"), "hint was: {hint}");
    }

    #[test]
    fn hint_carries_the_resolved_target() {
        let manifest = serde_json::json!({ "scripts": { "dev": "nuxt dev" } });
        let hint = run_hint(&refusal("Nuxt", "dev", manifest, Some("apps/web")));
        assert!(hint.contains("vp -C apps/web run dev"), "hint was: {hint}");

        let hint = run_hint(&refusal("Nuxt", "build", serde_json::json!({}), Some("app")));
        assert!(hint.contains("vp -C app exec nuxt build"), "hint was: {hint}");
    }

    #[test]
    fn contains_word_needs_whitespace_boundaries() {
        assert!(contains_word("nuxt dev", "nuxt dev"));
        assert!(contains_word("NODE_OPTIONS=--inspect nuxt dev --host", "nuxt dev"));
        assert!(!contains_word("nuxt devtools enable", "nuxt dev"));
        assert!(!contains_word("pnpm-nuxt dev", "nuxt dev"));
    }

    #[test]
    fn arg_classification_drives_the_guarded_directory() {
        let owned = |args: &[&str]| args.iter().map(|arg| (*arg).to_string()).collect::<Vec<_>>();
        // Explicit invocations reach the tool untouched.
        for args in [
            vec!["--help"],
            vec!["-h"],
            vec!["--version"],
            vec!["-v"],
            vec!["--port", "5000", "--help"],
            vec!["--config", "vite.config.ts"],
            vec!["--config=vite.config.ts"],
            vec!["-c", "vite.config.ts"],
        ] {
            assert!(
                matches!(classify_args("dev", &owned(&args)), ArgTarget::Explicit),
                "expected Explicit for {args:?}"
            );
        }
        // A positional is the Vite root and becomes the guarded directory.
        assert!(matches!(classify_args("dev", &owned(&["web"])), ArgTarget::Positional("web")));
        assert!(matches!(
            classify_args("dev", &owned(&["--cors", "web"])),
            ArgTarget::Positional("web")
        ));
        // A flag value is not a positional; a bare command guards the
        // invocation directory.
        assert!(matches!(classify_args("dev", &owned(&["--port", "5000"])), ArgTarget::Bare));
    }
}
