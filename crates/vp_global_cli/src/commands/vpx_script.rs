//! `vpx <script>`: run a script file on the project's Node.js with the oxc-node
//! loader that is compiled into the Vite+ native binding. `vpx --watch <script>` runs
//! oxc-node's watcher instead of Node.js' own watch mode, and the watcher reruns the
//! script with that loader.
//!
//! See `rfcs/vpx-script-execution.md`. The CLI/loader contract is
//! `node --require <dist/script-preload.cjs> --import <file URL of dist/script-register.js>`
//! plus the `VP_SCRIPT_TSCONFIG` environment variable, and `node <dist/script-watch.js>
//! <watcher args>` for `--watch`; keep it stable, because the global `vp` may run a newer
//! or older project-local `vite-plus` loader.

use std::io::Read as _;

use vp_shared::{env_vars, output};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::js_executor::JsExecutor;

/// `vite-plus/dist` entries: the CommonJS `--require` preload and the `--import`.
const PRELOAD_ENTRY: &str = "script-preload.cjs";
const IMPORT_ENTRY: &str = "script-register.js";
/// The `vpx --watch` watcher, which runs scripts with the loader next to it.
const WATCH_ENTRY: &str = "script-watch.js";

/// What `vpx <script>` needs from `vite-plus/dist`.
const SCRIPT_ENTRIES: &[&str] = &[PRELOAD_ENTRY, IMPORT_ENTRY];
/// What `vpx --watch <script>` needs from `vite-plus/dist`.
const WATCH_ENTRIES: &[&str] = &[PRELOAD_ENTRY, IMPORT_ENTRY, WATCH_ENTRY];

/// Extensions that make a token a script.
const SCRIPT_EXTENSIONS: &[&str] = &[".ts", ".mts", ".cts", ".tsx", ".js", ".mjs", ".cjs", ".jsx"];

/// A missing file with one of these is an error, never a package name.
const TS_EXTENSIONS: &[&str] = &[".ts", ".mts", ".cts", ".tsx"];

/// Node.js options that take a required value, which may come as the next argument:
/// those `node --help` lists as `--option=...` on Node.js 22, 24, and 26. Best effort:
/// a missing entry only moves the missing-file check for the script to Node.js.
const NODE_OPTIONS_WITH_VALUE: &[&str] = &[
    "-C",
    "-e",
    "-p",
    "-pe",
    "-r",
    "--allow-fs-read",
    "--allow-fs-write",
    "--build-sea",
    "--build-snapshot-config",
    "--conditions",
    "--cpu-prof-dir",
    "--cpu-prof-interval",
    "--cpu-prof-name",
    "--debug-port",
    "--diagnostic-dir",
    "--disable-proto",
    "--disable-warning",
    "--dns-result-order",
    "--env-file",
    "--env-file-if-exists",
    "--eval",
    "--experimental-config-file",
    "--experimental-default-type",
    "--experimental-loader",
    "--experimental-package-map",
    "--experimental-sea-config",
    "--experimental-test-isolation",
    "--experimental-test-tag-filter",
    "--heap-prof-dir",
    "--heap-prof-interval",
    "--heap-prof-name",
    "--heapsnapshot-near-heap-limit",
    "--heapsnapshot-signal",
    "--icu-data-dir",
    "--import",
    "--input-type",
    "--inspect-port",
    "--inspect-publish-uid",
    "--loader",
    "--localstorage-file",
    "--max-http-header-size",
    "--max-old-space-size-percentage",
    "--network-family-autoselection-attempt-timeout",
    "--openssl-config",
    "--print",
    "--redirect-warnings",
    "--report-dir",
    "--report-directory",
    "--report-filename",
    "--report-signal",
    "--require",
    "--run",
    "--secure-heap",
    "--secure-heap-min",
    "--snapshot-blob",
    "--test-concurrency",
    "--test-coverage-branches",
    "--test-coverage-exclude",
    "--test-coverage-functions",
    "--test-coverage-include",
    "--test-coverage-lines",
    "--test-global-setup",
    "--test-isolation",
    "--test-name-pattern",
    "--test-random-seed",
    "--test-reporter",
    "--test-reporter-destination",
    "--test-rerun-failures",
    "--test-shard",
    "--test-skip-pattern",
    "--test-timeout",
    "--title",
    "--tls-cipher-list",
    "--tls-keylog",
    "--trace-event-categories",
    "--trace-event-file-pattern",
    "--trace-require-module",
    "--unhandled-rejections",
    "--use-largepages",
    "--v8-pool-size",
    "--watch-kill-signal",
    "--watch-path",
];

