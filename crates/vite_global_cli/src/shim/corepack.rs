//! Corepack shim dispatch.
//!
//! `corepack` is a default shim (created by `vp env setup`), but unlike the
//! core tools (node/npm/npx) it is not always available in the resolved
//! Node.js installation: Node.js 25 removed the bundled corepack.
//!
//! Resolution order:
//! 1. vp-managed global package (`vp install -g corepack`) — an explicit
//!    install wins and provides a consistent corepack across Node.js versions
//! 2. corepack bundled with the project-resolved Node.js (Node.js <= 24)
//! 3. auto-install corepack as a vp-managed global package
//!
//! `corepack enable`/`corepack disable` create or remove package-manager
//! launchers next to the corepack binary found in PATH, which under the shim
//! would be the per-version Node.js bin directory (not on PATH). To keep the
//! launchers reachable, these commands get `--install-directory ~/.vite-plus/bin`
//! injected when not explicitly set, and Vite+-owned shims are restored
//! afterwards if corepack removed or replaced them.

use vite_path::{AbsolutePath, AbsolutePathBuf, current_dir};
use vite_shared::{PrependOptions, env_vars, output, prepend_to_path_env};

use super::{
    dispatch::{
        ensure_installed, find_package_for_binary, locate_package_binary, locate_tool,
        resolve_with_cache,
    },
    exec,
};
use crate::commands::env::{
    bin_config::{BinConfig, BinSource},
    config, setup,
};

/// Binary names corepack `enable`/`disable` may create or remove in the
/// install directory (npm/npx only when explicitly requested).
const COREPACK_MANAGED_BIN_NAMES: &[&str] = &["npm", "npx", "pnpm", "pnpx", "yarn", "yarnpkg"];

/// How to invoke the resolved corepack binary.
struct CorepackInvocation {
    /// Program to execute (the corepack binary itself, or node for a JS entry)
    program: AbsolutePathBuf,
    /// Arguments to pass before the user-supplied args (e.g., the JS entry path)
    pre_args: Vec<String>,
}

/// Dispatch a `corepack` shim invocation.
pub(crate) async fn dispatch_corepack(args: &[String]) -> i32 {
    let invocation = match resolve_corepack_invocation().await {
        Ok(invocation) => invocation,
        Err(exit_code) => return exit_code,
    };

    // enable/disable: run with spawn+wait so Vite+-owned shims can be
    // restored after corepack touched the install directory.
    if let Ok(bin_dir) = config::get_bin_dir()
        && let Some(link_args) = corepack_link_command_args(args, &bin_dir)
    {
        let mut full_args = invocation.pre_args.clone();
        full_args.extend(link_args);
        let exit_code = exec::spawn_tool(&invocation.program, &full_args);
        restore_vp_owned_shims(&bin_dir).await;
        return exit_code;
    }

    let mut full_args = invocation.pre_args.clone();
    full_args.extend(args.iter().cloned());
    exec::exec_tool(&invocation.program, &full_args)
}

/// Resolve which corepack binary to execute.
///
/// Returns an exit code on failure (errors are already printed).
async fn resolve_corepack_invocation() -> Result<CorepackInvocation, i32> {
    // 1. An explicit `vp install -g corepack` wins.
    match managed_corepack_invocation().await {
        Ok(Some(invocation)) => return Ok(invocation),
        Ok(None) => {}
        Err(e) => {
            eprintln!("vp: Failed to resolve installed corepack: {e}");
            return Err(1);
        }
    }

    // 2. corepack bundled with the project-resolved Node.js (Node.js <= 24).
    let cwd = match current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("vp: Failed to get current directory: {e}");
            return Err(1);
        }
    };
    let resolution = match resolve_with_cache(&cwd).await {
        Ok(resolution) => resolution,
        Err(e) => {
            eprintln!("vp: Failed to resolve Node version: {e}");
            eprintln!("vp: Run 'vp env doctor' for diagnostics");
            return Err(1);
        }
    };
    if let Err(e) = ensure_installed(&resolution.version).await {
        eprintln!("vp: Failed to install Node {}: {e}", resolution.version);
        return Err(1);
    }
    if let Ok(corepack_path) = locate_tool(&resolution.version, "corepack") {
        prepend_node_bin_dir(&resolution.version);
        // Match the core-tool dispatch: nested core-tool shims pass through.
        // SAFETY: Setting env vars at this point before exec/spawn is safe
        unsafe {
            std::env::set_var(env_vars::VP_TOOL_RECURSION, "1");
        }
        return Ok(CorepackInvocation { program: corepack_path, pre_args: Vec::new() });
    }

    // 3. Node.js 25+ no longer bundles corepack: install it as a vp-managed
    //    global package, then run that copy. Only the `corepack` bin is
    //    linked; the pnpm/yarn launchers the package also declares stay
    //    unexposed (that is `corepack enable`'s job) and must not conflict
    //    with vp-managed package managers.
    output::info(&format!(
        "corepack is not bundled with Node.js {}; installing it as a managed global package",
        resolution.version
    ));
    if let Err((_, error)) = crate::commands::global::install::install(
        &["corepack".to_string()],
        None,
        false,
        1,
        false,
        Some(&["corepack"]),
    )
    .await
    {
        eprintln!("vp: Failed to install corepack: {error}");
        eprintln!("vp: Run 'vp install -g corepack' manually, then retry");
        return Err(1);
    }
    match managed_corepack_invocation().await {
        Ok(Some(invocation)) => Ok(invocation),
        _ => {
            eprintln!("vp: corepack was installed but its binary could not be located");
            Err(1)
        }
    }
}

