//! Helpers for suppressing interactive-terminal echo while a progress bar is
//! shown.
//!
//! The global-install spinner redraws a single row with ANSI escape sequences.
//! If the terminal driver echoes a pressed Enter (`\r\n`) between redraws, the
//! previous row cannot be erased and the user sees duplicate spinner lines. This
//! module puts stdin into a mode where keystrokes are not echoed, and restores
//! the original mode when the guard is dropped.

use std::io;

/// RAII guard that disables local echo on stdin and restores it on drop.
///
/// Creating the guard is a no-op when stdin is not connected to an interactive
/// terminal, so callers may always construct it without checking first.
pub struct StdinEchoGuard {
    #[cfg(unix)]
    restore: Option<UnixRestore>,
    #[cfg(windows)]
    restore: Option<WindowsRestore>,
}

impl StdinEchoGuard {
    /// Disable echo on stdin if it is a terminal and the underlying OS calls
    /// succeed. The original mode is restored when the returned guard is dropped.
    pub fn disable() -> Self {
        // Check the TTY state directly instead of through the cached helper.
        // Caching `stdin().is_terminal()` is fine for the normal CLI path, but
        // unit tests that fork a PTY child must see the child's TTY, not a
        // value memoized in the parent process.
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            return Self { restore: None };
        }
        Self { restore: restore_mode() }
    }
}

#[cfg(unix)]
struct UnixRestore {
    fd: std::os::unix::io::RawFd,
    original: libc::termios,
}

#[cfg(unix)]
fn restore_mode() -> Option<UnixRestore> {
    use std::os::unix::io::AsRawFd;

    let fd = io::stdin().as_raw_fd();
    let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: `tcgetattr` fills the uninitialized struct on success.
    if unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: `tcgetattr` returned success, so `termios` is initialized.
    let original = unsafe { termios.assume_init() };
    let mut modified = original;
    modified.c_lflag &= !(libc::ECHO | libc::ECHONL);
    // SAFETY: `tcsetattr` accepts a pointer to an initialized termios struct.
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, std::ptr::addr_of!(modified)) } != 0 {
        return None;
    }
    Some(UnixRestore { fd, original })
}

#[cfg(unix)]
impl Drop for StdinEchoGuard {
    fn drop(&mut self) {
        if let Some(restore) = &self.restore {
            // SAFETY: the fd and saved termios came from a successful
            // `tcgetattr` for this file descriptor.
            let _ = unsafe {
                libc::tcsetattr(restore.fd, libc::TCSANOW, std::ptr::addr_of!(restore.original))
            };
        }
    }
}

#[cfg(windows)]
struct WindowsRestore {
    handle: *mut std::ffi::c_void,
    original: u32,
}

#[cfg(windows)]
const STD_INPUT_HANDLE: u32 = 0xFFFF_FFF6;
#[cfg(windows)]
const ENABLE_ECHO_INPUT: u32 = 0x0004;

#[cfg(windows)]
extern "system" {
    fn GetStdHandle(n_std_handle: u32) -> *mut std::ffi::c_void;
    fn GetConsoleMode(h_console_handle: *mut std::ffi::c_void, lp_mode: *mut u32) -> i32;
    fn SetConsoleMode(h_console_handle: *mut std::ffi::c_void, dw_mode: u32) -> i32;
}

#[cfg(windows)]
fn restore_mode() -> Option<WindowsRestore> {
    // SAFETY: `GetStdHandle` is a well-defined Windows API call.
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    let mut mode = 0_u32;
    // SAFETY: `GetConsoleMode` reads the handle and writes to the out-param.
    if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
        return None;
    }
    let new_mode = mode & !ENABLE_ECHO_INPUT;
    // SAFETY: `SetConsoleMode` accepts a valid console handle.
    if unsafe { SetConsoleMode(handle, new_mode) } == 0 {
        return None;
    }
    Some(WindowsRestore { handle, original: mode })
}

