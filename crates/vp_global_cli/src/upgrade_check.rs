//! Background upgrade check state for the vp CLI.
//!
//! Shell integrations launch `vp upgrade --background-check` as an OS-native
//! background process. That command records a retry cooldown before touching
//! the network, then queries the npm registry and caches only whether an update
//! is available. Foreground commands only read this cache.

use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use vp_setup::registry;

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const PROMPT_INTERVAL_SECS: u64 = 24 * 60 * 60;
const CACHE_DIR_NAME: &str = "cache";
const CACHE_FILE_NAME: &str = "upgrade-check.json";
const LOCK_FILE_NAME: &str = "upgrade-check.lock";
const UPGRADE_NOTICE: &str = "A new version of vp is available. Run `vp upgrade` to update.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UpgradeCheckStatus {
    Unknown,
    Current,
    Available,
}

#[expect(clippy::disallowed_types)] // String required for serde JSON round-trip
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpgradeCheckCache {
    checked_for: String,
    status: UpgradeCheckStatus,
    checked_at: u64,
    prompted_at: u64,
}

impl UpgradeCheckCache {
    fn needs_check(&self, current_version: &str, now: u64) -> bool {
        self.checked_for != current_version
            || now.saturating_sub(self.checked_at) > CHECK_INTERVAL_SECS
    }

    fn notice_due(&self, current_version: &str, now: u64) -> bool {
        self.checked_for == current_version
            && self.status == UpgradeCheckStatus::Available
            && now.saturating_sub(self.prompted_at) > PROMPT_INTERVAL_SECS
    }
}

struct UpgradeCheckLock {
    _file: File,
    cache_dir: vt_path::AbsolutePathBuf,
    #[expect(clippy::disallowed_types)] // UUID token is persisted in the lock file
    token: String,
}

impl UpgradeCheckLock {
    fn is_current(&self) -> bool {
        std::fs::read_to_string(self.cache_dir.join(LOCK_FILE_NAME).as_path())
            .is_ok_and(|token| token == self.token)
    }

    fn write_cache(&self, cache: &UpgradeCheckCache) -> std::io::Result<()> {
        if !self.is_current() {
            return Err(std::io::ErrorKind::NotFound.into());
        }

        persist_cache(&self.cache_dir, cache)
    }
}

fn cache_dir(install_dir: &vt_path::AbsolutePath) -> vt_path::AbsolutePathBuf {
    install_dir.join(CACHE_DIR_NAME)
}

fn cache_path(install_dir: &vt_path::AbsolutePath) -> vt_path::AbsolutePathBuf {
    cache_dir(install_dir).join(CACHE_FILE_NAME)
}

