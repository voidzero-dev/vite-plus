#[cfg(target_os = "macos")]
#[path = "./macos.rs"]
mod os_impl;

#[cfg(target_os = "linux")]
#[path = "./linux.rs"]
mod os_impl;

pub(crate) use os_impl::*;
