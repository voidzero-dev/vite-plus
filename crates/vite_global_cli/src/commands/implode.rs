//! `vp implode` — completely remove vp and all its data from this system.

use std::{
    io::{IsTerminal, Write},
    process::ExitStatus,
};

use directories::BaseDirs;
use owo_colors::OwoColorize;
use vite_path::AbsolutePathBuf;
use vite_shared::output;
use vite_str::Str;

use crate::{
    cli::exit_status,
    commands::shell::{
        ALL_SHELL_PROFILES, ShellProfileKind, abbreviate_home_path, resolve_profile_path,
    },
    error::Error,
};

/// Comment marker written by the install script above the sourcing line.
const VITE_PLUS_COMMENT: &str = "# Vite+ bin";

pub fn execute(yes: bool) -> Result<ExitStatus, Error> {
    let Ok(home_dir) = vite_shared::get_vp_home() else {
        output::info("vite-plus is not installed (could not determine home directory)");
        return Ok(exit_status(0));
    };

    if !home_dir.as_path().exists() {
        output::info("vite-plus is not installed (directory does not exist)");
        return Ok(exit_status(0));
    }

    // Resolve user home for shell profile paths
    let base_dirs = BaseDirs::new()
        .ok_or_else(|| Error::Other("Could not determine user home directory".into()))?;
    let user_home = AbsolutePathBuf::new(base_dirs.home_dir().to_path_buf()).unwrap();

    let source_matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);

    // Collect shell profiles that contain Vite+ lines (content cached for cleaning)
    let affected_profiles = collect_affected_profiles(&user_home, &source_matcher);

    // Confirmation
    if !yes && !confirm_implode(&home_dir, &affected_profiles)? {
        return Ok(exit_status(0));
    }

    // Clean shell profiles using cached content (no re-read)
    clean_affected_profiles(&affected_profiles, &source_matcher);

    // Remove Windows PATH entry
    #[cfg(windows)]
    {
        let bin_path = home_dir.join("bin");
        if let Err(e) = remove_windows_path_entry(&bin_path) {
            output::warn(&vite_str::format!("Failed to clean Windows PATH: {e}"));
        } else {
            output::success("Removed vite-plus from Windows PATH");
        }
    }

    // Remove the directory
    remove_vite_plus_dir(&home_dir)?;

    output::raw("");
    output::success("vite-plus has been removed from your system.");
    output::note("Restart your terminal to apply shell changes.");

    Ok(exit_status(0))
}

/// A shell profile that contains Vite+ sourcing lines.
struct AffectedProfile {
    /// Display name (e.g. ".zshrc", ".config/fish/conf.d/vite-plus.fish").
    name: Str,
    /// Absolute path to the file.
    path: AbsolutePathBuf,
    kind: AffectedProfileKind,
}

// Indicating whether it's a snippet (remove file) or a main profile (remove lines).
enum AffectedProfileKind {
    // A snippet, uninstall would be as easy as removing the file
    Snippet,
    Main {
        /// File content read during detection (reused for cleaning).
        content: Str,
        env_file: &'static str,
    },
}

/// Collect shell profiles that contain Vite+ sourcing lines.
/// Content is cached so we don't need to re-read during cleaning.
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
    home_dir: &AbsolutePathBuf,
    affected_profiles: &[AffectedProfile],
) -> Result<bool, Error> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::UserMessage(
            "Cannot prompt for confirmation: stdin is not a TTY. Use --yes to skip confirmation."
                .into(),
        ));
    }

    output::warn("This will completely remove vite-plus from your system!");
    output::raw("");
    output::raw(&vite_str::format!("  Directory: {}", home_dir.as_path().display()));
    if !affected_profiles.is_empty() {
        output::raw("  Shell profiles to clean:");
        for profile in affected_profiles {
            output::raw(&vite_str::format!("    - {}", profile.name));
        }
    }
    output::raw("");
    output::raw(&vite_str::format!("Type {} to confirm:", "uninstall".bold()));

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
                    Ok(()) => output::success(&vite_str::format!("Cleaned {}", profile.name)),
                    Err(e) => {
                        output::warn(&vite_str::format!("Failed to clean {}: {e}", profile.name));
                    }
                }
            }
            AffectedProfileKind::Snippet => match std::fs::remove_file(&profile.path) {
                Ok(()) => output::success(&vite_str::format!("Removed {}", profile.name)),
                Err(e) => {
                    output::warn(&vite_str::format!("Failed to remove {}: {e}", profile.name));
                }
            },
        }
    }
}

