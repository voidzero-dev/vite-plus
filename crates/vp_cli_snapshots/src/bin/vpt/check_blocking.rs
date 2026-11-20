use std::{io, os::fd::AsFd};

use nix::fcntl::{FcntlArg, OFlag, fcntl};

/// check-blocking
///
/// Task payload for `vp run` tests: prints whether the inherited stdin is in
/// blocking mode, without reading from it. A blocking read, such as a prompt
/// waiting for a key, fails with EAGAIN when it is not.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let flags = OFlag::from_bits_retain(fcntl(stdin.as_fd(), FcntlArg::F_GETFL)?);
    let mode = if flags.contains(OFlag::O_NONBLOCK) { "non-blocking" } else { "blocking" };
    println!("stdin:{mode}");
    Ok(())
}