fn read_cache(install_dir: &vt_path::AbsolutePath) -> Option<UpgradeCheckCache> {
    let data = std::fs::read_to_string(cache_path(install_dir).as_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn persist_cache(
    cache_dir: &vt_path::AbsolutePath,
    cache: &UpgradeCheckCache,
) -> std::io::Result<()> {
    let cache_path = cache_dir.join(CACHE_FILE_NAME);
    let data = serde_json::to_vec(cache).map_err(std::io::Error::other)?;
    let mut temp = tempfile::NamedTempFile::new_in(cache_dir.as_path())?;
    temp.write_all(&data)?;
    temp.as_file().sync_all()?;
    temp.persist(cache_path.as_path()).map_err(|error| error.error)?;
    Ok(())
}

fn try_acquire_lock(install_dir: &vt_path::AbsolutePath) -> Option<UpgradeCheckLock> {
    let cache_dir = cache_dir(install_dir);
    if let Err(error) = std::fs::create_dir(cache_dir.as_path())
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        return None;
    }
    let path = cache_dir.join(LOCK_FILE_NAME);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.as_path())
        .ok()?;
    file.try_lock().ok()?;

    let token = uuid::Uuid::new_v4().to_string();
    file.set_len(0).ok()?;
    file.seek(SeekFrom::Start(0)).ok()?;
    file.write_all(token.as_bytes()).ok()?;
    file.sync_all().ok()?;
    Some(UpgradeCheckLock { _file: file, cache_dir, token })
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn checks_disabled() -> bool {
    std::env::var_os("VP_NO_UPDATE_CHECK").is_some()
        || vp_shared::EnvConfig::get().is_ci
        || std::env::var_os("VP_CLI_TEST").is_some()
}

fn should_check(cache: Option<&UpgradeCheckCache>, current_version: &str, now: u64) -> bool {
    !checks_disabled() && cache.is_none_or(|cache| cache.needs_check(current_version, now))
}

fn read_due_notice(
    install_dir: &vt_path::AbsolutePath,
    current_version: &str,
    now: u64,
) -> Option<UpgradeCheckCache> {
    read_cache(install_dir).filter(|cache| cache.notice_due(current_version, now))
}

fn status_for_versions(current: &str, latest: &str) -> UpgradeCheckStatus {
    if current == "0.0.0" {
        return UpgradeCheckStatus::Current;
    }

    match (node_semver::Version::parse(current), node_semver::Version::parse(latest)) {
        (Ok(current), Ok(latest)) if latest > current => UpgradeCheckStatus::Available,
        (Ok(_), Ok(_)) => UpgradeCheckStatus::Current,
        _ => UpgradeCheckStatus::Unknown,
    }
}

#[expect(clippy::disallowed_types)] // String returned from serde deserialization
async fn resolve_version_string() -> Option<String> {
    registry::resolve_version_string("latest", None).await.ok()
}

/// Refresh the cached update status. This function intentionally runs in the
/// current process; shell integrations decide how to run that process in the
/// background.
pub async fn run_background_check() {
    let Ok(install_dir) = vp_shared::get_vp_home() else {
        return;
    };
    let current_version = env!("CARGO_PKG_VERSION");
    let now = now_secs();

    if !should_check(read_cache(&install_dir).as_ref(), current_version, now) {
        return;
    }

    let Some(lock) = try_acquire_lock(&install_dir) else {
        return;
    };

    // Another process may have refreshed the cache before this process won the
    // lock, so check again while holding it.
    let cache = read_cache(&install_dir);
    let now = now_secs();
    if !should_check(cache.as_ref(), current_version, now) {
        return;
    }

    let prompted_at = cache
        .as_ref()
        .filter(|cache| cache.checked_for == current_version)
        .map_or(0, |cache| cache.prompted_at);
    let pending = UpgradeCheckCache {
        checked_for: current_version.to_owned(),
        status: UpgradeCheckStatus::Unknown,
        checked_at: now,
        prompted_at,
    };

    // Persist the cooldown before the first await. If the shell or OS ends this
    // process during the request, subsequent commands still avoid a retry storm.
    if lock.write_cache(&pending).is_err() {
        return;
    }

    let status = resolve_version_string().await.map_or(UpgradeCheckStatus::Unknown, |latest| {
        status_for_versions(current_version, &latest)
    });
    let completed = UpgradeCheckCache { status, checked_at: now_secs(), ..pending };
    let _ = lock.write_cache(&completed);
}

/// Print a generic one-line upgrade notice from cache and record the prompt time.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn display_cached_upgrade_notice() {
    if checks_disabled() {
        return;
    }

    let Ok(install_dir) = vp_shared::get_vp_home() else {
        return;
    };
    let current_version = env!("CARGO_PKG_VERSION");
    let now = now_secs();
    if read_due_notice(&install_dir, current_version, now).is_none() {
        return;
    }

    let Some(lock) = try_acquire_lock(&install_dir) else {
        return;
    };
    let Some(mut cache) = read_due_notice(&install_dir, current_version, now) else {
        return;
    };

    eprintln!("\n{UPGRADE_NOTICE}");

    cache.prompted_at = now;
    let _ = lock.write_cache(&cache);
}

/// Whether a foreground command may display a cached upgrade notice.
/// Returns `false` for commands excluded by design, quiet modes, and
/// machine-readable output flags (--silent, -s, --json, --parseable, --format json).
pub fn should_display_for_command(args: &crate::cli::Args) -> bool {
    if !cfg!(test) && !vp_shared::is_stderr_terminal() {
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
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();

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
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        assert!(read_cache(&dir_path).is_none());
    }

    #[test]
    fn read_cache_returns_none_for_corrupt_file() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
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
