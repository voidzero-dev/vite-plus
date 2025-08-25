use std::{
    convert::Infallible,
    env::temp_dir,
    ffi::{CStr, c_char, c_void},
    fs::{File, OpenOptions, create_dir},
    io, mem,
    os::windows::{ffi::OsStrExt, io::AsRawHandle, process::ChildExt as _},
    path::Path,
    ptr::{null, null_mut},
    str::from_utf8,
    sync::Arc,
};

use crate::{TrackedChild, arena::PathAccessArena, command::Command};
use bincode::borrow_decode_from_slice;
use const_format::formatcp;
use fspy_shared::{
    ipc::{BINCODE_CONFIG, PathAccess},
    windows::{PAYLOAD_ID, Payload},
};
use futures_util::{
    FutureExt, Stream,
    stream::try_unfold,
};
use ms_detours::{DetourCopyPayloadToProcess, DetourUpdateProcessWithDll};
use tokio::{
    io::AsyncReadExt,
    net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions},
};
// use detours_sys2::{DetourAttach,};

use winapi::{
    shared::minwindef::{FALSE, TRUE},
    um::{
        handleapi::DuplicateHandle,
        processthreadsapi::{GetCurrentProcess, ResumeThread},
        winbase::CREATE_SUSPENDED,
        winnt::{DUPLICATE_SAME_ACCESS},
    },
};
// use windows_sys::Win32::System::Threading::{CREATE_SUSPENDED, ResumeThread};
use winsafe::co::{CP, WC};
use xxhash_rust::const_xxh3::xxh3_128;

use crate::fixture::{Fixture, fixture};


const PRELOAD_CDYLIB_BINARY: &[u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_FSPY_PRELOAD_WINDOWS"));
const INTERPOSE_CDYLIB: Fixture =  Fixture {
            name: "fsyp_preload",
            content: PRELOAD_CDYLIB_BINARY,
            hash: formatcp!("{:x}", xxh3_128(PRELOAD_CDYLIB_BINARY)),
        };

fn luid() -> io::Result<u64> {
    let mut luid = unsafe { std::mem::zeroed::<winapi::um::winnt::LUID>() };
    let ret = unsafe { winapi::um::securitybaseapi::AllocateLocallyUniqueId(&mut luid) };
    if ret == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((u64::from(luid.HighPart as u32)) << 32 | u64::from(luid.LowPart))
}

fn named_pipe_server_stream(
    opts: ServerOptions,
    addr: String,
) -> io::Result<impl Stream<Item = io::Result<NamedPipeServer>>> {
    let server = opts.clone().first_pipe_instance(true).create(&addr)?;
    Ok(try_unfold(
        (opts, server, addr),
        |(opts, mut server, addr)| async move {
            server.connect().await?;
            let connected_client = server;
            server = opts.create(&addr)?;
            io::Result::Ok(Some((connected_client, (opts, server, addr))))
        },
    ))
}

pub struct PathAccessIterable {
    arena: PathAccessArena,
    // pipe_receiver: NamedPipeServer,
}

const MESSAGE_MAX_LEN: usize = 4096;

impl PathAccessIterable {
    pub fn iter(&self) -> impl Iterator<Item = PathAccess<'_>> {
        self.arena.borrow_accesses().iter().copied()
    }
    //     pub async fn next<'a>(&mut self, buf: &'a mut Vec<u8>) -> io::Result<Option<PathAccess<'a>>> {
    //         buf.resize(MESSAGE_MAX_LEN, 0);
    //         let n = self.pipe_receiver.read(buf.as_mut_slice()).await?;
    //         if n == 0 {
    //             return Ok(None);
    //         }
    //         let msg = &buf[..n];
    //         let (path_access, decoded_len) =
    //             borrow_decode_from_slice::<'_, PathAccess, _>(msg, BINCODE_CONFIG).unwrap();
    //         assert_eq!(decoded_len, msg.len());
    //         Ok(Some(path_access))
    //     }
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
        Ok(Self {
            asni_dll_path_with_nul: asni_dll_path_with_nul.into(),
        })
    }
}

pub(crate) async fn spawn_impl(mut command: Command) -> io::Result<TrackedChild> {
    let asni_dll_path_with_nul = Arc::clone(&command.spy_inner.asni_dll_path_with_nul);
    let mut command = command.into_tokio_command();

    command.creation_flags(CREATE_SUSPENDED);

    let pipe_name = format!(r"\\.\pipe\fspy_ipc_{:x}", luid()?);

    let mut pipe_receiver = ServerOptions::new()
        .pipe_mode(PipeMode::Message)
        .access_outbound(false)
        .access_inbound(true)
        .in_buffer_size(1024)
        // .out_buffer_size(100 * 1024 * 1024)
        .create(&pipe_name)?;

    let connect_fut = pipe_receiver.connect();

    let pipe_sender = OpenOptions::new().write(true).open(&pipe_name).unwrap();

    connect_fut.await?;

    let accesses_future = async move {
        let mut arena = PathAccessArena::default();

        let mut buf = [0u8; MESSAGE_MAX_LEN];
        loop {
            let n = pipe_receiver.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            let msg = &buf[..n];
            let (path_access, decoded_len) =
                borrow_decode_from_slice::<'_, PathAccess, _>(msg, BINCODE_CONFIG).unwrap();
            assert_eq!(decoded_len, msg.len());
            arena.add(path_access);
        }
        io::Result::Ok(PathAccessIterable { arena })
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

        let mut handle_in_child: *mut c_void = null_mut();
        let ret = unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                pipe_sender.as_raw_handle(),
                process_handle,
                &mut handle_in_child,
                0,
                FALSE,
                DUPLICATE_SAME_ACCESS,
            )
        };
        if ret == 0 {
            return Err(io::Error::last_os_error());
        }

        let payload = Payload {
            pipe_handle: handle_in_child.addr(),
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

    drop(pipe_sender);
    Ok(TrackedChild {
        tokio_child: child,
        accesses_future,
    })
}