/// Node.js options after which positionals are not an entry file: the code of `--eval` /
/// `--print` or the package script of `--run` runs instead, and `--test` takes test files
/// and globs.
const NODE_OPTIONS_WITHOUT_ENTRY: &[&str] =
    &["-e", "--eval", "-p", "-pe", "--print", "--run", "--test"];

/// The watcher's options that take a value, which may come as the next argument, so that
/// value is not taken for the script.
const WATCH_OPTIONS_WITH_VALUE: &[&str] =
    &["--exclude", "--ignore", "--include", "--kill-signal", "--kill-timeout"];

/// Whether `options` lists the option `name` (an argument without `=value`). Node.js
/// accepts `_` for `-` in long option names, as in `--redirect_warnings`.
fn lists_option(options: &[&str], name: &str) -> bool {
    options.iter().any(|option| {
        option.len() == name.len()
            && option.bytes().zip(name.bytes()).enumerate().all(|(index, (expected, actual))| {
                expected == actual || (index >= 2 && expected == b'-' && actual == b'_')
            })
    })
}

/// Whether the option `name`, given without `=value`, takes `next` as its value. Node.js
/// 24.19, 26.1, and later read a bare `--experimental-config-file` as `node.config.json`,
/// and older releases take the next argument as the config file, so that argument is its
/// value only when it is a `.json` file, which cannot be the script either way.
fn takes_next_arg(name: &str, next: &str) -> bool {
    if lists_option(&["--experimental-config-file"], name) {
        return next.ends_with(".json") && !next.starts_with('-');
    }
    lists_option(NODE_OPTIONS_WITH_VALUE, name) || lists_option(WATCH_OPTIONS_WITH_VALUE, name)
}

/// A `vpx` invocation that runs Node.js with the script loader.
#[derive(Debug, PartialEq, Eq)]
pub struct ScriptInvocation {
    /// Node.js options, the script, and its arguments, in the order given.
    pub node_args: Vec<String>,
    /// `--tsconfig` given among the Node.js options.
    pub tsconfig: Option<String>,
    /// `--watch` or `--watch-path` among the Node.js options: the watcher runs the script,
    /// never Node.js' own watch mode. `--watch` is removed from `node_args`; the watcher
    /// rejects `--watch-path` with a hint.
    pub watch: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScriptError {
    NotFound(String),
    IsDirectory(String),
    MissingTsconfigValue,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(script) => write!(f, "Script not found: {script}"),
            Self::IsDirectory(script) => write!(f, "Script is a directory: {script}"),
            Self::MissingTsconfigValue => write!(f, "--tsconfig requires a path"),
        }
    }
}

/// Decide whether `positional` (the arguments after the `vpx` options) runs a
/// script. Returns `None` for package mode. Never rewrites the arguments except
/// for removing `--tsconfig` and `--watch`.
pub fn detect(
    positional: &[String],
    cwd: &AbsolutePath,
) -> Result<Option<ScriptInvocation>, ScriptError> {
    let Some(first) = positional.first() else {
        return Ok(None);
    };

    if !first.starts_with('-') {
        return Ok(is_script(first, cwd)?.then(|| ScriptInvocation {
            node_args: positional.to_vec(),
            tsconfig: None,
            watch: false,
        }));
    }

    // A leading option never named a package bin, so this always runs Node.js.
    // The first non-option token is the script (or `-` for stdin); everything
    // from it on belongs to the script.
    let mut node_args = Vec::with_capacity(positional.len());
    let mut tsconfig = None;
    let mut watch = false;
    let mut entry = true;
    let mut index = 0;
    while let Some(arg) = positional.get(index) {
        if arg == "--" || arg == "-" || !arg.starts_with('-') {
            let script = if arg == "--" { positional.get(index + 1) } else { Some(arg) };
            if let Some(script) = script.filter(|_| entry) {
                check_script_after_options(script, cwd)?;
            }
            node_args.extend_from_slice(&positional[index..]);
            break;
        }
        if arg == "--tsconfig" {
            tsconfig =
                Some(positional.get(index + 1).ok_or(ScriptError::MissingTsconfigValue)?.clone());
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--tsconfig=") {
            tsconfig = Some(value.to_owned());
            index += 1;
            continue;
        }
        if arg == "--watch" {
            watch = true;
            index += 1;
            continue;
        }
        let name = arg.split_once('=').map_or(arg.as_str(), |(name, _)| name);
        watch |= lists_option(&["--watch-path"], name);
        entry &= !lists_option(NODE_OPTIONS_WITHOUT_ENTRY, name);
        node_args.push(arg.clone());
        if name.len() == arg.len()
            && let Some(value) = positional.get(index + 1)
            && takes_next_arg(name, value)
        {
            node_args.push(value.clone());
            index += 1;
        }
        index += 1;
    }

    Ok(Some(ScriptInvocation { node_args, tsconfig, watch }))
}

