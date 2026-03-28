//! Shared git helpers used by release planning and repository introspection.
//!
//! These helpers intentionally use `std::process::Command` rather than an async runtime. The
//! commands here are short-lived process invocations in already-async callsites, so keeping the
//! shared utility crate synchronous avoids forcing a heavier dependency such as `tokio` onto every
//! consumer of `vite_shared`.

use std::{
    process::{Command, Output},
    string::String,
};

use thiserror::Error;
use vite_path::AbsolutePath;

/// Error raised while invoking git commands or interpreting their result.
#[derive(Debug, Error)]
pub enum GitError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Command(String),
}

/// Runs `git` and returns trimmed stdout on success.
///
/// The helper collects command arguments once up front so the same owned argument list can be used
/// for both process execution and rich error reporting.
pub fn capture_git<I, S>(cwd: &AbsolutePath, args: I) -> Result<String, GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let output = Command::new("git").args(&args).current_dir(cwd.as_path()).output()?;

    if !output.status.success() {
        return Err(command_error(&args, &output));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Runs `git` and returns only success or a structured command error.
pub fn run_git<I, S>(cwd: &AbsolutePath, args: I) -> Result<(), GitError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = collect_args(args);
    let output = Command::new("git").args(&args).current_dir(cwd.as_path()).output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(command_error(&args, &output))
}

/// Returns whether the current worktree has no staged or unstaged changes.
pub fn is_clean_git_worktree(cwd: &AbsolutePath) -> Result<bool, GitError> {
    Ok(capture_git(cwd, ["status", "--porcelain"])?.trim().is_empty())
}

/// Extracts an `owner/repo` slug from common GitHub remote URL formats.
///
/// # Examples
///
/// ```rust
/// use vite_shared::parse_github_repo_slug;
///
/// assert_eq!(
///     parse_github_repo_slug("git@github.com:voidzero-dev/vite-plus.git"),
///     Some("voidzero-dev/vite-plus".into()),
/// );
/// assert_eq!(
///     parse_github_repo_slug("https://github.com/voidzero-dev/vite-plus"),
///     Some("voidzero-dev/vite-plus".into()),
/// );
/// ```
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
    let mut slug = String::with_capacity(owner.len() + repo.len() + 1);
    slug.push_str(owner);
    slug.push('/');
    slug.push_str(repo);
    Some(slug)
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // Preserve one owned argument buffer for both process spawning and later error reporting.
    let iter = args.into_iter();
    let (lower, _) = iter.size_hint();
    let mut collected = Vec::with_capacity(lower);
    for arg in iter {
        collected.push(arg.as_ref().to_owned());
    }
    collected
}

fn command_error(args: &[String], output: &Output) -> GitError {
    // Construct the git invocation string directly rather than joining/formatting multiple
    // temporaries, since this path is also used by release dry-runs and failure reporting.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    let args_len: usize = args.iter().map(String::len).sum();
    let mut message = String::with_capacity(args_len + stderr.len() + 16 + args.len());
    message.push_str("`git");
    for arg in args {
        message.push(' ');
        message.push_str(arg);
    }
    message.push_str("` failed");
    if !stderr.is_empty() {
        message.push_str(": ");
        message.push_str(stderr);
    }
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
