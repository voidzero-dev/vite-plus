//! Setup command implementation for creating bin directory and shims.
//!
//! Creates the following structure:
//! - `<BIN>/`     - Contains vp symlink and node/npm/npx/corepack shims
//! - `<DATA>/current/` - Contains the actual vp CLI binary
//!
//! On Unix:
//! - bin/vp is a symlink to the active vp binary
//! - bin/node, bin/npm, bin/npx, bin/corepack are symlinks to the active vp binary
//! - Symlinks preserve argv[0], allowing tool detection via the symlink name
//!
//! On Windows:
//! - bin/vp.exe, bin/node.exe, bin/npm.exe, bin/npx.exe, bin/corepack.exe are trampoline executables
//! - Each trampoline detects its tool name from its own filename and spawns
//!   current\bin\vp.exe with VP_SHIM_TOOL env var set
//! - This avoids the "Terminate batch job (Y/N)?" prompt from .cmd wrappers

use std::process::ExitStatus;

use crate::{error::Error, help};

/// Shells that get a generated `<CONFIG>/env.*` setup script.
#[derive(Clone, Copy, Debug)]
enum EnvShell {
    Posix,
    Fish,
    Nu,
    Powershell,
}

const UTF8_BOM: char = '\u{feff}';

impl EnvShell {
    /// File name written under `<CONFIG>/` for this shell's setup script.
    const fn env_file_name(self) -> &'static str {
        match self {
            EnvShell::Posix => "env",
            EnvShell::Fish => "env.fish",
            EnvShell::Nu => "env.nu",
            EnvShell::Powershell => "env.ps1",
        }
    }
}

/// Tools to create shims for (node, npm, npx, corepack, vpx, vpr)
pub(crate) const SHIM_TOOLS: &[&str] = &["node", "npm", "npx", "corepack", "vpx", "vpr"];

