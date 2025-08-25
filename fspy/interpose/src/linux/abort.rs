use std::{fmt::Display, fs::File, io::Write, mem::ManuallyDrop, os::fd::FromRawFd};

pub fn abort_with(msg: impl Display) -> ! {
    libc_print::libc_eprintln!("{}", msg);
    unsafe { libc::abort() }
}
