//! PATH environment variable manipulation utilities.
//!
//! This module provides functions for prepending directories to the PATH
//! environment variable with various deduplication strategies.

use std::{collections::BTreeSet, env, ffi::OsString, io, path::Path};

use vt_path::AbsolutePath;

use crate::env_vars;

/// PATH and the tools whose real binary directories Vite+ has injected into it.
/// Keep both values together when preparing a child process environment.
#[derive(Debug, Clone)]
pub struct ToolPathEnv {
    path: OsString,
    tools: BTreeSet<String>,
}

impl ToolPathEnv {
    pub fn new(path: OsString, tools: &str) -> Self {
        Self {
            path,
            tools: tools.split(',').filter(|tool| !tool.is_empty()).map(str::to_owned).collect(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(
            env::var_os("PATH").unwrap_or_default(),
            &env::var(env_vars::VP_PATH_INJECTED_TOOLS).unwrap_or_default(),
        )
    }

    pub fn contains(&self, tool: &str) -> bool {
        self.tools.contains(tool)
    }

    /// Only pass tools supplied by this directory, never the names of vp shims.
    /// Use `dedupe_anywhere` to retain an existing directory's PATH precedence;
    /// explicit version selection instead moves the directory to the front.
    pub fn prepend(
        &mut self,
        dir: impl AsRef<Path>,
        tools: &[&str],
        options: PrependOptions,
    ) -> io::Result<()> {
        let dir = dir.as_ref();
        if !options.dedupe_anywhere || !env::split_paths(&self.path).any(|path| path == dir) {
            let mut paths = vec![dir.to_path_buf()];
            paths.extend(env::split_paths(&self.path).filter(|path| path != dir));
            self.path = env::join_paths(paths)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
        }
        self.tools.extend(tools.iter().map(|tool| (*tool).to_owned()));
        Ok(())
    }

    pub fn into_envs(self) -> [(&'static str, OsString); 2] {
        [
            ("PATH", self.path),
            (
                env_vars::VP_PATH_INJECTED_TOOLS,
                self.tools.into_iter().collect::<Vec<_>>().join(",").into(),
            ),
        ]
    }
}

/// Inject tools at existing process-wide PATH initialization boundaries.
/// Child-process-only callers should use `ToolPathEnv::into_envs` instead.
///
/// Only call before spawning threads or when no other threads access the
/// process environment.
pub fn prepend_tools_to_path_env(
    dir: &AbsolutePath,
    tools: &[&str],
    options: PrependOptions,
) -> io::Result<()> {
    let mut env = ToolPathEnv::from_env();
    env.prepend(dir, tools, options)?;
    for (key, value) in env.into_envs() {
        // SAFETY: Caller ensures exclusive environment access, as for PATH initialization.
        unsafe { std::env::set_var(key, value) };
    }
    Ok(())
}

/// Options for deduplication behavior when prepending to PATH.
#[derive(Debug, Clone, Copy, Default)]
pub struct PrependOptions {
    /// Retain an existing directory's position instead of moving it to the front.
    pub dedupe_anywhere: bool,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn injected_tools_accumulate_without_changing_other_environments() {
        let original = ToolPathEnv::new(OsString::new(), "node,npm,npx,node");
        let mut child = original.clone();
        child.prepend("/pnpm/bin", &["pnpm", "pnpx"], PrependOptions::default()).unwrap();
        assert!(child.contains("node"));
        assert!(child.contains("pnpx"));
        assert!(!child.contains("npmx"));
        assert!(!original.contains("pnpm"));
        assert_eq!(child.into_envs()[1].1, "node,npm,npx,pnpm,pnpx");
    }

    #[test]
    fn reinjecting_a_directory_moves_it_ahead_of_other_versions() {
        let path = env::join_paths(["/system/bin", "/selected/bin", "/vp/bin"]).unwrap();
        let mut child = ToolPathEnv::new(path, "");
        child.prepend("/selected/bin", &["node"], PrependOptions::default()).unwrap();
        child.prepend("/selected/bin", &["npm", "npx"], PrependOptions::default()).unwrap();
        let envs = child.into_envs();
        let paths: Vec<_> = env::split_paths(&envs[0].1).collect();
        assert_eq!(
            paths,
            [
                PathBuf::from("/selected/bin"),
                PathBuf::from("/system/bin"),
                PathBuf::from("/vp/bin")
            ]
        );
        assert_eq!(envs[1].1, "node,npm,npx");
    }

    #[test]
    fn recording_an_existing_runtime_preserves_package_manager_precedence() {
        let path = env::join_paths(["/npm/bin", "/node/bin", "/vp/bin"]).unwrap();
        let mut child = ToolPathEnv::new(path.clone(), "npm,npx");
        child.prepend("/node/bin", &["node"], PrependOptions { dedupe_anywhere: true }).unwrap();
        let envs = child.into_envs();
        assert_eq!(envs[0].1, path);
        assert_eq!(envs[1].1, "node,npm,npx");
    }

    #[test]
    fn failed_path_injection_does_not_record_tools_or_change_path() {
        let mut child = ToolPathEnv::new("/original/bin".into(), "node");
        let original = child.clone().into_envs();
        let invalid = if cfg!(windows) { "/invalid\"path" } else { "/invalid:path" };
        assert!(child.prepend(invalid, &["pnpm", "pnpx"], PrependOptions::default()).is_err());
        assert_eq!(child.into_envs(), original);
    }
}
