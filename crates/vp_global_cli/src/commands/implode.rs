//! `vp implode` — completely remove vp and all its data from this system.

use std::{
    io::Write,
    path::{Component, Path, PathBuf},
    process::ExitStatus,
};

use owo_colors::OwoColorize;
use rustc_hash::FxHashSet;
use vp_shared::output;
use vt_path::AbsolutePathBuf;
use vt_str::Str;

use crate::{
    cli::exit_status,
    commands::{
        env::setup::{SHIM_TOOLS, shim_filename},
        global::install::is_vp_shim_target,
        shell::{ALL_SHELL_PROFILES, ShellProfileKind, abbreviate_home_path, resolve_profile_path},
    },
    error::Error,
};

/// Comment marker written by the install script above the sourcing line.
const VITE_PLUS_COMMENT: &str = "# Vite+ bin";

/// Private mode used by a temporary copy of `vp.exe` to remove renamed roots.
#[cfg(windows)]
const DEFERRED_DELETE_MODE: &str = "__vite-plus-deferred-delete";

#[cfg(windows)]
const DEFERRED_DELETE_ATTEMPTS: usize = 100;

#[cfg(windows)]
const DEFERRED_DELETE_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(100);

fn lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

pub fn execute(yes: bool) -> Result<ExitStatus, Error> {
    let env_config = vp_shared::EnvConfig::get();
    let dirs = &env_config.dirs;

    // Build a unique set of Vite+-owned roots. In a single-root layout, data,
    // config, and state use the same directory. Cache is inside that directory.
    // Removing each category separately could remove the same path twice.
    //
    // The default Unix `<BIN>` is under `<DATA>`, so removal of `<DATA>` also
    // removes it. Never remove a separately resolved `<BIN>` because a bin from
    // an explicit override group can be shared. Remove only Vite+-owned shims.
    let mut roots: Vec<AbsolutePathBuf> = [&dirs.data, &dirs.cache, &dirs.config, &dirs.state]
        .into_iter()
        .map(|root| {
            AbsolutePathBuf::new(lexical_path(root.as_path()))
                .expect("resolved Vite+ roots remain absolute after lexical normalization")
        })
        .collect();
    roots.sort_by(|a, b| a.as_path().cmp(b.as_path()));
    roots.dedup();
    let mut delete_set: Vec<AbsolutePathBuf> = Vec::new();
    for root in roots {
        if !delete_set.iter().any(|kept| root.as_path().starts_with(kept.as_path())) {
            delete_set.push(root);
        }
    }

    if !delete_set.iter().any(|root| root.as_path().exists()) {
        output::info("vite-plus is not installed. No installation directory exists.");
        return Ok(exit_status(0));
    }

    // Use the user home to resolve shell-profile paths.
    let user_home = &env_config.user_home;

    let source_matcher = VitePlusSourceMatcher::new(&dirs.config, user_home);

    // Find shell profiles that contain Vite+ lines. Keep their content for cleanup.
    let affected_profiles = collect_affected_profiles(user_home, &source_matcher);

    // Confirmation
    if !yes && !confirm_implode(&delete_set, &dirs.bin, &affected_profiles)? {
        return Ok(exit_status(0));
    }

    // A native helper must live outside every root that it removes. Copy the
    // running executable before changing any path because the data root can
    // contain that executable.
    #[cfg(windows)]
    let cleanup_source = match prepare_deferred_delete_source(&delete_set) {
        Ok(source) => source,
        Err(e) => {
            output::error(&vt_str::format!("Failed to prepare the removal helper: {e}"));
            return Err(Error::CommandExecution(e));
        }
    };

    // Clean shell profiles with the stored content. Do not read them again.
    clean_affected_profiles(&affected_profiles, &source_matcher);

    // Remove Windows PATH entry
    #[cfg(windows)]
    {
        if let Err(e) = remove_windows_path_entry(&dirs.bin) {
            output::warn(&vt_str::format!("Vite+ could not clean the Windows PATH: {e}"));
        } else {
            output::success("Vite+ removed its bin directory from the Windows PATH.");
        }
    }

    // Remove vp-owned shim files from the (potentially shared) bin directory,
    // then the owned roots.
    remove_shim_files(dirs);
    for root in &delete_set {
        if root.as_path().exists() {
            #[cfg(unix)]
            remove_vite_plus_dir(root)?;
            #[cfg(windows)]
            remove_vite_plus_dir(root, &cleanup_source)?;
        }
    }

    output::raw("");
    output::success("Vite+ removed its managed files and shell entries from your system.");
    output::note("Restart your terminal to apply shell changes.");

    Ok(exit_status(0))
}

/// Remove the shim files vite-plus owns from the bin directory.
///
/// Do not remove the bin directory directly because a bin from an explicit
/// override group can be shared with other tools. Removal of the default Unix
/// `<DATA>` root also removes its bin directory.
///
/// Get package-shim names from `<DATA>/bins/*.json`. Also check `vp` and the
/// default environment shims because those files are not in the metadata.
/// Remove a Unix candidate only if it links to this install's `vp`. Remove a
/// Windows candidate only if Vite+ created the trampoline.
fn remove_shim_files(dirs: &vp_shared::VpDirs) {
    let mut names = recorded_bin_shim_names(dirs);
    names.insert(shim_filename("vp"));
    names.extend(SHIM_TOOLS.iter().map(|tool| shim_filename(tool)));
    #[cfg(windows)]
    names.insert("vp-use.cmd".to_string());

    let mut removed = 0;
    #[cfg(windows)]
    let mut scheduled = 0;
    for name in names {
        let path = dirs.bin.join(&name);
        if !is_vp_shim_target(&path) {
            continue;
        }
        let pointer = std::path::Path::new(&name)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(|stem| dirs.bin.join(vp_shared::shim_pointer_file_name(stem)));
        match std::fs::remove_file(path.as_path()) {
            Ok(()) => {
                removed += 1;
                if let Some(pointer) = pointer.as_ref() {
                    remove_shim_pointer(pointer, &name);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(pointer) = pointer.as_ref() {
                    remove_shim_pointer(pointer, &name);
                }
            }
            Err(e) => {
                #[cfg(windows)]
                if let Some(pointer) = pointer.as_ref() {
                    match schedule_deferred_shim_delete(path.as_path(), pointer.as_path()) {
                        Ok(_) => {
                            scheduled += 1;
                            continue;
                        }
                        Err(schedule_error) => {
                            output::warn(&vt_str::format!(
                                "Vite+ could not schedule removal of shim {name}: {schedule_error}"
                            ));
                        }
                    }
                }
                output::warn(&vt_str::format!("Vite+ could not remove shim {name}: {e}"));
            }
        }
    }
    if removed > 0 {
        output::success(&vt_str::format!(
            "Vite+ removed {removed} shim{} from {}",
            if removed == 1 { "" } else { "s" },
            dirs.bin.as_path().display()
        ));
    }
    #[cfg(windows)]
    if scheduled > 0 {
        output::success(&vt_str::format!(
            "Vite+ scheduled removal of {scheduled} locked shim{} from {}",
            if scheduled == 1 { "" } else { "s" },
            dirs.bin.as_path().display()
        ));
    }
}

fn remove_shim_pointer(pointer: &AbsolutePathBuf, name: &str) {
    match std::fs::remove_file(pointer.as_path()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            output::warn(&vt_str::format!("Vite+ could not remove the sidecar for {name}: {e}"));
        }
    }
}

