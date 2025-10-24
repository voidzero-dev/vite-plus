use std::{
    ffi::OsStr,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, net::UnixStream},
    },
};

use libc::sock_filter;
use nix::sys::prctl::set_no_new_privs;
use passfd::FdPassingExt;

use crate::{bindings::install_unotify_filter, payload::SeccompPayload};

pub fn install_target(payload: &SeccompPayload) -> nix::Result<()> {
    set_no_new_privs()?;
    let sock_filters =
        payload.filter.0.iter().copied().map(sock_filter::from).collect::<Vec<sock_filter>>();
    let notify_fd = install_unotify_filter(&sock_filters)?;
    let ipc_path = OsStr::from_bytes(&payload.ipc_path);
    let ipc_unix_stream = UnixStream::connect(ipc_path)
        .map_err(|err| nix::Error::try_from(err).unwrap_or(nix::Error::UnknownErrno))?;
    ipc_unix_stream
        .send_fd(notify_fd.as_raw_fd())
        .map_err(|err| nix::Error::try_from(err).unwrap_or(nix::Error::UnknownErrno))?;
    Ok(())
}
