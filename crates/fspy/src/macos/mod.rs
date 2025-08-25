mod fixtures;

use std::{
    env::{self, current_dir, temp_dir},
    ffi::{OsStr, OsString},
    fs::create_dir,
    future::{Future, pending},
    io,
    mem::ManuallyDrop,
    net::Shutdown,
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd, RawFd},
        unix::{ffi::OsStrExt, process::CommandExt as _},
    },
    path::{Path, PathBuf},
    pin::pin,
    process::ExitStatus,
    sync::Arc,
    task::Poll,
};

use std::process::{Child as StdChild, Command as StdCommand};

use allocator_api2::SliceExt;
use bincode::config;
use bumpalo::Bump;

use fspy_shared::{
    ipc::{BINCODE_CONFIG, PathAccess},
    macos::{
        Fixtures, Payload, encode_payload,
        inject::{PayloadWithEncodedString, inject},
    },
};
use futures_util::{
    Stream, TryStream,
    future::{Either, join, select, select_all, try_select},
    stream::poll_fn,
};
use libc::PIPE_BUF;
use nix::{
    fcntl::{FcntlArg, FdFlag, OFlag, fcntl},
    sys::socket::{getsockopt, sockopt::SndBuf},
};

use passfd::tokio::FdPassingExt;
use slab::Slab;
use tokio::{
    io::{AsyncReadExt, BufReader, ReadBuf},
    net::{
        UnixDatagram, UnixStream,
        unix::pipe::{Receiver, pipe},
    },
    process::{Child as TokioChild, Command as TokioCommand},
};

use crate::Command;

pub fn update_fd_flag(fd: BorrowedFd<'_>, f: impl FnOnce(&mut FdFlag)) -> io::Result<()> {
    fcntl(
        fd,
        FcntlArg::F_SETFD({
            let mut fd_flag = FdFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFD)?);
            // dbg!((fd_flag, FdFlag::FD_CLOEXEC));
            f(&mut fd_flag);
            fd_flag
        }),
    )?;
    Ok(())
}

fn alloc_os_str<'a>(bump: &'a Bump, src: &OsStr) -> &'a OsStr {
    OsStr::from_bytes(SliceExt::to_vec_in(src.as_bytes(), bump).leak())
}
pub struct Child {
    pub tokio_child: TokioChild,
    pub path_access_stream: PathAccessIter,
}

#[derive(Debug)]
pub struct PathAccessIter {
    channel_receiver: Option<UnixStream>, // None when reaches eof
    channels: Slab<BufReader<Receiver>>,
}

