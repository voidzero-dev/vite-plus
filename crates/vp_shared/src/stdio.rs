/// Ensure inherited standard streams use the blocking semantics expected by
/// Rust's standard library and by the tools embedded in Vite+.
///
/// Node.js marks non-TTY stdio as non-blocking, and child processes inherit
/// that flag through the shared open file description. Clear `O_NONBLOCK`
/// once at process entry so unowned writers do not panic or truncate output
/// when a consumer applies backpressure.
#[cfg(unix)]
pub fn ensure_blocking_stdio() {
    use std::{io, os::fd::AsFd};

    let stdin = io::stdin();
    let stdout = io::stdout();
    let stderr = io::stderr();

    for fd in [stdin.as_fd(), stdout.as_fd(), stderr.as_fd()] {
        ensure_blocking_fd(fd);
    }
}

#[cfg(unix)]
fn ensure_blocking_fd(fd: std::os::fd::BorrowedFd<'_>) {
    use nix::{
        fcntl::{FcntlArg, OFlag, fcntl},
        unistd::isatty,
    };

    if isatty(fd).unwrap_or(false) {
        return;
    }

    let Ok(flags) = fcntl(fd, FcntlArg::F_GETFL) else {
        return;
    };
    let flags = OFlag::from_bits_retain(flags);
    if flags.contains(OFlag::O_NONBLOCK) {
        let _ = fcntl(fd, FcntlArg::F_SETFL(flags.difference(OFlag::O_NONBLOCK)));
    }
}

#[cfg(not(unix))]
pub fn ensure_blocking_stdio() {}

#[cfg(all(test, unix))]
mod tests {
    use std::os::{fd::AsFd, unix::net::UnixStream};

    use nix::fcntl::{FcntlArg, OFlag, fcntl};

    use super::ensure_blocking_fd;

    #[test]
    fn clears_nonblocking_from_a_non_tty_file_description() {
        let (stream, _peer) = UnixStream::pair().unwrap();
        stream.set_nonblocking(true).unwrap();

        ensure_blocking_fd(stream.as_fd());

        let flags = fcntl(&stream, FcntlArg::F_GETFL).unwrap();
        assert!(!OFlag::from_bits_retain(flags).contains(OFlag::O_NONBLOCK));
    }
}