/// Binary names recorded in `<DATA>/bins/*.json`.
fn recorded_bin_shim_names(dirs: &vp_shared::VpDirs) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    let Ok(entries) = std::fs::read_dir(dirs.data.join("bins").as_path()) else {
        return names;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
        {
            names.insert(shim_filename(stem));
        }
    }
    names
}

/// A shell profile that contains Vite+ sourcing lines.
struct AffectedProfile {
    /// Display name (e.g. ".zshrc", ".config/fish/conf.d/vite-plus.fish").
    name: Str,
    /// Absolute path to the file.
    path: AbsolutePathBuf,
    kind: AffectedProfileKind,
}

// Specify a snippet file or a main profile.
enum AffectedProfileKind {
    // Remove a snippet file during uninstall.
    Snippet,
    Main {
        /// File content read during detection (reused for cleaning).
        content: Str,
        env_file: &'static str,
    },
}

/// Find shell profiles that contain Vite+ source lines. Store their content so
/// cleanup does not read the files again.
fn collect_affected_profiles(
    user_home: &AbsolutePathBuf,
    source_matcher: &VitePlusSourceMatcher,
) -> Vec<AffectedProfile> {
    let mut affected = Vec::new();

    for profile in ALL_SHELL_PROFILES {
        let path = resolve_profile_path(profile, user_home);
        let name = abbreviate_home_path(&path, user_home);

        // Read directly — if the file doesn't exist, read_to_string returns Err
        // which .ok().filter() handles gracefully (no redundant exists() check).
        if let Some(content) = std::fs::read_to_string(&path).ok().filter(|c| {
            c.lines().any(|line| source_matcher.is_vite_plus_source_line(line, profile.env_file))
        }) {
            if matches!(profile.kind, ShellProfileKind::Snippet) {
                affected.push(AffectedProfile { name, path, kind: AffectedProfileKind::Snippet });
                continue;
            }
            affected.push(AffectedProfile {
                name,
                path,
                kind: AffectedProfileKind::Main {
                    content: Str::from(content),
                    env_file: profile.env_file,
                },
            });
        }
    }
    affected
}

/// Show confirmation prompt and require the user to type "uninstall".
/// Returns `Ok(true)` if confirmed, `Ok(false)` if aborted.
fn confirm_implode(
    delete_set: &[AbsolutePathBuf],
    bin_dir: &vt_path::AbsolutePath,
    affected_profiles: &[AffectedProfile],
) -> Result<bool, Error> {
    if !vp_shared::is_stdin_terminal() {
        return Err(Error::UserMessage(
            "Cannot prompt for confirmation: stdin is not a TTY. Use --yes to skip confirmation."
                .into(),
        ));
    }

    output::warn("This will completely remove vite-plus from your system!");
    output::raw("");
    output::raw("  Directories to remove:");
    for root in delete_set {
        output::raw(&vt_str::format!("    - {}", root.as_path().display()));
    }
    output::raw(&vt_str::format!("  Shim files to remove from: {}", bin_dir.as_path().display()));
    if !affected_profiles.is_empty() {
        output::raw("  Shell profiles to clean:");
        for profile in affected_profiles {
            output::raw(&vt_str::format!("    - {}", profile.name));
        }
    }
    output::raw("");
    output::raw(&vt_str::format!("Type {} to confirm:", "uninstall".bold()));

    // String is needed here for read_line
    #[expect(clippy::disallowed_types)]
    let mut input = String::new();
    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut input)?;

    if input.trim() != "uninstall" {
        output::info("Aborted.");
        return Ok(false);
    }

    Ok(true)
}

/// Clean all affected shell profiles using cached content (no re-read).
fn clean_affected_profiles(
    affected_profiles: &[AffectedProfile],
    source_matcher: &VitePlusSourceMatcher,
) {
    for profile in affected_profiles {
        match &profile.kind {
            AffectedProfileKind::Main { content, env_file } => {
                let cleaned = remove_vite_plus_lines(content, source_matcher, env_file);
                match std::fs::write(&profile.path, cleaned.as_bytes()) {
                    Ok(()) => output::success(&vt_str::format!("Cleaned {}", profile.name)),
                    Err(e) => {
                        output::warn(&vt_str::format!("Failed to clean {}: {e}", profile.name));
                    }
                }
            }
            AffectedProfileKind::Snippet => match std::fs::remove_file(&profile.path) {
                Ok(()) => output::success(&vt_str::format!("Removed {}", profile.name)),
                Err(e) => {
                    output::warn(&vt_str::format!("Failed to remove {}: {e}", profile.name));
                }
            },
        }
    }
}

/// Remove a vite-plus root directory.
#[cfg(unix)]
fn remove_vite_plus_dir(home_dir: &AbsolutePathBuf) -> Result<(), Error> {
    match std::fs::remove_dir_all(home_dir) {
        Ok(()) => {
            output::success(&vt_str::format!("Removed {}", home_dir.as_path().display()));
            Ok(())
        }
        Err(e) => {
            output::error(&vt_str::format!(
                "Failed to remove {}: {e}",
                home_dir.as_path().display()
            ));
            Err(Error::CommandExecution(e))
        }
    }
}

/// Rename a Vite+ root and start a native helper that removes it after this
/// process exits. The helper uses Rust's Windows filesystem implementation,
/// which supports extended-length paths and does not follow reparse points.
#[cfg(windows)]
fn remove_vite_plus_dir(home_dir: &AbsolutePathBuf, cleanup_source: &Path) -> Result<(), Error> {
    let parent_pid = std::process::id();
    let suffix = deferred_delete_suffix(parent_pid);
    let trash_path = home_dir.as_path().with_extension(&suffix);
    let failure_log = deferred_cleanup_error_path(&trash_path);
    let helper_path = match copy_deferred_delete_helper(cleanup_source) {
        Ok(path) => path,
        Err(e) => {
            output::error(&vt_str::format!(
                "Failed to prepare removal of {}: {e}",
                home_dir.as_path().display()
            ));
            return Err(Error::CommandExecution(e));
        }
    };

    if let Err(e) = std::fs::rename(home_dir, &trash_path) {
        let _ = std::fs::remove_file(&helper_path);
        output::error(&vt_str::format!(
            "Failed to rename {} for removal: {e}",
            home_dir.as_path().display()
        ));
        return Err(Error::CommandExecution(e));
    }

    let helper = match spawn_deferred_delete(&helper_path, &trash_path, parent_pid, &suffix) {
        Ok(helper) => helper,
        Err(e) => {
            let _ = std::fs::remove_file(&helper_path);
            let _ = std::fs::rename(&trash_path, home_dir);
            output::error(&vt_str::format!(
                "Failed to schedule removal of {}: {e}",
                home_dir.as_path().display()
            ));
            return Err(Error::CommandExecution(e));
        }
    };

    if let Err(e) = spawn_deferred_helper_delete(&helper_path, helper.id()) {
        output::warn(&vt_str::format!(
            "Vite+ could not schedule removal of temporary helper {}: {e}",
            helper_path.display()
        ));
    }

    output::success(&vt_str::format!(
        "Scheduled removal of {} (will complete shortly)",
        home_dir.as_path().display()
    ));
    output::note(&vt_str::format!(
        "Cleanup failures will be recorded at {}",
        failure_log.display()
    ));
    Ok(())
}

