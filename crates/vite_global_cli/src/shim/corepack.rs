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
        create_bin_link, ensure_installed, find_package_for_binary, locate_tool,
        package_binary_invocation, resolve_with_cache,
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
    let CorepackInvocation { program, pre_args } = invocation;
    let mut full_args = pre_args;

    // enable/disable create or remove launchers in the install directory.
    // Inject the vp bin dir (so they land on PATH), run with spawn+wait, and
    // restore any Vite+-owned shims corepack removed or replaced. The arg
    // check runs first so the common path skips bin-dir resolution entirely.
    if is_corepack_link_command(args) {
        match config::get_bin_dir() {
            Ok(bin_dir) => {
                full_args.extend(inject_install_directory(args, &bin_dir));
                let owned_shims = snapshot_vp_owned_shims(&bin_dir).await;
                let exit_code = exec::spawn_tool(&program, &full_args);
                restore_vp_owned_shims(&bin_dir, &owned_shims).await;
                return exit_code;
            }
            Err(e) => {
                // Without a bin dir there is nothing to inject or restore;
                // run corepack as-is, but say so instead of failing silently.
                output::warn(&format!(
                    "Cannot resolve the Vite+ bin directory ({e}); running corepack without \
                     an --install-directory default, created launchers may not be on PATH"
                ));
            }
        }
    }

    full_args.extend(args.iter().cloned());
    exec::exec_tool(&program, &full_args)
}

