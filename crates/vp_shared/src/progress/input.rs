//! Block keyboard input while a progress row is visible, like clack's `block()`.
//! The terminal switches to Node's raw mode except that Ctrl+C and Ctrl+Z keep
//! sending signals, so no reader thread is needed. Pending input is discarded
//! on release so it does not reach the shell. Signal handlers and a panic hook
//! restore the terminal on paths where `Drop` never runs.

use std::sync::{
    Once,
    atomic::{AtomicBool, Ordering},
};

/// Only one row blocks input at a time, which keeps the saved state exclusive.
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub(super) struct InputBlock(sys::Guard);

/// Returns `None` unless stdin is a terminal in its normal line-editing mode.
/// A terminal another program already switched to raw mode is left alone.
pub(super) fn block() -> Option<InputBlock> {
    if ACTIVE.swap(true, Ordering::SeqCst) {
        return None;
    }
    let Some(guard) = sys::Guard::new() else {
        ACTIVE.store(false, Ordering::SeqCst);
        return None;
    };
    install_panic_hook();
    Some(InputBlock(guard))
}

impl Drop for InputBlock {
    fn drop(&mut self) {
        self.0.release();
        ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// Release builds abort on panic, so `Drop` never runs there.
fn install_panic_hook() {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            sys::restore_if_blocked();
            previous(info);
        }));
    });
}

#[cfg(unix)]
mod sys {
    use std::{
        cell::UnsafeCell,
        mem::MaybeUninit,
        os::fd::RawFd,
        sync::atomic::{AtomicBool, Ordering},
    };

    use nix::{
        libc,
        sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
    };

    struct Saved(UnsafeCell<MaybeUninit<libc::termios>>);

    // SAFETY: only the guard that won `ACTIVE` writes this, before it sets
    // `BLOCKED` and installs signal handlers. Every other access reads it.
    unsafe impl Sync for Saved {}

    static SAVED: Saved = Saved(UnsafeCell::new(MaybeUninit::uninit()));
    static BLOCKED: AtomicBool = AtomicBool::new(false);

    pub(super) struct Guard {
        previous: Vec<(Signal, SigAction)>,
    }

    impl Guard {
        /// Arms restoration before the terminal changes, so a signal at any
        /// point leaves the terminal in its saved state.
        pub(super) fn new() -> Option<Self> {
            let raw = save(libc::STDIN_FILENO)?;
            BLOCKED.store(true, Ordering::SeqCst);
            let mut guard = Self { previous: install_handlers() };
            if !apply(libc::STDIN_FILENO, &raw) {
                guard.disarm();
                return None;
            }
            Some(guard)
        }

        /// Restores before disarming, so a signal in between still restores.
        pub(super) fn release(&mut self) {
            restore(libc::STDIN_FILENO);
            self.disarm();
        }

        fn disarm(&mut self) {
            BLOCKED.store(false, Ordering::SeqCst);
            for (signal, previous) in self.previous.drain(..) {
                // SAFETY: reinstalls the disposition captured by `install_handlers`.
                let _ = unsafe { sigaction(signal, &previous) };
            }
        }
    }

    /// A background job that changes or flushes its controlling terminal is
    /// stopped with SIGTTOU, so it leaves the terminal to the foreground job.
    /// A terminal that is not the controlling one has no foreground job.
    fn in_background(fd: RawFd) -> bool {
        // SAFETY: both calls only read process state and are async-signal-safe.
        let (foreground, own) = unsafe { (libc::tcgetpgrp(fd), libc::getpgrp()) };
        foreground != -1 && foreground != own
    }