#[cfg(windows)]
fn prepare_deferred_delete_source(
    delete_set: &[AbsolutePathBuf],
) -> std::io::Result<tempfile::TempPath> {
    let system_temp = lexical_path(&std::env::temp_dir());
    let helper_dir = if system_temp.is_absolute()
        && !delete_set.iter().any(|root| system_temp.starts_with(root.as_path()))
    {
        system_temp
    } else {
        delete_set
            .iter()
            .find(|root| root.as_path().exists())
            .and_then(|root| root.as_path().parent())
            .ok_or_else(|| std::io::Error::other("no directory is available outside the roots"))?
            .to_path_buf()
    };
    let source_path = tempfile::Builder::new()
        .prefix("vite-plus-cleanup-source-")
        .suffix(".exe")
        .tempfile_in(helper_dir)?
        .into_temp_path();
    std::fs::copy(std::env::current_exe()?, &source_path)?;
    Ok(source_path)
}

#[cfg(windows)]
fn copy_deferred_delete_helper(source: &Path) -> std::io::Result<PathBuf> {
    let helper_dir = source
        .parent()
        .ok_or_else(|| std::io::Error::other("the cleanup source has no parent directory"))?;
    let temporary = tempfile::Builder::new()
        .prefix("vite-plus-cleanup-")
        .suffix(".exe")
        .tempfile_in(helper_dir)?;
    let (file, path) = temporary.keep().map_err(|e| e.error)?;
    drop(file);
    if let Err(e) = std::fs::copy(source, &path) {
        let _ = std::fs::remove_file(&path);
        return Err(e);
    }
    Ok(path)
}

#[cfg(windows)]
fn deferred_delete_suffix(parent_pid: u32) -> Str {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    vt_str::format!("removing-{parent_pid}-{nonce}")
}

#[cfg(any(windows, test))]
fn deferred_cleanup_error_path(trash_path: &Path) -> PathBuf {
    let mut name = trash_path.file_name().unwrap_or_default().to_os_string();
    name.push(".cleanup-error.txt");
    trash_path.with_file_name(name)
}

#[cfg(any(windows, test))]
struct DeferredDeleteArgs {
    parent_pid: u32,
    trash_path: PathBuf,
}

#[cfg(any(windows, test))]
fn parse_deferred_delete_args(
    mut args: impl Iterator<Item = std::ffi::OsString>,
) -> Option<DeferredDeleteArgs> {
    let parent_pid = args.next()?.to_str()?.parse::<u32>().ok()?;
    let suffix = args.next()?;
    let trash_path = PathBuf::from(args.next()?);
    if args.next().is_some() || !trash_path.is_absolute() {
        return None;
    }

    let suffix = suffix.to_str()?;
    let expected_prefix = vt_str::format!("removing-{parent_pid}-");
    if !suffix.starts_with(&*expected_prefix)
        || suffix.len() == expected_prefix.len()
        || trash_path.extension() != Some(std::ffi::OsStr::new(suffix))
    {
        return None;
    }

    Some(DeferredDeleteArgs { parent_pid, trash_path })
}

/// Run the private native cleanup mode before normal CLI initialization.
///
/// Windows access control is the authorization boundary: another process that
/// runs as the same user can already delete any accessible tree. To prevent
/// accidental deletion through this private mode, the parser accepts only an
/// absolute path with the exact `removing-<parent_pid>-<nonce>` suffix that the
/// parent creates before it starts the helper. It also rejects extra arguments.
#[cfg(windows)]
pub(crate) fn maybe_run_deferred_delete_helper(
    mut process_args: impl Iterator<Item = std::ffi::OsString>,
) -> Option<u8> {
    process_args.next();
    if process_args.next().as_deref() != Some(std::ffi::OsStr::new(DEFERRED_DELETE_MODE)) {
        return None;
    }

    let Some(args) = parse_deferred_delete_args(process_args) else {
        return Some(1);
    };
    wait_for_process(args.parent_pid);

    match remove_tree_with_retries(&args.trash_path) {
        Ok(()) => {
            let _ = std::fs::remove_file(deferred_cleanup_error_path(&args.trash_path));
            Some(0)
        }
        Err(e) => {
            write_deferred_cleanup_error(&args.trash_path, &e);
            Some(1)
        }
    }
}

#[cfg(windows)]
fn remove_tree_with_retries(path: &Path) -> std::io::Result<()> {
    let mut last_error = None;
    for attempt in 0..DEFERRED_DELETE_ATTEMPTS {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => last_error = Some(e),
        }
        if attempt + 1 < DEFERRED_DELETE_ATTEMPTS {
            std::thread::sleep(DEFERRED_DELETE_RETRY_DELAY);
        }
    }
    Err(last_error.expect("at least one removal attempt must run"))
}

#[cfg(windows)]
fn write_deferred_cleanup_error(trash_path: &Path, error: &std::io::Error) {
    let message = vt_str::format!(
        "Vite+ could not remove this directory after retrying for 10 seconds.\n\
         Directory: {}\n\
         Error: {error}\n\
         Close programs that use files in this directory, then delete it manually.\n",
        trash_path.display()
    );
    let _ = std::fs::write(deferred_cleanup_error_path(trash_path), message.as_bytes());
}

#[cfg(windows)]
fn wait_for_process(process_id: u32) {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const INFINITE: u32 = u32::MAX;

    unsafe extern "system" {
        fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> Handle;
        fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
        fn CloseHandle(handle: Handle) -> i32;
    }

    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, process_id);
        if !handle.is_null() {
            WaitForSingleObject(handle, INFINITE);
            CloseHandle(handle);
        }
    }
}

