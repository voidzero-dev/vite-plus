#![cfg_attr(unix, feature(c_variadic))]
#![feature(sync_unsafe_cell)]

mod stack_once;


#[cfg(target_os = "macos")]
pub mod macos;

#[doc(hidden)]
#[cfg(target_os = "macos")]
pub use macos::_CTOR;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;


#[cfg(unix)]
pub mod unix;

// #[cfg(not(any(target_os = "macos", target_os = "windows")))]
// #[link(name = "fspy_do_not_build_this_cydlib")]
// unsafe extern { }

