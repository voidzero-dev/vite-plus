#![cfg(unix)]
// required for defining inteposed `open`/`openat`(https://man7.org/linux/man-pages/man2/open.2.html)
#![feature(c_variadic)]

mod macros;
mod interceptions;
mod libc;
mod client;