/// Execute the setup command.
pub async fn execute(refresh: bool, env_only: bool) -> Result<ExitStatus, Error> {
    let config = vp_shared::EnvConfig::get();
    let dirs = &config.dirs;

    // Ensure config directory exists (env files are written here)
    tokio::fs::create_dir_all(&dirs.config).await?;

    // Create env files with PATH guard (prevents duplicate PATH entries)
    create_env_files().await?;

    if env_only {
        println!("{}", help::render_heading("Setup"));
        println!("  Updated shell environment files.");
        println!("  Run {} to verify setup.", help::accent_command("vp env doctor"));
        return Ok(ExitStatus::default());
    }

    let bin_dir = &dirs.bin;

    println!("{}", help::render_heading("Setup"));
    println!("  Preparing vite-plus environment.");
    println!();

    // Ensure bin directory exists
    tokio::fs::create_dir_all(bin_dir).await?;

    #[cfg(windows)]
    tokio::fs::write(bin_dir.join("vp-use.cmd"), vp_use_cmd_content(&config)).await?;

    // Get the current executable path (for shims)
    let current_exe = std::env::current_exe()?;

    // Create wrapper script in bin/
    setup_vp_wrapper(&current_exe, bin_dir, refresh).await?;

    // Create shims for node, npm, npx, corepack
    let mut created = Vec::new();
    let mut skipped = Vec::new();

    for tool in SHIM_TOOLS {
        let result = create_shim(&current_exe, bin_dir, tool, refresh).await?;
        if result {
            created.push(*tool);
        } else {
            skipped.push(*tool);
        }

        // Remove corepack-written .cmd/.ps1/extensionless launchers that
        // would shadow an existing trampoline .exe in PowerShell/Git Bash
        // (create_shim skips existing shims without cleaning siblings).
        #[cfg(windows)]
        cleanup_legacy_windows_shim(bin_dir, tool).await;

        // Drop stale `npm install -g` link configs for default shim names
        // (e.g. a pre-default-shim `npm i -g corepack`): the link itself is
        // replaced by the shim above, and a leftover Npm-sourced BinConfig
        // would let a later `npm uninstall -g` delete the default shim.
        if let Ok(Some(config)) = super::bin_config::BinConfig::load(tool).await
            && config.source == super::bin_config::BinSource::Npm
        {
            let _ = super::bin_config::BinConfig::delete(tool).await;
        }
    }

    #[cfg(windows)]
    if refresh {
        if let Err(e) = refresh_package_shims(bin_dir).await {
            tracing::warn!("Failed to refresh package shims: {}", e);
        }
    }

    // Best-effort cleanup of .old files from rename-before-copy on Windows
    #[cfg(windows)]
    if refresh {
        cleanup_old_files(bin_dir).await;
    }

    // Print results
    if !created.is_empty() {
        println!("{}", help::render_heading("Created Shims"));
        for tool in &created {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
    }

    if !skipped.is_empty() && !refresh {
        if !created.is_empty() {
            println!();
        }
        println!("{}", help::render_heading("Skipped Shims"));
        for tool in &skipped {
            let shim_path = bin_dir.join(shim_filename(tool));
            println!("  {}", shim_path.as_path().display());
        }
        println!();
        println!("  Use --refresh to update existing shims.");
    }

    println!();
    print_path_instructions(&dirs.config);

    Ok(ExitStatus::default())
}

/// Create symlink in bin/ that points to the active vp binary.
async fn setup_vp_wrapper(
    current_exe: &std::path::Path,
    bin_dir: &vt_path::AbsolutePath,
    refresh: bool,
) -> Result<(), Error> {
    #[cfg(unix)]
    {
        let bin_vp = bin_dir.join("vp");
        let target = resolve_unix_vp_shim_target(current_exe).await?;
        let existing = tokio::fs::symlink_metadata(&bin_vp).await.ok();

        let should_create_symlink = match existing.as_ref() {
            Some(metadata) if refresh || !metadata.file_type().is_symlink() => true,
            Some(_) => {
                let broken_symlink = !std::fs::exists(bin_vp.as_path()).unwrap_or(false);
                let wrong_target = tokio::fs::read_link(&bin_vp)
                    .await
                    .map(|existing_target| existing_target != target)
                    .unwrap_or(true);
                broken_symlink || wrong_target
            }
            None => true,
        };

        if should_create_symlink {
            // Remove existing if present (could be old wrapper script or file)
            if existing.is_some() {
                tokio::fs::remove_file(&bin_vp).await?;
            }
            tokio::fs::symlink(&target, &bin_vp).await?;
            tracing::debug!("Created symlink {:?} -> {:?}", bin_vp, target);
        }
    }

    #[cfg(windows)]
    {
        let _ = current_exe;
        let bin_vp_exe = bin_dir.join("vp.exe");

        // Create trampoline bin/vp.exe that forwards to current\bin\vp.exe
        let should_create = refresh || !tokio::fs::try_exists(&bin_vp_exe).await.unwrap_or(false);

        if should_create {
            let trampoline_src = get_trampoline_path()?;
            // On refresh, the existing vp.exe may still be running (the trampoline
            // that launched us). Windows prevents overwriting a running exe, so we
            // rename it to a timestamped .old file first, then copy the new one.
            if tokio::fs::try_exists(&bin_vp_exe).await.unwrap_or(false) {
                rename_to_old(&bin_vp_exe).await;
            }

            tokio::fs::copy(trampoline_src.as_path(), &bin_vp_exe).await?;
            write_shim_pointer_beside(bin_vp_exe.as_path());
            tracing::debug!("Created trampoline {:?}", bin_vp_exe);
        }

        // Clean up legacy .cmd and shell script wrappers from previous versions
        if refresh {
            cleanup_legacy_windows_shim(bin_dir, "vp").await;
        }
    }

    Ok(())
}

#[cfg(unix)]
pub(crate) async fn resolve_unix_vp_shim_target(
    current_exe: &std::path::Path,
) -> Result<std::path::PathBuf, Error> {
    let current_vp = crate::commands::global::install::package_shim_target();
    if tokio::fs::try_exists(&current_vp).await.unwrap_or(false) {
        let current_vp_canon = tokio::fs::canonicalize(&current_vp).await.ok();
        let current_exe_canon = tokio::fs::canonicalize(current_exe).await.ok();
        if current_vp_canon.is_some() && current_vp_canon == current_exe_canon {
            return Ok(current_vp.as_path().to_path_buf());
        }
    }

    Ok(current_exe.to_path_buf())
}

/// Create a single shim for a default shim tool (node/npm/npx/corepack/vpx/vpr).
///
/// Returns `true` if the shim was created, `false` if it already exists.
pub(crate) async fn create_shim(
    source: &std::path::Path,
    bin_dir: &vt_path::AbsolutePath,
    tool: &str,
    refresh: bool,
) -> Result<bool, Error> {
    let shim_path = bin_dir.join(shim_filename(tool));

    #[cfg(unix)]
    let desired_target = resolve_unix_vp_shim_target(source).await?;

    let existing = tokio::fs::symlink_metadata(&shim_path).await.ok();
    if existing.is_some() {
        let should_replace = if refresh {
            true
        } else {
            #[cfg(unix)]
            {
                existing.as_ref().is_some_and(|metadata| metadata.file_type().is_symlink())
                    && (!std::fs::exists(shim_path.as_path()).unwrap_or(false)
                        || tokio::fs::read_link(&shim_path)
                            .await
                            .map(|existing_target| existing_target != desired_target)
                            .unwrap_or(true))
            }

            #[cfg(windows)]
            {
                false
            }
        };

        if !should_replace {
            return Ok(false);
        }
        #[cfg(windows)]
        remove_or_rename_to_old(&shim_path).await;
        #[cfg(not(windows))]
        {
            tokio::fs::remove_file(&shim_path).await?;
        }
    }

    #[cfg(unix)]
    {
        create_unix_shim(source, &shim_path, tool).await?;
    }

    #[cfg(windows)]
    {
        create_windows_shim(source, bin_dir, tool).await?;
    }

    Ok(true)
}

/// Get the filename for a shim (platform-specific).
pub(crate) fn shim_filename(tool: &str) -> String {
    #[cfg(windows)]
    {
        // All tools use trampoline .exe files on Windows
        format!("{tool}.exe")
    }

    #[cfg(not(windows))]
    {
        tool.to_string()
    }
}

/// Create a Unix shim using symlink to the active vp binary.
///
/// Symlinks preserve argv[0], allowing the vp binary to detect which tool
/// was invoked. This is the same pattern used by Volta.
#[cfg(unix)]
async fn create_unix_shim(
    source: &std::path::Path,
    shim_path: &vt_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let target = resolve_unix_vp_shim_target(source).await?;
    tokio::fs::symlink(&target, shim_path).await?;
    tracing::debug!("Created {tool} symlink shim at {:?} -> {:?}", shim_path, target);

    Ok(())
}

/// Create Windows shims using trampoline `.exe` files.
///
/// Each tool gets a copy of the trampoline binary renamed to `<tool>.exe`.
/// The trampoline detects its tool name from its own filename and spawns
/// vp.exe with `VP_SHIM_TOOL` set, avoiding the "Terminate batch job?"
/// prompt that `.cmd` wrappers cause on Ctrl+C.
///
/// See: <https://github.com/voidzero-dev/vite-plus/issues/835>
#[cfg(windows)]
async fn create_windows_shim(
    _source: &std::path::Path,
    bin_dir: &vt_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let trampoline_src = get_trampoline_path()?;
    let shim_path = bin_dir.join(format!("{tool}.exe"));
    tokio::fs::copy(trampoline_src.as_path(), &shim_path).await?;
    write_shim_pointer_beside(shim_path.as_path());

    // Clean up legacy .cmd and shell script wrappers from previous versions
    cleanup_legacy_windows_shim(bin_dir, tool).await;

    tracing::debug!("Created trampoline shim {:?}", shim_path);

    Ok(())
}

/// Refresh trampoline `.exe` files for package shims installed via `vp install -g`.
///
/// Discovers all package binaries tracked by BinConfig with `source: Vp`
/// and replaces their `.exe` with the current trampoline.
#[cfg(windows)]
async fn refresh_package_shims(bin_dir: &vt_path::AbsolutePath) -> Result<(), Error> {
    use super::bin_config::BinConfig;

    let package_bins = BinConfig::find_all_vp_source().await?;

    if package_bins.is_empty() {
        return Ok(());
    }

    let trampoline_src = get_trampoline_path()?;

    for bin_name in &package_bins {
        // Core shims (SHIM_TOOLS + vp) are already refreshed by the main loop.
        if bin_name == "vp" || SHIM_TOOLS.contains(&bin_name.as_str()) {
            continue;
        }

        let shim_path = bin_dir.join(format!("{bin_name}.exe"));

        remove_or_rename_to_old(&shim_path).await;

        if let Err(e) = tokio::fs::copy(trampoline_src.as_path(), &shim_path).await {
            tracing::warn!("Failed to refresh package shim {}: {}", bin_name, e);
            continue;
        }
        write_shim_pointer_beside(shim_path.as_path());

        // Remove legacy .cmd/shell wrappers that could shadow the .exe in Git Bash.
        cleanup_legacy_windows_shim(bin_dir, bin_name).await;

        tracing::debug!("Refreshed package trampoline shim {:?}", shim_path);
    }

    Ok(())
}

/// Write `<name>.shim` next to a trampoline copy, with the resolved layout.
#[cfg(windows)]
fn write_shim_pointer_beside(exe_path: &std::path::Path) {
    if let Err(e) = vp_shared::EnvConfig::get().dirs.write_shim_pointer_beside(exe_path) {
        tracing::warn!("Vite+ could not write the shim pointer for {}: {e}", exe_path.display());
    }
}

/// Get the path to the trampoline template binary (vp-shim.exe).
///
/// The trampoline binary is distributed alongside vp.exe in the same directory.
/// In tests, `VP_TRAMPOLINE_PATH` can override the resolved path.
#[cfg(windows)]
pub(crate) fn get_trampoline_path() -> Result<vt_path::AbsolutePathBuf, Error> {
    // Allow tests to override the trampoline path
    if let Ok(override_path) = std::env::var(vp_shared::env_vars::VP_TRAMPOLINE_PATH) {
        let path = std::path::PathBuf::from(override_path);
        if path.exists() {
            return vt_path::AbsolutePathBuf::new(path)
                .ok_or_else(|| Error::Other("Invalid trampoline override path".into()));
        }
    }

    let current_exe = std::env::current_exe()?;
    let bin_dir = current_exe
        .parent()
        .ok_or_else(|| Error::Other("Cannot find parent directory of vp.exe".into()))?;
    let trampoline = bin_dir.join("vp-shim.exe");

    if !trampoline.exists() {
        return Err(Error::Other(
            format!(
                "Trampoline binary not found at {}. Re-install vite-plus to fix this.",
                trampoline.display()
            )
            .into(),
        ));
    }

    vt_path::AbsolutePathBuf::new(trampoline)
        .ok_or_else(|| Error::Other("Invalid trampoline path".into()))
}

/// Try to delete an `.exe` file; if deletion fails (e.g., file is locked by a
/// running process), fall back to renaming it to `.old`.
///
/// This avoids accumulating `.old` files when the exe is not in use.
#[cfg(windows)]
pub(crate) async fn remove_or_rename_to_old(path: &vt_path::AbsolutePath) {
    match tokio::fs::remove_file(path).await {
        Ok(()) => return,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::debug!("remove_file failed ({}), attempting rename", e);
        }
    }
    rename_to_old(path).await;
}

/// Rename an existing `.exe` to a timestamped `.old` file instead of deleting.
///
/// On Windows, running `.exe` files can't be deleted or overwritten, but they can
/// be renamed. The `.old` files are cleaned up by `cleanup_old_files()`.
#[cfg(windows)]
async fn rename_to_old(path: &vt_path::AbsolutePath) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if let Some(name) = path.as_path().file_name().and_then(|n| n.to_str()) {
        let old_name = format!("{name}.{timestamp}.old");
        let old_path = path.as_path().with_file_name(&old_name);
        if let Err(e) = tokio::fs::rename(path, &old_path).await {
            tracing::warn!("Failed to rename {} to {}: {}", name, old_name, e);
        }
    }
}