/// Build the shallow PowerShell script that removes the temporary native
/// helper after it exits. The root tree itself is never passed to PowerShell.
#[cfg(any(windows, test))]
fn build_deferred_helper_delete_script(helper_path: &Path, helper_pid: u32) -> Str {
    let path = powershell_path_literal(helper_path);
    vt_str::format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $helper='{path}';$helperPid={helper_pid};\
         Wait-Process -Id $helperPid -ErrorAction SilentlyContinue;\
         for($i=0;$i -lt 100;$i++){{\
           Remove-Item -LiteralPath $helper -Force;\
           if(-not (Test-Path -LiteralPath $helper)){{exit 0}};\
           Start-Sleep -Milliseconds 100\
         }};exit 1"
    )
}

#[cfg(windows)]
fn spawn_deferred_delete(
    helper_path: &Path,
    trash_path: &Path,
    parent_pid: u32,
    suffix: &str,
) -> std::io::Result<std::process::Child> {
    let mut command = std::process::Command::new(helper_path);
    #[cfg(not(test))]
    command.args([DEFERRED_DELETE_MODE, &parent_pid.to_string(), suffix]).arg(trash_path);
    #[cfg(test)]
    command
        .args(["deferred_delete_helper_child", "--nocapture"])
        .env("VP_IMPLODE_HELPER_PARENT_PID", parent_pid.to_string())
        .env("VP_IMPLODE_HELPER_SUFFIX", suffix)
        .env("VP_IMPLODE_HELPER_TRASH", trash_path);
    if let Some(helper_dir) = helper_path.parent() {
        command.current_dir(helper_dir);
    }
    use std::os::windows::process::CommandExt;
    command
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

#[cfg(windows)]
fn spawn_deferred_helper_delete(
    helper_path: &Path,
    helper_pid: u32,
) -> std::io::Result<std::process::Child> {
    let script = build_deferred_helper_delete_script(helper_path, helper_pid);
    use std::os::windows::process::CommandExt;
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script]);
    if let Some(helper_dir) = helper_path.parent() {
        command.current_dir(helper_dir);
    }
    command
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

#[cfg(any(windows, test))]
struct DeferredShimPaths {
    executable: PathBuf,
    pointer: PathBuf,
}

/// Rename a locked shim and its sidecar to unique paths, then remove that pair
/// after the process that uses the executable exits. A reinstall can use the
/// original paths immediately. The helper never refers to those original paths,
/// so it cannot remove the replacement.
#[cfg(windows)]
fn schedule_deferred_shim_delete(
    executable: &Path,
    pointer: &Path,
) -> std::io::Result<DeferredShimPaths> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let suffix = vt_str::format!("removing-{}-{nonce}", std::process::id());
    let paths = DeferredShimPaths {
        executable: deferred_shim_path(executable, &suffix),
        pointer: deferred_shim_path(pointer, &suffix),
    };

    std::fs::rename(pointer, &paths.pointer)?;
    if let Err(e) = std::fs::rename(executable, &paths.executable) {
        if std::fs::rename(&paths.pointer, pointer).is_err() {
            let _ = std::fs::copy(&paths.pointer, pointer);
        }
        return Err(e);
    }

    if let Err(e) = spawn_deferred_shim_delete(&paths) {
        if std::fs::rename(&paths.executable, executable).is_ok() {
            if std::fs::rename(&paths.pointer, pointer).is_err() {
                let _ = std::fs::copy(&paths.pointer, pointer);
            }
        }
        return Err(e);
    }

    Ok(paths)
}

#[cfg(any(windows, test))]
fn deferred_shim_path(path: &Path, suffix: &str) -> PathBuf {
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default();
    path.with_extension(vt_str::format!("{extension}.{suffix}"))
}

#[cfg(any(windows, test))]
fn powershell_path_literal(path: &Path) -> Str {
    Str::from(path.to_string_lossy().replace('\'', "''"))
}

/// Build a PowerShell script that retries removal of a renamed executable. It
/// removes the sidecar only after the executable is gone.
#[cfg(any(windows, test))]
fn build_deferred_shim_delete_script(paths: &DeferredShimPaths) -> Str {
    let executable = powershell_path_literal(&paths.executable);
    let pointer = powershell_path_literal(&paths.pointer);
    vt_str::format!(
        "$ErrorActionPreference='SilentlyContinue';\
         $exe='{executable}';$sidecar='{pointer}';\
         for($i=0;$i -lt 100;$i++){{\
           if(-not (Test-Path -LiteralPath $exe)){{\
             Remove-Item -LiteralPath $sidecar -Force;exit 0\
           }};\
           Remove-Item -LiteralPath $exe -Force;\
           if(-not (Test-Path -LiteralPath $exe)){{\
             Remove-Item -LiteralPath $sidecar -Force;exit 0\
           }};\
           Start-Sleep -Milliseconds 100\
         }};exit 1"
    )
}