/// Resolve which corepack binary to execute.
///
/// Returns an exit code on failure (errors are already printed).
async fn resolve_corepack_invocation() -> Result<CorepackInvocation, i32> {
    // 1. An explicit `vp install -g corepack` wins. Resolution errors (stale
    //    metadata, missing package files) fall through to the bundled copy
    //    instead of failing: broken managed state must not brick the shim.
    match managed_corepack_invocation().await {
        Ok(Some(invocation)) => return Ok(invocation),
        Ok(None) => {}
        Err(e) => {
            output::warn(&format!(
                "Ignoring unusable vp-managed corepack ({e}); falling back to the \
                 Node-bundled corepack. Run `vp remove -g corepack` to clear it."
            ));
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
        // The bundled corepack sits in the same bin directory as node;
        // prepend it so corepack's child processes see the same runtime.
        if let Some(node_bin_dir) = corepack_path.parent() {
            let _ = prepend_to_path_env(node_bin_dir, PrependOptions::default());
        }
        // Match the core-tool dispatch: nested core-tool shims pass through.
        // SAFETY: Setting env vars at this point before exec/spawn is safe
        unsafe {
            std::env::set_var(env_vars::VP_TOOL_RECURSION, "1");
        }
        return Ok(CorepackInvocation { program: corepack_path, pre_args: Vec::new() });
    }

    // 3. No usable corepack in the resolved Node.js (Node.js 25+ no longer
    //    bundles it; a bundled copy may also have been removed, e.g. by
    //    `npm uninstall -g corepack`): install it as a vp-managed global
    //    package, then run that copy. Only the `corepack` bin is linked; the
    //    pnpm/yarn launchers the package also declares stay unexposed (that
    //    is `corepack enable`'s job) and must not conflict with vp-managed
    //    package managers. The notice goes to stderr so the wrapped
    //    corepack's stdout stays parseable.
    eprintln!(
        "vp: corepack is not available for Node.js {}; installing it as a managed global package",
        resolution.version
    );
    // Preserve the shape of a previous explicit (unrestricted) install: the
    // reinstall must not silently drop launcher bins the user had exposed
    // (the stale-bin cleanup would delete their pnpm/yarn shims).
    let unrestricted = matches!(
        crate::commands::env::package_metadata::PackageMetadata::load("corepack").await,
        Ok(Some(previous)) if !previous.bins_restricted
    );
    let only_bins: Option<&[&str]> = if unrestricted { None } else { Some(&["corepack"]) };
    if let Err((_, error)) = crate::commands::global::install::install(
        &["corepack".to_string()],
        None,
        false,
        1,
        false,
        only_bins,
        // Keep the wrapped corepack's stdout parseable
        true,
    )
    .await
    {
        eprintln!("vp: Failed to install corepack: {error}");
        eprintln!("vp: Run 'vp install -g corepack' manually, then retry");
        return Err(1);
    }
    match managed_corepack_invocation().await {
        Ok(Some(invocation)) => Ok(invocation),
        Ok(None) => {
            eprintln!("vp: corepack was installed but its binary could not be located");
            Err(1)
        }
        Err(e) => {
            eprintln!("vp: corepack was installed but cannot be resolved: {e}");
            Err(1)
        }
    }
}

/// Resolve a corepack installed via `vp install -g corepack`, if any.
///
/// Uses the install-time Node.js version, like every other vp-managed
/// package binary.
async fn managed_corepack_invocation() -> Result<Option<CorepackInvocation>, String> {
    let Some(metadata) = find_package_for_binary("corepack").await? else {
        return Ok(None);
    };
    let (program, pre_args) =
        package_binary_invocation(&metadata, "corepack", &metadata.platform.node).await?;
    Ok(Some(CorepackInvocation { program, pre_args }))
}

/// Check whether the args invoke `corepack enable`/`corepack disable`
/// (the commands that create or remove launchers in the install directory).
///
/// Everything after a `--` separator is positional (package-manager names),
/// so the subcommand and help flags are only looked for before it.
fn is_corepack_link_command(args: &[String]) -> bool {
    let mut subcommand = None;
    for arg in args {
        if arg == "--" {
            break;
        }
        // Help output doesn't touch link files; run it as-is.
        if arg == "-h" || arg == "--help" {
            return false;
        }
        if subcommand.is_none() && !arg.starts_with('-') {
            subcommand = Some(arg.as_str());
        }
    }
    matches!(subcommand, Some("enable" | "disable"))
}

/// Return the user args for an intercepted `corepack enable`/`disable` run,
/// injecting `--install-directory <bin_dir>` when not explicitly set so the
/// created launchers land on PATH. The flag is inserted before any `--`
/// separator; tokens after it are package-manager names.
fn inject_install_directory(args: &[String], bin_dir: &AbsolutePath) -> Vec<String> {
    let mut rewritten = args.to_vec();
    let has_install_directory = args
        .iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--install-directory" || arg.starts_with("--install-directory="));
    if !has_install_directory {
        let insert_at = args.iter().position(|arg| arg == "--").unwrap_or(args.len());
        rewritten.insert(insert_at, bin_dir.as_path().display().to_string());
        rewritten.insert(insert_at, "--install-directory".to_string());
    }
    rewritten
}

/// A Vite+-owned bin entry that was intact before corepack ran.
enum OwnedShim {
    /// Default shim (npm/npx) — always belongs to Vite+.
    Core { name: &'static str },
    /// Binary installed via `vp install -g` (BinConfig source `vp`).
    Package { bin_config: BinConfig },
    /// Direct link created by the `npm install -g` interception
    /// (BinConfig source `npm`). `source` is the link target captured at
    /// snapshot time (Unix); when unavailable the restore falls back to the
    /// managed Node.js layout via `locate_tool`.
    NpmLink { bin_config: BinConfig, source: Option<AbsolutePathBuf> },
}

/// Snapshot which Vite+-owned shims among the corepack-managed launcher
/// names are intact before corepack runs. Only entries in this snapshot are
/// candidates for restoration, so shims the user removed on purpose are not
/// resurrected and untouched entries produce no spurious warnings.
///
/// Default shims already replaced by a corepack launcher (e.g. a previous
/// interrupted run) are included too, so the restore self-heals them.
async fn snapshot_vp_owned_shims(bin_dir: &AbsolutePath) -> Vec<OwnedShim> {
    let mut owned = Vec::new();
    for name in COREPACK_MANAGED_BIN_NAMES {
        if setup::SHIM_TOOLS.contains(name) {
            if core_shim_intact(bin_dir, name).await
                || corepack_launcher_present(bin_dir, name).await
            {
                owned.push(OwnedShim::Core { name });
            }
            continue;
        }
        let bin_config = match BinConfig::load(name).await {
            Ok(Some(config)) => config,
            Ok(None) => continue,
            Err(e) => {
                tracing::warn!("Skipping shim snapshot for '{}': {}", name, e);
                continue;
            }
        };
        match bin_config.source {
            BinSource::Vp => {
                if is_vp_shim(bin_dir, name).await {
                    owned.push(OwnedShim::Package { bin_config });
                }
            }
            BinSource::Npm => {
                if npm_link_intact(bin_dir, name).await {
                    let source = npm_link_source(bin_dir, name).await;
                    owned.push(OwnedShim::NpmLink { bin_config, source });
                }
            }
        }
    }
    owned
}

/// Restore Vite+-owned shims that corepack `enable`/`disable` removed or
/// replaced, based on the pre-invocation snapshot.
async fn restore_vp_owned_shims(bin_dir: &AbsolutePath, owned_shims: &[OwnedShim]) {
    // Resolved through the shim symlink chain so recreated shims point at the
    // real vp binary, not at this process's `corepack` shim path. A failure
    // only disables core-shim restores; package and npm-link restores below
    // don't need the executable path.
    let resolved_exe = if owned_shims.iter().any(|shim| matches!(shim, OwnedShim::Core { .. })) {
        match std::env::current_exe() {
            Ok(exe) => Some(tokio::fs::canonicalize(&exe).await.unwrap_or(exe)),
            Err(e) => {
                tracing::warn!("Cannot resolve the current executable for shim restore: {}", e);
                None
            }
        }
    } else {
        None
    };

    for shim in owned_shims {
        match shim {
            OwnedShim::Core { name } => {
                if core_shim_intact(bin_dir, name).await {
                    continue;
                }
                let Some(exe) = &resolved_exe else { continue };
                let _ = tokio::fs::remove_file(bin_dir.join(name)).await;
                match setup::create_shim(exe, bin_dir, name, false).await {
                    Ok(_) => output::warn(&format!(
                        "'{name}' is managed by Vite+ and was restored. Vite+ already resolves \
                         '{name}' per project, so corepack does not need to manage it."
                    )),
                    Err(e) => tracing::warn!("Failed to restore '{}' shim: {}", name, e),
                }
                // Remove corepack's extensionless/cmd/ps1 launchers that would
                // shadow the trampoline .exe in Git Bash or PowerShell.
                #[cfg(windows)]
                setup::cleanup_legacy_windows_shim(bin_dir, name).await;
            }
            OwnedShim::Package { bin_config } => {
                let name = bin_config.name.as_str();
                if is_vp_shim(bin_dir, name).await {
                    continue;
                }
                match crate::commands::global::install::create_package_shim(
                    bin_dir,
                    name,
                    &bin_config.package,
                )
                .await
                {
                    Ok(()) => output::warn(&format!(
                        "'{name}' is managed by `vp install -g {pkg}` and was restored. \
                         Run `vp remove -g {pkg}` first to let corepack manage '{name}'.",
                        pkg = bin_config.package
                    )),
                    Err(e) => tracing::warn!("Failed to restore '{}' shim: {}", name, e),
                }
            }
            OwnedShim::NpmLink { bin_config, source } => {
                let name = bin_config.name.as_str();
                if npm_link_intact(bin_dir, name).await {
                    continue;
                }
                // Prefer the captured original target; fall back to the
                // managed Node.js layout (validated per-OS by locate_tool).
                let source_path = match source {
                    Some(path) => path.clone(),
                    None => match locate_tool(&bin_config.node_version, name) {
                        Ok(path) => path,
                        Err(e) => {
                            output::warn(&format!(
                                "'{name}' was linked by `npm install -g {pkg}` and removed by \
                                 corepack, but its source could not be located ({e}). \
                                 Run `npm install -g {pkg}` to recreate it.",
                                pkg = bin_config.package
                            ));
                            continue;
                        }
                    },
                };
                output::warn(&format!(
                    "'{name}' was linked by `npm install -g {pkg}`; restoring the link.",
                    pkg = bin_config.package
                ));
                let _ = tokio::fs::remove_file(bin_dir.join(name)).await;
                create_bin_link(
                    bin_dir,
                    name,
                    &source_path,
                    &bin_config.package,
                    &bin_config.node_version,
                );
            }
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

/// Check whether the bin entry currently holds a corepack launcher (the shape
/// corepack `enable` writes). Used to self-heal default shims clobbered by a
/// previous run that never reached its restore step: present launchers are
/// snapshotted as Vite+-owned, while an absent entry (deliberately removed by
/// the user) is not.
#[cfg(unix)]
async fn corepack_launcher_present(bin_dir: &AbsolutePath, name: &str) -> bool {
    match tokio::fs::read_link(bin_dir.join(name)).await {
        // corepack launchers are symlinks to corepack's dist/<name>.js
        Ok(target) => {
            target.extension().is_some_and(|extension| extension == "js" || extension == "cjs")
        }
        Err(_) => false,
    }
}

/// Check whether the bin entry currently holds a corepack launcher.
///
/// corepack's cmd-shim writes `.cmd`/`.ps1`/extensionless wrappers; Vite+
/// never creates those for default shim names.
#[cfg(windows)]
async fn corepack_launcher_present(bin_dir: &AbsolutePath, name: &str) -> bool {
    let cmd_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.cmd"))).await.unwrap_or(false);
    let ps1_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.ps1"))).await.unwrap_or(false);
    let sh_exists = tokio::fs::try_exists(&bin_dir.join(name)).await.unwrap_or(false);
    cmd_exists || ps1_exists || sh_exists
}

/// Capture the current target of an npm-interception link so the restore can
/// recreate it exactly (it may point at a custom npm prefix, not the managed
/// Node.js directory).
#[cfg(unix)]
async fn npm_link_source(bin_dir: &AbsolutePath, name: &str) -> Option<AbsolutePathBuf> {
    let target = tokio::fs::read_link(bin_dir.join(name)).await.ok()?;
    AbsolutePathBuf::new(target)
}

/// Capture the current target of an npm-interception link.
///
/// On Windows the link is a `.cmd` wrapper whose target is embedded in its
/// body; reconstructing it is not worth the parsing, so the restore falls
/// back to the managed Node.js layout via `locate_tool`.
#[cfg(windows)]
async fn npm_link_source(_bin_dir: &AbsolutePath, _name: &str) -> Option<AbsolutePathBuf> {
    None
}

/// Check whether the bin entry is an intact Vite+ package shim.
#[cfg(unix)]
async fn is_vp_shim(bin_dir: &AbsolutePath, name: &str) -> bool {
    match tokio::fs::read_link(bin_dir.join(name)).await {
        Ok(target) => {
            target == std::path::Path::new(crate::commands::global::install::PACKAGE_SHIM_TARGET)
        }
        Err(_) => false,
    }
}

/// Check whether the bin entry is an intact Vite+ package shim.
///
/// Trampoline shims are `.exe` files; corepack's cmd-shim launchers are
/// `.cmd`/`.ps1`/extensionless files that shadow the trampoline in Git Bash
/// (extensionless) and PowerShell (`.ps1`), so any of them present means the
/// shim needs restoring.
#[cfg(windows)]
async fn is_vp_shim(bin_dir: &AbsolutePath, name: &str) -> bool {
    let exe_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.exe"))).await.unwrap_or(false);
    let cmd_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.cmd"))).await.unwrap_or(false);
    let ps1_exists =
        tokio::fs::try_exists(&bin_dir.join(format!("{name}.ps1"))).await.unwrap_or(false);
    let sh_exists = tokio::fs::try_exists(&bin_dir.join(name)).await.unwrap_or(false);
    exe_exists && !cmd_exists && !ps1_exists && !sh_exists
}

/// Check whether a link created by the `npm install -g` interception is
/// still intact.
///
/// On Unix these are symlinks pointing at the binary of the same name in a
/// managed Node.js bin directory; corepack launchers point at `dist/*.js`
/// files instead.
#[cfg(unix)]
async fn npm_link_intact(bin_dir: &AbsolutePath, name: &str) -> bool {
    let link_path = bin_dir.join(name);
    match tokio::fs::read_link(&link_path).await {
        Ok(target) => {
            target.file_name().is_some_and(|file_name| file_name == name)
                && std::fs::exists(link_path.as_path()).unwrap_or(false)
        }
        Err(_) => false,
    }
}

/// Check whether a link created by the `npm install -g` interception is
/// still intact.
///
/// On Windows npm links are `.cmd` wrappers. corepack also writes `.cmd`
/// launchers, so an overwritten (rather than deleted) link is not detected;
/// the deletion case (`corepack disable`) is the one that matters.
#[cfg(windows)]
async fn npm_link_intact(bin_dir: &AbsolutePath, name: &str) -> bool {
    tokio::fs::try_exists(&bin_dir.join(format!("{name}.cmd"))).await.unwrap_or(false)
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
    fn test_is_corepack_link_command() {
        assert!(is_corepack_link_command(&s(&["enable"])));
        assert!(is_corepack_link_command(&s(&["disable", "yarn"])));
        assert!(is_corepack_link_command(&s(&["enable", "--install-directory", "/custom"])));
        assert!(is_corepack_link_command(&s(&["enable", "--", "pnpm"])));

        assert!(!is_corepack_link_command(&s(&[])));
        assert!(!is_corepack_link_command(&s(&["--version"])));
        assert!(!is_corepack_link_command(&s(&["use", "pnpm@9"])));
        assert!(!is_corepack_link_command(&s(&["pnpm", "install"])));
        assert!(!is_corepack_link_command(&s(&["up"])));

        // Everything after `--` is positional, not a subcommand
        assert!(!is_corepack_link_command(&s(&["--", "enable"])));

        // Help output doesn't touch link files
        assert!(!is_corepack_link_command(&s(&["enable", "--help"])));
        assert!(!is_corepack_link_command(&s(&["enable", "-h"])));
    }

    #[test]
    fn test_inject_install_directory_appends_when_missing() {
        let bin_dir = bin_dir();
        let rewritten = inject_install_directory(&s(&["enable"]), &bin_dir);
        assert_eq!(
            rewritten,
            s(&["enable", "--install-directory", &bin_dir.as_path().display().to_string()])
        );

        let rewritten = inject_install_directory(&s(&["disable", "yarn"]), &bin_dir);
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
    fn test_inject_install_directory_keeps_explicit_value() {
        let bin_dir = bin_dir();
        let args = s(&["enable", "--install-directory", "/custom/dir"]);
        assert_eq!(inject_install_directory(&args, &bin_dir), args);

        let args = s(&["enable", "--install-directory=/custom/dir"]);
        assert_eq!(inject_install_directory(&args, &bin_dir), args);
    }

    #[test]
    fn test_inject_install_directory_inserts_before_separator() {
        let bin_dir = bin_dir();
        let dir = bin_dir.as_path().display().to_string();

        // The injected flag must precede `--`; tokens after it are
        // package-manager names.
        let rewritten = inject_install_directory(&s(&["enable", "--", "pnpm"]), &bin_dir);
        assert_eq!(rewritten, s(&["enable", "--install-directory", &dir, "--", "pnpm"]));

        // An --install-directory after `--` is a positional, not the flag
        let rewritten =
            inject_install_directory(&s(&["enable", "--", "--install-directory"]), &bin_dir);
        assert_eq!(
            rewritten,
            s(&["enable", "--install-directory", &dir, "--", "--install-directory"])
        );
    }
}
