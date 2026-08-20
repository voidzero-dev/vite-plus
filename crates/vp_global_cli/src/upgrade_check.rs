//! Background upgrade check state for the vp CLI.
//!
//! Eligible foreground commands launch `vp upgrade --background-check` as a
//! detached process when the cache is stale. That command records a retry
//! cooldown before touching the network, then queries the npm registry and
//! caches the discovered version under the configured cache root.

use std::{
    fs::{File, OpenOptions},
    io::Write,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};
use vp_setup::registry;

const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
const PROMPT_INTERVAL_SECS: u64 = 24 * 60 * 60;
const CACHE_FILE_NAME: &str = "upgrade-check.json";
const LOCK_FILE_NAME: &str = "upgrade-check.lock";

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
    latest: String,
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
    identity: same_file::Handle,
}

impl UpgradeCheckLock {
    fn is_current(&self) -> bool {
        same_file::Handle::from_path(self.cache_dir.join(LOCK_FILE_NAME).as_path())
            .is_ok_and(|identity| identity == self.identity)
    }

    fn write_cache(&self, cache: &UpgradeCheckCache) -> std::io::Result<()> {
        if !self.is_current() {
            return Err(std::io::ErrorKind::NotFound.into());
        }

        persist_cache(&self.cache_dir, cache)
    }
}

fn cache_path(cache_dir: &vt_path::AbsolutePath) -> vt_path::AbsolutePathBuf {
    cache_dir.join(CACHE_FILE_NAME)
}

fn read_cache(cache_dir: &vt_path::AbsolutePath) -> Option<UpgradeCheckCache> {
    let data = std::fs::read_to_string(cache_path(cache_dir).as_path()).ok()?;
    serde_json::from_str(&data).ok()
}

#[cfg(test)]
fn write_cache(
    cache_dir: &vt_path::AbsolutePath,
    cache: &UpgradeCheckCache,
) -> std::io::Result<()> {
    std::fs::create_dir_all(cache_dir.as_path())?;
    persist_cache(cache_dir, cache)
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

fn try_acquire_lock(
    cache_dir: &vt_path::AbsolutePath,
    data_dir: &vt_path::AbsolutePath,
) -> Option<UpgradeCheckLock> {
    let create_result = if cache_dir.as_path().starts_with(data_dir.as_path()) {
        std::fs::create_dir(cache_dir.as_path())
    } else {
        std::fs::create_dir_all(cache_dir.as_path())
    };
    if let Err(error) = create_result
        && error.kind() != std::io::ErrorKind::AlreadyExists
    {
        return None;
    }
    let path = cache_dir.join(LOCK_FILE_NAME);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.as_path())
        .ok()?;
    let identity = same_file::Handle::from_file(file.try_clone().ok()?).ok()?;
    file.try_lock().ok()?;
    Some(UpgradeCheckLock { _file: file, cache_dir: cache_dir.to_absolute_path_buf(), identity })
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
    cache_dir: &vt_path::AbsolutePath,
    current_version: &str,
    now: u64,
) -> Option<UpgradeCheckCache> {
    read_cache(cache_dir).filter(|cache| cache.notice_due(current_version, now))
}