/// After Node.js options, only a missing explicit path is an error. A bare name may
/// be the value of an option missing from `NODE_OPTIONS_WITH_VALUE`, such as a
/// coverage glob, and this position never falls back to a package, so Node.js
/// reports a missing one itself.
fn check_script_after_options(token: &str, cwd: &AbsolutePath) -> Result<(), ScriptError> {
    if is_explicit_path(token)
        && SCRIPT_EXTENSIONS.iter().any(|extension| token.ends_with(extension))
        && !cwd.join(token).as_path().is_file()
    {
        return Err(missing_script(token, &cwd.join(token)));
    }
    Ok(())
}

/// The error for a script path that is not a file.
fn missing_script(token: &str, path: &AbsolutePath) -> ScriptError {
    if path.as_path().is_dir() {
        ScriptError::IsDirectory(token.to_owned())
    } else {
        ScriptError::NotFound(token.to_owned())
    }
}

/// Why package mode cannot run an explicit path as a program.
#[derive(Debug, PartialEq, Eq)]
pub struct ProgramError {
    pub error: String,
    pub note: Option<String>,
}

/// Package mode runs an explicit path as a program. An existing file must be
/// executable, and a directory needs a `package.json` for `vp dlx`, so either fails
/// here with a clear error instead of in `vp dlx`. Mirrors `checkExplicitProgram` in
/// `packages/cli/src/vpx-script.ts`.
pub fn check_explicit_program(token: &str, cwd: &AbsolutePath) -> Result<(), ProgramError> {
    if !is_explicit_path(token) {
        return Ok(());
    }
    let path = cwd.join(token);
    let Ok(metadata) = std::fs::metadata(path.as_path()) else {
        return Ok(());
    };
    if metadata.is_dir() && !path.join("package.json").as_path().is_file() {
        return Err(ProgramError {
            error: format!("'{token}' is a directory without a package.json"),
            note: None,
        });
    }
    if metadata.is_file() && !is_executable(&metadata) {
        return Err(ProgramError {
            error: format!("'{token}' is not executable"),
            note: Some(format!("Make it executable with `chmod +x {token}`.")),
        });
    }
    Ok(())
}

/// An executable mode bit; always true on Windows.
#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

/// Whether `token` names a script. A missing explicit path, or a missing file
/// with a TypeScript extension, is an error: it must never fall through to a
/// remote package download.
fn is_script(token: &str, cwd: &AbsolutePath) -> Result<bool, ScriptError> {
    let path = cwd.join(token);
    if let Some(extension) = SCRIPT_EXTENSIONS.iter().find(|extension| token.ends_with(**extension))
    {
        if path.as_path().is_file() {
            return Ok(true);
        }
        if is_explicit_path(token) || TS_EXTENSIONS.contains(extension) {
            return Err(missing_script(token, &path));
        }
        return Ok(false);
    }
    if !is_explicit_path(token) {
        return Ok(false);
    }
    // A missing explicit path never falls through to a package download.
    if !path.as_path().exists() {
        return Err(ScriptError::NotFound(token.to_owned()));
    }
    // `#!/usr/bin/env vpx` in an extensionless file. Without this, the `PATH`
    // lookup would exec the file, which re-enters `vpx` forever.
    Ok(path.as_path().is_file() && has_vpx_shebang(&path))
}

fn is_explicit_path(token: &str) -> bool {
    if token.starts_with("./") || token.starts_with("../") || token.starts_with('/') {
        return true;
    }
    if cfg!(windows) {
        let bytes = token.as_bytes();
        return token.starts_with(".\\")
            || token.starts_with("..\\")
            || token.starts_with("\\\\")
            || (bytes.len() >= 3
                && bytes[0].is_ascii_alphabetic()
                && bytes[1] == b':'
                && (bytes[2] == b'\\' || bytes[2] == b'/'));
    }
    false
}

