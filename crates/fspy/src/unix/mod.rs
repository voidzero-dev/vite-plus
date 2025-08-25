// use std::{os::unix::process::CommandExt};

// use tokio::process::Command;

#[cfg(target_os = "linux")]
mod syscall_handler;

#[cfg(target_os = "macos")]
mod macos_fixtures;

#[cfg(target_os = "macos")]
use fspy_shared::ipc::NativeString;
#[cfg(target_os = "macos")]
use fspy_shared_unix::payload::Fixtures;
use fspy_shared_unix::{
    exec::ExecResolveConfig,
    payload::{Payload, encode_payload},
    spawn::handle_exec,
};
use memmap2::Mmap;

#[cfg(target_os = "linux")]
use seccomp_unotify::supervisor::supervise;
#[cfg(target_os = "macos")]
use std::path::Path;
use std::{
    cell::RefCell,
    ffi::{CString, OsStr, OsString},
    fs::File,
    io::{self, Write},
    iter,
    mem::ManuallyDrop,
    ops::{ControlFlow, Deref, DerefMut},
    os::{
        fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
        unix::{
            ffi::{OsStrExt, OsStringExt},
            process::CommandExt,
        },
    },
    sync::{
        Arc, LazyLock,
        atomic::{AtomicU8, AtomicU16, Ordering, fence},
    },
};

#[cfg(target_os = "linux")]
use syscall_handler::SyscallHandler;

use bincode::{borrow_decode_from_slice, error::DecodeError};
use bumpalo::Bump;
use passfd::{FdPassingExt as _, tokio::FdPassingExt as _};

use tokio::{net::UnixStream, process::Child as TokioChild};

use fspy_shared::ipc::{BINCODE_CONFIG, PathAccess};
use futures_util::{FutureExt, future::try_join};
use nix::fcntl::{FcntlArg, FdFlag, OFlag, fcntl};

#[cfg(target_os = "linux")]
use nix::sys::memfd::{MFdFlags, memfd_create};

use crate::{Command, TrackedChild, arena::PathAccessArena};

#[derive(Debug, Clone)]
pub struct SpyInner {
    #[cfg(target_os = "linux")]
    preload_lib_memfd: Arc<OwnedFd>,

    #[cfg(target_os = "macos")]
    fixtures: Fixtures,

    #[cfg(target_os = "macos")]
    preload_path: NativeString,
}

const PRELOAD_CDYLIB_BINARY: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_FSPY_PRELOAD_UNIX"));

impl SpyInner {
    #[cfg(target_os = "linux")]
    pub fn init() -> io::Result<Self> {
        let preload_lib_memfd = memfd_create("fspy_preload", MFdFlags::MFD_CLOEXEC)?;
        let mut execve_host_memfile = File::from(preload_lib_memfd);
        execve_host_memfile.write_all(PRELOAD_CDYLIB_BINARY)?;

        let preload_lib_memfd = duplicate_until_safe(OwnedFd::from(execve_host_memfile))?;

        Ok(Self {
            preload_lib_memfd: Arc::new(preload_lib_memfd),
        })
    }

    #[cfg(target_os = "macos")]
    pub fn init_in(dir: &Path) -> io::Result<Self> {
        use crate::fixture::Fixture;
        use const_format::formatcp;
        use xxhash_rust::const_xxh3::xxh3_128;
        let coreutils_path = macos_fixtures::COREUTILS_BINARY.write_to(&dir, "")?;
        let bash_path = macos_fixtures::OILS_BINARY.write_to(&dir, "")?;

        const PRELOAD_CDYLIB: Fixture = Fixture {
            name: "fspy_preload",
            content: PRELOAD_CDYLIB_BINARY,
            hash: formatcp!("{:x}", xxh3_128(PRELOAD_CDYLIB_BINARY)),
        };

        let preload_cdylib_path = PRELOAD_CDYLIB.write_to(&dir, ".dylib")?;
        let fixtures = Fixtures {
            bash_path: bash_path.as_path().into(), //Path::new("/opt/homebrew/bin/bash"),//brush.as_path(),
            coreutils_path: coreutils_path.as_path().into(),
        };
        Ok(Self {
            fixtures,
            preload_path: preload_cdylib_path.as_path().into(),
        })
    }
}

fn unset_fd_flag(fd: BorrowedFd<'_>, flag_to_remove: FdFlag) -> io::Result<()> {
    fcntl(
        fd,
        FcntlArg::F_SETFD({
            let mut fd_flag = FdFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFD)?);
            fd_flag.remove(flag_to_remove);
            fd_flag
        }),
    )?;
    Ok(())
}
// fn unset_fl_flag(fd: BorrowedFd<'_>, flag_to_remove: OFlag) -> io::Result<()> {
//     fcntl(
//         fd,
//         FcntlArg::F_SETFL({
//             let mut fd_flag = OFlag::from_bits_retain(fcntl(fd, FcntlArg::F_GETFL)?);
//             fd_flag.remove(flag_to_remove);
//             fd_flag
//         }),
//     )?;
//     Ok(())
// }

pub struct PathAccessIterable {
    arenas: Vec<PathAccessArena>,
    shm_mmaps: Vec<Mmap>,
}

impl PathAccessIterable {
    pub fn iter(&self) -> impl Iterator<Item = PathAccess<'_>> {
        let accesses_in_arena = self
            .arenas
            .iter()
            .flat_map(|arena| arena.borrow_accesses().iter())
            .copied();