/// Best-effort cleanup of accumulated `.old` files from previous rename-before-copy operations.
///
/// When refreshing `bin/vp.exe` on Windows, the running trampoline is renamed to a
/// timestamped `.old` file. This function tries to delete all such files. Files still
/// in use by a running process will silently fail to delete and be cleaned up next time.
#[cfg(windows)]
async fn cleanup_old_files(bin_dir: &vt_path::AbsolutePath) {
    let Ok(mut entries) = tokio::fs::read_dir(bin_dir).await else {
        return;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.ends_with(".old") {
            let _ = tokio::fs::remove_file(entry.path()).await;
        }
    }
}

/// Remove legacy `.cmd` and shell script wrappers from previous versions.
#[cfg(windows)]
pub(crate) async fn cleanup_legacy_windows_shim(bin_dir: &vt_path::AbsolutePath, tool: &str) {
    // Remove old .cmd wrapper (best-effort, ignore NotFound)
    let cmd_path = bin_dir.join(format!("{tool}.cmd"));
    let _ = tokio::fs::remove_file(&cmd_path).await;

    // Remove .ps1 launchers (corepack's cmd-shim writes them; PowerShell
    // resolves `<tool>.ps1` ahead of `<tool>.exe`, so a leftover would shadow
    // the trampoline). Vite+ never creates per-tool .ps1 files in bin.
    let ps1_path = bin_dir.join(format!("{tool}.ps1"));
    let _ = tokio::fs::remove_file(&ps1_path).await;

    // Remove old shell script wrapper (extensionless, for Git Bash)
    // Only remove if it starts with #!/bin/sh (not a binary or other file)
    // Read only the first 9 bytes to avoid loading large files into memory
    let sh_path = bin_dir.join(tool);
    let is_shell_script = async {
        use tokio::io::AsyncReadExt;
        let mut file = tokio::fs::File::open(&sh_path).await.ok()?;
        let mut buf = [0u8; 9]; // b"#!/bin/sh".len()
        let n = file.read(&mut buf).await.ok()?;
        Some(buf[..n].starts_with(b"#!/bin/sh"))
        // file handle dropped here before remove_file
    }
    .await;
    if is_shell_script == Some(true) {
        let _ = tokio::fs::remove_file(&sh_path).await;
    }
}

// POSIX env file (bash/zsh)
// When sourced multiple times, removes existing entry and re-prepends to front
// Uses parameter expansion to split PATH around the bin entry in O(1) operations
// Includes vp() shell function wrapper for `vp env use` (evals stdout)
// Includes shell completion support
const ENV_TEMPLATE_POSIX: &str = r#"#!/bin/sh
# Vite+ environment setup (https://viteplus.dev)
__ENV_EXPORTS____vp_bin="__VP_BIN__"
case ":${PATH}:" in
    *":${__vp_bin}:"*)
        __vp_tmp=":${PATH}:"
        __vp_before="${__vp_tmp%%":${__vp_bin}:"*}"
        __vp_before="${__vp_before#:}"
        __vp_after="${__vp_tmp#*":${__vp_bin}:"}"
        __vp_after="${__vp_after%:}"
        export PATH="${__vp_bin}${__vp_before:+:${__vp_before}}${__vp_after:+:${__vp_after}}"
        unset __vp_tmp __vp_before __vp_after
        ;;
    *)
        export PATH="$__vp_bin:$PATH"
        ;;
esac
unset __vp_bin

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
vp() {
    if [ "$1" = "env" ] && [ "$2" = "use" ]; then
        case " $* " in *" -h "*|*" --help "*) command vp "$@"; return; esac
        __vp_out="$(VP_ENV_USE_EVAL_ENABLE=1 VP_SHELL=sh command vp "$@")" || return $?
        eval "$__vp_out"
    else
        command vp "$@"
    fi
}

# Dynamic shell completion for bash/zsh
if [ -n "$BASH_VERSION" ] && type complete >/dev/null 2>&1; then
    eval "$(VP_COMPLETE=bash command vp)"
elif [ -n "$ZSH_VERSION" ] && type compdef >/dev/null 2>&1; then
    eval "$(VP_COMPLETE=zsh command vp)"
    eval '
    _vpr_complete() {
        local -a orig=("${words[@]}")
        words=("vp" "run" "${orig[@]:1}")
        CURRENT=$((CURRENT + 1))
        ${=_comps[vp]}
    }
    compdef _vpr_complete vpr
    '
fi
"#;

const ENV_TEMPLATE_FISH: &str = r#"# Vite+ environment setup (https://viteplus.dev)
__ENV_EXPORTS__while set -l __vp_idx (contains -i -- "__VP_BIN__" $PATH)
    set -e PATH[$__vp_idx]
end
set -gx PATH "__VP_BIN__" $PATH

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
function vp
    if test (count $argv) -ge 2; and test "$argv[1]" = "env"; and test "$argv[2]" = "use"
        if contains -- -h $argv; or contains -- --help $argv
            command vp $argv; return
        end
        set -lx VP_ENV_USE_EVAL_ENABLE 1
        set -lx VP_SHELL fish
        set -l __vp_out (command vp $argv); or return $status
        for __vp_command in $__vp_out
            eval $__vp_command; or return $status
        end
        return 0
    else
        command vp $argv
    end
end

# Dynamic shell completion for fish
VP_COMPLETE=fish command vp | source

function __vpr_complete
    set -l tokens (commandline --current-process --tokenize --cut-at-cursor)
    set -l current (commandline --current-token)
    VP_COMPLETE=fish command vp -- vp run $tokens[2..] $current
end
complete -c vpr --keep-order --exclusive --arguments "(__vpr_complete)"
"#;

// Nushell env file with vp wrapper function.
// Completions delegate to Fish dynamically (VP_COMPLETE=fish) because clap_complete_nushell
// generates multiple rest params (e.g. for `vp install`), which Nushell does not support.
const ENV_TEMPLATE_NU: &str = r#"# Vite+ environment setup (https://viteplus.dev)
__ENV_EXPORTS__$env.PATH = ($env.PATH | where { $in != "__VP_BIN__" } | prepend "__VP_BIN__")

# Shell function wrapper: intercepts `vp env use` to parse its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
def --env --wrapped vp [...args: string@"nu-complete vp"] {
    if ($args | length) >= 2 and $args.0 == "env" and $args.1 == "use" {
        if ("-h" in $args) or ("--help" in $args) {
            ^vp ...$args
            return
        }
        let out = (with-env { VP_ENV_USE_EVAL_ENABLE: "1", VP_SHELL: "nu" } {
            ^vp ...$args
        })
        let lines = ($out | lines)
        let exports = ($lines | where { $in =~ '^\$env\.' } | parse '$env.{key} = "{value}"')
        let export_keys = ($exports | get key? | default [])
        # Exclude keys that also appear in exports: when vp emits `hide-env X` then
        # `$env.X = "v"` (e.g. `vp env use` with no args resolving from .node-version),
        # the set should win.
        let unsets = ($lines | where { $in =~ '^hide-env ' } | parse 'hide-env {key}' | get key? | default [] | where { $in not-in $export_keys })
        if ($exports | is-not-empty) {
            load-env ($exports | reduce -f {} {|it, acc| $acc | insert $it.key $it.value})
        }
        for key in $unsets {
            if ($key in $env) { hide-env $key }
        }
    } else {
        ^vp ...$args
    }
}