    /// Save the state of `fd` and return its raw input form with signals kept.
    pub(super) fn save(fd: RawFd) -> Option<libc::termios> {
        if in_background(fd) {
            return None;
        }
        let mut termios = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: `tcgetattr` initializes `termios` when it succeeds, and the
        // caller owns `SAVED` through `ACTIVE`.
        unsafe {
            if libc::tcgetattr(fd, termios.as_mut_ptr()) != 0 {
                return None;
            }
            let original = termios.assume_init();
            let line_mode = libc::ECHO | libc::ICANON;
            if original.c_lflag & line_mode != line_mode {
                return None;
            }
            let mut raw = original;
            raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN);
            raw.c_iflag &= !(libc::IXON | libc::ICRNL | libc::BRKINT | libc::ISTRIP);
            (*SAVED.0.get()).write(original);
            Some(raw)
        }
    }

    pub(super) fn apply(fd: RawFd, raw: &libc::termios) -> bool {
        // SAFETY: `raw` is a complete termios returned by `save`.
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, raw) == 0 }
    }

    /// Discard pending input and restore the saved state. Only async-signal-safe
    /// calls, because the signal handlers use this too.
    pub(super) fn restore(fd: RawFd) {
        // The shell restored its own modes when it moved this job to the background.
        if in_background(fd) {
            return;
        }
        // SAFETY: callers only restore after `save` saved the state.
        unsafe {
            libc::tcflush(fd, libc::TCIFLUSH);
            libc::tcsetattr(fd, libc::TCSANOW, (*SAVED.0.get()).as_ptr());
        }
    }

    pub(super) fn restore_if_blocked() {
        if BLOCKED.load(Ordering::SeqCst) {
            restore(libc::STDIN_FILENO);
        }
    }

    fn install_handlers() -> Vec<(Signal, SigAction)> {
        let action = SigAction::new(
            SigHandler::Handler(restore_and_reraise),
            SaFlags::empty(),
            SigSet::empty(),
        );
        // ISIG stays on, so Ctrl+\ sends SIGQUIT as well as Ctrl+C sending SIGINT.
        [Signal::SIGINT, Signal::SIGQUIT, Signal::SIGTERM]
            .into_iter()
            // Keep a disposition someone else chose, such as an ignored SIGINT.
            .filter(|&signal| is_default(signal))
            .filter_map(|signal| {
                // SAFETY: the handler only calls async-signal-safe functions.
                let previous = unsafe { sigaction(signal, &action) }.ok()?;
                Some((signal, previous))
            })
            .collect()
    }

    /// Reads the disposition without replacing it, so a signal arriving now
    /// still gets the behavior its owner chose.
    fn is_default(signal: Signal) -> bool {
        let mut current = MaybeUninit::<libc::sigaction>::uninit();
        // SAFETY: a null action only writes the current disposition to `current`.
        unsafe {
            libc::sigaction(signal as libc::c_int, std::ptr::null(), current.as_mut_ptr()) == 0
                && current.assume_init().sa_sigaction == libc::SIG_DFL
        }
    }

    extern "C" fn restore_and_reraise(signal: libc::c_int) {
        restore_if_blocked();
        // SAFETY: `signal` and `raise` are async-signal-safe. The signal stays
        // blocked until this handler returns, then the default action ends the
        // process with the usual status.
        unsafe {
            libc::signal(signal, libc::SIG_DFL);
            libc::raise(signal);
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{
        ffi::c_void,
        sync::atomic::{AtomicU32, Ordering},
    };

    type Handle = *mut c_void;

    // `(DWORD)-10`
    const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6;
    const ENABLE_LINE_INPUT: u32 = 0x0002;
    const ENABLE_ECHO_INPUT: u32 = 0x0004;
    const LINE_MODE: u32 = ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT;

    /// The console mode before blocking, or 0 while input is not blocked.
    static SAVED: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" {
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn GetConsoleMode(handle: Handle, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: Handle, mode: u32) -> i32;
        fn FlushConsoleInputBuffer(handle: Handle) -> i32;
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(ctrl_type: u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    pub(super) struct Guard(());

    impl Guard {
        /// Keeps `ENABLE_PROCESSED_INPUT`, so Ctrl+C still ends the process.
        pub(super) fn new() -> Option<Self> {
            let mut mode = 0;
            // SAFETY: plain Win32 calls on this process's stdin handle. Getting
            // the mode fails for pipes and redirected files.
            unsafe {
                if GetConsoleMode(stdin(), &raw mut mode) == 0 || mode & LINE_MODE != LINE_MODE {
                    return None;
                }
                // Arm restoration before the console changes.
                SAVED.store(mode, Ordering::SeqCst);
                SetConsoleCtrlHandler(Some(restore_on_ctrl), 1);
                if SetConsoleMode(stdin(), mode & !LINE_MODE) == 0 {
                    SAVED.store(0, Ordering::SeqCst);
                    SetConsoleCtrlHandler(Some(restore_on_ctrl), 0);
                    return None;
                }
            }
            Some(Self(()))
        }

        #[expect(clippy::unused_self, reason = "matches the Unix guard")]
        pub(super) fn release(&self) {
            restore_if_blocked();
            // SAFETY: removes the routine `new` registered.
            unsafe { SetConsoleCtrlHandler(Some(restore_on_ctrl), 0) };
        }
    }

    fn stdin() -> Handle {
        // SAFETY: returns this process's stdin handle or an invalid handle.
        unsafe { GetStdHandle(STD_INPUT_HANDLE) }
    }

    /// Clears `SAVED` only after restoring, so a Ctrl+C handler running on
    /// another thread during release restores too instead of skipping.
    pub(super) fn restore_if_blocked() {
        let mode = SAVED.load(Ordering::SeqCst);
        if mode != 0 {
            // SAFETY: plain Win32 calls on this process's stdin handle.
            unsafe {
                FlushConsoleInputBuffer(stdin());
                SetConsoleMode(stdin(), mode);
            }
            let _ = SAVED.compare_exchange(mode, 0, Ordering::SeqCst, Ordering::SeqCst);
        }
    }

    /// Returns FALSE so the next handler still ends the process.
    unsafe extern "system" fn restore_on_ctrl(_ctrl_type: u32) -> i32 {
        restore_if_blocked();
        0
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::fd::{AsFd, AsRawFd, RawFd};

    use nix::{
        libc,
        poll::{PollFd, PollFlags, PollTimeout, poll},
        pty::openpty,
        sys::termios::{InputFlags, LocalFlags, tcgetattr},
        unistd::write,
    };

    use super::sys;

    fn block(fd: RawFd) -> bool {
        sys::save(fd).is_some_and(|raw| sys::apply(fd, &raw))
    }

    #[test]
    fn raw_input_keeps_signals_and_restores_the_terminal() {
        let pty = openpty(None, None).unwrap();
        let fd = pty.slave.as_raw_fd();
        let pending = |timeout: PollTimeout| {
            let mut fds = [PollFd::new(pty.slave.as_fd(), PollFlags::POLLIN)];
            poll(&mut fds, timeout).unwrap() > 0
        };
        let original = tcgetattr(&pty.slave).unwrap();

        let not_a_terminal = std::fs::File::open("/dev/null").unwrap();
        assert!(!block(not_a_terminal.as_raw_fd()));

        assert!(block(fd));
        // A terminal already in raw mode is left alone.
        assert!(!block(fd));
        let raw = tcgetattr(&pty.slave).unwrap();
        assert!(
            !raw.local_flags.intersects(LocalFlags::ECHO | LocalFlags::ICANON | LocalFlags::IEXTEN)
        );
        assert!(raw.local_flags.contains(LocalFlags::ISIG));
        assert!(!raw.input_flags.contains(InputFlags::IXON));

        write(&pty.master, b"ls\r").unwrap();
        // Input reaches the slave asynchronously, so wait before restoring.
        assert!(pending(PollTimeout::from(1000u16)));

        sys::restore(fd);
        let restored = tcgetattr(&pty.slave).unwrap();
        // BSD kernels set PENDIN whenever canonical mode returns. The queue is
        // already empty, so it only clears itself on the next read.
        assert_eq!(restored.local_flags - LocalFlags::PENDIN, original.local_flags);
        assert_eq!(restored.input_flags, original.input_flags);
        assert!(!pending(PollTimeout::ZERO));
    }

    #[test]
    fn background_job_leaves_the_terminal_alone() {
        let pty = openpty(None, None).unwrap();
        let slave = pty.slave.as_raw_fd();

        // Only a new session can make the pty its controlling terminal, so fork.
        // SAFETY: the children neither allocate nor take locks before `_exit`.
        let status = unsafe {
            let session = libc::fork();
            assert!(session >= 0, "fork failed");
            if session == 0 {
                // A new session on the pty as stdin, whose group is the foreground job.
                if libc::login_tty(slave) == -1 {
                    libc::_exit(3);
                }
                let fd = libc::STDIN_FILENO;
                let job = libc::fork();
                if job == 0 {
                    // Another group in the session is a background job.
                    libc::setpgid(0, 0);
                    let blocked = block(fd);
                    sys::restore(fd);
                    libc::_exit(i32::from(blocked));
                }
                let mut status = 0;
                libc::waitpid(job, &raw mut status, libc::WUNTRACED);
                if libc::WIFSTOPPED(status) {
                    libc::kill(job, libc::SIGKILL);
                    libc::_exit(2);
                }
                libc::_exit(if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { 4 });
            }
            let mut status = 0;
            libc::waitpid(session, &raw mut status, 0);
            status
        };

        // A session that died from a signal also reports exit status 0.
        assert!(libc::WIFEXITED(status), "session ended abnormally: {status:#x}");
        assert_eq!(
            libc::WEXITSTATUS(status),
            0,
            "1: blocked input, 2: stopped by SIGTTOU, 3: no session, 4: job ended abnormally"
        );
    }

    const SIGNAL_CHILD: &str = "VP_SHARED_INPUT_SIGNAL_CHILD";

    /// Ends a process holding the input guard with each handled signal and
    /// checks that the terminal is restored and the signal still ends it. A
    /// separate process runs this same test, because the signal ends it and
    /// forking cannot allocate safely in the multi-threaded test runner.
    #[test]
    fn signals_restore_the_terminal_and_still_end_the_process() {
        use std::{os::unix::process::ExitStatusExt, process::Command};

        if let Ok(signal) = std::env::var(SIGNAL_CHILD) {
            let _guard = sys::Guard::new().expect("stdin is a terminal in line mode");
            let no_core = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            // SAFETY: SIGQUIT should not leave a core dump, and the signal is
            // expected to end this process.
            unsafe {
                libc::setrlimit(libc::RLIMIT_CORE, &raw const no_core);
                libc::raise(signal.parse().unwrap());
            }
            unreachable!("signal {signal} did not end the process");
        }

        for signal in [libc::SIGINT, libc::SIGQUIT, libc::SIGTERM] {
            let pty = openpty(None, None).unwrap();
            let original = tcgetattr(&pty.slave).unwrap();
            let status = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "progress::input::tests::signals_restore_the_terminal_and_still_end_the_process"])
                .env(SIGNAL_CHILD, signal.to_string())
                .stdin(pty.slave.try_clone().unwrap())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();

            assert_eq!(status.signal(), Some(signal), "{status}");
            let restored = tcgetattr(&pty.slave).unwrap();
            assert_eq!(restored.local_flags - LocalFlags::PENDIN, original.local_flags);
            assert_eq!(restored.input_flags, original.input_flags);
        }
    }
}
