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
    pub bin_dir: PathBuf,
    /// Source global `vp` binary to install into each case's `VP_HOME/current`.
    pub global_vp: PathBuf,
    /// Source JS CLI bin dir for the local flavor.
    pub local_cli_bin_dir: Option<PathBuf>,
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
fn local_cli_bin_dir() -> Result<PathBuf, String> {
    let overridden = std::env::var_os("VP_SNAP_LOCAL_CLI_BIN_DIR");
    let bin_dir =
        overridden.as_ref().map_or_else(|| repo_root().join("packages/cli/bin"), PathBuf::from);
    let dist_entry = bin_dir.parent().map(|p| p.join("dist/bin.js"));
    if !dist_entry.as_deref().is_some_and(Path::is_file) {
        return Err(format!(
            "local CLI is not built: expected {} (run `pnpm build`, or point \
             VP_SNAP_LOCAL_CLI_BIN_DIR at a built packages/cli/bin)",
            dist_entry.map_or_else(String::new, |p| p.display().to_string()),
        ));
    }
    // A stale dist silently tests old code; fail fast when sources are newer
    // (the legacy runner did the same for the global binary via mtimes).
    // Skipped in CI, where dist is always freshly built, and under the
    // override, which points at another checkout on purpose.
    if overridden.is_none() && std::env::var_os("GITHUB_ACTIONS").is_none() {
        // packages/core shares the freshness requirement: it is linked into
        // the run-root node_modules and its exports load its dist. prompts
        // has no dist of its own; it is bundled into the CLI dist. Keep this
        // list in sync with the packages feeding the CLI build (see
        // packages/cli/BUNDLING.md): a new bundled package needs an entry.
        let cli_pkg = bin_dir.parent().unwrap().to_path_buf();
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
    Ok(bin_dir)
}

pub fn cli_package_dir() -> PathBuf {
    repo_root().join("packages/cli")
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

/// Directory holding an already-provisioned managed JS runtime that each
/// case's `VP_HOME` is seeded with (symlinked, read-mostly). Without a seed,
/// any command that touches the managed runtime downloads ~50MB per case.
/// Override with `VP_SNAP_JS_RUNTIME_DIR` (CI restores a cached runtime
/// Home-layout names, shared with `CaseHome` in main.rs so the product's
/// `~/.vite-plus/js_runtime` layout is spelled once.
pub const VP_HOME_DIR: &str = ".vite-plus";
pub const JS_RUNTIME_DIR: &str = "js_runtime";

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

/// Installs `name` into `bin_dir`, pointing at `target`. Symlink on Unix; on
/// Windows, native executables are copied and scripts get a `.cmd` shim that
/// invokes `node` directly.
fn install_tool(bin_dir: &Path, name: &str, target: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, bin_dir.join(name))
            .map_err(|e| format!("failed to link {name}: {e}"))
    }
    #[cfg(windows)]
    {
        if target.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")) {
            // Hard links are free and CI's bin dir shares a volume with the
            // source; fall back to a real copy across volumes.
            let dest = bin_dir.join(format!("{name}.exe"));
            std::fs::hard_link(target, &dest)
                .or_else(|_| std::fs::copy(target, &dest).map(|_| ()))
                .map_err(|e| format!("failed to copy {name}: {e}"))
        } else {
            let shim = format!("@node \"{}\" %*\r\n", target.display());
            std::fs::write(bin_dir.join(format!("{name}.cmd")), shim)
                .map_err(|e| format!("failed to write {name}.cmd: {e}"))
        }
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
/// privileges, so a junction (which never does) is the fallback; only if
/// both fail does resolution fall back to whatever the fixture vendors.
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
    let bin_dir = run_root.join(format!("bin-{}", flavor.as_str()));
    std::fs::create_dir_all(&bin_dir).map_err(|e| format!("failed to create bin dir: {e}"))?;

    install_tool(&bin_dir, "vpt", &vpt_path()?)?;

    let global_vp = global_vp_path()?;
    let local_cli_bin_dir = if flavor == Flavor::Local { Some(local_cli_bin_dir()?) } else { None };
    Ok(FlavorRuntime { bin_dir, global_vp, local_cli_bin_dir })
}

impl FlavorRuntime {
    /// Resolves a step's `argv[0]` to an absolute path. Only the vp family,
    /// `vpt`, and an allow-list of real tools may run as steps; everything
    /// else belongs behind a `vpt` subcommand so fixtures stay
    /// platform-identical. Keep the allow-list in sync with
    /// `PASSTHROUGH_PROGRAMS` in packages/tools/src/migrate-snap-tests.ts.
    /// Real tools resolve through the CASE's `PATH`, then the resolved path is
    /// checked against the flavor-owned tool dirs.
    pub fn resolve_program(
        &self,
        program: &str,
        case_path: &std::ffi::OsStr,
        cwd: &Path,
        allowed_tool_dirs: &[PathBuf],
    ) -> Result<PathBuf, String> {
        match program {
            "vp" | "vpr" | "vpx" | "oxfmt" | "oxlint" => {
                self.resolve_case_tool(program, case_path, cwd, allowed_tool_dirs)
            }
            // vpt is the runner's own assertion tool: a case-created shim must
            // never shadow it, so it resolves only from the flavor bin dir.
            "vpt" => self.bin_dir_tool(program),
            "node" | "git" | "npm" | "pnpm" | "yarn" | "bun" => {
                self.resolve_case_tool(program, case_path, cwd, allowed_tool_dirs)
            }
            other => Err(format!(
                "step program `{other}` is not allowed; use a `vpt` subcommand instead"
            )),
        }
    }

    fn resolve_case_tool(
        &self,
        program: &str,
        case_path: &std::ffi::OsStr,
        cwd: &Path,
        allowed_tool_dirs: &[PathBuf],
    ) -> Result<PathBuf, String> {
        let found = which::which_in(program, Some(case_path), cwd)
            .map_err(|e| format!("`{program}` not found on the case PATH: {e}"))?;
        if allowed_tool_dirs.iter().any(|dir| found.starts_with(dir)) {
            return Ok(found);
        }
        Err(format!("`{program}` resolved outside the flavor-owned tool dirs: {}", found.display()))
    }

    /// Looks a tool up directly in the flavor bin dir.
    fn bin_dir_tool(&self, program: &str) -> Result<PathBuf, String> {
        if cfg!(windows) {
            // Installed as either .exe or .cmd; try both.
            ["exe", "cmd"]
                .iter()
                .map(|ext| self.bin_dir.join(format!("{program}.{ext}")))
                .find(|p| p.is_file())
                .ok_or_else(|| format!("`{program}` is not available in this flavor"))
        } else {
            let p = self.bin_dir.join(program);
            if !p.is_file() && !p.is_symlink() {
                return Err(format!("`{program}` is not available in this flavor"));
            }
            Ok(p)
        }
    }
}
