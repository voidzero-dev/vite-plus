//! Provisioning for the two `vp` flavors a case can run under.
//!
//! Both flavors install the built global `vp` binary into each case's isolated
//! `VP_HOME` and run `vp env setup`. The local flavor additionally exposes the
//! checkout package's JS bin directory from inside that same case home.
//!
//! Each flavor gets one runner bin directory per run (created under the run
//! temp root) for runner-owned helpers. Only `vpt` lives there.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
    Local,
    Global,
}

impl Flavor {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }
}

/// Everything the runner needs to spawn commands under one flavor.
pub struct FlavorRuntime {
    pub runner_bin_dir: PathBuf,
    pub vpt: PathBuf,
    /// Source global `vp` binary to install into each case's `VP_HOME/current`.
    pub global_vp: PathBuf,
    /// Source package installed into each case's `VP_HOME/current/node_modules`.
    pub cli_package_dir: PathBuf,
}

/// The runner crate's manifest dir. The runtime env var wins: cargo sets it
/// for test processes, and nextest rewrites it when running a relocated
/// archive (`--workspace-remap`), where the compile-time path is a
/// build-machine path that no longer exists.
pub fn manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

pub fn repo_root() -> PathBuf {
    manifest_dir().parent().unwrap().parent().unwrap().to_path_buf()
}

/// Searches for `name` next to the test executable (`target/<profile>/deps/`)
/// and one directory up (`target/<profile>/`), where cargo puts bin targets
/// and where nextest extracts archived binaries.
fn find_beside_test_exe(name: &str) -> Result<Option<PathBuf>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
    let deps_dir = exe.parent().ok_or("test executable has no parent dir")?;
    for dir in [deps_dir, deps_dir.parent().unwrap_or(deps_dir)] {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Locates the freshly built global `vp` binary next to this test executable
/// (test binaries run from `target/<profile>/deps/`, the product binaries sit
/// one directory up). Build ordering is the entry-point recipe's job, so a
/// missing binary fails fast with that instruction instead of silently
/// testing a stale build.
fn global_vp_path() -> Result<PathBuf, String> {
    // `VP_SNAP_GLOBAL_VP` points at an already-built binary (CI uses the
    // release binary that `bootstrap-cli` installed), skipping the cargo
    // build of vite_global_cli that `just snapshot-test` performs.
    if let Some(vp) = std::env::var_os("VP_SNAP_GLOBAL_VP") {
        let vp = PathBuf::from(vp);
        if vp.is_file() {
            return Ok(vp);
        }
        return Err(format!("VP_SNAP_GLOBAL_VP is set but {} does not exist", vp.display()));
    }
    let name = format!("vp{}", std::env::consts::EXE_SUFFIX);
    find_beside_test_exe(&name)?.ok_or_else(|| {
        "global `vp` binary not found next to the test executable; run \
         `just snapshot-test` (or `cargo build -p vite_global_cli`) first"
            .to_owned()
    })
}

/// Locates the local JS CLI bin directory. `VP_SNAP_LOCAL_CLI_BIN_DIR`
/// overrides the default `<repo>/packages/cli/bin` (useful when the built
/// `dist/` lives in another checkout or a CI artifact directory).
fn local_cli_package_dir() -> Result<PathBuf, String> {
    let overridden = std::env::var_os("VP_SNAP_LOCAL_CLI_BIN_DIR");
    let bin_dir =
        overridden.as_ref().map_or_else(|| repo_root().join("packages/cli/bin"), PathBuf::from);
    let package_dir = bin_dir.parent().ok_or("local CLI bin dir has no parent")?;
    let dist_entry = package_dir.join("dist/bin.js");
    if !dist_entry.is_file() {
        return Err(format!(
            "local CLI is not built: expected {} (run `pnpm build`, or point \
             VP_SNAP_LOCAL_CLI_BIN_DIR at a built packages/cli/bin)",
            dist_entry.display(),
        ));
    }
    // A stale dist silently tests old code; fail fast when sources are newer.
    // Skipped in CI, where dist is always freshly built, and under the
    // override, which points at another checkout on purpose.
    if overridden.is_none() && std::env::var_os("GITHUB_ACTIONS").is_none() {
        // packages/core shares the freshness requirement: it is linked into
        // the run-root node_modules and its exports load its dist. prompts
        // has no dist of its own; it is bundled into the CLI dist. Keep this
        // list in sync with the packages feeding the CLI build (see
        // packages/cli/BUNDLING.md): a new bundled package needs an entry.
        let cli_pkg = package_dir.to_path_buf();
        let core_pkg = repo_root().join("packages/core");
        let checks = [
            (cli_pkg.join("src"), cli_pkg.join("dist"), "packages/cli"),
            (core_pkg.join("src"), core_pkg.join("dist"), "packages/core"),
            (
                repo_root().join("packages/prompts/src"),
                cli_pkg.join("dist"),
                "packages/prompts (bundled into packages/cli/dist)",
            ),
        ];
        for (src_dir, dist_dir, label) in checks {
            if let (Some(src), Some(dist)) = (newest_mtime(&src_dir), newest_mtime(&dist_dir))
                && src > dist
            {
                return Err(format!(
                    "{label} sources are newer than the built dist; run `pnpm build`, or set \
                     VP_SNAP_SKIP_FLAVORS=local to skip local-flavor cases"
                ));
            }
        }
    }
    Ok(package_dir.to_path_buf())
}

fn newest_mtime(dir: &Path) -> Option<std::time::SystemTime> {
    let mut newest = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        let candidate =
            if meta.is_dir() { newest_mtime(&entry.path()) } else { meta.modified().ok() };
        if let Some(time) = candidate
            && newest.is_none_or(|n| time > n)
        {
            newest = Some(time);
        }
    }
    newest
}

