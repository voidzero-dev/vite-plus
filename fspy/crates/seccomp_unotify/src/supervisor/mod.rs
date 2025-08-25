pub mod handler;
mod listener;

use std::{
    io::{self, IoSliceMut},
    os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
};

pub use handler::SeccompNotifyHandler;
use listener::NotifyListener;
use nix::{
    cmsg_space,
    fcntl::{FcntlArg, FdFlag, fcntl},
    sys::socket::{ControlMessageOwned, MsgFlags, recvmsg},
};
use passfd::tokio::FdPassingExt;
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};
use tokio::{io::Interest, net::UnixStream, task::JoinSet};
use tracing::{Level, span};

use crate::{
    bindings::alloc::alloc_seccomp_notif_resp,
    payload::{Filter, SeccompPayload},
};

pub struct Supervisor<F> {
    pub payload: SeccompPayload,
    pub pre_exec: PreExec,
    pub handling_loop: F,
}

pub struct PreExec(OwnedFd);
impl PreExec {
    pub fn run(&mut self) -> nix::Result<()> {
        let mut fd_flag = FdFlag::from_bits_retain(fcntl(&self.0, FcntlArg::F_GETFD)?);
        fd_flag.remove(FdFlag::FD_CLOEXEC);
        fcntl(&self.0, FcntlArg::F_SETFD(fd_flag))?;
        Ok(())
    }
}

pub fn supervise<H: SeccompNotifyHandler + Default + Send + 'static>()
-> io::Result<Supervisor<impl Future<Output = io::Result<Vec<H>>> + Send>> {
    let (notify_fd_receiver, notify_fd_sender) = UnixStream::pair()?;
    let notify_fd_sender = notify_fd_sender.into_std()?;
    notify_fd_sender.set_nonblocking(false)?;

    let filter = SeccompFilter::new(
        H::syscalls()
            .iter()
            .map(|sysno| (sysno.id().into(), vec![]))
            .collect(),
        SeccompAction::Allow,
        SeccompAction::Raw(libc::SECCOMP_RET_USER_NOTIF),
        std::env::consts::ARCH.try_into().unwrap(),
    )
    .unwrap();

    let filter = Filter(
        BpfProgram::try_from(filter)
            .unwrap()
            .into_iter()
            .map(|sock_filter| sock_filter.into())
            .collect(),
    );

    let payload = SeccompPayload {
        ipc_fd: notify_fd_sender.as_raw_fd(),
        filter,
    };

    let handling_loop = async move {
        let mut join_set: JoinSet<io::Result<H>> = JoinSet::new();

        loop {
            let notify_fd = match notify_fd_receiver.recv_fd().await {
                Ok(fd) => unsafe { OwnedFd::from_raw_fd(fd) },
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    } else {
                        return Err(err);
                    }
                }
            };
            let mut listener = NotifyListener::try_from(notify_fd)?;

            let mut handler = H::default();
            let mut resp_buf = alloc_seccomp_notif_resp();

            join_set.spawn(async move {
                while let Some(notify) = listener.next().await? {
                    let _span = span!(Level::TRACE, "notify loop tick");
                    // Errors on the supervisor side shouldn't block the syscall.
                    let handle_result = handler.handle_notify(notify);
                    let notify_id = notify.id;
                    listener.send_continue(notify_id, &mut resp_buf)?;
                    handle_result?;
                }
                io::Result::Ok(handler)
            });
        }
        let mut handlers = Vec::<H>::new();
        while let Some(handler) = join_set.join_next().await.transpose()? {
            handlers.push(handler?);
        }
        Ok(handlers)
    };
    Ok(Supervisor {
        payload,
        pre_exec: PreExec(notify_fd_sender.into()),
        handling_loop,
    })
}
