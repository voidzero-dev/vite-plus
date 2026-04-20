//! `vp implode` — completely remove vp and all its data from this system.

use std::{
    io::{IsTerminal, Write},
    process::ExitStatus,
};

use directories::BaseDirs;
use owo_colors::OwoColorize;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_shared::output;
use vite_str::Str;

use crate::{cli::exit_status, error::Error};

/// All shell profile paths to check, with `is_snippet` flag.
const SHELL_PROFILES: &[(&str, bool)] = &[
    (".zshenv", false),
    (".zshrc", false),
    (".bash_profile", false),
    (".bashrc", false),
    (".profile", false),
    (".config/fish/conf.d/vite-plus.fish", true),
];

/// Abbreviate a path for display: replace `$HOME` prefix with `~`.
fn abbreviate_home_path(path: &AbsolutePath, user_home: &AbsolutePath) -> Str {
    match path.strip_prefix(user_home) {
        Ok(Some(suffix)) => vite_str::format!("~/{suffix}"),
        _ => Str::from(path.to_string()),
    }
}

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

    // Collect shell profiles that contain Vite+ lines (content cached for cleaning)
    let affected_profiles = collect_affected_profiles(&user_home);

    // Confirmation
    if !yes && !confirm_implode(&home_dir, &affected_profiles)? {
        return Ok(exit_status(0));
    }

    // Clean shell profiles using cached content (no re-read)
    clean_affected_profiles(&affected_profiles);

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
    },
}

