//! Package-manager lifecycle environment for script execution.
//!
//! When pnpm, npm, or yarn run a `package.json` script, they stamp environment
//! variables (`npm_execpath`, `npm_config_user_agent`, …) that let child
//! tooling — npm-run-all, `ni`, package-manager detectors — identify which
//! package manager owns the script run. `vp run` executes scripts itself, so
//! without stamping those variables child runners fall back to npm even in
//! pnpm projects (voidzero-dev/vite-plus#2317).
//!
//! Only the session-constant subset is computed here; stamping it into the
//! process env is the caller's job. Per-script variables
//! (`npm_lifecycle_event`, `npm_lifecycle_script`, `npm_package_*`,
//! `PNPM_SCRIPT_SRC_DIR`) name the script being run or the package that owns
//! it, so they belong to the task engine, which knows each script's name and
//! package.

use std::{env, ffi::OsString, path::PathBuf};

use vt_path::AbsolutePathBuf;

use crate::package_manager::{PackageManager, PackageManagerType, package_manager_bin_path};

/// Everything [`PackageManager::lifecycle_env_vars`] needs beyond the package
/// manager itself.
#[derive(Debug)]
pub struct LifecycleEnvContext {
    /// Directory `vp run` was invoked in (`INIT_CWD`).
    pub init_cwd: AbsolutePathBuf,
    /// Node.js version (i.e. `process.version`) for the user-agent string.
    pub node_version: Option<String>,
    /// Path to the running Node.js binary (`npm_node_execpath`/`NODE`), i.e.
    /// `process.execPath`.
    pub node_execpath: Option<PathBuf>,
}

impl PackageManager {
    /// The path the package manager identifies itself with in `npm_execpath`
    /// when it runs lifecycle scripts: the JS CLI entry for JS distributions
    /// (`pnpm.cjs`, `npm-cli.js`, `yarn.js`), the native binary for pnpm >= 12,
    /// with the bin shim as fallback.
    ///
    /// Mirrors the package managers' own stamping code: npm sets
    /// `npm_execpath = config.npmBin` (its `bin/npm-cli.js`), and pnpm sets it
    /// to the CLI's `process.argv[1]` — or `process.execPath` when bundled as
    /// a binary, the case the native pnpm >= 12 layout mirrors
    /// (https://github.com/npm/cli/blob/latest/workspaces/config/lib/set-envs.js,
    /// https://github.com/pnpm/npm-lifecycle/blob/main/index.js).
    ///
    /// Child runners (e.g. npm-run-all) execute `.js`/`.cjs` values through the
    /// current Node.js binary, which works on every platform — unlike
    /// extensionless shims on Windows.
    #[must_use]
    pub fn lifecycle_exec_path(&self) -> AbsolutePathBuf {
        let bin_dir = self.install_dir.join("bin");
        let js_entry_name = match self.client {
            PackageManagerType::Pnpm => Some("pnpm.cjs"),
            PackageManagerType::Npm => Some("npm-cli.js"),
            PackageManagerType::Yarn => Some("yarn.js"),
            // bun is a native binary; it has no JS CLI entry.
            PackageManagerType::Bun => None,
        };
        if let Some(name) = js_entry_name {
            let entry = bin_dir.join(name);
            if entry.as_path().is_file() {
                return entry;
            }
        }
        // pnpm >= 12 ships a native binary (see `download_pnpm_native_package_manager`)
        // and identifies with the running executable itself.
        if matches!(self.client, PackageManagerType::Pnpm) {
            let native = if cfg!(windows) {
                bin_dir.join("pnpm.native.exe")
            } else {
                bin_dir.join("pnpm.native")
            };
            if native.as_path().is_file() {
                return native;
            }
        }
        let shim = package_manager_bin_path(&self.install_dir, &self.client.to_string());
        // The shim breaks child runners on Windows (see above), so if this
        // shows up in a log the on-disk layout probably changed. (bun never
        // has a JS CLI entry, so the message would be misleading there.)
        if js_entry_name.is_some() {
            tracing::debug!(
                "No JS CLI entry under {bin_dir:?}, using package-manager shim {shim:?} for npm_execpath"
            );
        }
        shim
    }