# Shell completion for nushell (delegates to fish completions dynamically)
def "nu-complete vp" [context: string] {
    let fish_cmd = $"VP_COMPLETE=fish command vp | source; complete '--do-complete=($context)'"
    fish --command $fish_cmd | from tsv --flexible --noheaders --no-infer | rename value description | update value {|row|
        let value = $row.value
        let need_quote = ['\' ',' '[' ']' '(' ')' ' ' '\t' "'" '"' "`"] | any {$in in $value}
        if ($need_quote and ($value | path exists)) {
            let expanded_path = if ($value starts-with ~) {$value | path expand --no-symlink} else {$value}
            $'"($expanded_path | str replace --all "\"" "\\\"")"'
        } else {$value}
    }
}
# Completion logic for vpr (translates context to 'vp run ...')
def "nu-complete vpr" [context: string] {
    let modified_context = ($context | str replace -r '^vpr' 'vp run')
    let fish_cmd = $"VP_COMPLETE=fish command vp | source; complete '--do-complete=($modified_context)'"
    fish --command $fish_cmd | from tsv --flexible --noheaders --no-infer | rename value description | update value {|row|
        let value = $row.value
        let need_quote = ['\' ',' '[' ']' '(' ')' ' ' '\t' "'" '"' "`"] | any {$in in $value}
        if ($need_quote and ($value | path exists)) {
            let expanded_path = if ($value starts-with ~) {$value | path expand --no-symlink} else {$value}
            $'"($expanded_path | str replace --all "\"" "\\\"")"'
        } else {$value}
    }
}
export extern "vpr" [...args: string@"nu-complete vpr"]
"#;

const ENV_TEMPLATE_PS1: &str = r#"# Vite+ environment setup (https://viteplus.dev)
__ENV_EXPORTS__$__vp_bin = '__VP_BIN_WIN__'
if ($env:Path -split ';' -notcontains $__vp_bin) {
    $env:Path = "$__vp_bin;$env:Path"
}

# Shell function wrapper: intercepts `vp env use` to eval its stdout,
# which sets/unsets VP_NODE_VERSION in the current shell session.
function vp {
    if ($args.Count -ge 2 -and $args[0] -eq "env" -and $args[1] -eq "use") {
        if ($args -contains "-h" -or $args -contains "--help") {
            & (Join-Path $__vp_bin "vp") @args; return
        }
        $env:VP_ENV_USE_EVAL_ENABLE = "1"
        $env:VP_SHELL = "pwsh"
        $output = & (Join-Path $__vp_bin "vp") @args 2>&1 | ForEach-Object {
            if ($_ -is [System.Management.Automation.ErrorRecord]) {
                Write-Host $_.Exception.Message
            } else {
                $_
            }
        }
        Remove-Item Env:VP_ENV_USE_EVAL_ENABLE -ErrorAction SilentlyContinue
        Remove-Item Env:VP_SHELL -ErrorAction SilentlyContinue
        if ($LASTEXITCODE -eq 0 -and $output) {
            Invoke-Expression ($output -join "`n")
        }
    } else {
        & (Join-Path $__vp_bin "vp") @args
    }
}

# Dynamic shell completion for PowerShell
$env:VP_COMPLETE = "powershell"
& (Join-Path $__vp_bin "vp") | Out-String | Invoke-Expression
Remove-Item Env:\VP_COMPLETE -ErrorAction SilentlyContinue

$__vpr_comp = {
    param($wordToComplete, $commandAst, $cursorPosition)
    $prev = $env:VP_COMPLETE
    $env:VP_COMPLETE = "powershell"
    $commandLine = $commandAst.Extent.Text
    $args = $commandLine.Substring(0, [math]::Min($cursorPosition, $commandLine.Length))
    $args = $args -replace '^(vpr\.exe|vpr)\b', 'vp run'
    if ($wordToComplete -eq "") { $args += " ''" }
    $results = Invoke-Expression @"
& (Join-Path $__vp_bin 'vp') -- $args
"@;
    if ($prev) { $env:VP_COMPLETE = $prev } else { Remove-Item Env:\VP_COMPLETE }
    $results | ForEach-Object {
        $split = $_.Split("`t")
        $cmd = $split[0];
        if ($split.Length -eq 2) { $help = $split[1] } else { $help = $split[0] }
        [System.Management.Automation.CompletionResult]::new($cmd, $cmd, 'ParameterValue', $help)
    }
}
Register-ArgumentCompleter -Native -CommandName vpr -ScriptBlock $__vpr_comp
"#;

// cmd.exe wrapper for `vp env use` (cmd.exe cannot define shell functions).
// Users run `vp-use 24` in cmd.exe instead of `vp env use 24`.
#[cfg(windows)]
fn vp_use_cmd_content(config: &vp_shared::EnvConfig) -> String {
    let vp_exe = config.dirs.data.join("current").join("bin").join("vp.exe");
    let mut exports: Vec<_> = config.dir_envs.iter().collect();
    exports.sort_unstable_by_key(|(name, _)| *name);
    let export_lines: String =
        exports.into_iter().map(|(name, value)| format!("set {name}={value}\r\n")).collect();
    format!(
        "@echo off\r\nset VP_ENV_USE_EVAL_ENABLE=1\r\n{export_lines}for /f \"delims=\" %%i in ('\"{}\" env use %*') do %%i\r\nset VP_ENV_USE_EVAL_ENABLE=\r\n",
        vp_exe.as_path().display()
    )
}

fn render_home_relative_path(path: &std::path::Path, home_dir: &std::path::Path) -> String {
    fn render_path(path: &std::path::Path) -> String {
        let rendered = path.display().to_string();
        // Windows: `C:\Users\xxx\.vite-plus` → `C:/Users/xxx/.vite-plus`
        // Unix: `/tmp/vp\home` → `/tmp/vp\home` (the backslash is preserved)
        if cfg!(windows) { rendered.replace('\\', "/") } else { rendered }
    }

    // Use $HOME-relative path if install dir is under HOME (like rustup's ~/.cargo/env).
    // This makes the env file portable across sessions where HOME may differ.
    path.strip_prefix(home_dir)
        .ok()
        .map(|s| {
            if s.as_os_str().is_empty() {
                "$HOME".to_string()
            } else {
                // Normalize to forward slashes for $HOME/... paths (POSIX-style)
                format!("$HOME/{}", render_path(s))
            }
        })
        .unwrap_or_else(|| render_path(path))
}

fn render_nu_path_ref(path_ref: &str) -> String {
    match path_ref.strip_prefix("$HOME") {
        Some("") => "~".to_string(),
        Some(suffix) if suffix.starts_with('/') => format!("~{suffix}"),
        _ => path_ref.to_string(),
    }
}

/// Escape a value for a POSIX-shell double-quoted string.
fn escape_posix_double_quoted_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('$', "\\$").replace('`', "\\`").replace('"', "\\\"")
}

/// Escape a value for a Fish double-quoted string.
fn escape_fish_double_quoted_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('$', "\\$").replace('"', "\\\"")
}

/// Escape only the literal suffix of a `$HOME`-relative path. Keep `$HOME`
/// available for expansion when the POSIX or Fish environment file runs.
fn escape_home_relative_double_quoted_path(path_ref: &str, escape: fn(&str) -> String) -> String {
    path_ref
        .strip_prefix("$HOME")
        .map_or_else(|| escape(path_ref), |suffix| format!("$HOME{}", escape(suffix)))
}