impl PathAccessIter {
    pub async fn next<'a>(&mut self, buf: &'a mut Vec<u8>) -> io::Result<Option<PathAccess<'a>>> {
        loop {
            let new_channel_fut = async {
                let Some(channel_receiver) = &self.channel_receiver else {
                    return pending::<io::Result<Option<Receiver>>>().await;
                };
                {
                    let channel = match channel_receiver.recv_fd().await {
                        Err(err) => {
                            return if err.kind() == io::ErrorKind::UnexpectedEof {
                                Ok(None)
                            } else {
                                Err(err)
                            };
                        }
                        Ok(ok) => unsafe { OwnedFd::from_raw_fd(ok) },
                    };
                    update_fd_flag(channel.as_fd(), |flags| flags.insert(FdFlag::FD_CLOEXEC))?;

                    Ok(Some(Receiver::from_owned_fd(channel)?))
                }
            };
            let new_channel_fut = pin!(new_channel_fut);

            let either: Either<Option<Receiver>, usize> = if self.channels.is_empty() {
                Either::Left(new_channel_fut.await?)
            } else {
                let readable_fut_iter = self.channels.iter().map(|(key, channel)| {
                    Box::pin(async move {
                        if !channel.buffer().is_empty() {
                            return io::Result::Ok(key);
                        }
                        channel.get_ref().readable().await?;
                        io::Result::Ok(key)
                    })
                });

                let readable_fut = select_all(readable_fut_iter);

                match select(new_channel_fut, readable_fut).await {
                    Either::Left((new_channel_result, _)) => Either::Left(new_channel_result?),
                    Either::Right(((readable_key_result, _, _), _)) => {
                        Either::Right(readable_key_result?)
                    }
                }
            };

            let action: Either<Receiver, usize> = match either {
                Either::Left(new_channel) => {
                    if let Some(new_channel) = new_channel {
                        Either::Left(new_channel)
                    } else {
                        // channel_receiver eof
                        self.channel_receiver = None;
                        if self.channels.is_empty() {
                            return Ok(None);
                        }
                        continue;
                    }
                }
                Either::Right(readable_key) => Either::Right(readable_key),
            };

            match action {
                Either::Left(new_channel) => {
                    self.channels.insert(BufReader::new(new_channel));
                }
                Either::Right(readable_channel_key) => {
                    let readable_channel = &mut self.channels[readable_channel_key];

                    let mut msg_size = [0u8; size_of::<u32>()];
                    if !readable_channel.buffer().is_empty() {
                        readable_channel.read_exact(&mut msg_size).await?;
                    } else {
                        let n = match readable_channel.get_mut().try_read(&mut msg_size) {
                            Ok(ok) => ok,
                            Err(err) => {
                                if err.kind() == io::ErrorKind::WouldBlock {
                                    continue;
                                }
                                return Err(err);
                            }
                        };
                        if n == 0 {
                            // this channel eof. deleting it
                            self.channels.remove(readable_channel_key);
                            if self.channel_receiver.is_none() {
                                return Ok(None);
                            }
                            continue;
                        }
                        if n < msg_size.len() {
                            readable_channel.read_exact(&mut msg_size[n..]).await?;
                        }
                    }
                    let msg_size = usize::try_from(u32::from_be_bytes(msg_size)).unwrap();

                    buf.resize(msg_size, 0);

                    readable_channel.read_exact(buf.as_mut_slice()).await?;

                    let (path_access, _) = bincode::borrow_decode_from_slice::<PathAccess<'a>, _>(
                        buf.as_slice(),
                        BINCODE_CONFIG,
                    )
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

                    return Ok(Some(path_access));
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SpyInner {
    fixtures: Fixtures,
}

impl SpyInner {
    pub fn init_in_dir(path: &Path) -> io::Result<Self> {
        let coreutils = fixtures::COREUTILS_BINARY.write_to(&path, "")?;
        let bash_path = fixtures::OILS_BINARY.write_to(&path, "")?;
        let interpose_cdylib = fixtures::INTERPOSE_CDYLIB.write_to(&path, ".dylib")?;

        let fixtures = Fixtures {
            bash_path: bash_path.as_path().into(), //Path::new("/opt/homebrew/bin/bash"),//brush.as_path(),
            coreutils_path: coreutils.as_path().into(),
            interpose_cdylib_path: interpose_cdylib.as_path().into(),
        };
        Ok(Self { fixtures })
    }
}

pub(crate) async fn spawn_impl(mut command: Command) -> io::Result<(TokioChild, PathAccessIter)> {
    let (channel_receiver, channel_sender) = UnixStream::pair()?;

    let channel_sender = channel_sender.into_std()?;
    channel_sender.set_nonblocking(false)?;
    let channel_sender = OwnedFd::from(channel_sender);

    let payload = Payload {
        ipc_fd: channel_sender.as_raw_fd(),
        fixtures: command.spy_inner.fixtures.clone(),
    };
    let payload_string = encode_payload(&payload);
    let payload_with_str = PayloadWithEncodedString {
        payload,
        payload_string,
    };
    let bump = Bump::new();
    command.resolve_program()?;
    command.with_info(&bump, |cmd_info| inject(&bump, cmd_info, &payload_with_str))?;

    let mut command = command.into_tokio_command();
    unsafe {
        command.pre_exec(move || {
            update_fd_flag(channel_sender.as_fd(), |flag| {
                flag.remove(FdFlag::FD_CLOEXEC)
            })
        })
    };
    let child = command.spawn()?;
    // drop channel_sender in the parent process,
    // so that channel_receiver reaches eof as soon as the last descendant process exits.
    drop(command);

    Ok((
        child,
        PathAccessIter {
            channel_receiver: Some(channel_receiver),
            channels: Slab::with_capacity(32),
        },
    ))
}

// pub fn spy(
//     program: impl AsRef<OsStr>,
//     cwd: Option<impl AsRef<OsStr>>,
//     arg0: Option<impl AsRef<OsStr>>,
//     args: impl IntoIterator<Item = impl AsRef<OsStr>>,
//     envs: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
// ) -> io::Result<(
//     impl Future<Output = io::Result<ExitStatus>>,
//     PathAccessIter,
// )> {
//     let tmp_dir = temp_dir().join("fspy");
//     let _ = create_dir(&tmp_dir);

//     let ipc_datagram =
//         tempfile::Builder::new().make_in(&tmp_dir, |path| UnixDatagram::bind(path))?;

//     let ipc_fd_string = ipc_datagram.path().to_path_buf();

//     let acc_buf_size = getsockopt(ipc_datagram.as_file(), SndBuf).unwrap();

//     let coreutils = fixtures::COREUTILS_BINARY.write_to(&tmp_dir, "").unwrap();
//     let brush = fixtures::BRUSH_BINARY.write_to(&tmp_dir, "").unwrap();
//     let interpose_cdylib = fixtures::INTERPOSE_CDYLIB
//         .write_to(&tmp_dir, ".dylib")
//         .unwrap();

//     let program = which::which(program).unwrap();
//     let mut bump = Bump::new();

//     let mut arg_vec = Vec::new_in(&bump);

//     let arg0 = if let Some(arg0) = arg0.as_ref() {
//         Some(arg0.as_ref())
//     } else {
//         None
//     };

//     arg_vec.push(arg0.unwrap_or(program.as_os_str()));
//     arg_vec.extend(
//         args.into_iter()
//             .map(|arg| alloc_os_str(&bump, arg.as_ref())),
//     );

//     let mut env_vec = Vec::new_in(&bump);
//     for (name, value) in envs {
//         let name = alloc_os_str(&bump, name.as_ref());
//         // let name = OsStr::from_bytes(SliceExt::to_vec_in(name, &bump).leak());
//         let value = alloc_os_str(&bump, value.as_ref());
//         env_vec.push((name, value));
//     }
//     let mut cmd = command::Command::<'_, &Bump> {
//         program: program.as_path(),
//         args: arg_vec,
//         envs: env_vec,
//     };

//     let context = Context {
//         ipc_fd: ipc_fd_string.as_os_str(),
//         bash: Path::new(
//             "/Users/patr0nus/Downloads/oils-for-unix-0.29.0/_bin/cxx-opt-sh/oils-for-unix",
//         ), //Path::new("/opt/homebrew/bin/bash"),//brush.as_path(),
//         coreutils: coreutils.as_path(),
//         interpose_cdylib: interpose_cdylib.as_path(),
//     };

//     command::interpose_command(&bump, &mut cmd, context).unwrap();

//     let mut os_cmd = TokioCommand::new(cmd.program);
//     os_cmd
//         .arg0(cmd.args[0])
//         .args(&cmd.args[1..])
//         .env_clear()
//         .envs(cmd.envs.iter().copied());

//     if let Some(cwd) = cwd {
//         os_cmd.current_dir(cwd.as_ref());
//     }

//     let status_fut = os_cmd.status();

//     drop(cmd);
//     drop(os_cmd);

//     bump.reset();

//     Ok((
//         status_fut,
//         todo!(),
//     ))
// }

// pub struct Spy {}
