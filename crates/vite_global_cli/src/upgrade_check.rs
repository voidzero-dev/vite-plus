//! Background upgrade check for the vp CLI.
//!
//! Periodically queries the npm registry for the latest version and caches the
//! result to `~/.vite-plus/.upgrade-check.json`. Displays a one-line notice on
//! stderr when a newer version is available.

use std::{
    io::IsTerminal,
    time::{SystemTime, UNIX_EPOCH},
};

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use vite_install::{config::npm_registry, request::HttpClient};

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const CACHE_FILE_NAME: &str = ".upgrade-check.json";

#[expect(clippy::disallowed_types)] // String required for serde JSON round-trip
#[derive(Debug, Serialize, Deserialize)]
struct UpgradeCheckCache {
    latest: String,
    checked_at: u64,
}

#[expect(clippy::disallowed_types)] // String required for serde deserialization
#[derive(Deserialize)]
struct VersionOnly {
    version: String,
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
        || std::env::var_os("VITE_PLUS_CLI_TEST").is_some()
    {
        return false;
    }

    cache.is_none_or(|c| now.saturating_sub(c.checked_at) > CHECK_INTERVAL_SECS)
}

#[expect(clippy::disallowed_types)] // String returned from serde deserialization
async fn resolve_latest_version() -> Option<String> {
    let registry_raw = npm_registry();
    let registry = registry_raw.trim_end_matches('/');
    let url = vite_str::format!("{registry}/vite-plus/latest");
    let client = HttpClient::new();
    let meta: VersionOnly = client.get_json(&url).await.ok()?;
    Some(meta.version)
}

/// Returns the latest version string if it differs from the current version,
/// or `None` if up to date / check disabled / network error.
#[expect(clippy::disallowed_types)] // String returned to caller for display
pub async fn check_for_update() -> Option<String> {
    let install_dir = vite_shared::get_vite_plus_home().ok()?;
    let current_version = env!("CARGO_PKG_VERSION");
    let cache = read_cache(&install_dir);
    let now = now_secs();

    if !should_check(cache.as_ref(), now) {
        return cache.filter(|c| c.latest != current_version).map(|c| c.latest);
    }

    let latest = resolve_latest_version().await?;
    write_cache(&install_dir, &UpgradeCheckCache { latest: latest.clone(), checked_at: now });
    (latest != current_version).then_some(latest)
}

/// Print a one-line upgrade notice to stderr.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn display_upgrade_notice(new_version: &str) {
    let current_version = env!("CARGO_PKG_VERSION");
    if !std::io::stderr().is_terminal() {
        return;
    }
    eprintln!(
        "\n{} {} {} {}, run {}",
        "vp update available:".bright_black(),
        current_version.bright_black(),
        "\u{2192}".bright_black(),
        new_version.green().bold(),
        "`vp upgrade`".bright_black().bold(),
    );
}

/// Whether the upgrade check should run for the given command args.
/// Returns `false` for commands excluded by design (upgrade, implode, --version)
/// and for any command invoked with `--silent` or `--json`.
pub fn should_run_for_command(args: &crate::cli::Args, raw_args: &[String]) -> bool {
    if args.version {
        return false;
    }

    if matches!(
        &args.command,
        Some(
            crate::cli::Commands::Upgrade { .. }
                | crate::cli::Commands::Implode { .. }
                | crate::cli::Commands::Lint { .. }
                | crate::cli::Commands::Fmt { .. }
        )
    ) {
        return false;
    }

    // Suppress for --silent and --json flags (before -- terminator)
    for arg in raw_args {
        if arg == "--" {
            break;
        }
        if arg == "--silent" || arg == "--json" {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();

        let cache = UpgradeCheckCache { latest: "1.2.3".to_owned(), checked_at: 1000 };
        write_cache(&dir_path, &cache);

        let loaded = read_cache(&dir_path).expect("should read back cache");
        assert_eq!(loaded.latest, "1.2.3");
        assert_eq!(loaded.checked_at, 1000);
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
        let test = std::env::var_os("VITE_PLUS_CLI_TEST");
        let no_check = std::env::var_os("VP_NO_UPDATE_CHECK");
        unsafe {
            std::env::remove_var("CI");
            std::env::remove_var("VITE_PLUS_CLI_TEST");
            std::env::remove_var("VP_NO_UPDATE_CHECK");
        }

        f();

        unsafe {
            if let Some(v) = ci {
                std::env::set_var("CI", v);
            }
            if let Some(v) = test {
                std::env::set_var("VITE_PLUS_CLI_TEST", v);
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
            let cache = UpgradeCheckCache { latest: "1.0.0".to_owned(), checked_at: now };
            assert!(!should_check(Some(&cache), now));
        });
    }

    #[test]
    #[serial]
    fn should_check_returns_true_when_cache_stale() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let stale_time = now - CHECK_INTERVAL_SECS - 1;
            let cache = UpgradeCheckCache { latest: "1.0.0".to_owned(), checked_at: stale_time };
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

    fn parse_args(args: &[&str]) -> crate::cli::Args {
        let full: Vec<String> =
            std::iter::once("vp").chain(args.iter().copied()).map(String::from).collect();
        crate::try_parse_args_from(full).unwrap()
    }

    fn raw_args(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| String::from(*s)).collect()
    }

    #[test]
    fn should_run_for_normal_command() {
        let args = parse_args(&["build"]);
        assert!(should_run_for_command(&args, &raw_args(&["build"])));
    }

    #[test]
    fn should_not_run_for_upgrade() {
        let args = parse_args(&["upgrade"]);
        assert!(!should_run_for_command(&args, &raw_args(&["upgrade"])));
    }

    #[test]
    fn should_not_run_for_silent_flag() {
        let args = parse_args(&["install", "--silent"]);
        assert!(!should_run_for_command(&args, &raw_args(&["install", "--silent"])));
    }

    #[test]
    fn should_not_run_for_json_flag() {
        let args = parse_args(&["why", "lodash", "--json"]);
        assert!(!should_run_for_command(&args, &raw_args(&["why", "lodash", "--json"])));
    }

    #[test]
    fn should_run_when_json_after_terminator() {
        let args = parse_args(&["build"]);
        assert!(should_run_for_command(&args, &raw_args(&["build", "--", "--json"])));
    }
}
