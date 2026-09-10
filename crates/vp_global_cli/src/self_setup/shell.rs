//! Persist shell entrypoints using the same profiles that env doctor and implode inspect.

use std::io::Write;

use vp_shared::EnvConfig;

use crate::{
    commands::{
        env::setup,
        shell::{ALL_SHELL_PROFILES, ShellProfileKind, ShellProfileRoot, resolve_profile_path},
    },
    error::Error,
};

pub(super) async fn configure() -> Result<(), Error> {
    let config = EnvConfig::get();
    for profile in ALL_SHELL_PROFILES {
        let shell = match profile.root {
            ShellProfileRoot::Zsh => "zsh",
            ShellProfileRoot::Home => "bash",
            ShellProfileRoot::Fish => "fish",
            ShellProfileRoot::NushellConfig => continue,
            ShellProfileRoot::NushellData => "nu",
        };
        if super::find_on_path(shell).is_none() {
            continue;
        }
        // Fish and Nushell use managed snippets; never rewrite the user's main config.
        if shell == "fish" && matches!(profile.kind, ShellProfileKind::Main) {
            continue;
        }
        let mut path = resolve_profile_path(profile, &config.user_home);
        if shell == "nu" {
            let result = tokio::process::Command::new("nu")
                .args(["-c", "$nu.vendor-autoload-dirs | last"])
                .output()
                .await?;
            if !result.status.success() {
                return Err(Error::Other(
                    "Could not determine Nushell vendor autoload directory".into(),
                ));
            }
            let directory = String::from_utf8_lossy(&result.stdout).trim().to_string();
            path = vt_path::AbsolutePathBuf::new(directory.into())
                .ok_or_else(|| {
                    Error::Other("Nushell returned a non-absolute autoload directory".into())
                })?
                .join("vite-plus.nu");
        }
        let env = config.dirs.config.join(profile.env_file).to_string();
        let escaped = match shell {
            "fish" => setup::escape_fish_double_quoted_string(&env),
            "nu" => setup::escape_nu_double_quoted_string(&env),
            _ => setup::escape_posix_double_quoted_string(&env),
        };
        let source = if shell == "bash" || shell == "zsh" { "." } else { "source" };
        let line = format!("{source} \"{escaped}\"");
        let content = format!("# Vite+ bin (https://viteplus.dev)\n{line}\n");
        match profile.kind {
            ShellProfileKind::Snippet => {
                let parent = path.parent().ok_or(Error::CliBinaryNotFound)?;
                tokio::fs::create_dir_all(parent).await?;
                tokio::fs::write(&path, content).await?;
            }
            ShellProfileKind::Main => {
                if !path.as_path().exists() && profile.path != ".zshenv" {
                    continue;
                }
                let existing = match tokio::fs::read_to_string(&path).await {
                    Ok(content) => content,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(error) => return Err(error.into()),
                };
                let relative = config
                    .dirs
                    .config
                    .as_path()
                    .strip_prefix(config.user_home.as_path())
                    .ok()
                    .map(|suffix| format!("$HOME/{}/{}", suffix.display(), profile.env_file));
                if existing.contains(&env)
                    || relative.is_some_and(|reference| existing.contains(&reference))
                {
                    continue;
                }
                let parent = path.parent().ok_or(Error::CliBinaryNotFound)?;
                tokio::fs::create_dir_all(parent).await?;
                let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
                write!(file, "\n{content}")?;
            }
        }
    }
    #[cfg(windows)]
    {
        let bin = setup::escape_powershell_single_quoted_string(&config.dirs.bin.to_string());
        let script = format!(
            "$bin = '{bin}'; $path = [Environment]::GetEnvironmentVariable('Path', 'User'); if (($path -split ';') -notcontains $bin) {{ [Environment]::SetEnvironmentVariable('Path', ($bin + ';' + $path), 'User') }}"
        );
        let result = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .await?;
        if !result.status.success() {
            return Err(Error::Other(
                format!(
                    "Could not configure user PATH: {}",
                    String::from_utf8_lossy(&result.stderr)
                )
                .into(),
            ));
        }
    }
    Ok(())
}
