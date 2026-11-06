use std::{hash::Hasher, io::Read, path::Path, sync::Arc, time::Duration};

use notify::{Event, EventKind, RecursiveMode, TargetMode, WatchMode, Watcher};
use rustc_hash::{FxHashMap, FxHasher};
use tokio::sync::mpsc;
use vp_error::Error;
use vp_shared::output;
use vt::ExitStatus;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::cli::{EnvMap, SubcommandResolver};

/// Checks run serially. A bounded channel coalesces changes received while a
/// check is running without losing the pending re-run or growing a queue.
pub(crate) async fn execute_watch(
    resolver: &SubcommandResolver,
    quiet: bool,
    deny_warnings: bool,
    no_fmt: bool,
    no_lint: bool,
    no_error_on_unmatched_pattern: bool,
    paths: Vec<String>,
    envs: &Arc<EnvMap>,
    cwd: &AbsolutePathBuf,
    debounce_ms: u64,
) -> Result<ExitStatus, Error> {
    let (sender, mut receiver) = mpsc::channel(1);
    let root: Arc<AbsolutePath> = Arc::from(resolver.workspace_path());
    let mut contents = FileContents::default();
    let mut watcher = notify::RecommendedWatcher::new(
        move |event: notify::Result<Event>| {
            let relevant = match &event {
                Ok(event) => relevant_event(event, &root) && contents.changed(event, &root),
                Err(_) => true,
            };
            if relevant {
                let _ = sender.try_send(event.map(|_| ()));
            }
        },
        notify::Config::default(),
    )
    .map_err(anyhow::Error::from)?;
    watcher
        .watch(
            resolver.workspace_path().as_path(),
            WatchMode {
                recursive_mode: RecursiveMode::Recursive,
                target_mode: TargetMode::TrackPath,
            },
        )
        .map_err(anyhow::Error::from)?;

    output::note("Watching for changes. Press Ctrl+C to stop.");
    let interrupt = tokio::signal::ctrl_c();
    tokio::pin!(interrupt);
    loop {
        // Do not drop a running check on Ctrl+C: its child processes receive
        // the terminal signal too and must be reaped before we return.
        let check = super::execute_check(
            resolver,
            false,
            quiet,
            deny_warnings,
            no_fmt,
            no_lint,
            no_error_on_unmatched_pattern,
            paths.clone(),
            envs,
            cwd,
        );
        tokio::pin!(check);
        let result = tokio::select! {
            biased;
            result = &mut interrupt => {
                result?;
                let _ = check.await;
                return Ok(ExitStatus::SUCCESS);
            }
            result = &mut check => result,
        };
        if let Err(error) = result {
            output::error(&error.to_string());
        }

        tokio::select! {
            result = &mut interrupt => {
                result?;
                return Ok(ExitStatus::SUCCESS);
            }
            changed = wait_for_changes(&mut receiver, Duration::from_millis(debounce_ms)) => {
                if !changed? {
                    return Ok(ExitStatus::SUCCESS);
                }
            }
        }
        output::note("Files changed. Re-running checks...");
    }
}

async fn wait_for_changes(
    receiver: &mut mpsc::Receiver<notify::Result<()>>,
    delay: Duration,
) -> Result<bool, Error> {
    match receiver.recv().await {
        Some(Ok(())) => {}
        Some(Err(error)) => return Err(anyhow::Error::from(error).into()),
        None => return Ok(false),
    }
    // Every new event restarts the inactivity interval. Changes received
    // during the preceding check are deliberately kept for this run.
    loop {
        match tokio::time::timeout(delay, receiver.recv()).await {
            Err(_) => return Ok(true),
            Ok(Some(Ok(()))) => {}
            Ok(Some(Err(error))) => return Err(anyhow::Error::from(error).into()),
            Ok(None) => return Ok(false),
        }
    }
}

fn relevant_event(event: &Event, root: &AbsolutePath) -> bool {
    if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
        || matches!(event.kind, EventKind::Modify(notify::event::ModifyKind::Metadata(_)))
    {
        return false;
    }
    event.paths.iter().any(|path| relevant_path(path, root))
}

fn relevant_path(path: &Path, root: &AbsolutePath) -> bool {
    path.strip_prefix(root.as_path()).is_ok_and(|relative| {
        !relative.components().any(|component| {
            component.as_os_str().to_str().is_some_and(|name| {
                matches!(
                    name,
                    "node_modules" | ".git" | "dist" | "target" | ".vite" | ".cache" | ".idea"
                ) || name.ends_with("___jb_tmp___")
                    || name.ends_with("___jb_old___")
                    || name.ends_with('~')
                    || name.ends_with(".swp")
                    || name.ends_with(".swx")
            })
        })
    })
}

#[derive(Default)]
struct FileContents {
    hashes: FxHashMap<std::path::PathBuf, Option<u64>>,
}

