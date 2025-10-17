// use std::{os::unix::process::CommandExt};

// use tokio::process::Command;

#[cfg(target_os = "linux")]
mod syscall_handler;

#[cfg(target_os = "macos")]
mod macos_fixtures;

#[cfg(target_os = "macos")]
use std::path::Path;
use std::{
    io::{self},
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use bincode::borrow_decode_from_slice;
#[cfg(target_os = "linux")]
use fspy_seccomp_unotify::supervisor::supervise;
#[cfg(target_os = "macos")]
use fspy_shared::ipc::NativeString;
use fspy_shared::ipc::{BINCODE_CONFIG, PathAccess, channel::ShmReader};
#[cfg(target_os = "macos")]
use fspy_shared_unix::payload::Fixtures;
use fspy_shared_unix::{
    exec::ExecResolveConfig,
    payload::{Payload, encode_payload},
    spawn::handle_exec,
};
use futures_util::{FutureExt, future::try_join};
use memmap2::Mmap;
use nix::{
    fcntl::{FcntlArg, FdFlag, OFlag, fcntl},
    sys::{
        mman::{shm_open, shm_unlink},
        stat::Mode,
    },
    unistd::{ftruncate, getpid},
};
use tokio::{io::AsyncReadExt, net::UnixStream};

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

        Ok(Self { preload_lib_memfd: Arc::new(preload_lib_memfd) })
    }

    #[cfg(target_os = "macos")]
    pub fn init_in(dir: &Path) -> io::Result<Self> {
        use const_format::formatcp;
        use xxhash_rust::const_xxh3::xxh3_128;

        use crate::fixture::Fixture;
        let coreutils_path = macos_fixtures::COREUTILS_BINARY.write_to(dir, "")?;
        let bash_path = macos_fixtures::OILS_BINARY.write_to(dir, "")?;

        const PRELOAD_CDYLIB: Fixture = Fixture {
            name: "fspy_preload",
            content: PRELOAD_CDYLIB_BINARY,
            hash: formatcp!("{:x}", xxh3_128(PRELOAD_CDYLIB_BINARY)),
        };

        let preload_cdylib_path = PRELOAD_CDYLIB.write_to(dir, ".dylib")?;
        let fixtures = Fixtures {
            bash_path: bash_path.as_path().into(), //Path::new("/opt/homebrew/bin/bash"),//brush.as_path(),
            coreutils_path: coreutils_path.as_path().into(),
        };
        Ok(Self { fixtures, preload_path: preload_cdylib_path.as_path().into() })
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
    shm_reader: ShmReader<Mmap>,
}

impl PathAccessIterable {
    pub fn iter(&self) -> impl Iterator<Item = PathAccess<'_>> {
        let accesses_in_arena =
            self.arenas.iter().flat_map(|arena| arena.borrow_accesses().iter()).copied();