#[cfg(windows)]
fn spawn_deferred_shim_delete(paths: &DeferredShimPaths) -> std::io::Result<std::process::Child> {
    let script = build_deferred_shim_delete_script(paths);
    std::process::Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

/// Matches shell-profile `source` lines that reference *this* install's env
/// files, so a second Vite+ install's lines are left untouched.
///
/// The recognized spellings must mirror what the writers emit:
/// `install.sh`/`install.ps1` (shell PATH setup) and `render_env_content` in
/// `env/setup.rs`. `env/doctor.rs::check_profile_files` derives the same
/// variants for its profile scan; keep them in sync.
struct VitePlusSourceMatcher {
    /// Env-dir spellings with forward-slash separators: the absolute path,
    /// plus `$HOME`- and `~`-relative forms when the dir is under `$HOME`.
    roots: Vec<Str>,
}

impl VitePlusSourceMatcher {
    fn new(env_dir: &AbsolutePathBuf, user_home: &AbsolutePathBuf) -> Self {
        let mut roots = vec![normalize_path_separators(&env_dir.as_path().display().to_string())];

        if let Ok(Some(suffix)) = env_dir.strip_prefix(user_home) {
            // `RelativePathBuf` guarantees forward-slash separators.
            let suffix = vt_str::format!("{suffix}");
            if suffix.is_empty() {
                roots.push(Str::from("$HOME"));
                roots.push(Str::from("~"));
            } else {
                roots.push(vt_str::format!("$HOME/{suffix}"));
                roots.push(vt_str::format!("~/{suffix}"));
            }
        }

        Self { roots }
    }

    fn is_vite_plus_source_line(&self, line: &str, env_file: &str) -> bool {
        let Some(arg) = source_line_arg(line) else {
            return false;
        };

        // Windows profiles may spell the path with backslashes (e.g. Nushell's
        // `source '~\.vite-plus\env.nu'`); compare in forward-slash form.
        let arg = normalize_path_separators(arg);
        self.roots.iter().any(|root| arg == join_path_ref(root, env_file))
    }
}

fn join_path_ref(root: &str, env_file: &str) -> Str {
    let separator = if root.ends_with('/') { "" } else { "/" };
    vt_str::format!("{root}{separator}{env_file}")
}

fn normalize_path_separators(path: &str) -> Str {
    Str::from(path.replace('\\', "/"))
}

fn source_line_arg(line: &str) -> Option<&str> {
    let rest = source_command_remainder(line)?.trim_start();
    if let Some(rest) = rest.strip_prefix('"') {
        return rest.find('"').map(|end| &rest[..end]);
    }
    if let Some(rest) = rest.strip_prefix('\'') {
        return rest.find('\'').map(|end| &rest[..end]);
    }
    rest.split_whitespace().next()
}

fn source_command_remainder(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed.strip_prefix(". ").or_else(|| trimmed.strip_prefix("source "))
}

/// Remove Vite+ lines from content, returning the cleaned string.
fn remove_vite_plus_lines(
    content: &str,
    source_matcher: &VitePlusSourceMatcher,
    env_file: &str,
) -> Str {
    let lines: Vec<&str> = content.lines().collect();
    let mut remove_indices = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if source_matcher.is_vite_plus_source_line(line, env_file) {
            remove_indices.push(i);
            // Also remove the comment line above
            if i > 0 && lines[i - 1].contains(VITE_PLUS_COMMENT) {
                remove_indices.push(i - 1);
                // Also remove the blank line before the comment
                if i > 1 && lines[i - 2].trim().is_empty() {
                    remove_indices.push(i - 2);
                }
            }
        }
    }

    if remove_indices.is_empty() {
        return Str::from(content);
    }

    #[expect(clippy::disallowed_types)]
    let mut result = String::with_capacity(content.len());
    for (i, line) in lines.iter().enumerate() {
        if !remove_indices.contains(&i) {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Preserve trailing newline behavior of original
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    Str::from(result)
}

/// Remove the vp bin directory from the Windows User PATH via PowerShell.
#[cfg(windows)]
fn remove_windows_path_entry(bin_path: &vt_path::AbsolutePath) -> std::io::Result<()> {
    let bin_str = bin_path.as_path().to_string_lossy();
    let script = vt_str::format!(
        "[Environment]::SetEnvironmentVariable('Path', \
         ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | \
         Where-Object {{ $_ -ne '{bin_str}' }}) -join ';', 'User')"
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(std::io::ErrorKind::Other, "PowerShell command failed"))
    }
}

#[cfg(test)]
mod tests {
    use vp_shared::env_vars;

    use super::*;

    fn test_absolute_path(posix: &str, windows: &str) -> AbsolutePathBuf {
        let path = if cfg!(windows) { windows } else { posix };
        AbsolutePathBuf::new(path.into()).unwrap()
    }

    fn default_user_home() -> AbsolutePathBuf {
        test_absolute_path("/home/user", r"C:\Users\user")
    }

    fn custom_user_home() -> AbsolutePathBuf {
        test_absolute_path("/Users/test", r"C:\Users\test")
    }

    fn shell_path(path: &AbsolutePathBuf) -> Str {
        normalize_path_separators(&path.as_path().display().to_string())
    }

    fn default_source_matcher() -> VitePlusSourceMatcher {
        let user_home = default_user_home();
        let home_dir = user_home.join(".vite-plus");
        VitePlusSourceMatcher::new(&home_dir, &user_home)
    }

    #[test]
    fn test_remove_vite_plus_lines_posix() {
        let matcher = default_source_matcher();
        let content = "# existing config\nexport FOO=bar\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, "# existing config\nexport FOO=bar\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_no_match() {
        let matcher = default_source_matcher();
        let content = "# just a normal config\nexport PATH=/usr/bin\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, content);
    }

    #[test]
    fn test_remove_vite_plus_lines_absolute_path() {
        let user_home = default_user_home();
        let home_dir = user_home.join(".vite-plus");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);
        let env_path = shell_path(&home_dir.join("env"));
        let content = vt_str::format!("# existing\n. \"{env_path}\"\n");
        let result = remove_vite_plus_lines(&content, &matcher, "env");
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_custom_absolute_path() {
        let user_home = custom_user_home();
        let home_dir = user_home.join("tools").join("vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);
        let env_path = shell_path(&home_dir.join("env"));
        let content = vt_str::format!("# existing\n. \"{env_path}\"\n");
        let result = remove_vite_plus_lines(&content, &matcher, "env");
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_custom_home_relative_path() {
        let user_home = custom_user_home();
        let home_dir = user_home.join("tools").join("vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);
        let content = "# existing\n. \"$HOME/tools/vp/env\"\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_custom_tilde_path() {
        let user_home = custom_user_home();
        let home_dir = user_home.join("tools").join("vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);
        let content = "# existing\nsource '~/tools/vp/env.nu'\n";
        let result = remove_vite_plus_lines(content, &matcher, "env.nu");
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_ignores_marker_with_unmatched_path() {
        let matcher = default_source_matcher();
        let content = "# existing\n\n# Vite+ bin (https://viteplus.dev)\n. \"/opt/old-vp/env\"\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, content);
    }

    #[test]
    fn test_remove_vite_plus_lines_env_does_not_match_env_fish() {
        let matcher = default_source_matcher();
        let content = "# existing\nsource \"$HOME/.vite-plus/env.fish\"\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, content);
    }

    #[test]
    fn test_remove_vite_plus_lines_fish() {
        let matcher = default_source_matcher();
        let content = "# existing config\n\n# Vite+ bin (https://viteplus.dev)\nsource \"$HOME/.vite-plus/env.fish\"\n";
        let result = remove_vite_plus_lines(content, &matcher, "env.fish");
        assert_eq!(&*result, "# existing config\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_nushell() {
        let matcher = default_source_matcher();
        let content = "# existing config\n\n# Vite+ bin (https://viteplus.dev)\nsource '~/.vite-plus/env.nu'\n";
        let result = remove_vite_plus_lines(content, &matcher, "env.nu");
        assert_eq!(&*result, "# existing config\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_nushell_windows_path() {
        let matcher = default_source_matcher();
        let content = "# existing config\nsource '~\\.vite-plus\\env.nu'\n";
        let result = remove_vite_plus_lines(content, &matcher, "env.nu");
        assert_eq!(&*result, "# existing config\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_preserves_surrounding() {
        let matcher = default_source_matcher();
        let content = "# before\nexport A=1\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n# after\nexport B=2\n";
        let result = remove_vite_plus_lines(content, &matcher, "env");
        assert_eq!(&*result, "# before\nexport A=1\n# after\nexport B=2\n");
    }

    #[test]
    fn test_clean_affected_profiles_integration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let home_dir = temp_path.join(".vite-plus");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &temp_path);
        let profile_path = temp_path.join(".zshrc");
        let original = "# my config\nexport FOO=bar\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n";
        std::fs::write(&profile_path, original).unwrap();

        let profiles = vec![AffectedProfile {
            name: Str::from(".zshrc"),
            path: profile_path.clone(),
            kind: AffectedProfileKind::Main { content: Str::from(original), env_file: "env" },
        }];
        clean_affected_profiles(&profiles, &matcher);

        let result = std::fs::read_to_string(&profile_path).unwrap();
        assert_eq!(result, "# my config\nexport FOO=bar\n");
        assert!(!result.contains(".vite-plus/env"));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_remove_vite_plus_dir_success() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let target = dir.join("to-remove");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("file.txt"), "data").unwrap();

        let result = remove_vite_plus_dir(&target);
        assert!(result.is_ok());
        assert!(!target.as_path().exists());
    }

    #[test]
    #[cfg(not(windows))]
    fn test_remove_vite_plus_dir_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let target = dir.join("does-not-exist");

        let result = remove_vite_plus_dir(&target);
        assert!(result.is_err());
    }

    #[test]
    fn test_deferred_delete_args_require_the_unique_renamed_root() {
        let suffix = "removing-9876-123456";
        let path = test_absolute_path(
            "/home/test/data.removing-9876-123456",
            r"C:\Users\test $&' 测试\data.removing-9876-123456",
        );
        let parsed = parse_deferred_delete_args(
            [
                std::ffi::OsString::from("9876"),
                suffix.into(),
                path.as_path().as_os_str().to_os_string(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(parsed.parent_pid, 9876);
        assert_eq!(parsed.trash_path, path.as_path());

        let unrenamed = test_absolute_path("/home/test/data", r"C:\Users\test\data");
        assert!(
            parse_deferred_delete_args(
                [
                    std::ffi::OsString::from("9876"),
                    suffix.into(),
                    unrenamed.as_path().as_os_str().to_os_string(),
                ]
                .into_iter()
            )
            .is_none()
        );
    }

    #[test]
    fn test_build_deferred_helper_delete_script_is_shallow() {
        let path = std::path::Path::new(r"C:\Temp\vite-plus-cleanup $&' 测试.exe");
        let script = build_deferred_helper_delete_script(path, 9876);
        assert!(script.contains("$helperPid=9876"));
        assert!(script.contains("Wait-Process -Id $helperPid"));
        assert!(script.contains("Remove-Item -LiteralPath $helper"));
        assert!(script.contains("Start-Sleep -Milliseconds 100"));
        assert!(script.contains(r"C:\Temp\vite-plus-cleanup $&'' 测试.exe"));
        assert!(!script.contains("Remove-VpTree"));
        assert!(!script.contains("Get-ChildItem"));
    }

    #[test]
    fn test_deferred_cleanup_error_path_is_next_to_the_renamed_root() {
        let trash = test_absolute_path(
            "/home/test/data.removing-9876-123456",
            r"C:\Users\test\data.removing-9876-123456",
        );
        let expected = test_absolute_path(
            "/home/test/data.removing-9876-123456.cleanup-error.txt",
            r"C:\Users\test\data.removing-9876-123456.cleanup-error.txt",
        );
        assert_eq!(deferred_cleanup_error_path(trash.as_path()), expected.as_path());
    }

    #[test]
    fn test_build_deferred_shim_delete_script_uses_only_renamed_paths() {
        assert_eq!(
            deferred_shim_path(Path::new(r"C:\Users\test\bin\vp.exe"), "removing-1234"),
            PathBuf::from(r"C:\Users\test\bin\vp.exe.removing-1234")
        );
        let paths = DeferredShimPaths {
            executable: PathBuf::from(r"C:\Users\test\bin\vp.exe.removing-1234"),
            pointer: PathBuf::from(r"C:\Users\test\bin\vp.shim.removing-1234"),
        };
        let script = build_deferred_shim_delete_script(&paths);
        assert!(script.contains("Remove-Item -LiteralPath $exe"));
        assert!(script.contains("Remove-Item -LiteralPath $sidecar"));
        assert!(script.contains(r"C:\Users\test\bin\vp.exe.removing-1234"));
        assert!(script.contains(r"C:\Users\test\bin\vp.shim.removing-1234"));
        assert!(!script.contains(r"$exe='C:\Users\test\bin\vp.exe';"));
    }

    #[test]
    #[cfg(windows)]
    fn locked_executable_child() {
        if std::env::var_os("VP_IMPLODE_LOCK_TEST").is_some() {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    #[test]
    #[cfg(windows)]
    fn deferred_root_cleanup_child() {
        let Some(root) = std::env::var_os("VP_IMPLODE_ROOT_TEST") else {
            return;
        };
        let root = AbsolutePathBuf::new(root.into()).unwrap();
        let cleanup_source = prepare_deferred_delete_source(std::slice::from_ref(&root)).unwrap();
        remove_vite_plus_dir(&root, &cleanup_source).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    #[test]
    #[cfg(windows)]
    fn deferred_delete_helper_child() {
        let (Some(parent_pid), Some(suffix), Some(trash_path)) = (
            std::env::var_os("VP_IMPLODE_HELPER_PARENT_PID"),
            std::env::var_os("VP_IMPLODE_HELPER_SUFFIX"),
            std::env::var_os("VP_IMPLODE_HELPER_TRASH"),
        ) else {
            return;
        };
        let args = [
            std::ffi::OsString::from("vp"),
            std::ffi::OsString::from(DEFERRED_DELETE_MODE),
            parent_pid,
            suffix,
            trash_path,
        ];
        assert_eq!(maybe_run_deferred_delete_helper(args.into_iter()), Some(0));
    }

    #[test]
    #[cfg(windows)]
    fn deferred_root_delete_waits_and_preserves_an_immediate_reinstall() {
        let temp_dir = tempfile::tempdir().unwrap();
        let parent = temp_dir.path().join("root $&' 测试");
        let original = parent.join("data");
        let old_version = original.join("version");
        let locked_executable = old_version.join("bin/vp.exe");
        std::fs::create_dir_all(locked_executable.parent().unwrap()).unwrap();
        std::fs::copy(std::env::current_exe().unwrap(), &locked_executable).unwrap();

        let long_file = old_version
            .join("node_modules/.pnpm")
            .join("peer-dependencies-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .join("node_modules")
            .join("package-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .join("dist/client/orchestrator.js");
        std::fs::create_dir_all(long_file.parent().unwrap()).unwrap();
        std::fs::write(&long_file, b"long-path").unwrap();
        use std::os::windows::ffi::OsStrExt;
        assert!(
            long_file.as_os_str().encode_wide().count() > 260,
            "test setup must create a path longer than MAX_PATH"
        );

        let current = original.join("current");
        let junction_status = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&current)
            .arg(&old_version)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(junction_status.success(), "test setup must create the current junction");

        let dangling_target = parent.join("removed-junction-target");
        let dangling = original.join("dangling-pnpm-junction");
        std::fs::create_dir_all(&dangling_target).unwrap();
        let dangling_status = std::process::Command::new("cmd.exe")
            .args(["/D", "/C", "mklink", "/J"])
            .arg(&dangling)
            .arg(&dangling_target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(dangling_status.success(), "test setup must create the dangling junction");
        std::fs::remove_dir(&dangling_target).unwrap();

        let mut child = std::process::Command::new(&locked_executable)
            .args(["deferred_root_cleanup_child", "--nocapture"])
            .env("VP_IMPLODE_ROOT_TEST", &original)
            .spawn()
            .unwrap();
        let trash_prefix = vt_str::format!("data.removing-{}-", child.id());
        let mut trash = None;
        for _ in 0..50 {
            trash = std::fs::read_dir(&parent).ok().and_then(|entries| {
                entries.flatten().map(|entry| entry.path()).find(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with(&*trash_prefix))
                })
            });
            if trash.is_some() && !original.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let trash = trash.expect("the child must rename the old root");
        let failure_log = deferred_cleanup_error_path(&trash);
        assert!(trash.exists(), "the child must rename the old root");
        assert!(!original.exists(), "the original root must be free immediately");
        assert!(child.try_wait().unwrap().is_none(), "the cleanup helper must wait for vp");

        let replacement = original.join("version/replacement.txt");
        std::fs::create_dir_all(replacement.parent().unwrap()).unwrap();
        std::fs::write(&replacement, b"keep").unwrap();
        assert!(child.wait().unwrap().success());

        for _ in 0..150 {
            let removing_roots = std::fs::read_dir(&parent)
                .unwrap()
                .flatten()
                .map(|entry| entry.file_name())
                .filter(|name| name.to_string_lossy().starts_with("data.removing-"))
                .count();
            if removing_roots == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        assert!(!trash.exists(), "the renamed root must be removed");
        assert!(!failure_log.exists(), "successful cleanup must not write a failure log");
        assert_eq!(std::fs::read(&replacement).unwrap(), b"keep");
    }

    #[test]
    #[cfg(windows)]
    fn deferred_shim_delete_cannot_remove_an_immediate_reinstall() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bin = temp_dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("vp.exe");
        let pointer = bin.join("vp.shim");
        let unrelated = bin.join("unrelated.txt");
        std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        std::fs::write(&pointer, b"old-sidecar").unwrap();
        std::fs::write(&unrelated, b"keep").unwrap();

        let mut child = std::process::Command::new(&executable)
            .args(["locked_executable_child", "--nocapture"])
            .env("VP_IMPLODE_LOCK_TEST", "1")
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(child.try_wait().unwrap().is_none(), "test executable must still be running");
        assert!(std::fs::remove_file(&executable).is_err(), "running executable must be locked");

        let deferred = schedule_deferred_shim_delete(&executable, &pointer).unwrap();
        assert!(!executable.exists(), "the original executable path must be free immediately");
        assert!(!pointer.exists(), "the original sidecar path must be free immediately");

        // Simulate an installer that starts as soon as `vp implode` returns.
        std::fs::copy(std::env::current_exe().unwrap(), &executable).unwrap();
        std::fs::write(&pointer, b"new-sidecar").unwrap();

        assert!(child.wait().unwrap().success());
        for _ in 0..100 {
            if !deferred.executable.exists() && !deferred.pointer.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        assert!(!deferred.executable.exists(), "the renamed executable must be removed");
        assert!(!deferred.pointer.exists(), "the renamed sidecar must be removed");
        assert!(executable.exists(), "deferred cleanup must preserve the replacement executable");
        assert_eq!(std::fs::read(&pointer).unwrap(), b"new-sidecar");
        assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_abbreviate_home_path() {
        let home = AbsolutePathBuf::new("/home/user".into()).unwrap();
        // Under home → ~/...
        let under = AbsolutePathBuf::new("/home/user/.zshrc".into()).unwrap();
        assert_eq!(&*abbreviate_home_path(&under, &home), "~/.zshrc");
        // Outside home → absolute path as-is
        let outside = AbsolutePathBuf::new("/opt/zdotdir/.zshenv".into()).unwrap();
        assert_eq!(&*abbreviate_home_path(&outside, &home), "/opt/zdotdir/.zshenv");
    }

    #[test]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let home_dir = home.join(".vite-plus");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &home);

        // Clear env overrides so the test environment doesn't affect results
        temp_env::with_vars_unset(["ZDOTDIR", "XDG_CONFIG_HOME", "XDG_DATA_HOME"], || {
            // Main profile with vite-plus line
            std::fs::write(home.join(".zshrc"), ". \"$HOME/.vite-plus/env\"\n").unwrap();
            // Unrelated profile (should be ignored)
            std::fs::write(home.join(".bashrc"), "export PATH=/usr/bin\n").unwrap();
            // Snippet file with a matching Vite+ source line
            let fish_dir = home.join(".config/fish/conf.d");
            std::fs::create_dir_all(&fish_dir).unwrap();
            std::fs::write(fish_dir.join("vite-plus.fish"), "source ~/.vite-plus/env.fish\n")
                .unwrap();

            let profiles = collect_affected_profiles(&home, &matcher);
            assert_eq!(profiles.len(), 2);
            assert!(matches!(&profiles[0].kind, AffectedProfileKind::Main { .. }));
            assert!(matches!(&profiles[1].kind, AffectedProfileKind::Snippet));
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_custom_home_relative_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let home_dir = home.join("tools/vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &home);

        temp_env::with_vars_unset(["ZDOTDIR", "XDG_CONFIG_HOME", "XDG_DATA_HOME"], || {
            std::fs::write(home.join(".zshrc"), ". \"$HOME/tools/vp/env\"\n").unwrap();
            std::fs::write(home.join(".bashrc"), ". \"$HOME/.vite-plus/env\"\n").unwrap();
            let fish_dir = home.join(".config/fish/conf.d");
            std::fs::create_dir_all(&fish_dir).unwrap();
            std::fs::write(fish_dir.join("vite-plus.fish"), "source ~/.vite-plus/env.fish\n")
                .unwrap();

            let profiles = collect_affected_profiles(&home, &matcher);
            assert_eq!(profiles.len(), 1);
            assert!(matches!(&profiles[0].kind, AffectedProfileKind::Main { .. }));
        });
    }

    #[test]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_zdotdir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("home")).unwrap();
        let zdotdir = temp_dir.path().join("zdotdir");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&zdotdir).unwrap();

        std::fs::write(zdotdir.join(".zshenv"), ". \"$HOME/.vite-plus/env\"\n").unwrap();

        temp_env::with_vars(
            [
                ("ZDOTDIR", Some(zdotdir.as_os_str())),
                ("XDG_CONFIG_HOME", None),
                ("XDG_DATA_HOME", None),
            ],
            || {
                let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

                let profiles = collect_affected_profiles(&home, &matcher);
                let zdotdir_profiles: Vec<_> =
                    profiles.iter().filter(|p| p.path.as_path().starts_with(&zdotdir)).collect();
                assert_eq!(zdotdir_profiles.len(), 1);
                assert!(matches!(&zdotdir_profiles[0].kind, AffectedProfileKind::Main { .. }));
            },
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_xdg_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("home")).unwrap();
        let xdg_config = temp_dir.path().join("xdg_config");
        let fish_dir = xdg_config.join("fish/conf.d");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&fish_dir).unwrap();

        std::fs::write(fish_dir.join("vite-plus.fish"), "source \"$HOME/.vite-plus/env.fish\"\n")
            .unwrap();

        temp_env::with_vars(
            [
                ("ZDOTDIR", None),
                ("XDG_CONFIG_HOME", Some(xdg_config.as_os_str())),
                ("XDG_DATA_HOME", None),
            ],
            || {
                let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

                let profiles = collect_affected_profiles(&home, &matcher);
                let xdg_profiles: Vec<_> =
                    profiles.iter().filter(|p| p.path.as_path().starts_with(&xdg_config)).collect();
                assert_eq!(xdg_profiles.len(), 1);
                assert!(matches!(&xdg_profiles[0].kind, AffectedProfileKind::Snippet));
            },
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_xdg_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("home")).unwrap();
        let xdg_data = temp_dir.path().join("xdg_data");
        let nushell_dir = xdg_data.join("nushell/vendor/autoload");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&nushell_dir).unwrap();

        std::fs::write(nushell_dir.join("vite-plus.nu"), "source '~/.vite-plus/env.nu'\n").unwrap();

        temp_env::with_vars(
            [
                ("ZDOTDIR", None),
                ("XDG_CONFIG_HOME", None),
                ("XDG_DATA_HOME", Some(xdg_data.as_os_str())),
            ],
            || {
                let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

                let profiles = collect_affected_profiles(&home, &matcher);
                let xdg_profiles: Vec<_> =
                    profiles.iter().filter(|p| p.path.as_path().starts_with(&xdg_data)).collect();
                assert_eq!(xdg_profiles.len(), 1);
                assert!(matches!(&xdg_profiles[0].kind, AffectedProfileKind::Snippet));
            },
        );
    }

    #[test]
    fn test_execute_not_installed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let non_existent = temp_dir.path().join("does-not-exist");
        vp_shared::EnvConfig::with_vars([(env_vars::VP_HOME, &non_existent)], |_| {
            let result = execute(true);
            assert!(result.is_ok());
            assert!(result.unwrap().success());
        });
    }

    #[test]
    fn execute_normalizes_category_roots_before_deduplication() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = temp_dir.path().join("home");
        let bin = temp_dir.path().join("bin");
        let data = temp_dir.path().join("data");
        let cache = data.join("../cache");
        let normalized_cache = temp_dir.path().join("cache");
        let xdg_config = temp_dir.path().join("config-base");
        let xdg_state = temp_dir.path().join("state-base");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::create_dir_all(&normalized_cache).unwrap();
        std::fs::write(data.join("data.txt"), b"data").unwrap();
        std::fs::write(normalized_cache.join("cache.txt"), b"cache").unwrap();

        vp_shared::EnvConfig::with_vars(
            [
                (env_vars::VP_HOME, None),
                (env_vars::VP_BIN_DIR, Some(bin.as_os_str())),
                (env_vars::VP_DATA_DIR, Some(data.as_os_str())),
                (env_vars::VP_CACHE_DIR, Some(cache.as_os_str())),
                (env_vars::XDG_CONFIG_HOME, Some(xdg_config.as_os_str())),
                (env_vars::XDG_STATE_HOME, Some(xdg_state.as_os_str())),
                ("HOME", Some(home.as_os_str())),
                ("USERPROFILE", Some(home.as_os_str())),
            ],
            |_| {
                let result = execute(true).unwrap();
                assert!(result.success());
                assert!(!data.exists(), "data root must be removed");
                assert!(
                    !normalized_cache.exists(),
                    "lexically distinct cache root must be removed"
                );
            },
        );
    }

    #[cfg(unix)]
    #[test]
    fn remove_shim_files_deletes_only_vp_symlinks() {
        vp_shared::EnvConfig::scoped(|config| {
            let bin = &config.dirs.bin;
            let bins_dir = config.dirs.data.join("bins");
            std::fs::create_dir_all(bin).unwrap();
            std::fs::create_dir_all(&bins_dir).unwrap();

            std::fs::write(bin.join("node").as_path(), b"system-node").unwrap();
            let vp_target = crate::commands::global::install::package_shim_target();
            std::os::unix::fs::symlink(vp_target.as_path(), bin.join("vp").as_path()).unwrap();
            // Leftover relative link from a monolithic `<DATA>/bin` must still
            // resolve to `<DATA>/current/bin/vp` and be removed.
            std::os::unix::fs::symlink("../current/bin/vp", bin.join("npm").as_path()).unwrap();

            std::fs::write(bins_dir.join("tsc.json").as_path(), "{}").unwrap();
            std::os::unix::fs::symlink(vp_target.as_path(), bin.join("tsc").as_path()).unwrap();
            std::fs::write(bin.join("tsc.shim").as_path(), "data\n").unwrap();

            std::fs::write(bins_dir.join("eslint.json").as_path(), "{}").unwrap();
            std::os::unix::fs::symlink("/usr/bin/eslint", bin.join("eslint").as_path()).unwrap();

            remove_shim_files(&config.dirs);

            assert!(bin.join("node").as_path().is_file(), "unrelated node binary must be kept");
            assert!(
                std::fs::symlink_metadata(bin.join("eslint").as_path()).is_ok(),
                "recorded shim that does not point at vp must be kept"
            );
            assert!(
                std::fs::symlink_metadata(bin.join("vp").as_path()).is_err(),
                "vp symlink must be removed"
            );
            assert!(
                std::fs::symlink_metadata(bin.join("npm").as_path()).is_err(),
                "default env shim that points at vp must be removed"
            );
            assert!(
                std::fs::symlink_metadata(bin.join("tsc").as_path()).is_err(),
                "recorded package shim that points at vp must be removed"
            );
            assert!(
                !bin.join("tsc.shim").as_path().exists(),
                "sidecar next to a removed shim must be removed"
            );
        });
    }

    #[cfg(windows)]
    #[test]
    fn remove_shim_files_preserves_unowned_windows_entries() {
        vp_shared::EnvConfig::scoped(|config| {
            let bin = &config.dirs.bin;
            std::fs::create_dir_all(bin).unwrap();
            std::fs::write(bin.join("vp.exe").as_path(), b"foreign-vp").unwrap();
            std::fs::write(bin.join("unrelated.txt").as_path(), b"keep").unwrap();
            std::fs::write(bin.join("node.exe").as_path(), b"owned-node").unwrap();
            config.dirs.write_shim_pointer("node").unwrap();

            remove_shim_files(&config.dirs);

            assert_eq!(std::fs::read(bin.join("vp.exe").as_path()).unwrap(), b"foreign-vp");
            assert_eq!(std::fs::read(bin.join("unrelated.txt").as_path()).unwrap(), b"keep");
            assert!(!bin.join("node.exe").as_path().exists());
            assert!(!bin.join("node.shim").as_path().exists());
        });
    }
}