/// Resolve a corepack installed via `vp install -g corepack`, if any.
///
/// Mirrors `dispatch_package_binary`: uses the install-time Node.js version
/// and prepends its bin directory to PATH for child processes.
async fn managed_corepack_invocation() -> Result<Option<CorepackInvocation>, String> {
    let Some(metadata) = find_package_for_binary("corepack").await? else {
        return Ok(None);
    };

    let node_version = metadata.platform.node.clone();
    ensure_installed(&node_version).await?;
    let binary_path = locate_package_binary(&metadata.name, "corepack")?;
    let node_path = locate_tool(&node_version, "node")?;
    let node_bin_dir =
        node_path.parent().ok_or_else(|| "Node has no parent directory".to_string())?;
    let _ = prepend_to_path_env(node_bin_dir, PrependOptions::default());

    if metadata.is_js_binary("corepack") {
        Ok(Some(CorepackInvocation {
            program: node_path,
            pre_args: vec![binary_path.as_path().display().to_string()],
        }))
    } else {
        Ok(Some(CorepackInvocation { program: binary_path, pre_args: Vec::new() }))
    }
}

/// Prepend the resolved Node.js bin directory to PATH for child processes.
fn prepend_node_bin_dir(version: &str) {
    if let Ok(node_path) = locate_tool(version, "node")
        && let Some(node_bin_dir) = node_path.parent()
    {
        let _ = prepend_to_path_env(node_bin_dir, PrependOptions::default());
    }
}

/// Detect a `corepack enable`/`corepack disable` invocation and return the
/// user args to run it with, injecting `--install-directory <bin_dir>` when
/// not explicitly set so launchers land on PATH.
///
/// Returns None for all other corepack commands (plain exec, no restore).
fn corepack_link_command_args(args: &[String], bin_dir: &AbsolutePath) -> Option<Vec<String>> {
    let subcommand = args.iter().find(|arg| !arg.starts_with('-'))?;
    if subcommand != "enable" && subcommand != "disable" {
        return None;
    }
    // Help output doesn't touch link files; run it as-is.
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        return None;
    }

    let mut rewritten = args.to_vec();
    let has_install_directory = args
        .iter()
        .any(|arg| arg == "--install-directory" || arg.starts_with("--install-directory="));
    if !has_install_directory {
        rewritten.push("--install-directory".to_string());
        rewritten.push(bin_dir.as_path().display().to_string());
    }
    Some(rewritten)
}

/// Restore Vite+-owned shims that corepack `enable`/`disable` may have
/// removed or replaced in the bin directory.
///
/// - Default shims (npm/npx) always belong to Vite+; they already resolve the
///   project package-manager version, so corepack must not manage them.
/// - Binaries installed via `vp install -g` (BinConfig source `vp`) are
///   restored with a warning pointing at `vp remove -g`.
async fn restore_vp_owned_shims(bin_dir: &AbsolutePath) {
    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };
    // Resolve through the shim symlink chain so recreated shims point at the
    // real vp binary, not at this process's `corepack` shim path.
    let current_exe = tokio::fs::canonicalize(&current_exe).await.unwrap_or(current_exe);

    for name in COREPACK_MANAGED_BIN_NAMES {
        if setup::SHIM_TOOLS.contains(name) {
            if core_shim_intact(bin_dir, name).await {
                continue;
            }
            let _ = tokio::fs::remove_file(bin_dir.join(name)).await;
            match setup::create_shim(&current_exe, bin_dir, name, false).await {
                Ok(_) => output::warn(&format!(
                    "'{name}' is managed by Vite+ and was restored. Vite+ already resolves \
                     '{name}' per project, so corepack does not need to manage it."
                )),
                Err(e) => tracing::warn!("Failed to restore '{}' shim: {}", name, e),
            }
            // Remove corepack's extensionless/cmd launchers that would shadow
            // the trampoline .exe in Git Bash.
            #[cfg(windows)]
            setup::cleanup_legacy_windows_shim(bin_dir, name).await;
            continue;
        }

        // Binaries installed via `vp install -g`.
        let Ok(Some(bin_config)) = BinConfig::load(name).await else {
            continue;
        };
        if bin_config.source != BinSource::Vp || is_vp_shim(bin_dir, name).await {
            continue;
        }
        output::warn(&format!(
            "'{name}' is managed by `vp install -g {pkg}` and was restored. \
             Run `vp remove -g {pkg}` first to let corepack manage '{name}'.",
            pkg = bin_config.package
        ));
        if let Err(e) = crate::commands::global::install::create_package_shim(
            bin_dir,
            name,
            &bin_config.package,
        )
        .await
        {
            tracing::warn!("Failed to restore '{}' shim: {}", name, e);
        }
    }
}

