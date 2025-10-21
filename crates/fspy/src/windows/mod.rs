use std::{
    ffi::{CStr, c_char},
    io,
    os::windows::{ffi::OsStrExt, io::AsRawHandle, process::ChildExt as _},
    path::Path,
    sync::Arc,
};

use const_format::formatcp;
use fspy_detours_sys::{DetourCopyPayloadToProcess, DetourUpdateProcessWithDll};
use fspy_shared::{
    ipc::{BINCODE_CONFIG, PathAccess, channel::channel},
    windows::{PAYLOAD_ID, Payload},
};
use futures_util::FutureExt;
use winapi::{
    shared::minwindef::TRUE,
    um::{processthreadsapi::ResumeThread, winbase::CREATE_SUSPENDED},
};
use winsafe::co::{CP, WC};
use xxhash_rust::const_xxh3::xxh3_128;

use crate::{
    TrackedChild,
    command::Command,
    fixture::Fixture,
    ipc::{OwnedReceiverLockGuard, SHM_CAPACITY},
};

const PRELOAD_CDYLIB_BINARY: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_FSPY_PRELOAD_WINDOWS"));
const INTERPOSE_CDYLIB: Fixture = Fixture::new(
    "fsyp_preload",
    PRELOAD_CDYLIB_BINARY,
    formatcp!("{:x}", xxh3_128(PRELOAD_CDYLIB_BINARY)),
);

pub struct PathAccessIterable {
    ipc_receiver_lock_guard: OwnedReceiverLockGuard,
}

impl PathAccessIterable {
    pub fn iter(&self) -> impl Iterator<Item = PathAccess<'_>> {
        self.ipc_receiver_lock_guard.iter_path_acceses()
    }
}

// pub struct TracedProcess {
//     pub child: Child,
//     pub path_access_stream: PathAccessIter,
// }

#[derive(Debug, Clone)]
pub struct SpyInner {
    asni_dll_path_with_nul: Arc<CStr>,
}

impl SpyInner {
    pub fn init_in(path: &Path) -> io::Result<Self> {
        let dll_path = INTERPOSE_CDYLIB.write_to(&path, ".dll").unwrap();

        let wide_dll_path = dll_path.as_os_str().encode_wide().collect::<Vec<u16>>();
        let mut asni_dll_path =
            winsafe::WideCharToMultiByte(CP::ACP, WC::NoValue, &wide_dll_path, None, None)
                .map_err(|err| io::Error::from_raw_os_error(err.raw() as i32))?;

        asni_dll_path.push(0);

        let asni_dll_path_with_nul =
            unsafe { CStr::from_bytes_with_nul_unchecked(asni_dll_path.as_slice()) };
        Ok(Self { asni_dll_path_with_nul: asni_dll_path_with_nul.into() })
    }
}

pub(crate) async fn spawn_impl(command: Command) -> io::Result<TrackedChild> {
    let asni_dll_path_with_nul = Arc::clone(&command.spy_inner.asni_dll_path_with_nul);
    let mut command = command.into_tokio_command();

    command.creation_flags(CREATE_SUSPENDED);

    let (channel_conf, receiver) = channel(SHM_CAPACITY)?;

    let accesses_future = async move {
        let ipc_receiver_lock_guard = OwnedReceiverLockGuard::lock_async(receiver).await?;
        io::Result::Ok(PathAccessIterable { ipc_receiver_lock_guard })
    }
    .boxed();

    // let path_access_stream = PathAccessIterable { pipe_receiver };

    let child = command.spawn_with(|std_command| {
        let std_child = std_command.spawn()?;

        let mut dll_paths = asni_dll_path_with_nul.as_ptr().cast::<c_char>();
        let process_handle = std_child.as_raw_handle().cast::<winapi::ctypes::c_void>();
        let success = unsafe { DetourUpdateProcessWithDll(process_handle, &mut dll_paths, 1) };
        if success != TRUE {
            return Err(io::Error::last_os_error());
        }

        let payload = Payload {
            channel_conf: channel_conf.clone(),
            asni_dll_path_with_nul: asni_dll_path_with_nul.to_bytes(),
        };
        let payload_bytes = bincode::encode_to_vec(payload, BINCODE_CONFIG).unwrap();
        let success = unsafe {
            DetourCopyPayloadToProcess(
                process_handle,
                &PAYLOAD_ID,
                payload_bytes.as_ptr().cast(),
                payload_bytes.len().try_into().unwrap(),
            )
        };
        if success != TRUE {
            return Err(io::Error::last_os_error());
        }

        let main_thread_handle = std_child.main_thread_handle();
        let resume_thread_ret =
            unsafe { ResumeThread(main_thread_handle.as_raw_handle().cast()) } as i32;

        if resume_thread_ret == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(std_child)
    })?;

    Ok(TrackedChild { tokio_child: child, accesses_future })
}
