pub mod handler;
mod listener;

use std::{
    convert::Infallible,
    io::{self},
    os::{
        fd::{FromRawFd, OwnedFd},
        unix::ffi::OsStrExt,
    },
};

use futures_util::{
    future::{Either, select},
    pin_mut,
};
pub use handler::SeccompNotifyHandler;
use listener::NotifyListener;
use passfd::tokio::FdPassingExt;
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};
use tracing::{Level, span};

use crate::{
    bindings::alloc::alloc_seccomp_notif_resp,
    payload::{Filter, SeccompPayload},
};

pub struct Supervisor<H> {
    payload: SeccompPayload,
    cancel_tx: oneshot::Sender<Infallible>,
    handling_loop_task: JoinHandle<io::Result<Vec<H>>>,
}

impl<H> Supervisor<H> {
    pub fn payload(&self) -> &SeccompPayload {
        &self.payload
    }

    pub async fn stop(self) -> io::Result<Vec<H>> {
        drop(self.cancel_tx);
        self.handling_loop_task.await.expect("handling loop task panicked")
    }
}

pub fn supervise<H: SeccompNotifyHandler + Default + Send + 'static>() -> io::Result<Supervisor<H>>
{
    let notify_listener = tempfile::Builder::new()
        .prefix("fspy_seccomp_notify")
        .make(|path| UnixListener::bind(path))?;

    let filter = SeccompFilter::new(
        H::syscalls().iter().map(|sysno| (sysno.id().into(), vec![])).collect(),
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

    let payload =
        SeccompPayload { ipc_path: notify_listener.path().as_os_str().as_bytes().to_vec(), filter };

    // The oneshot channel is used to cancel the accept loop.
    // The sender doesn't need to actually send anything. Drop is enough.
    let (cancel_tx, mut cancel_rx) = oneshot::channel::<Infallible>();

    let handling_loop = async move {
        let mut join_set: JoinSet<io::Result<H>> = JoinSet::new();

        loop {
            let accept_future = notify_listener.as_file().accept();
            pin_mut!(accept_future);
            let (incoming_stream, _) = match select(&mut cancel_rx, accept_future).await {
                Either::Left((Err(_), _)) => break,
                Either::Right((incoming, _)) => incoming?,
            };
            let notify_fd = incoming_stream.recv_fd().await?;
            let notify_fd = unsafe { OwnedFd::from_raw_fd(notify_fd) };
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
    Ok(Supervisor { payload, cancel_tx, handling_loop_task: tokio::spawn(handling_loop) })
}