        let accesses_in_shm = self.shm_mmaps.iter().flat_map(|mmap| {
            let buf = mmap.deref();
            let mut position = 0usize;
            iter::from_fn(move || {
                let (flag_buf, data_buf) = buf[position..].split_first()?;
                let atomic_flag = unsafe { AtomicU8::from_ptr((flag_buf as *const u8).cast_mut()) };
                let flag = atomic_flag.load(Ordering::Acquire);
                if flag == 0 {
                    return None;
                };
                fence(Ordering::Acquire);
                let (path_access, decoded_size) =
                    borrow_decode_from_slice::<PathAccess<'_>, _>(data_buf, BINCODE_CONFIG)
                        .unwrap();

                position += decoded_size + 1;

                Some(path_access)
            })
        });
        accesses_in_shm.chain(accesses_in_arena)
    }
}

// https://github.com/nodejs/node/blob/5794e644b724c6c6cac02d306d87a4d6b78251e5/deps/uv/src/unix/core.c#L803-L808
fn duplicate_until_safe(mut fd: OwnedFd) -> io::Result<OwnedFd> {
    let mut fds: Vec<OwnedFd> = vec![];
    const SAFE_FD_NUM: RawFd = 17;
    while fd.as_raw_fd() < SAFE_FD_NUM {
        let new_fd = fd.try_clone()?;
        fds.push(fd);
        fd = new_fd;
    }
    Ok(fd)
}

pub(crate) async fn spawn_impl(mut command: Command) -> io::Result<TrackedChild> {
    let (shm_fd_sender, shm_fd_receiver) = UnixStream::pair()?;

    let shm_fd_sender = shm_fd_sender.into_std()?;
    shm_fd_sender.set_nonblocking(false)?;
    let shm_fd_sender = duplicate_until_safe(OwnedFd::from(shm_fd_sender))?;

    #[cfg(target_os = "linux")]
    let supervisor = supervise::<SyscallHandler>()?;

    #[cfg(target_os = "linux")]
    let mut supervisor_pre_exec = supervisor.pre_exec;

    let payload = Payload {
        ipc_fd: shm_fd_sender.as_raw_fd(),

        #[cfg(target_os = "macos")]
        fixtures: command.spy_inner.fixtures.clone(),

        #[cfg(target_os = "macos")]
        preload_path: command.spy_inner.preload_path.clone(),

        #[cfg(target_os = "linux")]
        preload_path: format!(
            "/proc/self/fd/{}",
            command.spy_inner.preload_lib_memfd.as_raw_fd()
        )
        .into(),

        #[cfg(target_os = "linux")]
        seccomp_payload: supervisor.payload,
    };

    let encoded_payload = encode_payload(payload);

    #[cfg(target_os = "linux")]
    let preload_lib_memfd = Arc::clone(&command.spy_inner.preload_lib_memfd);

    let mut exec = command.get_exec();
    let mut exec_resolve_accesses = PathAccessArena::default();
    let mut pre_exec = handle_exec(
        &mut exec,
        ExecResolveConfig::search_path_enabled(None),
        &encoded_payload,
        |path_access| {
            exec_resolve_accesses.add(path_access);
        },
    )?;
    command.set_exec(exec);

    let mut tokio_command = command.into_tokio_command();

    unsafe {
        tokio_command.pre_exec(move || {
            #[cfg(target_os = "linux")]
            unset_fd_flag(preload_lib_memfd.as_fd(), FdFlag::FD_CLOEXEC)?;
            unset_fd_flag(shm_fd_sender.as_fd(), FdFlag::FD_CLOEXEC)?;

            #[cfg(target_os = "linux")]
            supervisor_pre_exec.run()?;
            if let Some(pre_exec) = &mut pre_exec {
                pre_exec.run()?;
            }
            Ok(())
        });
    }

    let child = tokio_command.spawn()?;
    // drop channel_sender in the parent process,
    // so that channel_receiver reaches eof as soon as the last descendant process exits.
    drop(tokio_command);

    // #[cfg(target_os = "linux")]
    let arenas_future = async move {
        let arenas = std::iter::once(exec_resolve_accesses);
        #[cfg(target_os = "linux")]
        let arenas = arenas.chain(
            supervisor
                .handling_loop
                .await?
                .into_iter()
                .map(|handler| handler.arena),
        );
        io::Result::Ok(arenas.collect::<Vec<_>>())
    };

    let shm_future = async move {
        let mut shm_fds = Vec::<OwnedFd>::new();
        loop {
            let shm_fd = match shm_fd_receiver.recv_fd().await {
                Ok(fd) => unsafe { OwnedFd::from_raw_fd(fd) },
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    } else {
                        return Err(err);
                    }
                }
            };
            shm_fds.push(shm_fd);
        }
        io::Result::Ok(shm_fds)
    };

    let accesses_future = async move {
        let (arenas, shm_fds) = try_join(arenas_future, shm_future).await?;
        let shm_mmaps = shm_fds
            .into_iter()
            .map(|fd| unsafe { Mmap::map(&fd) })
            .collect::<io::Result<Vec<Mmap>>>()?;
        Ok(PathAccessIterable { arenas, shm_mmaps })
    }
    .boxed();

    Ok(TrackedChild {
        tokio_child: child,
        accesses_future,
    })
}
