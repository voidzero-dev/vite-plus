#![cfg(unix)]

pub mod exec;
pub(crate) mod open_exec;
pub mod payload;
pub mod spawn;

#[cfg(target_os = "linux")]
mod elf;

#[cfg(target_os = "linux")] // exposed for verifying static executables in fspy tests
pub use elf::is_dynamically_linked_to_libc;
