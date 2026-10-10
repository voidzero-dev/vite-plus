//! `vpx <script>`: run a script file on the project's Node.js with the oxc-node
//! loader that is compiled into the Vite+ native binding.
//!
//! See `rfcs/vpx-script-execution.md`. The CLI/loader contract is
//! `node --require <dist/script-preload.cjs> --import <file URL of dist/script-register.js>`
//! plus the `VP_SCRIPT_TSCONFIG` environment variable; keep it stable, because the
//! global `vp` may run a newer or older project-local `vite-plus` loader.

use std::io::Read as _;

use vp_shared::{ToolPathEnv, env_vars, output};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::js_executor::JsExecutor;

/// `vite-plus/dist` entries: the CommonJS `--require` preload and the `--import`.
const PRELOAD_ENTRY: &str = "script-preload.cjs";
const IMPORT_ENTRY: &str = "script-register.js";

/// Extensions that make a token a script.
const SCRIPT_EXTENSIONS: &[&str] = &[".ts", ".mts", ".cts", ".tsx", ".js", ".mjs", ".cjs", ".jsx"];

/// A missing file with one of these is an error, never a package name.
const TS_EXTENSIONS: &[&str] = &[".ts", ".mts", ".cts", ".tsx"];

/// Node.js options that take their value as the next argument. Best effort: a
/// missing entry only moves the missing-file check for the script to Node.js.
const NODE_OPTIONS_WITH_VALUE: &[&str] = &[
    "-C",
    "-e",
    "-p",
    "-r",
    "--conditions",
    "--cpu-prof-dir",
    "--cpu-prof-name",
    "--diagnostic-dir",
    "--disable-warning",
    "--dns-result-order",
    "--env-file",
    "--env-file-if-exists",
    "--eval",
    "--experimental-config-file",
    "--experimental-loader",
    "--heap-prof-dir",
    "--heap-prof-name",
    "--heapsnapshot-signal",
    "--icu-data-dir",
    "--import",
    "--input-type",
    "--inspect-port",
    "--loader",
    "--localstorage-file",
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
    "--test-name-pattern",
    "--test-reporter",
    "--test-reporter-destination",
    "--test-shard",
    "--test-skip-pattern",
    "--test-timeout",
    "--title",
    "--tls-cipher-list",
    "--tls-keylog",
    "--unhandled-rejections",
    "--watch-kill-signal",
    "--watch-path",
];

/// A `vpx` invocation that runs Node.js with the script loader.
#[derive(Debug, PartialEq, Eq)]
pub struct ScriptInvocation {
    /// Node.js options, the script, and its arguments, in the order given.
    pub node_args: Vec<String>,
    /// `--tsconfig` given among the Node.js options.
    pub tsconfig: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ScriptError {
    NotFound(String),
    MissingTsconfigValue,
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(script) => write!(f, "Script not found: {script}"),
            Self::MissingTsconfigValue => write!(f, "--tsconfig requires a path"),
        }
    }
}