/// Collect shell profiles that contain Vite+ sourcing lines.
/// Content is cached so we don't need to re-read during cleaning.
fn collect_affected_profiles(user_home: &AbsolutePathBuf) -> Vec<AffectedProfile> {
    let mut affected = Vec::new();

    // Build full list of (display_name, path, is_snippet) from the base set
    let mut profiles: Vec<(Str, AbsolutePathBuf, bool)> = SHELL_PROFILES
        .iter()
        .map(|&(name, is_snippet)| {
            (vite_str::format!("~/{name}"), user_home.join(name), is_snippet)
        })
        .collect();

    // If ZDOTDIR is set and differs from $HOME, also check there.
    if let Ok(zdotdir) = std::env::var("ZDOTDIR")
        && let Some(zdotdir_path) = AbsolutePathBuf::new(zdotdir.into())
        && zdotdir_path != *user_home
    {
        for name in [".zshenv", ".zshrc"] {
            let path = zdotdir_path.join(name);
            let display = abbreviate_home_path(&path, user_home);
            profiles.push((display, path, false));
        }
    }

    // If XDG_CONFIG_HOME is set and differs from $HOME/.config, also check there.
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME")
        && let Some(xdg_path) = AbsolutePathBuf::new(xdg_config.into())
        && xdg_path != user_home.join(".config")
    {
        let path = xdg_path.join("fish/conf.d/vite-plus.fish");
        let display = abbreviate_home_path(&path, user_home);
        profiles.push((display, path, true));
    }

    for (name, path, is_snippet) in profiles {
        // For snippets, check if the file exists only
        if is_snippet {
            if let Ok(true) = std::fs::exists(&path) {
                affected.push(AffectedProfile { name, path, kind: AffectedProfileKind::Snippet })
            }
            continue;
        }
        // Read directly — if the file doesn't exist, read_to_string returns Err
        // which .ok().filter() handles gracefully (no redundant exists() check).
        if let Some(content) =
            std::fs::read_to_string(&path).ok().filter(|c| has_vite_plus_lines(c))
        {
            affected.push(AffectedProfile {
                name,
                path,
                kind: AffectedProfileKind::Main { content: Str::from(content) },
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
fn clean_affected_profiles(affected_profiles: &[AffectedProfile]) {
    for profile in affected_profiles {
        match &profile.kind {
            AffectedProfileKind::Main { content } => {
                let cleaned = remove_vite_plus_lines(content);
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

/// Check if file content contains Vite+ sourcing lines.
fn has_vite_plus_lines(content: &str) -> bool {
    let pattern = ".vite-plus/env\"";
    content.lines().any(|line| line.contains(pattern))
}

/// Remove Vite+ lines from content, returning the cleaned string.
fn remove_vite_plus_lines(content: &str) -> Str {
    let pattern = ".vite-plus/env\"";
    let lines: Vec<&str> = content.lines().collect();
    let mut remove_indices = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if line.contains(pattern) {
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

    #[test]
    fn test_remove_vite_plus_lines_posix() {
        let content = "# existing config\nexport FOO=bar\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n";
        let result = remove_vite_plus_lines(content);
        assert_eq!(&*result, "# existing config\nexport FOO=bar\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_no_match() {
        let content = "# just a normal config\nexport PATH=/usr/bin\n";
        let result = remove_vite_plus_lines(content);
        assert_eq!(&*result, content);
    }

    #[test]
    fn test_remove_vite_plus_lines_absolute_path() {
        let content = "# existing\n. \"/home/user/.vite-plus/env\"\n";
        let result = remove_vite_plus_lines(content);
        assert_eq!(&*result, "# existing\n");
    }

    #[test]
    fn test_remove_vite_plus_lines_preserves_surrounding() {
        let content = "# before\nexport A=1\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n# after\nexport B=2\n";
        let result = remove_vite_plus_lines(content);
        assert_eq!(&*result, "# before\nexport A=1\n# after\nexport B=2\n");
    }

    #[test]
    fn test_clean_affected_profiles_integration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let profile_path = temp_path.join(".zshrc");
        let original = "# my config\nexport FOO=bar\n\n# Vite+ bin (https://viteplus.dev)\n. \"$HOME/.vite-plus/env\"\n";
        std::fs::write(&profile_path, original).unwrap();

        let profiles = vec![AffectedProfile {
            name: Str::from(".zshrc"),
            path: profile_path.clone(),
            kind: AffectedProfileKind::Main { content: Str::from(original) },
        }];
        clean_affected_profiles(&profiles);

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

        // Clear ZDOTDIR/XDG_CONFIG_HOME so the test environment doesn't affect results
        let _guard = ProfileEnvGuard::new(None, None);

        // Main profile with vite-plus line
        std::fs::write(home.join(".zshrc"), ". \"$HOME/.vite-plus/env\"\n").unwrap();
        // Unrelated profile (should be ignored)
        std::fs::write(home.join(".bashrc"), "export PATH=/usr/bin\n").unwrap();
        // Snippet file (just needs to exist)
        let fish_dir = home.join(".config/fish/conf.d");
        std::fs::create_dir_all(&fish_dir).unwrap();
        std::fs::write(fish_dir.join("vite-plus.fish"), "source ~/.vite-plus/env.fish\n").unwrap();

        let profiles = collect_affected_profiles(&home);
        assert_eq!(profiles.len(), 2);
        assert!(matches!(&profiles[0].kind, AffectedProfileKind::Main { .. }));
        assert!(matches!(&profiles[1].kind, AffectedProfileKind::Snippet));
    }

    /// Guard that saves and restores ZDOTDIR and XDG_CONFIG_HOME env vars.
    #[cfg(not(windows))]
    struct ProfileEnvGuard {
        original_zdotdir: Option<std::ffi::OsString>,
        original_xdg_config: Option<std::ffi::OsString>,
    }

    #[cfg(not(windows))]
    impl ProfileEnvGuard {
        fn new(zdotdir: Option<&std::path::Path>, xdg_config: Option<&std::path::Path>) -> Self {
            let guard = Self {
                original_zdotdir: std::env::var_os("ZDOTDIR"),
                original_xdg_config: std::env::var_os("XDG_CONFIG_HOME"),
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

        let _guard = ProfileEnvGuard::new(Some(&zdotdir), None);

        let profiles = collect_affected_profiles(&home);
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

        std::fs::write(fish_dir.join("vite-plus.fish"), "").unwrap();

        let _guard = ProfileEnvGuard::new(None, Some(&xdg_config));

        let profiles = collect_affected_profiles(&home);
        let xdg_profiles: Vec<_> =
            profiles.iter().filter(|p| p.path.as_path().starts_with(&xdg_config)).collect();
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