/// Remove the ~/.vite-plus directory.
fn remove_vite_plus_dir(home_dir: &AbsolutePathBuf) -> Result<(), Error> {
    #[cfg(unix)]
    {
        match std::fs::remove_dir_all(home_dir) {
            Ok(()) => {
                output::success(&vite_str::format!("Removed {}", home_dir.as_path().display()));
                Ok(())
            }
            Err(e) => {
                output::error(&vite_str::format!(
                    "Failed to remove {}: {e}",
                    home_dir.as_path().display()
                ));
                Err(Error::CommandExecution(e))
            }
        }
    }

    #[cfg(windows)]
    {
        // On Windows, the running `vp` binary is always locked, so direct
        // removal will fail.  Rename the directory first so the original path
        // is immediately free for reinstall, then schedule deletion of the
        // renamed directory via a detached process.
        let trash_path =
            home_dir.as_path().with_extension(vite_str::format!("removing-{}", std::process::id()));
        if let Err(e) = std::fs::rename(home_dir, &trash_path) {
            output::error(&vite_str::format!(
                "Failed to rename {} for removal: {e}",
                home_dir.as_path().display()
            ));
            return Err(Error::CommandExecution(e));
        }

        match spawn_deferred_delete(&trash_path) {
            Ok(_) => {
                output::success(&vite_str::format!(
                    "Scheduled removal of {} (will complete shortly)",
                    home_dir.as_path().display()
                ));
            }
            Err(e) => {
                output::error(&vite_str::format!(
                    "Failed to schedule removal of {}: {e}",
                    home_dir.as_path().display()
                ));
                return Err(Error::CommandExecution(e));
            }
        }
        Ok(())
    }
}

/// Build a `cmd.exe` script that retries `rmdir /S /Q` up to 10 times with
/// 1-second pauses, exiting as soon as the directory is gone.
#[cfg(windows)]
fn build_deferred_delete_script(trash_path: &std::path::Path) -> Str {
    let p = trash_path.to_string_lossy();
    vite_str::format!(
        "for /L %i in (1,1,10) do @(\
            if not exist \"{p}\" exit /B 0 & \
            rmdir /S /Q \"{p}\" 2>NUL & \
            if not exist \"{p}\" exit /B 0 & \
            timeout /T 1 /NOBREAK >NUL\
        )"
    )
}