fn has_vpx_shebang(path: &AbsolutePath) -> bool {
    let mut head = [0_u8; 256];
    let Ok(read) = std::fs::File::open(path.as_path()).and_then(|mut file| file.read(&mut head))
    else {
        return false;
    };
    let head = &head[..read];
    let Some(line) = head.strip_prefix(b"#!") else {
        return false;
    };
    let line = line.split(|byte| *byte == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(line).split_whitespace().any(|word| {
        let name = word.rsplit(['/', '\\']).next().unwrap_or(word);
        name == "vpx" || name == "vpx.exe"
    })
}

/// Find the `vite-plus/dist` directory holding `entries`: the project's `vite-plus`
/// first, so teams run the loader version pinned in their lockfile, then the global
/// install.
fn resolve_loader_dir(cwd: &AbsolutePath, entries: &[&str]) -> Option<AbsolutePathBuf> {
    choose_loader_dir(
        JsExecutor::resolve_local_vite_plus_package_dir(cwd),
        || JsExecutor::new(None).get_scripts_dir().ok(),
        entries,
    )
    .map(real_dir)
}

/// `dir` with symlinks resolved. pnpm links `vite-plus` into its store, where the native
/// binding package sits next to it; under `--preserve-symlinks` Node.js would resolve the
/// binding from the link instead and not find it.
fn real_dir(dir: AbsolutePathBuf) -> AbsolutePathBuf {
    dunce::canonicalize(dir.as_path()).ok().and_then(AbsolutePathBuf::new).unwrap_or(dir)
}

/// The project's `vite-plus/dist` when it ships every entry in `entries`, else the global
/// one. A project `vite-plus` too old for `vpx <script>` or `vpx --watch` falls back to the
/// global install; a change to the CLI/loader contract gets new entry names the same way.
fn choose_loader_dir(
    local_package_dir: Option<AbsolutePathBuf>,
    global_dist: impl FnOnce() -> Option<AbsolutePathBuf>,
    entries: &[&str],
) -> Option<AbsolutePathBuf> {
    let has_loader =
        |dir: &AbsolutePathBuf| entries.iter().all(|entry| dir.join(entry).as_path().is_file());
    local_package_dir
        .map(|dir| dir.join("dist"))
        .filter(has_loader)
        .or_else(|| global_dist().filter(has_loader))
}

/// Run Node.js with the script loader through the core `node` shim, so the
/// script gets the same runtime selection as `vp node`. With `--watch`, Node.js runs
/// oxc-node's watcher from `vite-plus/dist`, which reruns the script with the loader
/// next to it.
pub async fn execute(
    invocation: ScriptInvocation,
    tsconfig: Option<String>,
    cwd: &AbsolutePath,
) -> i32 {
    let entries = if invocation.watch { WATCH_ENTRIES } else { SCRIPT_ENTRIES };
    let Some(loader_dir) = resolve_loader_dir(cwd, entries) else {
        output::error(if invocation.watch {
            "vpx: The watcher was not found. Upgrade vite-plus to run scripts with vpx --watch."
        } else {
            "vpx: The script loader was not found. Upgrade vite-plus to run script files with vpx."
        });
        return 1;
    };

    let tsconfig = invocation.tsconfig.or(tsconfig);
    if let Some(tsconfig) = &tsconfig {
        let path = cwd.join(tsconfig);
        if !path.as_path().is_file() {
            output::error(&format!("vpx: tsconfig not found: {tsconfig}"));
            return 1;
        }
        // SAFETY: Setting env vars at this point before exec is safe.
        unsafe {
            std::env::set_var(env_vars::VP_SCRIPT_TSCONFIG, path.as_path());
        }
    } else {
        // Each `vpx` run selects its own tsconfig; forked children keep it.
        // SAFETY: Removing env vars at this point before exec is safe.
        unsafe {
            std::env::remove_var(env_vars::VP_SCRIPT_TSCONFIG);
        }
    }

    let mut args = Vec::with_capacity(invocation.node_args.len() + 4);
    if invocation.watch {
        // The watcher passes the loader entries next to it to each run.
        args.push(loader_dir.join(WATCH_ENTRY).as_path().display().to_string());
    } else {
        let Ok(import_url) = url::Url::from_file_path(loader_dir.join(IMPORT_ENTRY).as_path())
        else {
            output::error(&format!("vpx: Invalid loader path: {}", loader_dir.as_path().display()));
            return 1;
        };
        // The preload registers the hooks before any user `--require`; the `--import`
        // registers the off-thread hooks on older Node.js and routes the entry point
        // through the ESM loader.
        args.push("--require".to_owned());
        args.push(loader_dir.join(PRELOAD_ENTRY).as_path().display().to_string());
        args.push("--import".to_owned());
        args.push(import_url.to_string());
    }
    args.extend(invocation.node_args);
    run_node(args, cwd).await
}

/// Run Node.js with `args` through the core `node` shim, with every `node_modules/.bin`
/// from `cwd` up first on `PATH`, so a script gets the same runtime selection as `vp node`.
async fn run_node(args: Vec<String>, cwd: &AbsolutePath) -> i32 {
    let env = match super::vpx::local_bin_env(cwd) {
        Ok(env) => env,
        Err(error) => {
            output::error(&format!("vpx: {error}"));
            return 1;
        }
    };

    // stdout belongs to the script; route vp's own output to stderr.
    output::route_user_output_to_stderr();
    // As `vp node` does: dispatch assumes the shell's PATH lookup already preferred a
    // system-first Node.js, so repeat that lookup here.
    if let Ok(config) = super::env::config::load_config().await
        && config.shim_mode_for_tool("node") == super::env::config::ShimMode::SystemFirst
        && let Some(node) = crate::shim::dispatch::find_system_tool("node")
    {
        return crate::shim::exec::exec_tool(&node, &args, env);
    }
    // Boxed: shim dispatch is also what routes `vpx` here.
    Box::pin(crate::shim::dispatch("node", &args, env)).await
}

/// The cases both CLIs run, from `vpx_script_cases.json`.
#[cfg(test)]
pub(crate) fn shared_table() -> serde_json::Value {
    serde_json::from_str(include_str!("vpx_script_cases.json")).unwrap()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;

    /// A temporary project holding a shared case's `files`; the project directory is the cwd.
    fn case_project(case: &Value) -> (TempDir, AbsolutePathBuf) {
        let files: Vec<(String, String)> = case
            .get("files")
            .and_then(Value::as_object)
            .map(|files| {
                files
                    .iter()
                    .map(|(name, content)| (name.clone(), content.as_str().unwrap().to_owned()))
                    .collect()
            })
            .unwrap_or_default();
        project(&files)
    }

    fn project(files: &[(String, String)]) -> (TempDir, AbsolutePathBuf) {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let cwd = AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        (dir, cwd)
    }

    fn dist(files: &[&str]) -> (TempDir, AbsolutePathBuf) {
        let files: Vec<(String, String)> =
            files.iter().map(|file| (format!("dist/{file}"), String::new())).collect();
        project(&files)
    }

    #[test]
    fn loader_comes_from_the_project_when_it_ships_both_entries() {
        let (_local, local) = dist(&[PRELOAD_ENTRY, IMPORT_ENTRY]);
        let (_global, global) = dist(&[PRELOAD_ENTRY, IMPORT_ENTRY]);
        let global = global.join("dist");
        assert_eq!(
            choose_loader_dir(Some(local.clone()), || Some(global), SCRIPT_ENTRIES),
            Some(local.join("dist"))
        );
    }

    #[test]
    fn loader_falls_back_to_the_global_install_for_an_older_project() {
        // A project vite-plus from before `vpx <script>`, or with only one entry.
        let (_global, global) = dist(&[PRELOAD_ENTRY, IMPORT_ENTRY]);
        let global = global.join("dist");
        for local_files in [&[][..], &[PRELOAD_ENTRY][..], &[IMPORT_ENTRY][..]] {
            let (_local, local) = dist(local_files);
            assert_eq!(
                choose_loader_dir(Some(local), || Some(global.clone()), SCRIPT_ENTRIES),
                Some(global.clone())
            );
        }
        assert_eq!(
            choose_loader_dir(None, || Some(global.clone()), SCRIPT_ENTRIES),
            Some(global.clone())
        );
    }

    #[test]
    fn watcher_falls_back_to_the_global_install_for_a_project_without_it() {
        // A project vite-plus from before the watcher still runs `vpx <script>` itself.
        let (_local, local) = dist(&[PRELOAD_ENTRY, IMPORT_ENTRY]);
        let (_global, global) = dist(WATCH_ENTRIES);
        let global = global.join("dist");
        assert_eq!(
            choose_loader_dir(Some(local.clone()), || Some(global.clone()), WATCH_ENTRIES),
            Some(global.clone())
        );
        assert_eq!(
            choose_loader_dir(Some(local.clone()), || Some(global), SCRIPT_ENTRIES),
            Some(local.join("dist"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn loader_dir_resolves_symlinks() {
        let (_store, store) = dist(&[PRELOAD_ENTRY, IMPORT_ENTRY]);
        let (_project, project) = project(&[]);
        let link = project.join("vite-plus");
        std::os::unix::fs::symlink(store.as_path(), link.as_path()).unwrap();
        let real = AbsolutePathBuf::new(dunce::canonicalize(store.as_path()).unwrap()).unwrap();
        assert_eq!(real_dir(link.join("dist")), real.join("dist"));
    }

    #[test]
    fn loader_is_missing_when_neither_install_ships_it() {
        let (_local, local) = dist(&[]);
        let (_global, global) = dist(&[PRELOAD_ENTRY]);
        assert_eq!(
            choose_loader_dir(Some(local), || Some(global.join("dist")), SCRIPT_ENTRIES),
            None
        );
        assert_eq!(choose_loader_dir(None, || None, SCRIPT_ENTRIES), None);
    }

    /// `explicitPrograms` in `vpx_script_cases.json`, which the project-local bin's
    /// `checkExplicitProgram` runs too.
    #[test]
    fn shared_explicit_programs() {
        let table = shared_table();
        for case in table["explicitPrograms"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            if cfg!(not(unix)) && case.get("unixOnly").and_then(Value::as_bool) == Some(true) {
                continue;
            }
            let (_dir, cwd) = case_project(case);
            #[cfg(unix)]
            for file in case.get("executable").and_then(Value::as_array).into_iter().flatten() {
                use std::os::unix::fs::PermissionsExt;
                let path = cwd.join(file.as_str().unwrap());
                std::fs::set_permissions(path.as_path(), std::fs::Permissions::from_mode(0o755))
                    .unwrap();
            }
            let expected = match &case["expect"] {
                Value::String(ok) if ok == "ok" => Ok(()),
                expect => Err(ProgramError {
                    error: expect["error"].as_str().unwrap().to_owned(),
                    note: expect.get("note").and_then(Value::as_str).map(str::to_owned),
                }),
            };
            let actual = check_explicit_program(case["token"].as_str().unwrap(), &cwd);
            assert_eq!(actual, expected, "{name}");
        }
    }

    /// `vpx_script_cases.json` also lists what `packages/cli/src/vpx-script.ts` must
    /// declare, so the TypeScript port checks the same lists without parsing Rust.
    #[test]
    fn lists_match_the_shared_table() {
        let table = shared_table();
        let list = |key: &str| -> Vec<&str> {
            table[key].as_array().unwrap().iter().map(|item| item.as_str().unwrap()).collect()
        };
        assert_eq!(SCRIPT_EXTENSIONS, list("scriptExtensions"));
        assert_eq!(NODE_OPTIONS_WITH_VALUE, list("nodeOptionsWithValue"));
        assert_eq!(NODE_OPTIONS_WITHOUT_ENTRY, list("nodeOptionsWithoutEntry"));
        assert_eq!(WATCH_OPTIONS_WITH_VALUE, list("watchOptionsWithValue"));
    }

    /// The cases in `vpx_script_cases.json`, which
    /// `packages/cli/src/__tests__/vpx-script.spec.ts` runs against the project-local
    /// bin's detection, so the two implementations cannot drift apart.
    #[test]
    fn shared_detection_cases() {
        let table = shared_table();
        let cases = table["cases"].as_array().unwrap();
        assert!(cases.len() > 20);
        for case in cases {
            let name = case["name"].as_str().unwrap();
            let (_dir, cwd) = case_project(case);
            let root = cwd.as_path().display().to_string();
            let strings = |value: &Value| -> Vec<String> {
                value
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|item| item.as_str().unwrap().replace("{cwd}", &root))
                    .collect()
            };
            let expected = match &case["expect"] {
                Value::String(mode) if mode == "package" => Ok(None),
                expect if expect.get("error").is_some() => {
                    Err(expect["error"].as_str().unwrap().to_owned())
                }
                expect => Ok(Some(ScriptInvocation {
                    node_args: strings(&expect["script"]),
                    tsconfig: expect.get("tsconfig").and_then(Value::as_str).map(str::to_owned),
                    watch: expect.get("watch").and_then(Value::as_bool).unwrap_or(false),
                })),
            };
            let actual = detect(&strings(&case["args"]), &cwd).map_err(|error| error.to_string());
            assert_eq!(actual, expected, "{name}");
        }
    }
}