    /// Environment variables the package manager would stamp when running a
    /// `package.json` script, limited to the subset that is constant across a
    /// `vp run` session. Empty for bun: what `bun run` stamps is unverified,
    /// so its environment is left untouched rather than guessed at.
    ///
    /// Names follow npm's lifecycle script environment
    /// (https://docs.npmjs.com/cli/v10/using-npm/scripts#environment), which
    /// pnpm reproduces by routing `pnpm run` scripts through
    /// `@pnpm/npm-lifecycle`
    /// (https://github.com/pnpm/pnpm/blob/main/pnpm11/exec/lifecycle/src/runLifecycleHook.ts):
    /// `INIT_CWD` is the cwd the command was invoked in, `npm_node_execpath`
    /// and `NODE` the running Node.js binary
    /// (https://github.com/npm/cli/blob/latest/workspaces/config/lib/set-envs.js,
    /// https://github.com/pnpm/npm-lifecycle/blob/main/index.js).
    /// Verified against pnpm 11.21.0 and npm 10.9.8.
    #[must_use]
    pub fn lifecycle_env_vars(
        &self,
        context: &LifecycleEnvContext,
    ) -> Vec<(&'static str, OsString)> {
        if matches!(self.client, PackageManagerType::Bun) {
            return Vec::new();
        }

        let mut vars = vec![
            ("npm_execpath", self.lifecycle_exec_path().as_path().as_os_str().to_os_string()),
            (
                "npm_config_user_agent",
                OsString::from(user_agent(
                    self.client,
                    &self.version,
                    context.node_version.as_deref(),
                )),
            ),
            ("INIT_CWD", context.init_cwd.as_path().as_os_str().to_os_string()),
        ];

        if let Some(node_execpath) = &context.node_execpath {
            vars.push(("npm_node_execpath", node_execpath.as_os_str().to_os_string()));
            vars.push(("NODE", node_execpath.as_os_str().to_os_string()));
        }

        vars
    }
}

/// `npm_config_user_agent`, formatted the way the package manager itself does:
/// `pnpm/11.20.0 npm/? node/v22.23.1 linux x64` (pnpm, yarn) or
/// `npm/10.9.8 node/v22.23.1 linux x64 workspaces/false` (npm).
///
/// Formats follow pnpm's resolved `userAgent` config
/// (`{name}/{version} npm/? node/{version} {platform} {arch}`,
/// https://github.com/pnpm/pnpm/blob/main/pnpm11/config/reader/src/index.ts)
/// and npm's `user-agent` definition (`npm/{npm-version} node/{node-version}
/// {platform} {arch} workspaces/{workspaces}`,
/// https://github.com/npm/cli/blob/latest/workspaces/config/lib/definitions/definitions.js).
fn user_agent(
    package_manager_type: PackageManagerType,
    version: &str,
    node_version: Option<&str>,
) -> String {
    let node = node_version.map_or_else(String::new, |v| vt_str::format!(" node/{v}").to_string());
    let platform = node_platform(env::consts::OS);
    let arch = node_arch(env::consts::ARCH);
    match package_manager_type {
        PackageManagerType::Pnpm | PackageManagerType::Yarn => {
            vt_str::format!("{package_manager_type}/{version} npm/?{node} {platform} {arch}")
                .to_string()
        }
        // npm's `workspaces/` flag reflects the `--workspaces` command flag,
        // which `vp run` has no analogue of, so it stays `false` (verified
        // against npm 10.9.8, including inside a workspace root).
        PackageManagerType::Npm => {
            vt_str::format!("npm/{version}{node} {platform} {arch} workspaces/false").to_string()
        }
        // Callers skip bun before building a user agent.
        PackageManagerType::Bun => String::new(),
    }
}

