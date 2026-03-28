use std::process::Output;

use thiserror::Error;
use tokio::process::Command;
use vite_path::AbsolutePath;

#[derive(Debug, Error)]
pub enum GitError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Command(String),
}

pub async fn capture_git<I, S>(cwd: &AbsolutePath, args: I) -> Result<String, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let output = Command::new("git").args(&args).current_dir(cwd.as_path()).output().await?;

    if !output.status.success() {
        return Err(command_error(&args, &output));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub async fn run_git<I, S>(cwd: &AbsolutePath, args: I) -> Result<(), GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let output = Command::new("git").args(&args).current_dir(cwd.as_path()).output().await?;

    if output.status.success() {
        return Ok(());
    }

    Err(command_error(&args, &output))
}

pub async fn is_clean_git_worktree(cwd: &AbsolutePath) -> Result<bool, GitError> {
    Ok(capture_git(cwd, ["status", "--porcelain"]).await?.trim().is_empty())
}

#[must_use]
pub fn parse_github_repo_slug(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches('/');
    let path = if let Some(path) = url.strip_prefix("github:") {
        path
    } else if let Some((_, path)) = url.split_once("github.com/") {
        path
    } else if let Some((_, path)) = url.rsplit_once("github.com:") {
        path
    } else {
        return None;
    };

    let path = path.trim_start_matches('/').strip_suffix(".git").unwrap_or(path);
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    let owner = segments.next()?;
    let repo = segments.next()?;
    Some(format!("{owner}/{repo}"))
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().map(|arg| arg.as_ref().to_owned()).collect()
}

fn command_error(args: &[String], output: &Output) -> GitError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let message = if stderr.is_empty() {
        format!("`git {}` failed", args.join(" "))
    } else {
        format!("`git {}` failed: {}", args.join(" "), stderr)
    };
    GitError::Command(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_github_repo_slug_supports_common_remote_formats() {
        assert_eq!(
            parse_github_repo_slug("git@github.com:voidzero-dev/vite-plus.git"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(
            parse_github_repo_slug("https://github.com/voidzero-dev/vite-plus.git"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(
            parse_github_repo_slug("github:voidzero-dev/vite-plus"),
            Some("voidzero-dev/vite-plus".into())
        );
        assert_eq!(parse_github_repo_slug("https://example.com/acme/repo.git"), None);
    }
}