/// Spawn a detached `cmd.exe` process that retries deletion of `trash_path`.
#[cfg(windows)]
fn spawn_deferred_delete(trash_path: &std::path::Path) -> std::io::Result<std::process::Child> {
    let script = build_deferred_delete_script(trash_path);
    std::process::Command::new("cmd.exe")
        .args(["/C", &script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

/// Matches shell-profile `source` lines that reference *this* install's env
/// files, so a second Vite+ install's lines are left untouched.
///
/// The recognized home spellings must mirror what the writers emit:
/// `install.sh`/`install.ps1` (shell PATH setup) and `render_env_content` in
/// `env/setup.rs`. `env/doctor.rs::check_profile_files` derives the same
/// variants for its profile scan; keep them in sync.
struct VitePlusSourceMatcher {
    /// Home-dir spellings with forward-slash separators: the absolute path,
    /// plus `$HOME`- and `~`-relative forms when the home is under `$HOME`.
    roots: Vec<Str>,
}

impl VitePlusSourceMatcher {
    fn new(home_dir: &AbsolutePathBuf, user_home: &AbsolutePathBuf) -> Self {
        let mut roots = vec![normalize_path_separators(&home_dir.as_path().display().to_string())];

        if let Ok(Some(suffix)) = home_dir.strip_prefix(user_home) {
            // `RelativePathBuf` guarantees forward-slash separators.
            let suffix = vite_str::format!("{suffix}");
            if suffix.is_empty() {
                roots.push(Str::from("$HOME"));
                roots.push(Str::from("~"));
            } else {
                roots.push(vite_str::format!("$HOME/{suffix}"));
                roots.push(vite_str::format!("~/{suffix}"));
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
    vite_str::format!("{root}{separator}{env_file}")
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

/// Remove `.vite-plus\bin` from the Windows User PATH via PowerShell.
#[cfg(windows)]
fn remove_windows_path_entry(bin_path: &vite_path::AbsolutePath) -> std::io::Result<()> {
    let bin_str = bin_path.as_path().to_string_lossy();
    let script = vite_str::format!(
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
    #[cfg(not(windows))]
    use serial_test::serial;

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
        let content = vite_str::format!("# existing\n. \"{env_path}\"\n");
        let result = remove_vite_plus_lines(&content, &matcher, "env");
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_custom_absolute_path() {
        let user_home = custom_user_home();
        let home_dir = user_home.join("tools").join("vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &user_home);
        let env_path = shell_path(&home_dir.join("env"));
        let content = vite_str::format!("# existing\n. \"{env_path}\"\n");
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
    fn test_remove_vite_plus_dir_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dir = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let target = dir.join("does-not-exist");

        let result = remove_vite_plus_dir(&target);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_build_deferred_delete_script() {
        let path = std::path::Path::new(r"C:\Users\test\.vite-plus.removing-1234");
        let script = build_deferred_delete_script(path);
        assert!(script.contains("rmdir /S /Q"));
        assert!(script.contains(r"C:\Users\test\.vite-plus.removing-1234"));
        assert!(script.contains("for /L %i in (1,1,10)"));
        assert!(script.contains("timeout /T 1 /NOBREAK"));
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
    #[serial]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let home_dir = home.join(".vite-plus");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &home);

        // Clear env overrides so the test environment doesn't affect results
        let _guard = ProfileEnvGuard::new(None, None, None);

        // Main profile with vite-plus line
        std::fs::write(home.join(".zshrc"), ". \"$HOME/.vite-plus/env\"\n").unwrap();
        // Unrelated profile (should be ignored)
        std::fs::write(home.join(".bashrc"), "export PATH=/usr/bin\n").unwrap();
        // Snippet file with a matching Vite+ source line
        let fish_dir = home.join(".config/fish/conf.d");
        std::fs::create_dir_all(&fish_dir).unwrap();
        std::fs::write(fish_dir.join("vite-plus.fish"), "source ~/.vite-plus/env.fish\n").unwrap();

        let profiles = collect_affected_profiles(&home, &matcher);
        assert_eq!(profiles.len(), 2);
        assert!(matches!(&profiles[0].kind, AffectedProfileKind::Main { .. }));
        assert!(matches!(&profiles[1].kind, AffectedProfileKind::Snippet));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_custom_home_relative_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let home_dir = home.join("tools/vp");
        let matcher = VitePlusSourceMatcher::new(&home_dir, &home);

        let _guard = ProfileEnvGuard::new(None, None, None);

        std::fs::write(home.join(".zshrc"), ". \"$HOME/tools/vp/env\"\n").unwrap();
        std::fs::write(home.join(".bashrc"), ". \"$HOME/.vite-plus/env\"\n").unwrap();
        let fish_dir = home.join(".config/fish/conf.d");
        std::fs::create_dir_all(&fish_dir).unwrap();
        std::fs::write(fish_dir.join("vite-plus.fish"), "source ~/.vite-plus/env.fish\n").unwrap();

        let profiles = collect_affected_profiles(&home, &matcher);
        assert_eq!(profiles.len(), 1);
        assert!(matches!(&profiles[0].kind, AffectedProfileKind::Main { .. }));
    }

    /// Guard that saves and restores profile-related env vars.
    #[cfg(not(windows))]
    struct ProfileEnvGuard {
        original_zdotdir: Option<std::ffi::OsString>,
        original_xdg_config: Option<std::ffi::OsString>,
        original_xdg_data: Option<std::ffi::OsString>,
    }

    #[cfg(not(windows))]
    impl ProfileEnvGuard {
        fn new(
            zdotdir: Option<&std::path::Path>,
            xdg_config: Option<&std::path::Path>,
            xdg_data: Option<&std::path::Path>,
        ) -> Self {
            let guard = Self {
                original_zdotdir: std::env::var_os("ZDOTDIR"),
                original_xdg_config: std::env::var_os("XDG_CONFIG_HOME"),
                original_xdg_data: std::env::var_os("XDG_DATA_HOME"),
            };
            unsafe {
                match zdotdir {
                    Some(v) => std::env::set_var("ZDOTDIR", v),
                    None => std::env::remove_var("ZDOTDIR"),
                }
                match xdg_config {
                    Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
                match xdg_data {
                    Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                    None => std::env::remove_var("XDG_DATA_HOME"),
                }
            }
            guard
        }
    }

    #[cfg(not(windows))]
    impl Drop for ProfileEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original_zdotdir {
                    Some(v) => std::env::set_var("ZDOTDIR", v),
                    None => std::env::remove_var("ZDOTDIR"),
                }
                match &self.original_xdg_config {
                    Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                    None => std::env::remove_var("XDG_CONFIG_HOME"),
                }
                match &self.original_xdg_data {
                    Some(v) => std::env::set_var("XDG_DATA_HOME", v),
                    None => std::env::remove_var("XDG_DATA_HOME"),
                }
            }
        }
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_zdotdir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("home")).unwrap();
        let zdotdir = temp_dir.path().join("zdotdir");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&zdotdir).unwrap();

        std::fs::write(zdotdir.join(".zshenv"), ". \"$HOME/.vite-plus/env\"\n").unwrap();

        let _guard = ProfileEnvGuard::new(Some(&zdotdir), None, None);
        let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

        let profiles = collect_affected_profiles(&home, &matcher);
        let zdotdir_profiles: Vec<_> =
            profiles.iter().filter(|p| p.path.as_path().starts_with(&zdotdir)).collect();
        assert_eq!(zdotdir_profiles.len(), 1);
        assert!(matches!(&zdotdir_profiles[0].kind, AffectedProfileKind::Main { .. }));
    }

    #[test]
    #[serial]
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

        let _guard = ProfileEnvGuard::new(None, Some(&xdg_config), None);
        let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

        let profiles = collect_affected_profiles(&home, &matcher);
        let xdg_profiles: Vec<_> =
            profiles.iter().filter(|p| p.path.as_path().starts_with(&xdg_config)).collect();
        assert_eq!(xdg_profiles.len(), 1);
        assert!(matches!(&xdg_profiles[0].kind, AffectedProfileKind::Snippet));
    }

    #[test]
    #[serial]
    #[cfg(not(windows))]
    fn test_collect_affected_profiles_xdg_data() {
        let temp_dir = tempfile::tempdir().unwrap();
        let home = AbsolutePathBuf::new(temp_dir.path().join("home")).unwrap();
        let xdg_data = temp_dir.path().join("xdg_data");
        let nushell_dir = xdg_data.join("nushell/vendor/autoload");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&nushell_dir).unwrap();

        std::fs::write(nushell_dir.join("vite-plus.nu"), "source '~/.vite-plus/env.nu'\n").unwrap();

        let _guard = ProfileEnvGuard::new(None, None, Some(&xdg_data));
        let matcher = VitePlusSourceMatcher::new(&home.join(".vite-plus"), &home);

        let profiles = collect_affected_profiles(&home, &matcher);
        let xdg_profiles: Vec<_> =
            profiles.iter().filter(|p| p.path.as_path().starts_with(&xdg_data)).collect();
        assert_eq!(xdg_profiles.len(), 1);
        assert!(matches!(&xdg_profiles[0].kind, AffectedProfileKind::Snippet));
    }

    #[test]
    fn test_execute_not_installed() {
        let temp_dir = tempfile::tempdir().unwrap();
        let non_existent = temp_dir.path().join("does-not-exist");
        // Use thread-local test guard instead of mutating process-global env
        let _guard = vite_shared::EnvConfig::test_guard(
            vite_shared::EnvConfig::for_test_with_home(&non_existent),
        );
        let result = execute(true);
        assert!(result.is_ok());
        assert!(result.unwrap().success());
    }
}