/// Resolves the `vpt` helper binary. The runtime env var wins: nextest
/// rewrites `CARGO_BIN_EXE_vpt` when running a relocated archive
/// (`--workspace-remap`), where the compile-time path is a build-machine
/// path that no longer exists. Falls back to the compile-time value (plain
/// `cargo test`), then to a sibling of the test executable.
fn vpt_path() -> Result<PathBuf, String> {
    if let Some(vpt) = std::env::var_os("CARGO_BIN_EXE_vpt") {
        let vpt = PathBuf::from(vpt);
        if vpt.is_file() {
            return Ok(vpt);
        }
    }
    let compile_time = PathBuf::from(env!("CARGO_BIN_EXE_vpt"));
    if compile_time.is_file() {
        return Ok(compile_time);
    }
    let name = format!("vpt{}", std::env::consts::EXE_SUFFIX);
    find_beside_test_exe(&name)?.ok_or_else(|| {
        "`vpt` binary not found (checked CARGO_BIN_EXE_vpt, the compile-time \
         path, and next to the test executable)"
            .to_owned()
    })
}

/// Home-layout names, shared with `CaseHome` in main.rs so the product's
/// `~/.vite-plus/js_runtime` layout is spelled once.
pub const VP_HOME_DIR: &str = ".vite-plus";
pub const JS_RUNTIME_DIR: &str = "js_runtime";

/// Directory holding an already-provisioned managed JS runtime that each
/// case's `VP_HOME` is seeded with (symlinked, read-mostly). Without a seed,
/// any command that touches the managed runtime downloads ~50MB per case.
/// Override with `VP_SNAP_JS_RUNTIME_DIR` (CI restores a cached runtime
/// there); defaults to the developer's real `~/.vite-plus/js_runtime`.
/// Cases that test runtime provisioning itself opt out via
/// `seed-runtime = false`.
pub fn js_runtime_seed_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("VP_SNAP_JS_RUNTIME_DIR") {
        let dir = PathBuf::from(dir);
        return dir.is_dir().then_some(dir);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    let dir = PathBuf::from(home).join(VP_HOME_DIR).join(JS_RUNTIME_DIR);
    dir.is_dir().then_some(dir)
}

/// The newest real `node` binary in the seed runtime, bypassing the case's
/// node shim (which resolves a project-pinned version from its cwd). Used for
/// runner infrastructure like the local-registry server, which needs a
/// TypeScript-capable Node regardless of what the fixture under test pins.
pub fn seed_runtime_node() -> Option<PathBuf> {
    let node_root = js_runtime_seed_dir()?.join("node");
    let mut versions: Vec<(Vec<u64>, PathBuf)> = std::fs::read_dir(&node_root)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let parts: Vec<u64> = name.split('.').map(str::parse).collect::<Result<_, _>>().ok()?;
            // Windows node dists put node.exe at the version root; Unix under bin/.
            let bin = if cfg!(windows) {
                e.path().join("node.exe")
            } else {
                e.path().join("bin").join("node")
            };
            bin.is_file().then_some((parts, bin))
        })
        .collect();
    versions.sort();
    versions.pop().map(|(_, bin)| bin)
}

/// Installs a runner-owned native tool. A symlink is enough on Unix; Windows
/// uses a hard link or copy so the executable keeps its `.exe` suffix.
fn install_runner_tool(bin_dir: &Path, name: &str, target: &Path) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        let dest = bin_dir.join(name);
        std::os::unix::fs::symlink(target, &dest)
            .map_err(|e| format!("failed to link {name}: {e}"))?;
        Ok(dest)
    }
    #[cfg(windows)]
    {
        // Hard links are free when source and destination share a volume;
        // fall back to a real copy across volumes.
        let dest = bin_dir.join(format!("{name}.exe"));
        std::fs::hard_link(target, &dest)
            .or_else(|_| std::fs::copy(target, &dest).map(|_| ()))
            .map_err(|e| format!("failed to copy {name}: {e}"))?;
        Ok(dest)
    }
}

/// Installs a real executable file. Used for the standalone global layout under
/// `VP_HOME/current/bin`, where symlinking to the build output would test the
/// wrong installation shape.
pub fn install_file(dest: &Path, source: &Path, label: &str) -> Result<(), String> {
    let source = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    std::fs::hard_link(&source, dest)
        .or_else(|_| std::fs::copy(&source, dest).map(|_| ()))
        .map_err(|e| format!("failed to install {label}: {e}"))
}

/// Best-effort directory link. On Windows, directory symlinks may require
/// privileges, so a junction (which never does) is the fallback.
pub fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    let _ = std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    if std::os::windows::fs::symlink_dir(target, link).is_err() {
        let _ = junction::create(target, link);
    }
}

/// Creates the per-run bin directory for `flavor` under `run_root`.
pub fn provision(flavor: Flavor, run_root: &Path) -> Result<FlavorRuntime, String> {
    let runner_bin_dir = run_root.join(format!("bin-{}", flavor.as_str()));
    std::fs::create_dir_all(&runner_bin_dir)
        .map_err(|e| format!("failed to create bin dir: {e}"))?;

    let vpt = install_runner_tool(&runner_bin_dir, "vpt", &vpt_path()?)?;
    let global_vp = global_vp_path()?;
    let cli_package_dir = match flavor {
        Flavor::Local => local_cli_package_dir()?,
        Flavor::Global => repo_root().join("packages/cli"),
    };
    Ok(FlavorRuntime { runner_bin_dir, vpt, global_vp, cli_package_dir })
}