/// Map Rust's `env::consts::OS` to Node.js `process.platform` spellings.
fn node_platform(os: &'static str) -> &'static str {
    match os {
        "macos" => "darwin",
        "windows" => "win32",
        "solaris" | "illumos" => "sunos",
        other => other,
    }
}

/// Map Rust's `env::consts::ARCH` to Node.js `process.arch` spellings.
fn node_arch(arch: &'static str) -> &'static str {
    match arch {
        "x86" => "ia32",
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "powerpc" => "ppc",
        "powerpc64" => "ppc64",
        "loongarch64" => "loong64",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn package_manager(
        package_manager_type: PackageManagerType,
        version: &str,
        install_dir: &std::path::Path,
    ) -> PackageManager {
        PackageManager {
            client: package_manager_type,
            version: version.into(),
            install_dir: AbsolutePathBuf::new(install_dir.to_path_buf()).unwrap(),
        }
    }

    fn project_dir() -> AbsolutePathBuf {
        let path = if cfg!(windows) { "C:\\project" } else { "/project" };
        AbsolutePathBuf::new(path.into()).unwrap()
    }

    fn node_path() -> PathBuf {
        PathBuf::from(if cfg!(windows) { "C:\\node\\node.exe" } else { "/node/bin/node" })
    }

    fn context(node_version: Option<&str>) -> LifecycleEnvContext {
        LifecycleEnvContext {
            init_cwd: project_dir(),
            node_version: node_version.map(str::to_string),
            node_execpath: Some(node_path()),
        }
    }

    fn vars_map<'a>(
        vars: &'a [(&'static str, OsString)],
    ) -> std::collections::HashMap<&'a str, &'a OsStr> {
        vars.iter().map(|(k, v)| (*k, v.as_os_str())).collect()
    }

    fn write_file(path: &std::path::Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    #[test]
    fn exec_path_prefers_js_cli_entry() {
        let cases = [
            (PackageManagerType::Pnpm, "pnpm.cjs"),
            (PackageManagerType::Npm, "npm-cli.js"),
            (PackageManagerType::Yarn, "yarn.js"),
        ];
        for (pm_type, js_entry) in cases {
            let dir = tempfile::tempdir().unwrap();
            let install_dir = dir.path().join("pm");
            write_file(&install_dir.join("bin").join(js_entry));
            let pm = package_manager(pm_type, "1.0.0", &install_dir);
            assert_eq!(
                pm.lifecycle_exec_path().as_path(),
                install_dir.join("bin").join(js_entry).as_path()
            );
        }
    }

    #[test]
    fn exec_path_uses_native_binary_for_native_pnpm() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        let native_name = if cfg!(windows) { "pnpm.native.exe" } else { "pnpm.native" };
        write_file(&install_dir.join("bin").join(native_name));
        let pm = package_manager(PackageManagerType::Pnpm, "12.0.0", &install_dir);
        assert_eq!(
            pm.lifecycle_exec_path().as_path(),
            install_dir.join("bin").join(native_name).as_path()
        );
    }

    #[test]
    fn exec_path_falls_back_to_bin_shim() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        std::fs::create_dir_all(install_dir.join("bin")).unwrap();
        let pm = package_manager(PackageManagerType::Pnpm, "1.0.0", &install_dir);
        let expected = if cfg!(windows) {
            install_dir.join("bin").join("pnpm.cmd")
        } else {
            install_dir.join("bin").join("pnpm")
        };
        assert_eq!(pm.lifecycle_exec_path().as_path(), expected.as_path());
    }

    #[test]
    fn pnpm_vars_match_pnpm_stamps() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        write_file(&install_dir.join("bin").join("pnpm.cjs"));
        let pm = package_manager(PackageManagerType::Pnpm, "11.20.0", &install_dir);

        let vars = pm.lifecycle_env_vars(&context(Some("v22.23.1")));
        let map = vars_map(&vars);

        assert_eq!(
            map["npm_execpath"],
            install_dir.join("bin").join("pnpm.cjs").as_path().as_os_str()
        );
        assert_eq!(
            map["npm_config_user_agent"],
            OsStr::new(&vt_str::format!(
                "pnpm/11.20.0 npm/? node/v22.23.1 {} {}",
                node_platform(env::consts::OS),
                node_arch(env::consts::ARCH)
            ))
        );
        assert_eq!(map["INIT_CWD"], project_dir().as_path().as_os_str());
        assert_eq!(map["npm_node_execpath"], node_path().as_os_str());
        assert_eq!(map["NODE"], node_path().as_os_str());
        // Per-script; belongs to the task engine, not the session stamp.
        assert!(!map.contains_key("PNPM_SCRIPT_SRC_DIR"));
    }

    #[test]
    fn npm_vars_match_npm_stamps() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        write_file(&install_dir.join("bin").join("npm-cli.js"));
        let pm = package_manager(PackageManagerType::Npm, "10.9.8", &install_dir);

        let vars = pm.lifecycle_env_vars(&context(Some("v22.23.1")));
        let map = vars_map(&vars);

        assert_eq!(
            map["npm_execpath"],
            install_dir.join("bin").join("npm-cli.js").as_path().as_os_str()
        );
        assert_eq!(
            map["npm_config_user_agent"],
            OsStr::new(&vt_str::format!(
                "npm/10.9.8 node/v22.23.1 {} {} workspaces/false",
                node_platform(env::consts::OS),
                node_arch(env::consts::ARCH)
            ))
        );
    }

    #[test]
    fn yarn_user_agent_matches_yarn_stamps() {
        let ua = user_agent(PackageManagerType::Yarn, "1.22.22", Some("v22.23.1"));
        assert_eq!(
            ua,
            vt_str::format!(
                "yarn/1.22.22 npm/? node/v22.23.1 {} {}",
                node_platform(env::consts::OS),
                node_arch(env::consts::ARCH)
            )
            .to_string()
        );
    }

    #[test]
    fn user_agent_omits_node_segment_without_version() {
        let ua = user_agent(PackageManagerType::Pnpm, "11.20.0", None);
        assert_eq!(
            ua,
            vt_str::format!(
                "pnpm/11.20.0 npm/? {} {}",
                node_platform(env::consts::OS),
                node_arch(env::consts::ARCH)
            )
            .to_string()
        );
        assert!(!ua.contains("node/"));
    }

    #[test]
    fn node_execpath_is_optional() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        let pm = package_manager(PackageManagerType::Pnpm, "11.20.0", &install_dir);
        let mut context = context(None);
        context.node_execpath = None;

        let vars = pm.lifecycle_env_vars(&context);
        let map = vars_map(&vars);

        assert!(!map.contains_key("npm_node_execpath"));
        assert!(!map.contains_key("NODE"));
        assert!(map.contains_key("npm_execpath"));
    }

    #[test]
    fn bun_stamps_no_lifecycle_vars() {
        let dir = tempfile::tempdir().unwrap();
        let install_dir = dir.path().join("pm");
        let pm = package_manager(PackageManagerType::Bun, "1.3.0", &install_dir);
        assert!(pm.lifecycle_env_vars(&context(Some("v22.23.1"))).is_empty());
    }

    #[test]
    fn node_platform_and_arch_match_node_spellings() {
        assert_eq!(node_platform("macos"), "darwin");
        assert_eq!(node_platform("windows"), "win32");
        assert_eq!(node_platform("linux"), "linux");
        assert_eq!(node_arch("x86_64"), "x64");
        assert_eq!(node_arch("x86"), "ia32");
        assert_eq!(node_arch("aarch64"), "arm64");
        assert_eq!(node_arch("powerpc"), "ppc");
    }
}