/// Decide whether `positional` (the arguments after the `vpx` options) runs a
/// script. Returns `None` for package mode. Never rewrites the arguments except
/// for removing `--tsconfig`.
pub fn detect(
    positional: &[String],
    cwd: &AbsolutePath,
) -> Result<Option<ScriptInvocation>, ScriptError> {
    let Some(first) = positional.first() else {
        return Ok(None);
    };

    if !first.starts_with('-') {
        return Ok(is_script(first, cwd)?
            .then(|| ScriptInvocation { node_args: positional.to_vec(), tsconfig: None }));
    }

    // A leading option never named a package bin, so this always runs Node.js.
    // The first non-option token is the script (or `-` for stdin); everything
    // from it on belongs to the script.
    let mut node_args = Vec::with_capacity(positional.len());
    let mut tsconfig = None;
    let mut index = 0;
    while let Some(arg) = positional.get(index) {
        if arg == "--" || arg == "-" || !arg.starts_with('-') {
            let script = if arg == "--" { positional.get(index + 1) } else { Some(arg) };
            if let Some(script) = script {
                is_script(script, cwd)?;
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
        node_args.push(arg.clone());
        if NODE_OPTIONS_WITH_VALUE.contains(&arg.as_str())
            && let Some(value) = positional.get(index + 1)
        {
            node_args.push(value.clone());
            index += 1;
        }
        index += 1;
    }

    Ok(Some(ScriptInvocation { node_args, tsconfig }))
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
            return Err(ScriptError::NotFound(token.to_owned()));
        }
        return Ok(false);
    }
    // `#!/usr/bin/env vpx` in an extensionless file. Without this, the `PATH`
    // lookup would exec the file, which re-enters `vpx` forever.
    Ok(is_explicit_path(token) && path.as_path().is_file() && has_vpx_shebang(&path))
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

/// Find the `vite-plus/dist` directory holding the loader entries: the project's
/// `vite-plus` first, so teams run the loader version pinned in their lockfile, then
/// the global install.
fn resolve_loader_dir(cwd: &AbsolutePath) -> Option<AbsolutePathBuf> {
    let has_loader = |dir: &AbsolutePath| {
        dir.join(PRELOAD_ENTRY).as_path().is_file() && dir.join(IMPORT_ENTRY).as_path().is_file()
    };
    if let Some(package_dir) = JsExecutor::resolve_local_vite_plus_package_dir(cwd) {
        let dist = package_dir.join("dist");
        if has_loader(&dist) {
            return Some(dist);
        }
    }
    let dist = JsExecutor::new(None).get_scripts_dir().ok()?;
    has_loader(&dist).then_some(dist)
}

/// Run Node.js with the script loader through the core `node` shim, so the
/// script gets the same runtime selection as `vp node`.
pub async fn execute(
    invocation: ScriptInvocation,
    tsconfig: Option<String>,
    cwd: &AbsolutePath,
) -> i32 {
    let Some(loader_dir) = resolve_loader_dir(cwd) else {
        output::error(
            "vpx: The script loader was not found. Upgrade vite-plus to run script files with vpx.",
        );
        return 1;
    };
    let preload = loader_dir.join(PRELOAD_ENTRY);
    let Ok(import_url) = url::Url::from_file_path(loader_dir.join(IMPORT_ENTRY).as_path()) else {
        output::error(&format!("vpx: Invalid loader path: {}", loader_dir.as_path().display()));
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

    let mut env = ToolPathEnv::from_env();
    if let Err(error) = super::vpx::prepend_node_modules_bin_to_path(cwd, &mut env) {
        output::error(&format!("vpx: {error}"));
        return 1;
    }

    let mut args = Vec::with_capacity(invocation.node_args.len() + 4);
    // The preload registers the hooks before any user `--require`; the `--import`
    // registers the off-thread hooks on older Node.js and routes the entry point
    // through the ESM loader.
    args.push("--require".to_owned());
    args.push(preload.as_path().display().to_string());
    args.push("--import".to_owned());
    args.push(import_url.to_string());
    args.extend(invocation.node_args);

    // stdout belongs to the script; route vp's own output to stderr.
    output::route_user_output_to_stderr();
    // Boxed: shim dispatch is also what routes `vpx` here.
    Box::pin(crate::shim::dispatch("node", &args, env)).await
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn project(files: &[(&str, &str)]) -> (TempDir, AbsolutePathBuf) {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let cwd = AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        (dir, cwd)
    }

    fn script(node_args: &[&str], tsconfig: Option<&str>) -> Option<ScriptInvocation> {
        Some(ScriptInvocation { node_args: args(node_args), tsconfig: tsconfig.map(str::to_owned) })
    }

    #[test]
    fn explicit_path_with_script_extension_is_a_script() {
        let (_dir, cwd) = project(&[("scripts/seed.ts", "")]);
        let positional = args(&["./scripts/seed.ts", "--dry-run"]);
        assert_eq!(
            detect(&positional, &cwd),
            Ok(script(&["./scripts/seed.ts", "--dry-run"], None))
        );
    }

    #[test]
    fn bare_relative_path_to_an_existing_file_is_a_script() {
        let (_dir, cwd) = project(&[("scripts/seed.ts", "")]);
        assert_eq!(
            detect(&args(&["scripts/seed.ts"]), &cwd),
            Ok(script(&["scripts/seed.ts"], None))
        );
    }

    #[test]
    fn missing_typescript_file_is_an_error() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(
            detect(&args(&["seed.ts"]), &cwd),
            Err(ScriptError::NotFound("seed.ts".to_owned()))
        );
    }

    #[test]
    fn missing_explicit_path_is_an_error() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(
            detect(&args(&["./missing.js"]), &cwd),
            Err(ScriptError::NotFound("./missing.js".to_owned()))
        );
    }

    #[test]
    fn bare_name_with_javascript_extension_and_no_file_is_a_package() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(detect(&args(&["highlight.js"]), &cwd), Ok(None));
    }

    #[test]
    fn scoped_path_is_a_script_not_a_version_spec() {
        let (_dir, cwd) = project(&[("@scope/tool.ts", "")]);
        assert_eq!(
            detect(&args(&["./@scope/tool.ts"]), &cwd),
            Ok(script(&["./@scope/tool.ts"], None))
        );
    }

    #[test]
    fn package_names_stay_in_package_mode() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(detect(&args(&["eslint", "."]), &cwd), Ok(None));
        assert_eq!(detect(&args(&["oxlint@1.85.0", "--version"]), &cwd), Ok(None));
        assert_eq!(detect(&args(&["@vue/cli@5.0.0"]), &cwd), Ok(None));
    }

    #[test]
    fn shell_script_without_vpx_shebang_is_a_package_lookup() {
        let (_dir, cwd) = project(&[("deploy.sh", "#!/bin/sh\necho hi\n")]);
        assert_eq!(detect(&args(&["./deploy.sh"]), &cwd), Ok(None));
    }

    #[test]
    fn extensionless_vpx_shebang_is_a_script() {
        let (_dir, cwd) = project(&[("bin/tool", "#!/usr/bin/env vpx\nconsole.log(1)\n")]);
        let tool = cwd.join("bin/tool").as_path().display().to_string();
        assert_eq!(detect(&args(&[&tool]), &cwd), Ok(script(&[&tool], None)));

        let (_dir, cwd) = project(&[("bin/tool", "#!/usr/bin/env -S vpx --tsconfig x.json\n")]);
        let tool = cwd.join("bin/tool").as_path().display().to_string();
        assert_eq!(detect(&args(&[&tool]), &cwd), Ok(script(&[&tool], None)));
    }

    #[test]
    fn leading_node_options_are_forwarded_before_the_script() {
        let (_dir, cwd) = project(&[("server.ts", "")]);
        assert_eq!(
            detect(&args(&["--watch", "./server.ts", "--port", "3000"]), &cwd),
            Ok(script(&["--watch", "./server.ts", "--port", "3000"], None))
        );
    }

    #[test]
    fn option_values_are_not_mistaken_for_the_script() {
        let (_dir, cwd) = project(&[("main.ts", ""), (".env", "")]);
        assert_eq!(
            detect(&args(&["--import", "./setup.ts", "./main.ts"]), &cwd),
            Ok(script(&["--import", "./setup.ts", "./main.ts"], None))
        );
        assert_eq!(
            detect(&args(&["--env-file", ".env", "./main.ts"]), &cwd),
            Ok(script(&["--env-file", ".env", "./main.ts"], None))
        );
    }

    #[test]
    fn missing_script_after_node_options_is_an_error() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(
            detect(&args(&["--watch", "./missing.ts"]), &cwd),
            Err(ScriptError::NotFound("./missing.ts".to_owned()))
        );
    }

    #[test]
    fn node_invocations_without_a_script_run_node() {
        let (_dir, cwd) = project(&[]);
        assert_eq!(
            detect(&args(&["--eval", "console.log(1)"]), &cwd),
            Ok(script(&["--eval", "console.log(1)"], None))
        );
        assert_eq!(detect(&args(&["--inspect"]), &cwd), Ok(script(&["--inspect"], None)));
        assert_eq!(detect(&args(&["-"]), &cwd), Ok(script(&["-"], None)));
        assert_eq!(
            detect(&args(&["--input-type", "module", "-"]), &cwd),
            Ok(script(&["--input-type", "module", "-"], None))
        );
    }

    #[test]
    fn tsconfig_among_node_options_is_extracted() {
        let (_dir, cwd) = project(&[("a.ts", "")]);
        assert_eq!(
            detect(&args(&["--watch", "--tsconfig", "tsconfig.scripts.json", "./a.ts"]), &cwd),
            Ok(script(&["--watch", "./a.ts"], Some("tsconfig.scripts.json")))
        );
        assert_eq!(
            detect(&args(&["--watch", "--tsconfig=x.json", "./a.ts", "--tsconfig", "y"]), &cwd),
            Ok(script(&["--watch", "./a.ts", "--tsconfig", "y"], Some("x.json")))
        );
        assert_eq!(
            detect(&args(&["--watch", "--tsconfig"]), &cwd),
            Err(ScriptError::MissingTsconfigValue)
        );
    }

    #[test]
    fn arguments_after_the_script_belong_to_the_script() {
        let (_dir, cwd) = project(&[("a.ts", "")]);
        assert_eq!(
            detect(&args(&["./a.ts", "--", "--watch", "b.ts"]), &cwd),
            Ok(script(&["./a.ts", "--", "--watch", "b.ts"], None))
        );
        assert_eq!(
            detect(&args(&["--", "./a.ts", "x"]), &cwd),
            Ok(script(&["--", "./a.ts", "x"], None))
        );
    }
}