/// Check whether a default shim (npm/npx) is still an intact Vite+ shim.
///
/// Vite+ shims always link to the vp binary (relative `../current/bin/vp` or
/// an absolute path in dev layouts); corepack launchers link to corepack's
/// `dist/*.js` files. Broken symlinks count as not intact.
#[cfg(unix)]
async fn core_shim_intact(bin_dir: &AbsolutePath, name: &str) -> bool {
    let shim_path = bin_dir.join(name);
    match tokio::fs::read_link(&shim_path).await {
        Ok(target) => {
            target.file_name().is_some_and(|file_name| file_name == "vp")
                && std::fs::exists(shim_path.as_path()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Check whether a default shim (npm/npx) is still an intact Vite+ shim.
#[cfg(windows)]
async fn core_shim_intact(bin_dir: &AbsolutePath, name: &str) -> bool {
    is_vp_shim(bin_dir, name).await
}

/// Check whether the bin entry is an intact Vite+ shim.
#[cfg(unix)]
async fn is_vp_shim(bin_dir: &AbsolutePath, name: &str) -> bool {
    match tokio::fs::read_link(bin_dir.join(name)).await {
        Ok(target) => target == std::path::Path::new("../current/bin/vp"),
        Err(_) => false,
    }
}

/// Check whether the bin entry is an intact Vite+ shim.
///
/// Trampoline shims are `.exe` files; corepack's cmd-shim launchers are
/// `.cmd`/`.ps1`/extensionless files that would shadow the trampoline in
/// Git Bash, so their presence means the shim needs restoring.
#[cfg(windows)]
async fn is_vp_shim(bin_dir: &AbsolutePath, name: &str) -> bool {
    let exe_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.exe"))).await.unwrap_or(false);
    let cmd_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.cmd"))).await.unwrap_or(false);
    let sh_exists = tokio::fs::try_exists(&bin_dir.join(name)).await.unwrap_or(false);
    exe_exists && !cmd_exists && !sh_exists
}

#[cfg(test)]
mod tests {
    use vite_path::AbsolutePathBuf;

    use super::*;

    fn bin_dir() -> AbsolutePathBuf {
        #[cfg(windows)]
        {
            AbsolutePathBuf::new(std::path::PathBuf::from("C:\\Users\\test\\.vite-plus\\bin"))
                .unwrap()
        }
        #[cfg(not(windows))]
        {
            AbsolutePathBuf::new(std::path::PathBuf::from("/home/test/.vite-plus/bin")).unwrap()
        }
    }

    fn s(strs: &[&str]) -> Vec<String> {
        strs.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn test_link_command_args_injects_install_directory() {
        let bin_dir = bin_dir();
        let rewritten = corepack_link_command_args(&s(&["enable"]), &bin_dir).unwrap();
        assert_eq!(
            rewritten,
            s(&["enable", "--install-directory", &bin_dir.as_path().display().to_string()])
        );

        let rewritten = corepack_link_command_args(&s(&["disable", "yarn"]), &bin_dir).unwrap();
        assert_eq!(
            rewritten,
            s(&[
                "disable",
                "yarn",
                "--install-directory",
                &bin_dir.as_path().display().to_string()
            ])
        );
    }

    #[test]
    fn test_link_command_args_keeps_explicit_install_directory() {
        let bin_dir = bin_dir();
        let args = s(&["enable", "--install-directory", "/custom/dir"]);
        let rewritten = corepack_link_command_args(&args, &bin_dir).unwrap();
        assert_eq!(rewritten, args);

        let args = s(&["enable", "--install-directory=/custom/dir"]);
        let rewritten = corepack_link_command_args(&args, &bin_dir).unwrap();
        assert_eq!(rewritten, args);
    }

    #[test]
    fn test_link_command_args_ignores_other_commands() {
        let bin_dir = bin_dir();
        assert!(corepack_link_command_args(&s(&[]), &bin_dir).is_none());
        assert!(corepack_link_command_args(&s(&["--version"]), &bin_dir).is_none());
        assert!(corepack_link_command_args(&s(&["use", "pnpm@9"]), &bin_dir).is_none());
        assert!(corepack_link_command_args(&s(&["pnpm", "install"]), &bin_dir).is_none());
        assert!(corepack_link_command_args(&s(&["up"]), &bin_dir).is_none());
    }

    #[test]
    fn test_link_command_args_skips_help() {
        let bin_dir = bin_dir();
        assert!(corepack_link_command_args(&s(&["enable", "--help"]), &bin_dir).is_none());
        assert!(corepack_link_command_args(&s(&["enable", "-h"]), &bin_dir).is_none());
    }
}