impl FileContents {
    fn changed(&mut self, event: &Event, root: &AbsolutePath) -> bool {
        let mut changed = false;
        // Visit every path, including both sides of a rename. Short-circuiting
        // would leave the remaining paths stale and cause a redundant run later.
        for path in event.paths.iter().filter(|path| relevant_path(path, root)) {
            if path.is_dir() {
                continue;
            }
            let hash = match file_hash(path) {
                Ok(hash) => hash,
                // An editor may temporarily lock the file while saving. Keep
                // the event so checks still run; don't cache the failed read.
                Err(_) => {
                    changed = true;
                    continue;
                }
            };
            let previous = self.hashes.insert(path.clone(), hash);
            if previous != Some(hash) {
                changed = true;
                tracing::debug!(path = %path.display(), "check watch: file contents changed");
            }
        }
        changed
    }
}

fn file_hash(path: &Path) -> std::io::Result<Option<u64>> {
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut hash = FxHasher::default();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(Some(hash.finish()));
        }
        hash.write(&buffer[..count]);
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

    use super::*;
    use crate::cli::SynthesizableSubcommand;

    #[derive(Parser)]
    struct Args {
        #[command(subcommand)]
        command: SynthesizableSubcommand,
    }

    #[tokio::test(start_paused = true)]
    async fn groups_changes_until_the_last_event_is_quiet() {
        let (sender, mut receiver) = mpsc::channel(1);
        let start = tokio::time::Instant::now();
        let producer = tokio::spawn(async move {
            for _ in 0..3 {
                sender.send(Ok(())).await.unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            // Keep the channel open after the final event.
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        assert!(wait_for_changes(&mut receiver, Duration::from_millis(300)).await.unwrap());
        assert_eq!(start.elapsed(), Duration::from_millis(500));
        assert!(receiver.try_recv().is_err());
        producer.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_one_pending_run_for_changes_during_a_check() {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.try_send(Ok(())).unwrap();
        assert!(sender.try_send(Ok(())).is_err());
        let start = tokio::time::Instant::now();
        assert!(wait_for_changes(&mut receiver, Duration::from_millis(300)).await.unwrap());
        assert_eq!(start.elapsed(), Duration::from_millis(300));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn watch_arguments_require_a_supported_combination() {
        assert!(Args::try_parse_from(["vp", "check", "--watch"]).is_ok());
        assert!(Args::try_parse_from(["vp", "check", "--watch", "--debounce", "500"]).is_ok());
        assert!(Args::try_parse_from(["vp", "check", "--debounce", "500"]).is_err());
        assert!(Args::try_parse_from(["vp", "check", "--watch", "--fix"]).is_err());
        assert!(Args::try_parse_from(["vp", "check", "--watch", "--debounce", "-1"]).is_err());
    }

    #[test]
    fn watches_changes_but_ignores_reads_and_generated_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        for kind in [
            EventKind::Create(CreateKind::File),
            EventKind::Modify(ModifyKind::Any),
            EventKind::Remove(RemoveKind::File),
        ] {
            assert!(relevant_event(
                &Event::new(kind).add_path(temp.path().join("src/main.ts")),
                &root
            ));
        }
        assert!(!relevant_event(
            &Event::new(EventKind::Access(AccessKind::Any))
                .add_path(temp.path().join("src/main.ts")),
            &root,
        ));
        for path in [
            "node_modules/pkg/index.js",
            ".git/index",
            "dist/index.js",
            "target/output",
            ".vite/cache",
            ".cache/check",
            ".idea/workspace.xml",
            "src/App.tsx___jb_tmp___",
            "src/App.tsx___jb_old___",
            "src/App.tsx~",
            "src/.App.tsx.swp",
        ] {
            assert!(!relevant_event(
                &Event::new(EventKind::Modify(ModifyKind::Any)).add_path(temp.path().join(path)),
                &root,
            ));
        }
    }

    #[test]
    fn repeated_save_events_only_trigger_when_contents_change() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let file = temp.path().join("App.tsx");
        let event = Event::new(EventKind::Modify(ModifyKind::Any)).add_path(file.clone());
        let mut contents = FileContents::default();
        std::fs::write(&file, "const value = 1;").unwrap();
        assert!(contents.changed(&event, &root));
        assert!(!contents.changed(&event, &root));
        std::fs::write(&file, "const value = 1;").unwrap();
        assert!(!contents.changed(&event, &root));
        std::fs::write(&file, "const value = 1").unwrap();
        assert!(contents.changed(&event, &root));
        assert!(!contents.changed(&event, &root));
        std::fs::remove_file(&file).unwrap();
        assert!(contents.changed(&event, &root));
        assert!(!contents.changed(&event, &root));
        std::fs::write(&file, "const value = 1;").unwrap();
        assert!(contents.changed(&event, &root));
    }
}