        let accesses_in_shm = self.shm_reader.iter_frames().map(|frame| {
            let (path_access, decoded_size) =
                borrow_decode_from_slice::<PathAccess<'_>, _>(frame, BINCODE_CONFIG).unwrap();
            assert_eq!(decoded_size, frame.len());
            path_access
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

// Shared memory size for storing path accesses.
// 4 GiB is large enough to store path accesses in almost any realistic scenario.
// This doesn't allocate physical memory until it's actually used.
const SHM_SIZE: i64 = 4 * 1024 * 1024 * 1024;

pub(crate) async fn spawn_impl(mut command: Command) -> io::Result<TrackedChild> {
    let (process_exit_sentinel_fd_sender, mut process_exit_sentinel_fd_receiver) =
        UnixStream::pair()?;

    static SHM_ID: AtomicUsize = AtomicUsize::new(0);

    let shm_name =
        format!("/fspy_shm_{}_{}", getpid().as_raw(), SHM_ID.fetch_add(1, Ordering::Relaxed));
    let shm_fd = shm_open(
        shm_name.as_str(),
        OFlag::O_CLOEXEC | OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_EXCL,
        Mode::empty(),
    )?;
    ftruncate(&shm_fd, SHM_SIZE)?;
    shm_unlink(shm_name.as_str())?; // make the shm anonymous and `shm_fd` the only reference to the shm.

    let process_exit_sentinel_fd_sender = process_exit_sentinel_fd_sender.into_std()?;
    process_exit_sentinel_fd_sender.set_nonblocking(false)?;
    let process_exit_sentinel_fd_sender =
        duplicate_until_safe(OwnedFd::from(process_exit_sentinel_fd_sender))?;

    let shm_fd = Arc::new(duplicate_until_safe(shm_fd)?);

    #[cfg(target_os = "linux")]
    let supervisor = supervise::<SyscallHandler>()?;

    #[cfg(target_os = "linux")]
    let supervisor_pre_exec = supervisor.pre_exec;

    let payload = Payload {
        process_exit_sentinel_fd: process_exit_sentinel_fd_sender.as_raw_fd(),
        shm_fd: shm_fd.as_raw_fd(),

        #[cfg(target_os = "macos")]
        fixtures: command.spy_inner.fixtures.clone(),

        #[cfg(target_os = "macos")]
        preload_path: command.spy_inner.preload_path.clone(),

        #[cfg(target_os = "linux")]
        preload_path: format!("/proc/self/fd/{}", command.spy_inner.preload_lib_memfd.as_raw_fd())
            .into(),

        #[cfg(target_os = "linux")]
        seccomp_payload: supervisor.payload,
    };

    let encoded_payload = encode_payload(payload);

    #[cfg(target_os = "linux")]
    let preload_lib_memfd = Arc::clone(&command.spy_inner.preload_lib_memfd);

    let mut exec = command.get_exec();
    let mut exec_resolve_accesses = PathAccessArena::default();
    let pre_exec = handle_exec(
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
        let shm_fd = Arc::clone(&shm_fd);
        tokio_command.pre_exec(move || {
            #[cfg(target_os = "linux")]
            unset_fd_flag(preload_lib_memfd.as_fd(), FdFlag::FD_CLOEXEC)?;
            unset_fd_flag(process_exit_sentinel_fd_sender.as_fd(), FdFlag::FD_CLOEXEC)?;
            unset_fd_flag(shm_fd.as_fd(), FdFlag::FD_CLOEXEC)?;

            #[cfg(target_os = "linux")]
            supervisor_pre_exec.run()?;
            if let Some(pre_exec) = pre_exec.as_ref() {
                pre_exec.run()?;
            }
            Ok(())
        });
    }

    let child = tokio_command.spawn()?;

    drop(tokio_command);
    let shm_fd = Arc::into_inner(shm_fd).expect(
        "pre_exec callback's reference to shm_fd should be dropped along with tokio_command",
    );

    // #[cfg(target_os = "linux")]
    let arenas_future = async move {
        let arenas = std::iter::once(exec_resolve_accesses);
        #[cfg(target_os = "linux")]
        let arenas =
            arenas.chain(supervisor.handling_loop.await?.into_iter().map(|handler| handler.arena));
        io::Result::Ok(arenas.collect::<Vec<_>>())
    };

    let shm_future = async move {
        let mut read_buf = [0u8; 1];

        let read_size = process_exit_sentinel_fd_receiver.read(&mut read_buf).await?;
        // eof reached means the last descendant process has exited.
        assert_eq!(read_size, 0, "the sentinel fd should never be written to");

        io::Result::Ok(shm_fd)
    };

    let accesses_future = async move {
        let (arenas, shm_fd) = try_join(arenas_future, shm_future).await?;
        let shm_mmap = unsafe { Mmap::map(&shm_fd) }?;
        let shm_reader = ShmReader::new(shm_mmap);
        Ok(PathAccessIterable { arenas, shm_reader })
    }
    .boxed();

    Ok(TrackedChild { tokio_child: child, accesses_future })
}
