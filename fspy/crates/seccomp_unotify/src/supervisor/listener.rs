use libc::{seccomp_notif, seccomp_notif_resp};
use std::{
    io, ops::Deref, os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd}
};
use tracing::trace;

use crate::bindings::{
    alloc::{Alloced, alloc_seccomp_notif},
    notif_recv,
};
use tokio::io::unix::AsyncFd;

pub struct NotifyListener {
    async_fd: AsyncFd<OwnedFd>,
    notif_buf: Alloced<libc::seccomp_notif>,
}

impl TryFrom<OwnedFd> for NotifyListener {
    type Error = io::Error;
    fn try_from(value: OwnedFd) -> Result<Self, Self::Error> {
        Ok(Self {
            async_fd: AsyncFd::new(value)?,
            notif_buf: alloc_seccomp_notif(),
        })
    }
}
impl AsFd for NotifyListener {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.async_fd.as_fd()
    }
}

const SECCOMP_IOCTL_NOTIF_SEND: libc::c_ulong = 3222806785;
const SECCOMP_IOCTL_NOTIF_RECV: libc::c_ulong = 3226476800;
const SECCOMP_IOCTL_NOTIF_ID_VALID: libc::c_ulong = 1074274562;

impl NotifyListener {
    pub fn send_continue(
        &self,
        req_id: u64,
        buf: &mut Alloced<seccomp_notif_resp>,
    ) -> io::Result<()> {
        let resp = buf.zeroed();
        resp.id = req_id;
        resp.flags = libc::SECCOMP_USER_NOTIF_FLAG_CONTINUE as _;

        let ret = unsafe {
            libc::ioctl(
                self.async_fd.as_raw_fd(),
                SECCOMP_IOCTL_NOTIF_SEND,
                &raw mut *resp,
            )
        };
        if ret < 0 {
            let err = nix::Error::last();
            // ignore error if target process's syscall was interrupted
            if err == nix::Error::ENOENT {
                return Ok(());
            };
            return Err(err.into());
        };
        Ok(())
    }
    pub async fn next(&mut self) -> io::Result<Option<&seccomp_notif>> {
        loop {
            let mut ready_guard = self.async_fd.readable().await?;
            let ready = ready_guard.ready();
            trace!("notify fd readable: {:?}", ready);
            if ready.is_read_closed() || ready.is_write_closed() {
                return Ok(None);
            }

            if !ready.is_readable() {
                continue;
            }
            // TODO: check why this call solves the issue that `is_read_closed || is_write_closed` is never true.
            ready_guard.clear_ready();

            match notif_recv(ready_guard.get_inner().as_fd(), &mut self.notif_buf) {
                Ok(()) => return Ok(Some(self.notif_buf.deref())),
                Err(nix::Error::EINTR | nix::Error::EWOULDBLOCK | nix::Error::ENOENT) => continue,
                Err(other_error) => return Err(other_error.into()),
            }
        }
    }
}