/// Returns `true` if `latest` is strictly newer than `current` per semver.
/// Returns `false` for equal versions, downgrades, or unparsable strings.
#[cfg(test)]
fn is_newer_version(current: &str, latest: &str) -> bool {
    status_for_versions(current, latest) == UpgradeCheckStatus::Available
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

pub(crate) fn spawn_background_check_if_needed() -> bool {
    let config = vp_shared::EnvConfig::get();
    let cache_dir = &config.dirs.cache;
    let current_version = env!("CARGO_PKG_VERSION");
    if !should_check(read_cache(cache_dir).as_ref(), current_version, now_secs()) {
        return false;
    }

    let Ok(current_exe) = std::env::current_exe() else {
        return false;
    };
    let mut command = Command::new(current_exe);
    command
        .args(["upgrade", "--background-check"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_background_process(&mut command);

    let Ok(mut child) = command.spawn() else {
        return false;
    };
    // A long-running foreground command must still reap a helper that exits first.
    let _ = std::thread::spawn(move || child.wait());
    true
}

fn configure_background_process(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;

        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;

        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }

    #[cfg(not(any(unix, windows)))]
    let _ = command;
}

/// Refresh the cached update status. This function intentionally runs in the
/// helper process so the foreground command never waits for registry I/O.
pub async fn run_background_check() {
    let config = vp_shared::EnvConfig::get();
    let cache_dir = &config.dirs.cache;
    let data_dir = &config.dirs.data;
    let current_version = env!("CARGO_PKG_VERSION");
    let now = now_secs();

    if !should_check(read_cache(cache_dir).as_ref(), current_version, now) {
        return;
    }

    let Some(lock) = try_acquire_lock(cache_dir, data_dir) else {
        return;
    };

    // Another process may have refreshed the cache before this process won the
    // lock, so check again while holding it.
    let cache = read_cache(cache_dir);
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
        latest: cache
            .as_ref()
            .filter(|cache| cache.checked_for == current_version)
            .map_or_else(String::new, |cache| cache.latest.clone()),
        status: UpgradeCheckStatus::Unknown,
        checked_at: now,
        prompted_at,
    };

    // Persist the cooldown before the first await. If the shell or OS ends this
    // process during the request, subsequent commands still avoid a retry storm.
    if lock.write_cache(&pending).is_err() {
        return;
    }

    let completed = match resolve_version_string().await {
        Some(latest) => UpgradeCheckCache {
            status: status_for_versions(current_version, &latest),
            latest,
            checked_at: now_secs(),
            ..pending
        },
        None => UpgradeCheckCache { checked_at: now_secs(), ..pending },
    };
    let _ = lock.write_cache(&completed);
}

/// Print a one-line upgrade notice from cache and record the prompt time.
#[expect(clippy::print_stderr, clippy::disallowed_macros)]
pub fn display_cached_upgrade_notice() {
    if checks_disabled() {
        return;
    }

    let config = vp_shared::EnvConfig::get();
    let cache_dir = &config.dirs.cache;
    let data_dir = &config.dirs.data;
    let current_version = env!("CARGO_PKG_VERSION");
    let now = now_secs();
    if read_due_notice(cache_dir, current_version, now).is_none() {
        return;
    }

    let Some(lock) = try_acquire_lock(cache_dir, data_dir) else {
        return;
    };
    let Some(mut cache) = read_due_notice(cache_dir, current_version, now) else {
        return;
    };

    eprintln!(
        "\n{} {} {} {}{} {}",
        "vp update available:".bright_black(),
        current_version.bright_black(),
        "\u{2192}".bright_black(),
        cache.latest.bright_green().bold(),
        ", run".bright_black(),
        "vp upgrade".bright_green().bold(),
    );

    cache.prompted_at = now;
    let _ = lock.write_cache(&cache);
}

/// Whether a foreground command may run the upgrade check and display its cached notice.
/// Returns `false` for commands excluded by design, quiet modes, and
/// machine-readable output flags (--silent, -s, --json, --parseable, --format json).
pub fn should_run_for_command(args: &crate::cli::Args) -> bool {
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
    use std::{
        ffi::OsStr,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();

        let cache = UpgradeCheckCache {
            checked_for: "1.2.3".to_owned(),
            latest: "2.0.0".to_owned(),
            status: UpgradeCheckStatus::Available,
            checked_at: 1000,
            prompted_at: 900,
        };
        write_cache(&dir_path, &cache).unwrap();

        let loaded = read_cache(&dir_path).expect("should read back cache");
        let expected_path = dir_path.join(CACHE_FILE_NAME);
        assert_eq!(cache_path(&dir_path), expected_path);
        assert!(expected_path.as_path().exists());
        assert_eq!(loaded.checked_for, "1.2.3");
        assert_eq!(loaded.latest, "2.0.0");
        assert_eq!(loaded.status, UpgradeCheckStatus::Available);
        assert_eq!(loaded.checked_at, 1000);
        assert_eq!(loaded.prompted_at, 900);
    }

    #[test]
    fn cache_write_atomically_replaces_existing_state() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        let mut cache = UpgradeCheckCache {
            checked_for: "1.2.3".to_owned(),
            latest: String::new(),
            status: UpgradeCheckStatus::Unknown,
            checked_at: 1000,
            prompted_at: 0,
        };
        write_cache(&dir_path, &cache).unwrap();

        cache.status = UpgradeCheckStatus::Available;
        cache.latest = "2.0.0".to_owned();
        cache.checked_at = 2000;
        write_cache(&dir_path, &cache).unwrap();

        let loaded = read_cache(&dir_path).unwrap();
        assert_eq!(loaded.status, UpgradeCheckStatus::Available);
        assert_eq!(loaded.latest, "2.0.0");
        assert_eq!(loaded.checked_at, 2000);
    }

    #[test]
    fn lock_creates_missing_split_cache_parent() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir =
            vt_path::AbsolutePathBuf::new(dir.path().join("missing").join("cache")).unwrap();
        let data_dir = vt_path::AbsolutePathBuf::new(dir.path().join("data")).unwrap();
        assert!(!cache_dir.as_path().exists());

        try_acquire_lock(&cache_dir, &data_dir).expect("should create split cache parents");

        assert!(cache_dir.as_path().is_dir());
    }

    #[test]
    fn lock_is_exclusive_and_released_by_its_owner() {
        let dir = tempfile::tempdir().unwrap();
        let dir_path = vt_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();

        let lock =
            try_acquire_lock(&dir_path, &dir_path).expect("first process should acquire the lock");
        assert!(
            try_acquire_lock(&dir_path, &dir_path).is_none(),
            "second process must not acquire the lock"
        );
        drop(lock);

        try_acquire_lock(&dir_path, &dir_path).expect("lock should be reusable after owner exits");
    }

    #[cfg(unix)]
    #[test]
    fn locked_cache_write_does_not_recreate_a_moved_install() {
        let dir = tempfile::tempdir().unwrap();
        let install_path = dir.path().join("vite-plus");
        std::fs::create_dir(&install_path).unwrap();
        let install_dir = vt_path::AbsolutePathBuf::new(install_path.clone()).unwrap();
        let cache_dir = vt_path::AbsolutePathBuf::new(install_path.join("cache")).unwrap();
        let lock =
            try_acquire_lock(&cache_dir, &install_dir).expect("worker should acquire the lock");
        let moved_path = dir.path().join("vite-plus.removing");
        std::fs::rename(&install_path, &moved_path).unwrap();
        let cache = UpgradeCheckCache {
            checked_for: "1.2.3".to_owned(),
            latest: "2.0.0".to_owned(),
            status: UpgradeCheckStatus::Available,
            checked_at: 1000,
            prompted_at: 0,
        };

        assert!(!lock.is_current(), "moving the install should invalidate the worker");
        assert!(
            lock.write_cache(&cache).is_err(),
            "an invalidated worker must not write its result"
        );
        assert!(!install_path.exists(), "the removed install path must stay absent");
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
        std::fs::create_dir_all(dir_path.as_path()).unwrap();
        std::fs::write(cache_path(&dir_path).as_path(), "not json").unwrap();
        assert!(read_cache(&dir_path).is_none());
    }

    fn with_env_vars_cleared<F: FnOnce()>(f: F) {
        temp_env::with_vars_unset(["CI", "VP_CLI_TEST", "VP_NO_UPDATE_CHECK"], f);
    }

    #[test]
    fn should_check_returns_true_when_no_cache() {
        with_env_vars_cleared(|| {
            assert!(should_check(None, "1.0.0", now_secs()));
        });
    }

    #[test]
    fn should_check_returns_false_when_cache_fresh() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let cache = UpgradeCheckCache {
                checked_for: "1.0.0".to_owned(),
                latest: "1.0.0".to_owned(),
                status: UpgradeCheckStatus::Current,
                checked_at: now,
                prompted_at: 0,
            };
            assert!(!should_check(Some(&cache), "1.0.0", now));
        });
    }

    #[test]
    fn should_check_returns_true_when_cache_stale() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let stale_time = now - CHECK_INTERVAL_SECS - 1;
            let cache = UpgradeCheckCache {
                checked_for: "1.0.0".to_owned(),
                latest: "1.0.0".to_owned(),
                status: UpgradeCheckStatus::Current,
                checked_at: stale_time,
                prompted_at: 0,
            };
            assert!(should_check(Some(&cache), "1.0.0", now));
        });
    }

    #[test]
    fn should_check_returns_true_for_a_different_cli_version() {
        with_env_vars_cleared(|| {
            let now = now_secs();
            let cache = UpgradeCheckCache {
                checked_for: "1.0.0".to_owned(),
                latest: "1.0.0".to_owned(),
                status: UpgradeCheckStatus::Current,
                checked_at: now,
                prompted_at: 0,
            };
            assert!(should_check(Some(&cache), "1.0.1", now));
        });
    }

    #[test]
    fn should_check_returns_false_when_disabled() {
        with_env_vars_cleared(|| {
            temp_env::with_var("VP_NO_UPDATE_CHECK", Some("1"), || {
                assert!(!should_check(None, "1.0.0", now_secs()));
            });
        });
    }

    #[test]
    fn notice_is_due_when_never_prompted() {
        let cache = UpgradeCheckCache {
            checked_for: "1.0.0".to_owned(),
            latest: "2.0.0".to_owned(),
            status: UpgradeCheckStatus::Available,
            checked_at: now_secs(),
            prompted_at: 0,
        };
        assert!(cache.notice_due("1.0.0", now_secs()));
    }

    #[test]
    fn notice_is_not_due_when_recently_prompted() {
        let now = now_secs();
        let cache = UpgradeCheckCache {
            checked_for: "1.0.0".to_owned(),
            latest: "2.0.0".to_owned(),
            status: UpgradeCheckStatus::Available,
            checked_at: now,
            prompted_at: now,
        };
        assert!(!cache.notice_due("1.0.0", now));
    }

    #[test]
    fn notice_is_due_when_prompt_stale() {
        let now = now_secs();
        let stale = now - PROMPT_INTERVAL_SECS - 1;
        let cache = UpgradeCheckCache {
            checked_for: "1.0.0".to_owned(),
            latest: "2.0.0".to_owned(),
            status: UpgradeCheckStatus::Available,
            checked_at: now,
            prompted_at: stale,
        };
        assert!(cache.notice_due("1.0.0", now));
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

    #[tokio::test(flavor = "current_thread")]
    async fn concurrent_slow_checks_start_one_request_and_back_off_before_it_finishes() {
        let home = tempfile::tempdir().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let registry = format!("http://{}", listener.local_addr().unwrap());
        let request_count = Arc::new(AtomicUsize::new(0));
        let server_request_count = Arc::clone(&request_count);
        let server = tokio::spawn(async move {
            loop {
                let (connection, _) = listener.accept().await.unwrap();
                server_request_count.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(async move {
                    let _connection = connection;
                    std::future::pending::<()>().await;
                });
            }
        });

        let vars: [(&'static str, Option<&OsStr>); 5] = [
            (vp_shared::env_vars::VP_HOME, Some(home.path().as_os_str())),
            (vp_shared::env_vars::NPM_CONFIG_REGISTRY, Some(OsStr::new(&registry))),
            ("CI", None),
            ("VP_CLI_TEST", None),
            ("VP_NO_UPDATE_CHECK", None),
        ];
        vp_shared::EnvConfig::with_vars_async(vars, |_| async {
            let checks = (0..5).map(|_| tokio::spawn(run_background_check())).collect::<Vec<_>>();

            tokio::time::timeout(Duration::from_secs(2), async {
                while request_count.load(Ordering::SeqCst) == 0 {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("at least one registry request should start");
            tokio::time::sleep(Duration::from_millis(200)).await;

            let observed_requests = request_count.load(Ordering::SeqCst);
            let cache_exists = home.path().join("cache").join(CACHE_FILE_NAME).exists();
            for check in checks {
                check.abort();
            }

            assert_eq!(observed_requests, 1, "concurrent checks must share one registry request");
            assert!(
                cache_exists,
                "the retry cooldown must be persisted before awaiting the registry"
            );
        })
        .await;
        server.abort();
    }
}
