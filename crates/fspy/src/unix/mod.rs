// use std::{os::unix::process::CommandExt};

// use tokio::process::Command;

#[cfg(target_os = "linux")]
mod syscall_handler;

#[cfg(target_os = "macos")]
mod macos_fixtures;

use std::{io, path::Path};

use bincode::borrow_decode_from_slice;
#[cfg(target_os = "linux")]
use fspy_seccomp_unotify::supervisor::supervise;
use fspy_shared::ipc::{
    BINCODE_CONFIG, NativeString, PathAccess,
    channel::{ReceiverLock, channel},
};
#[cfg(target_os = "macos")]
use fspy_shared_unix::payload::Fixtures;
use fspy_shared_unix::{
    exec::ExecResolveConfig,
    payload::{Payload, encode_payload},
    spawn::handle_exec,
};
use futures_util::FutureExt;
#[cfg(target_os = "linux")]
use syscall_handler::SyscallHandler;

use crate::{Command, TrackedChild, arena::PathAccessArena};

#[derive(Debug, Clone)]
pub struct SpyInner {
    #[cfg(target_os = "macos")]
    fixtures: Fixtures,

    preload_path: NativeString,
}

const PRELOAD_CDYLIB_BINARY: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_FSPY_PRELOAD_UNIX"));

impl SpyInner {
    pub fn init_in(dir: &Path) -> io::Result<Self> {
        use const_format::formatcp;
        use xxhash_rust::const_xxh3::xxh3_128;

        use crate::fixture::Fixture;

        const PRELOAD_CDYLIB: Fixture = Fixture {
            name: "fspy_preload",
            content: PRELOAD_CDYLIB_BINARY,
            hash: formatcp!("{:x}", xxh3_128(PRELOAD_CDYLIB_BINARY)),
        };

        let preload_cdylib_path = PRELOAD_CDYLIB.write_to(dir, ".dylib")?;
        Ok(Self {
            preload_path: preload_cdylib_path.as_path().into(),
            #[cfg(target_os = "macos")]
            fixtures: {
                let coreutils_path = macos_fixtures::COREUTILS_BINARY.write_to(dir, "")?;
                let bash_path = macos_fixtures::OILS_BINARY.write_to(dir, "")?;
                Fixtures {
                    bash_path: bash_path.as_path().into(),
                    coreutils_path: coreutils_path.as_path().into(),
                }
            },
        })
    }
}

pub struct PathAccessIterable {
    arenas: Vec<PathAccessArena>,
    ipc_receiver_lock: ReceiverLock,
}

impl PathAccessIterable {
    pub fn iter(&self) -> impl Iterator<Item = PathAccess<'_>> {
        let accesses_in_arena =
            self.arenas.iter().flat_map(|arena| arena.borrow_accesses().iter()).copied();

        let accesses_in_shm = self.ipc_receiver_lock.iter_frames().map(|frame| {
            let (path_access, decoded_size) =
                borrow_decode_from_slice::<PathAccess<'_>, _>(frame, BINCODE_CONFIG).unwrap();
            assert_eq!(decoded_size, frame.len());
            path_access
        });
        accesses_in_shm.chain(accesses_in_arena)
    }
}

// Shared memory size for storing path accesses.
// 4 GiB is large enough to store path accesses in almost any realistic scenario.
// This doesn't allocate physical memory until it's actually used.
const SHM_CAPACITY: usize = 4 * 1024 * 1024 * 1024;

pub(crate) async fn spawn_impl(mut command: Command) -> io::Result<TrackedChild> {
    #[cfg(target_os = "linux")]
    let supervisor = supervise::<SyscallHandler>()?;

    #[cfg(target_os = "linux")]
    let supervisor_pre_exec = supervisor.pre_exec;

    let (ipc_channel_conf, ipc_receiver) = channel(SHM_CAPACITY)?;

    let payload = Payload {
        ipc_channel_conf,

        #[cfg(target_os = "macos")]
        fixtures: command.spy_inner.fixtures.clone(),

        preload_path: command.spy_inner.preload_path.clone(),

        #[cfg(target_os = "linux")]
        seccomp_payload: supervisor.payload,
    };

    let encoded_payload = encode_payload(payload);

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
        tokio_command.pre_exec(move || {
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

    let arenas_future = async move {
        let arenas = std::iter::once(exec_resolve_accesses);
        #[cfg(target_os = "linux")]
        let arenas =
            arenas.chain(supervisor.handling_loop.await?.into_iter().map(|handler| handler.arena));
        io::Result::Ok(arenas.collect::<Vec<_>>())
    };

    let accesses_future = async move {
        let ipc_receiver_lock = ipc_receiver.lock()?;
        let arenas = arenas_future.await?;
        Ok(PathAccessIterable { arenas, ipc_receiver_lock })
    }
    .boxed();

    Ok(TrackedChild { tokio_child: child, accesses_future })
}
