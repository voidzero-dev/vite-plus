// On macOS,  function to be interposed in the interposer cydlib are already the real ones.
#[cfg(target_os = "macos")]
pub use super::libc::*;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;