#[cfg(windows)]
impl Drop for StdinEchoGuard {
    fn drop(&mut self) {
        if let Some(restore) = &self.restore {
            // SAFETY: the handle and mode came from a successful pair of
            // `GetStdHandle` / `GetConsoleMode` calls.
            let _ = unsafe { SetConsoleMode(restore.handle, restore.original) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_can_be_created_when_stdin_is_not_a_terminal() {
        // In `cargo test` stdin is usually a pipe or file, so the guard is a
        // no-op. This test mainly ensures the constructor does not panic and
        // that the Unix/Windows paths compile.
        let guard = StdinEchoGuard::disable();
        assert!(guard.restore.is_none(), "expected a no-op when stdin is not a terminal");
    }
}

#[cfg(all(test, unix))]
mod pty_tests {
    use std::{
        ffi::CStr,
        fs::File,
        io::{Read, Write},
        os::unix::io::FromRawFd,
        thread,
        time::Duration,
    };

    use crate::stdin_echo::StdinEchoGuard;

    /// Open a PTY, make sure echo is on, create a child with `StdinEchoGuard`,
    /// and verify from the parent side that pressing Enter does not produce a
    /// `\r\n` echo.
    #[test]
    fn guard_disables_terminal_echo() {
        // SAFETY: all libc calls below follow their documented preconditions.
        let master = unsafe { libc::open(c"/dev/ptmx".as_ptr() as *const _, libc::O_RDWR) };
        assert!(master >= 0, "failed to open /dev/ptmx");

        unsafe {
            assert!(libc::grantpt(master) == 0, "grantpt failed");
            assert!(libc::unlockpt(master) == 0, "unlockpt failed");
        }

        let slave_name_ptr = unsafe { libc::ptsname(master) };
        assert!(!slave_name_ptr.is_null(), "ptsname failed");
        let _slave_name = unsafe { CStr::from_ptr(slave_name_ptr) };

        let mut sync = [0_i32; 2];
        assert!(unsafe { libc::pipe(sync.as_mut_ptr()) } == 0);

        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");

        if pid == 0 {
            // Child: close unused ends and attach the PTY slave to stdio.
            unsafe {
                libc::close(sync[0]);
                libc::close(master);
            }
            let slave = unsafe { libc::open(slave_name_ptr, libc::O_RDWR) };
            assert!(slave >= 0, "failed to open PTY slave");
            unsafe { libc::setsid() };
            unsafe { libc::dup2(slave, 0) };
            unsafe { libc::dup2(slave, 1) };
            unsafe { libc::dup2(slave, 2) };
            if slave > 2 {
                unsafe { libc::close(slave) };
            }

            // Ensure echo is *on* before the guard, mimicking a real terminal.
            let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
            let mut termios = unsafe {
                assert!(libc::tcgetattr(0, termios.as_mut_ptr()) == 0);
                termios.assume_init()
            };
            termios.c_lflag |= libc::ECHO;
            unsafe {
                assert!(libc::tcsetattr(0, libc::TCSANOW, &termios) == 0);
            }

            let _guard = StdinEchoGuard::disable();

            unsafe {
                libc::write(sync[1], b"ready".as_ptr().cast::<std::ffi::c_void>(), 5);
                libc::close(sync[1]);
            }

            // Keep the guard alive while the parent checks for echo.
            let mut byte = 0_u8;
            let _ = unsafe {
                libc::read(0, std::ptr::addr_of_mut!(byte).cast::<std::ffi::c_void>(), 1)
            };
            std::process::exit(0);
        }

        // Parent.
        unsafe { libc::close(sync[1]) };
        let mut sync_file = unsafe { File::from_raw_fd(sync[0]) };
        let mut ready = [0_u8; 5];
        sync_file.read_exact(&mut ready).expect("child did not become ready");
        assert_eq!(ready, *b"ready");
        drop(sync_file);

        let mut master_file = unsafe { File::from_raw_fd(master) };
        master_file.write_all(b"\n").unwrap();
        master_file.flush().unwrap();

        // Give the terminal driver time to echo if echo were enabled.
        thread::sleep(Duration::from_millis(200));

        unsafe { libc::fcntl(master, libc::F_SETFL, libc::O_NONBLOCK) };
        let mut output = Vec::new();
        let mut buf = [0_u8; 256];
        loop {
            // SAFETY: read into a fixed-size buffer.
            let n = unsafe {
                libc::read(master, buf.as_mut_ptr().cast::<std::ffi::c_void>(), buf.len())
            };
            if n <= 0 {
                break;
            }
            output.extend_from_slice(&buf[..n as usize]);
        }

        let mut status = 0_i32;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        // With echo disabled the parent must not see the `\r\n` that the
        // terminal driver would otherwise echo for Enter.
        let has_echo = output.windows(2).any(|window| window == b"\r\n");
        assert!(!has_echo, "expected no \\r\\n echo, got: {}", String::from_utf8_lossy(&output));
    }
}
