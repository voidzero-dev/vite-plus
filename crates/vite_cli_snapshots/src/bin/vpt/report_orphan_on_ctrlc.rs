/// report-orphan-on-ctrlc `<verdict-file>` \[--ignore-interrupt\]
///
/// Simulates a task with a graceful shutdown phase (a vite dev server
/// closing on Ctrl+C) and records whether `vp run` let that shutdown finish
/// (issue #2036): emits a "ready" milestone, waits for Ctrl+C, then spends
/// 300 ms shutting down before exiting cleanly.
///
/// With `--ignore-interrupt` the task instead announces the interrupt (a
/// printed line plus an "interrupted" milestone, so a test can synchronize
/// a second Ctrl+C on it) and keeps running forever, like a task whose
/// shutdown hangs. Its graceful shutdown then never completes; ending it
/// takes vp's force-kill escalation.
///
/// The verdict is written by a watcher process spawned into its own process
/// group, insulated from whatever signals vp sends the task tree during
/// teardown (the same way real dev-server grandchildren outlive `vp run`).
/// The watcher holds the read end of a pipe; the task writes "done" after
/// its shutdown completes. Pipe EOF without "done" means the task was torn
/// down mid-shutdown, and the watcher records that in `<verdict-file>`
/// (atomically, via a temp-file rename) for a follow-up `vpt wait-file`
/// step. Nothing else is printed, so the terminal snapshot stays
/// deterministic no matter how quickly the task tree is killed.
#[cfg(unix)]
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.first().is_some_and(|arg| arg == "--watch") {
        return watch(args.get(1).ok_or("--watch needs a verdict file")?);
    }

    use std::{io::Write as _, os::fd::OwnedFd, sync::Arc, time::Duration};

    let mut verdict_path = None;
    let mut ignore_interrupt = false;
    for arg in args {
        if arg == "--ignore-interrupt" {
            ignore_interrupt = true;
        } else {
            verdict_path = Some(arg);
        }
    }
    let Some(verdict_path) = verdict_path else {
        return Err("Usage: vpt report-orphan-on-ctrlc <verdict-file> [--ignore-interrupt]".into());
    };

    let (pipe_read, pipe_write): (OwnedFd, OwnedFd) = nix::unistd::pipe()?;
    // The write end must not leak into the watcher, or the pipe never
    // reports EOF (macOS has no pipe2, so CLOEXEC is set separately).
    nix::fcntl::fcntl(&pipe_write, nix::fcntl::FcntlArg::F_SETFD(nix::fcntl::FdFlag::FD_CLOEXEC))?;

    let mut watcher = std::process::Command::new(std::env::current_exe()?);
    watcher
        .arg("report-orphan-on-ctrlc")
        .arg("--watch")
        .arg(verdict_path)
        .stdin(std::process::Stdio::from(pipe_read))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    // SAFETY: setpgid is async-signal-safe; the watcher moves to its own
    // process group so group-directed teardown signals cannot reach it.
    unsafe {
        std::os::unix::process::CommandExt::pre_exec(&mut watcher, || {
            let _ =
                nix::unistd::setpgid(nix::unistd::Pid::from_raw(0), nix::unistd::Pid::from_raw(0));
            Ok(())
        });
    }
    watcher.spawn()?;

    let ctrlc_once_lock = Arc::new(std::sync::OnceLock::<()>::new());
    ctrlc::set_handler({
        let ctrlc_once_lock = Arc::clone(&ctrlc_once_lock);
        move || {
            let _ = ctrlc_once_lock.set(());
        }
    })?;

    pty_terminal_test_client::mark_milestone("ready");

    ctrlc_once_lock.wait();
    if ignore_interrupt {
        // A task whose shutdown hangs: announce the interrupt so the test
        // can synchronize the second Ctrl+C on it, then never finish. The
        // "done" marker below is unreachable, so the watcher records the
        // eventual force-kill as an unfinished shutdown.
        println!("ignoring interrupt; still running");
        pty_terminal_test_client::mark_milestone("interrupted");
        loop {
            std::thread::park();
        }
    }
    // The graceful shutdown a real dev server performs after SIGINT. A
    // buggy vp tears the task down milliseconds into this window, so the
    // "done" marker below never gets written.
    std::thread::sleep(Duration::from_millis(300));

    let mut done_pipe = std::fs::File::from(pipe_write);
    let _ = done_pipe.write_all(b"done\n");
    Ok(())
}

/// Watcher mode: reads the pipe on stdin until EOF (or a 10 s safety cap so
/// an orphaned watcher can never outlive the suite), then records whether
/// the task's shutdown completed.
#[cfg(unix)]
fn watch(verdict_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        io::Read as _,
        os::fd::AsFd as _,
        time::{Duration, Instant},
    };

    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};

    let mut received = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    loop {
        let mut fds = [PollFd::new(stdin_lock.as_fd(), PollFlags::POLLIN)];
        if poll(&mut fds, PollTimeout::from(100u8)).is_err() {
            break;
        }
        let revents = fds[0].revents().unwrap_or(PollFlags::empty());
        if revents.intersects(PollFlags::POLLIN) {
            let mut buf = [0u8; 64];
            match stdin_lock.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => received.extend_from_slice(&buf[..n]),
            }
        } else if revents.intersects(PollFlags::POLLHUP | PollFlags::POLLERR | PollFlags::POLLNVAL)
        {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
    }

    let verdict = if received.windows(4).any(|w| w == b"done") {
        "task completed its graceful shutdown"
    } else {
        "task was torn down before its graceful shutdown finished"
    };
    let tmp = format!("{verdict_path}.tmp");
    std::fs::write(&tmp, format!("{verdict}\n"))?;
    std::fs::rename(&tmp, verdict_path)?;
    Ok(())
}

#[cfg(not(unix))]
pub fn run(_args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    Err("report-orphan-on-ctrlc is unix-only (process-group/PTY semantics)".into())
}