/// Escape a value for a Nushell double-quoted string.
///
/// Example: `vp "home\with spaces"` → `vp \"home\\with spaces\"`
/// https://www.nushell.sh/book/working_with_strings.html#double-quoted-strings
fn escape_nu_double_quoted_string(value: &str) -> String {
    // `vp "home\with spaces"` → `vp \"home\\with spaces\"`
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a value for a PowerShell single-quoted string.
fn escape_powershell_single_quoted_string(value: &str) -> String {
    value.replace('\'', "''")
}

/// Render export lines for the persisted layout values in
/// [`EnvConfig::dir_envs`]. The map contains only an explicit absolute
/// `VP_HOME`. Sort the lines to keep the output stable.
fn render_dir_envs(shell: EnvShell, config: &vp_shared::EnvConfig) -> String {
    let home_dir = config.user_home.as_path();
    let mut exports: Vec<_> = config.dir_envs.iter().collect();
    exports.sort_unstable_by_key(|(name, _)| *name);
    exports
        .into_iter()
        .map(|(name, value)| match shell {
            EnvShell::Posix => {
                let path_ref = render_home_relative_path(std::path::Path::new(value), home_dir);
                let escaped = escape_home_relative_double_quoted_path(
                    &path_ref,
                    escape_posix_double_quoted_string,
                );
                format!("export {name}=\"{escaped}\"\n")
            }
            EnvShell::Fish => {
                let path_ref = render_home_relative_path(std::path::Path::new(value), home_dir);
                let escaped = escape_home_relative_double_quoted_path(
                    &path_ref,
                    escape_fish_double_quoted_string,
                );
                format!("set -gx {name} \"{escaped}\"\n")
            }
            EnvShell::Nu => {
                let path_ref = render_nu_path_ref(&render_home_relative_path(
                    std::path::Path::new(value),
                    home_dir,
                ));
                format!(
                    "$env.{name} = (\"{}\" | path expand --no-symlink)\n",
                    escape_nu_double_quoted_string(&path_ref)
                )
            }
            // PowerShell uses the actual absolute path (not $HOME-relative)
            EnvShell::Powershell => {
                let escaped = escape_powershell_single_quoted_string(value);
                format!("$env:{name} = '{escaped}'\n")
            }
        })
        .collect()
}

/// Render the environment-file content for `shell` and the resolved config.
///
/// Put the resolved bin directory on `PATH`. Persist an explicit `VP_HOME`, but
/// do not export the internal split-directory group.
fn render_env_content(shell: EnvShell, config: &vp_shared::EnvConfig) -> String {
    let dirs = &config.dirs;
    let home_dir = config.user_home.as_path();
    let bin_path_ref = render_home_relative_path(dirs.bin.as_path(), home_dir);
    let dir_envs = render_dir_envs(shell, config);

    match shell {
        EnvShell::Posix => {
            let bin_path_ref = escape_home_relative_double_quoted_path(
                &bin_path_ref,
                escape_posix_double_quoted_string,
            );
            ENV_TEMPLATE_POSIX
                .replace("__ENV_EXPORTS__", &dir_envs)
                .replace("__VP_BIN__", &bin_path_ref)
        }
        EnvShell::Fish => {
            let bin_path_ref = escape_home_relative_double_quoted_path(
                &bin_path_ref,
                escape_fish_double_quoted_string,
            );
            ENV_TEMPLATE_FISH
                .replace("__ENV_EXPORTS__", &dir_envs)
                .replace("__VP_BIN__", &bin_path_ref)
        }
        EnvShell::Nu => {
            // Nushell requires `~` instead of `$HOME` in string literals — `$HOME` is not
            // expanded at parse time, so PATH entries would contain a literal "$HOME/...".
            let bin_path_ref_nu =
                escape_nu_double_quoted_string(&render_nu_path_ref(&bin_path_ref));
            ENV_TEMPLATE_NU
                .replace("__ENV_EXPORTS__", &dir_envs)
                .replace("__VP_BIN__", &bin_path_ref_nu)
        }
        EnvShell::Powershell => {
            // PowerShell uses the actual absolute path (not $HOME-relative)
            let bin_path_win =
                escape_powershell_single_quoted_string(&dirs.bin.as_path().display().to_string());
            ENV_TEMPLATE_PS1
                .replace("__ENV_EXPORTS__", &dir_envs)
                .replace("__VP_BIN_WIN__", &bin_path_win)
        }
    }
}

/// Create environment files under `<CONFIG>/`. Add a guard that prevents
/// duplicate `PATH` entries.
///
/// Creates:
/// - `env` (POSIX shell — bash/zsh) with `vp()` wrapper function
/// - `env.fish` (fish shell) with `vp` wrapper function
/// - `env.nu` (Nushell) with `vp env use` wrapper function
/// - `env.ps1` (PowerShell) with PATH setup + `vp` function
async fn create_env_files() -> Result<(), Error> {
    let config = vp_shared::EnvConfig::get();
    for shell in [EnvShell::Posix, EnvShell::Fish, EnvShell::Nu, EnvShell::Powershell] {
        let mut content = render_env_content(shell, &config);
        if matches!(shell, EnvShell::Powershell) {
            // Windows PowerShell 5.1 uses the active ANSI code page for a
            // UTF-8 script without a BOM. The BOM preserves non-ASCII paths.
            content.insert(0, UTF8_BOM);
        }
        tokio::fs::write(config.dirs.config.join(shell.env_file_name()), content).await?;
    }

    Ok(())
}

/// Print instructions for sourcing the environment files and adding bin to `PATH`.
fn print_path_instructions(env_dir: &vt_path::AbsolutePath) {
    // Use paths relative to $HOME. POSIX and Fish use $HOME. Nushell cannot
    // expand $HOME in the parse-time `source` keyword, so use ~.
    let env_path = env_dir.as_path().display().to_string();
    let home = vp_shared::EnvConfig::get().user_home.as_path().display().to_string();
    let (env_path, nu_env_path) = if let Some(suffix) = env_path.strip_prefix(&home) {
        (format!("$HOME{suffix}"), format!("~{suffix}"))
    } else {
        (env_path.clone(), env_path)
    };

    println!("{}", help::render_heading("Next Steps"));
    println!("  Add to your shell profile (~/.zshrc, ~/.bashrc, etc.):");
    println!();
    println!("  . \"{env_path}/env\"");
    println!();
    println!("  For fish shell, add to ~/.config/fish/config.fish:");
    println!();
    println!("  source \"{env_path}/env.fish\"");
    println!();
    println!("  For Nushell, add to ~/.config/nushell/config.nu:");
    println!();
    println!("  source '{nu_env_path}/env.nu'");
    println!();
    println!("  For PowerShell, add to your $PROFILE:");
    println!();
    println!("  . \"{env_path}/env.ps1\"");
    println!();
    println!("  For IDE support (VS Code, Cursor), ensure bin directory is in system PATH:");

    #[cfg(target_os = "macos")]
    {
        println!("  - macOS: Add to ~/.profile or use launchd");
    }

    #[cfg(target_os = "linux")]
    {
        println!("  - Linux: Add to ~/.profile for display manager integration");
    }

    #[cfg(target_os = "windows")]
    {
        println!("  - Windows: System Properties -> Environment Variables -> Path");
    }

    println!();
    println!(
        "  Restart your terminal and IDE, then run {} to verify.",
        help::accent_command("vp env doctor")
    );
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vt_path::AbsolutePathBuf;

    use super::*;

    #[test]
    fn test_shim_tools_contains_default_shims() {
        // corepack is a default shim (#858, #1309). It must NOT be a core
        // shim: `vp install -g corepack` stays allowed (CORE_SHIMS guard) and
        // dispatch uses a dedicated resolution path instead of the core one.
        assert!(SHIM_TOOLS.contains(&"corepack"));
        assert!(!crate::commands::global::CORE_SHIMS.contains(&"corepack"));
    }

    /// Helper: vars pinning a single-root install at `root`; `user_home`
    /// is set separately (both `HOME` and `USERPROFILE`, for the platforms'
    /// differing precedence) to exercise $HOME-relative vs absolute rendering.
    fn test_env_vars<'a>(
        root: &'a std::path::Path,
        user_home: &'a std::path::Path,
    ) -> [(&'static str, &'a std::ffi::OsStr); 3] {
        [
            (vp_shared::env_vars::VP_HOME, root.as_os_str()),
            ("HOME", user_home.as_os_str()),
            ("USERPROFILE", user_home.as_os_str()),
        ]
    }

    #[cfg(windows)]
    fn write_fake_trampoline(dir: &std::path::Path) -> std::path::PathBuf {
        let trampoline = dir.join("vp-shim.exe");
        std::fs::write(&trampoline, b"fake-trampoline").unwrap();
        trampoline
    }

    #[test]
    fn test_render_env_content_does_not_export_split_dir_group() {
        let temp_dir = TempDir::new().unwrap();
        let custom_bin = temp_dir.path().join("custom-bin");
        let custom_data = temp_dir.path().join("custom-data");
        let custom_cache = temp_dir.path().join("custom-cache");
        vp_shared::EnvConfig::with_vars(
            [
                (vp_shared::env_vars::VP_HOME, None),
                (vp_shared::env_vars::VP_BIN_DIR, Some(custom_bin.as_os_str())),
                (vp_shared::env_vars::VP_DATA_DIR, Some(custom_data.as_os_str())),
                (vp_shared::env_vars::VP_CACHE_DIR, Some(custom_cache.as_os_str())),
                ("HOME", Some(temp_dir.path().as_os_str())),
                ("USERPROFILE", Some(temp_dir.path().as_os_str())),
            ],
            |_| {
                let config = vp_shared::EnvConfig::get();
                let content = render_env_content(EnvShell::Posix, &config);
                assert!(!content.contains("VP_BIN_DIR"));
                assert!(!content.contains("VP_DATA_DIR"));
                assert!(!content.contains("VP_CACHE_DIR"));
                assert!(content.contains("$HOME/custom-bin"));
            },
        );
    }

    #[test]
    fn test_render_env_content_omits_resolved_dirs_without_vp_home() {
        let temp_dir = TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars(
            [
                ("HOME", Some(temp_dir.path().as_os_str())),
                ("USERPROFILE", Some(temp_dir.path().as_os_str())),
                (vp_shared::env_vars::VP_HOME, None),
                (vp_shared::env_vars::VP_BIN_DIR, None),
                (vp_shared::env_vars::VP_DATA_DIR, None),
                (vp_shared::env_vars::VP_CACHE_DIR, None),
            ],
            |_| {
                let config = vp_shared::EnvConfig::get();
                for shell in [EnvShell::Posix, EnvShell::Fish, EnvShell::Nu, EnvShell::Powershell] {
                    let content = render_env_content(shell, &config);
                    assert!(
                        !content.contains("VP_HOME="),
                        "{shell:?} env file should not pin VP_HOME when unset, got: {content}"
                    );
                    assert!(
                        !content.contains("XDG_"),
                        "{shell:?} env file should not re-export XDG_*, got: {content}"
                    );
                    assert!(
                        !content.contains("VP_BIN_DIR")
                            && !content.contains("VP_DATA_DIR")
                            && !content.contains("VP_CACHE_DIR"),
                        "{shell:?} env file should omit VP_*_DIR, got: {content}"
                    );
                }
            },
        );
    }

    #[tokio::test]
    async fn test_create_env_files_creates_all_files() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let env_path = home.join("env");
                let env_fish_path = home.join("env.fish");
                let env_nu_path = home.join("env.nu");
                let env_ps1_path = home.join("env.ps1");
                assert!(env_path.as_path().exists(), "env file should be created");
                assert!(env_fish_path.as_path().exists(), "env.fish file should be created");
                assert!(env_nu_path.as_path().exists(), "env.nu file should be created");
                assert!(env_ps1_path.as_path().exists(), "env.ps1 file should be created");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_adds_one_bom_only_to_powershell() {
        const UTF8_BOM_BYTES: &[u8] = b"\xEF\xBB\xBF";

        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();
                create_env_files().await.unwrap();

                for shell in [EnvShell::Posix, EnvShell::Fish, EnvShell::Nu, EnvShell::Powershell] {
                    let bytes = tokio::fs::read(home.join(shell.env_file_name())).await.unwrap();
                    if matches!(shell, EnvShell::Powershell) {
                        assert!(bytes.starts_with(UTF8_BOM_BYTES));
                        assert!(!bytes[UTF8_BOM_BYTES.len()..].starts_with(UTF8_BOM_BYTES));
                    } else {
                        assert!(!bytes.starts_with(UTF8_BOM_BYTES));
                    }
                }
            },
        )
        .await;
    }

    #[test]
    fn test_escape_nu_double_quoted_string() {
        assert_eq!(
            escape_nu_double_quoted_string(r#"vp "home\with spaces""#),
            r#"vp \"home\\with spaces\""#
        );
    }

    #[test]
    fn test_escape_shell_string_literals() {
        let value = r#"$USER "double" `tick` slash\ single'"#;
        assert_eq!(
            escape_posix_double_quoted_string(value),
            r#"\$USER \"double\" \`tick\` slash\\ single'"#
        );
        assert_eq!(
            escape_fish_double_quoted_string(value),
            r#"\$USER \"double\" `tick` slash\\ single'"#
        );
        assert_eq!(
            escape_nu_double_quoted_string(value),
            r#"$USER \"double\" `tick` slash\\ single'"#
        );
        assert_eq!(
            escape_powershell_single_quoted_string(value),
            r#"$USER "double" `tick` slash\ single''"#
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_render_env_content_preserves_metacharacter_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let special = r#"$USER-"-'-`-\"#;
        let custom_bin = temp_dir.path().join(format!("bin-{special}"));
        let custom_data = temp_dir.path().join(format!("data-{special}"));
        let custom_cache = temp_dir.path().join(format!("cache-{special}"));

        vp_shared::EnvConfig::with_vars(
            [
                (vp_shared::env_vars::VP_HOME, None),
                (vp_shared::env_vars::VP_BIN_DIR, Some(custom_bin.as_os_str())),
                (vp_shared::env_vars::VP_DATA_DIR, Some(custom_data.as_os_str())),
                (vp_shared::env_vars::VP_CACHE_DIR, Some(custom_cache.as_os_str())),
                ("HOME", Some(temp_dir.path().as_os_str())),
                ("USERPROFILE", Some(temp_dir.path().as_os_str())),
            ],
            |_| {
                let config = vp_shared::EnvConfig::get();
                let home_dir = config.user_home.as_path();

                let bin_ref = render_home_relative_path(custom_bin.as_path(), home_dir);
                let posix_bin_ref = escape_home_relative_double_quoted_path(
                    &bin_ref,
                    escape_posix_double_quoted_string,
                );
                let fish_bin_ref = escape_home_relative_double_quoted_path(
                    &bin_ref,
                    escape_fish_double_quoted_string,
                );
                let nu_bin_ref = escape_nu_double_quoted_string(&render_nu_path_ref(&bin_ref));
                let ps_bin_ref = escape_powershell_single_quoted_string(
                    &custom_bin.as_path().display().to_string(),
                );
                let posix_content = render_env_content(EnvShell::Posix, &config);
                let fish_content = render_env_content(EnvShell::Fish, &config);
                let nu_content = render_env_content(EnvShell::Nu, &config);
                let ps_content = render_env_content(EnvShell::Powershell, &config);
                assert!(posix_content.contains(&format!("__vp_bin=\"{posix_bin_ref}\"")));
                assert!(fish_content.contains(&format!("contains -i -- \"{fish_bin_ref}\"")));
                assert!(nu_content.contains(&format!("prepend \"{nu_bin_ref}\"")));
                assert!(ps_content.contains(&format!("$__vp_bin = '{ps_bin_ref}'")));
                for content in [&posix_content, &fish_content, &nu_content, &ps_content] {
                    assert!(!content.contains("VP_BIN_DIR"));
                    assert!(!content.contains("VP_DATA_DIR"));
                    assert!(!content.contains("VP_CACHE_DIR"));
                }

                // Source the generated POSIX file and verify that PATH retains
                // every literal metacharacter without exporting the overrides.
                let env_file = temp_dir.path().join("env");
                std::fs::write(&env_file, posix_content).unwrap();
                let output = std::process::Command::new("sh")
                    .args([
                        "-c",
                        r#". "$1"; printf '%s\n%s\n%s\n%s\n' "${VP_BIN_DIR-unset}" "${VP_DATA_DIR-unset}" "${VP_CACHE_DIR-unset}" "${PATH%%:*}""#,
                        "sh",
                    ])
                    .arg(&env_file)
                    .env("HOME", temp_dir.path())
                    .env_remove(vp_shared::env_vars::VP_BIN_DIR)
                    .env_remove(vp_shared::env_vars::VP_DATA_DIR)
                    .env_remove(vp_shared::env_vars::VP_CACHE_DIR)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "sourcing env failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(
                    String::from_utf8(output.stdout).unwrap(),
                    format!("unset\nunset\nunset\n{}\n", custom_bin.display())
                );
            },
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_render_env_content_escapes_nu_paths() {
        let home = std::path::PathBuf::from(r#"/tmp/vp "home\with spaces""#);
        vp_shared::EnvConfig::with_vars(
            [
                (vp_shared::env_vars::VP_HOME, home.as_os_str()),
                ("HOME", std::ffi::OsStr::new("/nonexistent-home-dir")),
                ("USERPROFILE", std::ffi::OsStr::new("/nonexistent-home-dir")),
            ],
            |_| {
                let config = vp_shared::EnvConfig::get();
                let content = render_env_content(EnvShell::Nu, &config);

                assert!(
                    content.contains(r#"prepend "/tmp/vp \"home\\with spaces\"/bin""#),
                    "env.nu should escape the bin path for a Nushell string literal, got: {content}"
                );
            },
        );
    }

    #[tokio::test]
    async fn test_create_env_files_nu_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(test_env_vars(temp_dir.path(), temp_dir.path()), |_| async {
        create_env_files().await.unwrap();

        let nu_content = tokio::fs::read_to_string(home.join("env.nu")).await.unwrap();
        assert!(
            !nu_content.contains("__VP_BIN__"),
            "env.nu should not contain __VP_BIN__ placeholder"
        );
        assert!(
            nu_content.contains("~/bin"),
            "env.nu should reference ~/bin (not $HOME/bin — Nushell does not expand $HOME in string literals)"
        );
        assert!(
            nu_content.contains("VP_ENV_USE_EVAL_ENABLE"),
            "env.nu should set VP_ENV_USE_EVAL_ENABLE"
        );
        assert!(
            nu_content.contains("VP_COMPLETE=fish"),
            "env.nu should use dynamic Fish completion delegation"
        );
        assert!(nu_content.contains("load-env"), "env.nu should use load-env to apply exports");
    })
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_replaces_placeholder_with_home_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("vp_home")).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                tokio::fs::create_dir_all(&home).await.unwrap();

                create_env_files().await.unwrap();

                let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
                let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
                let nu_content = tokio::fs::read_to_string(home.join("env.nu")).await.unwrap();
                let ps1_content = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

                // Placeholder should be fully replaced
                assert!(
                    !env_content.contains("__VP_BIN__"),
                    "env file should not contain __VP_BIN__ placeholder"
                );
                assert!(
                    !fish_content.contains("__VP_BIN__"),
                    "env.fish file should not contain __VP_BIN__ placeholder"
                );
                assert!(
                    !env_content.contains("__ENV_EXPORTS__")
                        && !fish_content.contains("__ENV_EXPORTS__"),
                    "env files should not contain __ENV_EXPORTS__ placeholder"
                );
                assert!(
                    !nu_content.contains("__ENV_EXPORTS__")
                        && !ps1_content.contains("__ENV_EXPORTS__"),
                    "env files should not contain VP_HOME placeholders"
                );

                // Should use $HOME-relative path since install dir is under HOME
                assert!(
                    env_content.contains("$HOME/vp_home/bin"),
                    "env file should reference $HOME/vp_home/bin, got: {env_content}"
                );
                assert!(
                    fish_content.contains("$HOME/vp_home/bin"),
                    "env.fish file should reference $HOME/vp_home/bin, got: {fish_content}"
                );
                assert!(
                    nu_content.contains("~/vp_home/bin"),
                    "env.nu file should reference ~/vp_home/bin, got: {nu_content}"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_uses_absolute_path_when_not_under_home() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        // Set user_home to a different path so install dir is NOT under HOME.
        // A second tempdir keeps it absolute on every platform — Windows
        // rejects root-relative fakes like "/nonexistent-home-dir", and the
        // home fallback can't save a test that pins both HOME and
        // USERPROFILE.
        let other_home = TempDir::new().unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), other_home.path()),
            |_| async {
                create_env_files().await.unwrap();

                let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
                let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

                // Should use absolute path since install dir is not under HOME
                let expected_bin = home.join("bin");
                let expected_str = expected_bin.as_path().display().to_string().replace('\\', "/");
                assert!(
                    env_content.contains(&expected_str),
                    "env file should use absolute path {expected_str}, got: {env_content}"
                );
                assert!(
                    fish_content.contains(&expected_str),
                    "env.fish file should use absolute path {expected_str}, got: {fish_content}"
                );

                // Should NOT use $HOME-relative path
                assert!(
                    !env_content.contains("$HOME/bin"),
                    "env file should not reference $HOME/bin"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_posix_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();

                // Verify PATH guard structure: case statement checks for duplicate
                assert!(
                    env_content.contains("case \":${PATH}:\" in"),
                    "env file should contain PATH guard case statement"
                );
                assert!(
                    env_content.contains("*\":${__vp_bin}:\"*)"),
                    "env file should check for existing bin in PATH"
                );
                // Verify it re-prepends to front when already present
                assert!(
                    env_content.contains("export PATH=\"${__vp_bin}"),
                    "env file should re-prepend bin to front of PATH"
                );
                // Verify simple prepend for new entry
                assert!(
                    env_content.contains("export PATH=\"$__vp_bin:$PATH\""),
                    "env file should prepend bin to PATH for new entry"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_fish_contains_path_guard() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

                // Verify fish PATH guard: remove existing entry before prepending
                assert!(
                    fish_content.contains("contains -i --"),
                    "env.fish should check for existing bin in PATH"
                );
                assert!(
                    fish_content.contains("set -e PATH[$__vp_idx]"),
                    "env.fish should remove existing entry"
                );
                assert!(fish_content.contains("set -gx PATH"), "env.fish should set PATH globally");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_is_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                // Create env files twice
                create_env_files().await.unwrap();
                let first_env = tokio::fs::read_to_string(home.join("env")).await.unwrap();
                let first_fish = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
                let first_ps1 = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

                create_env_files().await.unwrap();
                let second_env = tokio::fs::read_to_string(home.join("env")).await.unwrap();
                let second_fish = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
                let second_ps1 = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

                assert_eq!(
                    first_env, second_env,
                    "env file should be identical after second write"
                );
                assert_eq!(
                    first_fish, second_fish,
                    "env.fish file should be identical after second write"
                );
                assert_eq!(
                    first_ps1, second_ps1,
                    "env.ps1 file should be identical after second write"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_posix_contains_vp_shell_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();

                // Verify vp() shell function wrapper is present
                assert!(
                    env_content.contains("vp() {"),
                    "env file should contain vp() shell function"
                );
                assert!(
                    env_content.contains("\"$1\" = \"env\""),
                    "env file should check for 'env' subcommand"
                );
                assert!(
                    env_content.contains("\"$2\" = \"use\""),
                    "env file should check for 'use' subcommand"
                );
                assert!(
                    env_content.contains("eval \"$__vp_out\""),
                    "env file should eval the output"
                );
                assert!(
                    env_content.contains("command vp \"$@\""),
                    "env file should use 'command vp' for passthrough"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_fish_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();

                // Verify fish vp function wrapper is present
                assert!(
                    fish_content.contains("function vp"),
                    "env.fish file should contain vp function"
                );
                assert!(
                    fish_content.contains("\"$argv[1]\" = \"env\""),
                    "env.fish should check for 'env' subcommand"
                );
                assert!(
                    fish_content.contains("\"$argv[2]\" = \"use\""),
                    "env.fish should check for 'use' subcommand"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_ps1_contains_vp_function() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let ps1_content = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

                // Verify PowerShell function is present
                assert!(
                    ps1_content.contains("function vp {"),
                    "env.ps1 should contain vp function"
                );
                assert!(
                    ps1_content.contains("Invoke-Expression"),
                    "env.ps1 should use Invoke-Expression"
                );
                // Should not contain placeholders
                assert!(
                    !ps1_content.contains("__VP_BIN_WIN__"),
                    "env.ps1 should not contain __VP_BIN_WIN__ placeholder"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_execute_creates_cmd_wrapper_in_fresh_home() {
        let temp_dir = TempDir::new().unwrap();
        let fresh_home = temp_dir.path().join("new-vite-plus");
        let trampoline = write_fake_trampoline(temp_dir.path());
        vp_shared::EnvConfig::with_vars_async(
            [
                (vp_shared::env_vars::VP_HOME, fresh_home.as_os_str()),
                ("HOME", temp_dir.path().as_os_str()),
                ("USERPROFILE", temp_dir.path().as_os_str()),
                (vp_shared::env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str()),
            ],
            |_| async {
                assert!(!fresh_home.exists(), "install root should not exist before initial setup");
                let status = execute(false, false).await.unwrap();

                assert!(status.success(), "initial vp env setup should succeed");
                let bin_dir = AbsolutePathBuf::new(fresh_home.join("bin")).unwrap();
                let cmd_content =
                    tokio::fs::read_to_string(bin_dir.join("vp-use.cmd")).await.unwrap();
                let expected_exe = fresh_home.join("current").join("bin").join("vp.exe");
                assert!(
                    cmd_content.contains(&format!("\"{}\" env use %*", expected_exe.display())),
                    "vp-use.cmd should invoke the install-local vp.exe, got: {cmd_content}"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_setup_does_not_claim_foreign_windows_executable() {
        let temp_dir = TempDir::new().unwrap();
        let install_root = temp_dir.path().join("vite-plus");
        let bin_dir = install_root.join("bin");
        let node = bin_dir.join("node.exe");
        let pointer = bin_dir.join("node.shim");
        let trampoline = write_fake_trampoline(temp_dir.path());

        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&node, b"foreign-node").await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            [
                (vp_shared::env_vars::VP_HOME, install_root.as_os_str()),
                ("HOME", temp_dir.path().as_os_str()),
                ("USERPROFILE", temp_dir.path().as_os_str()),
                (vp_shared::env_vars::VP_TRAMPOLINE_PATH, trampoline.as_os_str()),
            ],
            |_| async {
                execute(false, true).await.unwrap();
                assert!(!pointer.exists(), "--env-only must not write shim ownership markers");

                execute(false, false).await.unwrap();
                assert_eq!(tokio::fs::read(&node).await.unwrap(), b"foreign-node");
                assert!(
                    !pointer.exists(),
                    "setup without --refresh must not claim a skipped executable"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_create_env_files_does_not_create_cmd_wrapper_on_unix() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                let bin_dir = home.join("bin");
                tokio::fs::create_dir_all(&bin_dir).await.unwrap();

                create_env_files().await.unwrap();

                assert!(
                    !bin_dir.join("vp-use.cmd").as_path().exists(),
                    "vp-use.cmd should only be created on Windows"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_execute_env_only_creates_home_dir_and_env_files() {
        let temp_dir = TempDir::new().unwrap();
        let fresh_home = temp_dir.path().join("new-vite-plus");
        // Directory does NOT exist yet — execute should create it
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(&fresh_home, temp_dir.path()),
            |_| async {
                let status = execute(false, true).await.unwrap();
                assert!(status.success(), "execute --env-only should succeed");

                // Directory should now exist
                assert!(fresh_home.exists(), "config directory should be created");

                // Env files should be written
                assert!(fresh_home.join("env").exists(), "env file should be created");
                assert!(fresh_home.join("env.fish").exists(), "env.fish file should be created");
                assert!(fresh_home.join("env.ps1").exists(), "env.ps1 file should be created");
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_vp_shim_target_prefers_standalone_layout_for_current_exe() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let standalone_vp = home.join("current").join("bin").join("vp");

        tokio::fs::create_dir_all(standalone_vp.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&standalone_vp, b"vp").await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                let target = resolve_unix_vp_shim_target(standalone_vp.as_path()).await.unwrap();
                assert_eq!(target, standalone_vp.as_path());
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_vp_shim_target_uses_current_exe_when_standalone_is_stale() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let standalone_vp = home.join("current").join("bin").join("vp");
        let external_vp = temp_dir.path().join("external-vp");

        tokio::fs::create_dir_all(standalone_vp.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&standalone_vp, b"stale-vp").await.unwrap();
        tokio::fs::write(&external_vp, b"active-vp").await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                let target = resolve_unix_vp_shim_target(&external_vp).await.unwrap();
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_vp_shim_target_uses_current_exe_without_standalone_layout() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let external_vp = temp_dir.path().join("external-vp");

        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&external_vp, b"vp").await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                let target = resolve_unix_vp_shim_target(&external_vp).await.unwrap();
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_vp_shim_target_split_bin_uses_data_current() {
        let temp_dir = TempDir::new().unwrap();
        let user_home = temp_dir.path();
        let data = user_home.join(".local/share/vite-plus");
        let bin_dir = AbsolutePathBuf::new(data.join("bin")).unwrap();
        let version_vp = data.join("0.1.0").join("bin").join("vp");
        let current = data.join("current");

        tokio::fs::create_dir_all(version_vp.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&version_vp, b"vp").await.unwrap();
        tokio::fs::symlink("0.1.0", &current).await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            [
                (vp_shared::env_vars::VP_HOME, None),
                (vp_shared::env_vars::VP_BIN_DIR, None),
                (vp_shared::env_vars::VP_DATA_DIR, None),
                (vp_shared::env_vars::XDG_DATA_HOME, None),
                ("HOME", Some(user_home.as_os_str())),
                ("USERPROFILE", Some(user_home.as_os_str())),
            ],
            |_| async {
                let target = resolve_unix_vp_shim_target(&version_vp).await.unwrap();
                assert_eq!(target, data.join("current").join("bin").join("vp").as_path());
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_create_shim_replaces_stale_unix_symlink_without_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let standalone_vp = home.join("current").join("bin").join("vp");
        let external_vp = temp_dir.path().join("external-vp");
        let node_shim = bin_dir.join("node");

        tokio::fs::create_dir_all(standalone_vp.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&standalone_vp, b"stale-vp").await.unwrap();
        tokio::fs::write(&external_vp, b"active-vp").await.unwrap();
        tokio::fs::symlink("../current/bin/vp", &node_shim).await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                let created = create_shim(&external_vp, &bin_dir, "node", false).await.unwrap();
                let target = tokio::fs::read_link(&node_shim).await.unwrap();

                assert!(created, "stale shims should be recreated");
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_create_shim_replaces_broken_unix_symlink_without_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let external_vp = temp_dir.path().join("external-vp");
        let node_shim = bin_dir.join("node");

        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&external_vp, b"vp").await.unwrap();
        tokio::fs::symlink("../current/bin/vp", &node_shim).await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                let created = create_shim(&external_vp, &bin_dir, "node", false).await.unwrap();
                let target = tokio::fs::read_link(&node_shim).await.unwrap();

                assert!(created, "broken shims should be recreated");
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_setup_vp_wrapper_replaces_stale_unix_symlink_without_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let standalone_vp = home.join("current").join("bin").join("vp");
        let external_vp = temp_dir.path().join("external-vp");
        let vp_shim = bin_dir.join("vp");

        tokio::fs::create_dir_all(standalone_vp.parent().unwrap()).await.unwrap();
        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&standalone_vp, b"stale-vp").await.unwrap();
        tokio::fs::write(&external_vp, b"active-vp").await.unwrap();
        tokio::fs::symlink("../current/bin/vp", &vp_shim).await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                setup_vp_wrapper(&external_vp, &bin_dir, false).await.unwrap();
                let target = tokio::fs::read_link(&vp_shim).await.unwrap();
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_setup_vp_wrapper_replaces_broken_unix_symlink_without_refresh() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join(".vite-plus")).unwrap();
        let bin_dir = home.join("bin");
        let external_vp = temp_dir.path().join("external-vp");
        let vp_shim = bin_dir.join("vp");

        tokio::fs::create_dir_all(&bin_dir).await.unwrap();
        tokio::fs::write(&external_vp, b"vp").await.unwrap();
        tokio::fs::symlink("../current/bin/vp", &vp_shim).await.unwrap();

        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(home.as_path(), temp_dir.path()),
            |_| async {
                setup_vp_wrapper(&external_vp, &bin_dir, false).await.unwrap();
                let target = tokio::fs::read_link(&vp_shim).await.unwrap();
                assert_eq!(target, external_vp);
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_create_env_files_contains_dynamic_completion() {
        let temp_dir = TempDir::new().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        vp_shared::EnvConfig::with_vars_async(
            test_env_vars(temp_dir.path(), temp_dir.path()),
            |_| async {
                create_env_files().await.unwrap();

                let env_content = tokio::fs::read_to_string(home.join("env")).await.unwrap();
                let fish_content = tokio::fs::read_to_string(home.join("env.fish")).await.unwrap();
                let ps1_content = tokio::fs::read_to_string(home.join("env.ps1")).await.unwrap();

                assert!(
                    env_content.contains("VP_COMPLETE=bash")
                        && env_content.contains("VP_COMPLETE=zsh"),
                    "env file should contain completion for bash and zsh"
                );
                assert!(
                    fish_content.contains("VP_COMPLETE=fish"),
                    "env.fish file should contain completion for fish"
                );
                assert!(
                    ps1_content.contains("VP_COMPLETE = \"powershell\""),
                    "env.ps1 file should contain completion for PowerShell"
                );

                assert!(
                    env_content.contains("compdef _vpr_complete vpr"),
                    "env should have vpr completion for zsh"
                );
                assert!(
                    env_content.contains("eval '") && env_content.contains("_vpr_complete() {"),
                    "env should wrap zsh-specific code in eval"
                );
                assert!(
                    fish_content.contains("complete -c vpr"),
                    "env.fish should have vpr completion"
                );
                assert!(
                    ps1_content.contains("Register-ArgumentCompleter -Native -CommandName vpr"),
                    "env.ps1 should have vpr completion"
                );
            },
        )
        .await;
    }
}
