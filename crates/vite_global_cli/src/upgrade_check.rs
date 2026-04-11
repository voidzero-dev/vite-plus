//! Background upgrade check for the vp CLI.
//!
//! Periodically queries the npm registry for the latest version and caches the
//! result to `~/.vite-plus/.upgrade-check.json`. Displays a one-line notice on
//! stderr when a newer version is available, at most once per 24 hours.

use std::{
    io::IsTerminal,
    time::{SystemTime, UNIX_EPOCH},
};

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

use crate::commands::upgrade::registry;

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const PROMPT_INTERVAL_SECS: u64 = 24 * 60 * 60;
const CACHE_FILE_NAME: &str = ".upgrade-check.json";

#[expect(clippy::disallowed_types)] // String required for serde JSON round-trip
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpgradeCheckCache {
    latest: String,
    checked_at: u64,
    prompted_at: u64,
}

fn read_cache(install_dir: &vite_path::AbsolutePath) -> Option<UpgradeCheckCache> {
    let cache_path = install_dir.join(CACHE_FILE_NAME);
    let data = std::fs::read_to_string(cache_path.as_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(install_dir: &vite_path::AbsolutePath, cache: &UpgradeCheckCache) {
    let cache_path = install_dir.join(CACHE_FILE_NAME);
    if let Ok(data) = serde_json::to_string(cache) {
        let _ = std::fs::write(cache_path.as_path(), &data);
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn should_check(cache: Option<&UpgradeCheckCache>, now: u64) -> bool {
    if std::env::var_os("VP_NO_UPDATE_CHECK").is_some()
        || std::env::var_os("CI").is_some()
        || std::env::var_os("VP_CLI_TEST").is_some()
    {
        return false;
    }

    cache.is_none_or(|c| now.saturating_sub(c.checked_at) > CHECK_INTERVAL_SECS)
}

fn should_prompt(cache: Option<&UpgradeCheckCache>, now: u64) -> bool {
    cache.is_none_or(|c| now.saturating_sub(c.prompted_at) > PROMPT_INTERVAL_SECS)
}

/// Returns `true` if `latest` is strictly newer than `current` per semver.
/// Returns `false` for equal versions, downgrades, or unparsable strings.
fn is_newer_version(current: &str, latest: &str) -> bool {
    if latest.is_empty() || current == "0.0.0" {
        return false;
    }
    match (node_semver::Version::parse(current), node_semver::Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => false,
    }
}

#[expect(clippy::disallowed_types)] // String returned from serde deserialization
async fn resolve_version_string() -> Option<String> {
    registry::resolve_version_string("latest", None).await.ok()
}

pub struct UpgradeCheckResult {
    install_dir: vite_path::AbsolutePathBuf,
    cache: UpgradeCheckCache,
}

/// Returns an upgrade check result if a newer version is available and the user
/// hasn't been prompted within the last 24 hours. Returns `None` otherwise.
pub async fn check_for_update() -> Option<UpgradeCheckResult> {
    let install_dir = vite_shared::get_vp_home().ok()?;
    let current_version = env!("CARGO_PKG_VERSION");
    let now = now_secs();
    let mut cache = read_cache(&install_dir);

    if should_check(cache.as_ref(), now) {
        let prompted_at = cache.as_ref().map_or(0, |c| c.prompted_at);

        match resolve_version_string().await {
            Some(latest) => {
                let new_cache = UpgradeCheckCache { latest, checked_at: now, prompted_at };
                write_cache(&install_dir, &new_cache);
                cache = Some(new_cache);
            }
            None => {
                // Still update checked_at so we back off for 24h instead of
                // retrying on every command when the registry is unreachable.
                let latest = cache.as_ref().map(|c| c.latest.clone()).unwrap_or_default();
                let failed_cache = UpgradeCheckCache { latest, checked_at: now, prompted_at };
                write_cache(&install_dir, &failed_cache);
                cache = Some(failed_cache);
            }
        }
    }

    let cache = cache?;

    if !is_newer_version(current_version, &cache.latest) {
        return None;
    }

    if !should_prompt(Some(&cache), now) {
        return None;
    }

    Some(UpgradeCheckResult { install_dir, cache })
}

/// Print a one-line upgrade notice to stderr and record the prompt time.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn display_upgrade_notice(result: &UpgradeCheckResult) {
    let current_version = env!("CARGO_PKG_VERSION");
    eprintln!(
        "\n{} {} {} {}{} {}",
        "vp update available:".bright_black(),
        current_version.bright_black(),
        "\u{2192}".bright_black(),
        result.cache.latest.bright_green().bold(),
        ", run".bright_black(),
        "vp upgrade".bright_green().bold(),
    );

    let mut cache = result.cache.clone();
    cache.prompted_at = now_secs();
    write_cache(&result.install_dir, &cache);
}

/// Whether the upgrade check should run for the given command args.
/// Returns `false` for commands excluded by design, quiet modes, and
/// machine-readable output flags (--silent, -s, --json, --parseable, --format json).
pub fn should_run_for_command(args: &crate::cli::Args) -> bool {
    if !cfg!(test) && !std::io::stderr().is_terminal() {
        return false;
    }

    if args.version {
        return false;
    }

    match &args.command {
        Some(
            crate::cli::Commands::Upgrade { .. }
            | crate::cli::Commands::Implode { .. }
            | crate::cli::Commands::Lint { .. }
            | crate::cli::Commands::Fmt { .. },
        ) => false,
        Some(cmd) => !cmd.is_quiet_or_machine_readable(),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();

        let cache =
            UpgradeCheckCache { latest: "1.2.3".to_owned(), checked_at: 1000, prompted_at: 900 };
        write_cache(&dir_path, &cache);

        let loaded = read_cache(&dir_path).expect("should read back cache");
        assert_eq!(loaded.latest, "1.2.3");
        assert_eq!(loaded.checked_at, 1000);
        assert_eq!(loaded.prompted_at, 900);
    }

    #[test]
    fn read_cache_returns_none_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        assert!(read_cache(&dir_path).is_none());
    }

    #[test]
    fn read_cache_returns_none_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir_path.join(CACHE_FILE_NAME).as_path(), "not json").unwrap();
        assert!(read_cache(&dir_path).is_none());
    }

    fn with_env_vars_cleared<F: FnOnce()>(f: F) {
        let ci = std::env::var_os("CI");
        let test = std::env::var_os("VP_CLI_TEST");
        let no_check = std::env::var_os("VP_NO_UPDATE_CHECK");
        unsafe {
            std::env::remove_var("CI");
            std::env::remove_var("VP_CLI_TEST");
            std::env::remove_var("VP_NO_UPDATE_CHECK");
        }

        f();

        unsafe {
            if let Some(v) = ci {
                std::env::set_var("CI", v);
            }
            if let Some(v) = test {
                std::env::set_var("VP_CLI_TEST", v);
            }
            if let Some(v) = no_check {
                std::env::set_var("VP_NO_UPDATE_CHECK", v);
            }
        }
    }

    #[test]
    #[serial]
    fn should_check_returns_true_when_no_cache() {
        with_env_vars_cleared(|| {
            assert!(should_check(None, now_secs()));
        });
    }

    #[test]
    #[serial]
    fn should_check_returns_false_when_cache_fresh() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let cache =
                UpgradeCheckCache { latest: "1.0.0".to_owned(), checked_at: now, prompted_at: 0 };
            assert!(!should_check(Some(&cache), now));
        });
    }

    #[test]
    #[serial]
    fn should_check_returns_true_when_cache_stale() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let stale_time = now - CHECK_INTERVAL_SECS - 1;
            let cache = UpgradeCheckCache {
                latest: "1.0.0".to_owned(),
                checked_at: stale_time,
                prompted_at: 0,
            };
            assert!(should_check(Some(&cache), now));
        });
    }

    #[test]
    #[serial]
    fn should_check_returns_false_when_disabled() {
        with_env_vars_cleared(|| {
            unsafe {
                std::env::set_var("VP_NO_UPDATE_CHECK", "1");
            }
            assert!(!should_check(None, now_secs()));
        });
    }

    #[test]
    fn should_prompt_returns_true_when_no_cache() {
        assert!(should_prompt(None, now_secs()));
    }

    #[test]
    fn should_prompt_returns_true_when_never_prompted() {
        let cache = UpgradeCheckCache {
            latest: "2.0.0".to_owned(),
            checked_at: now_secs(),
            prompted_at: 0,
        };
        assert!(should_prompt(Some(&cache), now_secs()));
    }

    #[test]
    fn should_prompt_returns_false_when_recently_prompted() {
        let now = now_secs();
        let cache =
            UpgradeCheckCache { latest: "2.0.0".to_owned(), checked_at: now, prompted_at: now };
        assert!(!should_prompt(Some(&cache), now));
    }

    #[test]
    fn should_prompt_returns_true_when_prompt_stale() {
        let now = now_secs();
        let stale = now - PROMPT_INTERVAL_SECS - 1;
        let cache =
            UpgradeCheckCache { latest: "2.0.0".to_owned(), checked_at: now, prompted_at: stale };
        assert!(should_prompt(Some(&cache), now));
    }

    #[test]
    fn is_newer_version_detects_upgrade() {
        assert!(is_newer_version("0.1.0", "0.2.0"));
        assert!(is_newer_version("0.1.0", "1.0.0"));
        assert!(is_newer_version("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_version_rejects_same() {
        assert!(!is_newer_version("0.2.0", "0.2.0"));
    }

    #[test]
    fn is_newer_version_rejects_downgrade() {
        assert!(!is_newer_version("0.2.0", "0.1.0"));
    }

    #[test]
    fn is_newer_version_rejects_prerelease_downgrade_to_stable() {
        // User on alpha, latest stable is older — don't prompt
        assert!(!is_newer_version("0.3.0-alpha.1", "0.2.0"));
    }

    #[test]
    fn is_newer_version_prompts_prerelease_to_newer_stable() {
        assert!(is_newer_version("0.1.0-alpha.1", "0.2.0"));
    }

    #[test]
    fn is_newer_version_prompts_prerelease_to_same_base_release() {
        // 1.0.0 is newer than 1.0.0-alpha.1 per semver
        assert!(is_newer_version("1.0.0-alpha.1", "1.0.0"));
    }

    #[test]
    fn is_newer_version_rejects_empty_latest() {
        assert!(!is_newer_version("0.1.0", ""));
    }

    #[test]
    fn is_newer_version_skips_dev_build() {
        assert!(!is_newer_version("0.0.0", "0.2.0"));
    }

    #[test]
    fn is_newer_version_rejects_invalid_versions() {
        assert!(!is_newer_version("not-a-version", "0.2.0"));
        assert!(!is_newer_version("0.1.0", "not-a-version"));
    }

    fn parse_args(args: &[&str]) -> crate::cli::Args {
        let full: Vec<String> =
            std::iter::once("vp").chain(args.iter().copied()).map(String::from).collect();
        crate::try_parse_args_from(full).unwrap()
    }

    #[test]
    fn should_run_for_normal_command() {
        assert!(should_run_for_command(&parse_args(&["build"])));
    }

    #[test]
    fn should_not_run_for_upgrade() {
        assert!(!should_run_for_command(&parse_args(&["upgrade"])));
    }

    #[test]
    fn should_not_run_for_install_silent() {
        assert!(!should_run_for_command(&parse_args(&["install", "--silent"])));
    }

    #[test]
    fn should_not_run_for_dlx_short_silent() {
        assert!(!should_run_for_command(&parse_args(&["dlx", "-s", "pkg"])));
    }

    #[test]
    fn should_not_run_for_why_json() {
        assert!(!should_run_for_command(&parse_args(&["why", "lodash", "--json"])));
    }

    #[test]
    fn should_not_run_for_why_parseable() {
        assert!(!should_run_for_command(&parse_args(&["why", "lodash", "--parseable"])));
    }

    #[test]
    fn should_not_run_for_outdated_format_json() {
        assert!(!should_run_for_command(&parse_args(&["outdated", "--format", "json"])));
    }

    #[test]
    fn should_not_run_for_pm_list_parseable() {
        assert!(!should_run_for_command(&parse_args(&["pm", "list", "--parseable"])));
    }

    #[test]
    fn should_not_run_for_pm_list_json() {
        assert!(!should_run_for_command(&parse_args(&["pm", "list", "--json"])));
    }

    #[test]
    fn should_not_run_for_env_current_json() {
        assert!(!should_run_for_command(&parse_args(&["env", "current", "--json"])));
    }

    #[test]
    fn should_run_for_outdated_without_format() {
        assert!(should_run_for_command(&parse_args(&["outdated"])));
    }
}
