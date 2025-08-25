pub mod convert;
pub mod raw_exec;

use core::panic;
use std::{
    borrow::Cow,
    cell::{Ref, RefCell},
    ffi::CStr,
    fmt::Debug,
    io,
    ops::DerefMut as _,
    os::{
        fd::{AsRawFd, RawFd},
        unix::ffi::OsStrExt,
    },
    ptr::null,
    sync::{
        LazyLock, OnceLock,
        atomic::{AtomicU8, AtomicU16, AtomicUsize, Ordering, fence},
    },
    thread::panicking,
    time::{Instant, SystemTime},
};

use anyhow::Context;
use bincode::{
    enc::write::SizeWriter, encode_into_slice, encode_into_std_write, encode_into_writer,
};
use bstr::BStr;
use fspy_shared::ipc::{AccessMode, BINCODE_CONFIG, NativeStr, NativeString, PathAccess};
use fspy_shared_unix::{
    exec::ExecResolveConfig,
    payload::{EncodedPayload, decode_payload_from_env},
    spawn::{PreExec, handle_exec},
};

use convert::{ToAbsolutePath, ToAccessMode};
use libc::{off_t, pthread_atfork};
use memmap2::{Mmap, MmapMut};
use nix::{
    fcntl::OFlag,
    sys::{
        mman::{shm_open, shm_unlink},
        stat::Mode,
    },
    time::{ClockId, clock_gettime},
    unistd::{Pid, ftruncate, getpid},
};
use passfd::FdPassingExt;
use raw_exec::RawExec;
use thread_local::ThreadLocal;

#[derive(Debug)]
struct ShmCursor {
    mmap_mut: MmapMut,
    position: usize,
}
impl ShmCursor {
    pub fn advance(&mut self, len: usize) -> Option<&mut [u8]> {
        let new_position = self.position.checked_add(len)?;
        if new_position > self.mmap_mut.len() {
            return None;
        };
        let buf = &mut self.mmap_mut[self.position..new_position];
        self.position = new_position;
        Some(buf)
    }
}

pub struct Client {
    encoded_payload: EncodedPayload,
    shm_id: AtomicUsize,
    tls_shm_cursor: ThreadLocal<RefCell<ShmCursor>>,

    #[cfg(target_os = "macos")]
    posix_spawn_file_actions: OnceLock<libc::posix_spawn_file_actions_t>,
}

#[cfg(target_os = "macos")]
unsafe impl Sync for Client {}
#[cfg(target_os = "macos")]
unsafe impl Send for Client {}

impl Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish()
    }
}

const SHM_CHUNK_SIZE: off_t = 256 * 1024;

impl Client {
    fn from_env() -> Self {
        let encoded_payload = decode_payload_from_env().unwrap();
        Self {
            shm_id: AtomicUsize::new(0),
            encoded_payload,
            tls_shm_cursor: ThreadLocal::new(),
            #[cfg(target_os = "macos")]
            posix_spawn_file_actions: OnceLock::new(),
        }
    }
    fn new_shm(&self) -> io::Result<ShmCursor> {
        let shm_name = format!(
            "/fspy_shm_{}_{}",
            getpid().as_raw(),
            self.shm_id.fetch_add(1, Ordering::Relaxed),
        );
        let shm_fd = shm_open(
            shm_name.as_str(),
            OFlag::O_CLOEXEC | OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_EXCL,
            Mode::empty(),
        )?;
        shm_unlink(shm_name.as_str())?;
        self.encoded_payload
            .payload
            .ipc_fd
            .send_fd(shm_fd.as_raw_fd())?;
        ftruncate(&shm_fd, SHM_CHUNK_SIZE)?;
        let mmap_mut = unsafe { MmapMut::map_mut(&shm_fd) }?;
        Ok(ShmCursor {
            mmap_mut,
            position: 0,
        })
    }

    fn with_shm_buf<R>(
        &self,
        len: usize,
        f: impl FnOnce(&mut [u8]) -> anyhow::Result<R>,
    ) -> anyhow::Result<R> {
        let shm_buf = self
            .tls_shm_cursor
            .get_or_try(|| io::Result::Ok(RefCell::new(self.new_shm()?)))?;

        let mut shm_buf = shm_buf.borrow_mut();
        if let Some(buf) = shm_buf.advance(len) {
            f(buf)
        } else {
            *shm_buf = self.new_shm()?;
            let buf = shm_buf.advance(len).with_context(|| {
                format!(
                    "The requested buf ({}) is greater than the shm chunk size ({})",
                    len, SHM_CHUNK_SIZE
                )
            })?;
            f(buf)
        }
    }

    fn send(&self, path_access: PathAccess<'_>) -> anyhow::Result<()> {
        let path = path_access.path.as_bstr();
        if path.starts_with(b"/dev/")
            || (cfg!(target_os = "linux")
                && (path.starts_with(b"/proc/") || path.starts_with(b"/sys/")))
        {
            return Ok(());
        };
        let mut size_writer = SizeWriter::default();
        encode_into_writer(&path_access, &mut size_writer, BINCODE_CONFIG)?;

        self.with_shm_buf(1 + size_writer.bytes_written, |buf| {
            let data_buf = &mut buf[1..];
            let written_size = encode_into_slice(&path_access, data_buf, BINCODE_CONFIG)?;
            debug_assert_eq!(written_size, size_writer.bytes_written);

            let flag_ptr = buf.as_mut_ptr().cast::<u8>();
            fence(Ordering::Release);
            unsafe { AtomicU8::from_ptr(flag_ptr) }.store(1, Ordering::Release);
            Ok(())
        })?;

        Ok(())
    }

    pub unsafe fn handle_exec<R>(
        &self,
        config: ExecResolveConfig,
        raw_exec: RawExec,
        f: impl FnOnce(RawExec, Option<PreExec>) -> nix::Result<R>,
    ) -> nix::Result<R> {
        let mut exec = unsafe { raw_exec.to_exec() };
        let pre_exec = handle_exec(&mut exec, config, &self.encoded_payload, |path_access| {
            self.send(path_access).unwrap();
        })?;
        RawExec::from_exec(exec, |raw_command| f(raw_command, pre_exec))
    }

    pub unsafe fn try_handle_open(
        &self,
        path: impl ToAbsolutePath,
        mode: impl ToAccessMode,
    ) -> anyhow::Result<()> {
        let mode = unsafe { mode.to_access_mode() };
        let () = unsafe {
            path.to_absolute_path(|abs_path| {
                let Some(abs_path) = abs_path else {
                    return Ok(Ok(()));
                };
                Ok(self.send(PathAccess {
                    mode,
                    path: abs_path.into(),
                }))
            })
        }??;

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub unsafe fn handle_posix_spawn_opts(
        &self,
        _file_actions: &mut *const libc::posix_spawn_file_actions_t,
        _attrp: *const libc::posix_spawnattr_t,
    ) -> nix::Result<()> {
        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub unsafe fn handle_posix_spawn_opts(
        &self,
        file_actions: &mut *const libc::posix_spawn_file_actions_t,
        attrp: *const libc::posix_spawnattr_t,
    ) -> nix::Result<()> {
        use core::mem::zeroed;
        use libc::c_short;
        let cloexec_default = if attrp.is_null() {
            false
        } else {
            let mut flags = 0;
            let ret = unsafe { libc::posix_spawnattr_getflags(attrp, &mut flags) };
            if ret != 0 {
                return Err(nix::Error::from_raw(ret));
            }
            (flags & (libc::POSIX_SPAWN_CLOEXEC_DEFAULT as c_short)) != 0
        };

        if !cloexec_default {
            return Ok(());
        }

        unsafe extern "C" {
            unsafe fn posix_spawn_file_actions_addinherit_np(
                actions: *mut libc::posix_spawn_file_actions_t,
                fd: libc::c_int,
            ) -> libc::c_int;
        }

        // ensure ipc fd is inherited when POSIX_SPAWN_CLOEXEC_DEFAULT is set.
        if (*file_actions).is_null() {
            let shared_file_actions = self.posix_spawn_file_actions.get_or_init(|| {
                let mut fa: libc::posix_spawn_file_actions_t = unsafe { zeroed() };
                let ret = unsafe { libc::posix_spawn_file_actions_init(&mut fa) };
                assert_eq!(ret, 0);
                let ret = unsafe {
                    posix_spawn_file_actions_addinherit_np(
                        &mut fa,
                        self.encoded_payload.payload.ipc_fd,
                    )
                };
                assert_eq!(ret, 0);
                fa
            });
            *file_actions = shared_file_actions;
        } else {
            // this makes `file_actions` list grow indefinitely if it keeps being reused to spawn processes,
            // but I can't think of a better way. (no way to inspect or clone `file_actions`)
            let ret = unsafe {
                posix_spawn_file_actions_addinherit_np(
                    (*file_actions).cast_mut(),
                    self.encoded_payload.payload.ipc_fd,
                )
            };
            if ret != 0 {
                return Err(nix::Error::from_raw(ret));
            }
        }
        Ok(())
        //  posix_spawn_file_actions_addclose(actions, fd, path, oflag, mode)
    }
}

static CLIENT: OnceLock<Client> = OnceLock::new();

pub fn global_client() -> Option<&'static Client> {
    CLIENT.get()
}

pub unsafe fn handle_open(path: impl ToAbsolutePath, mode: impl ToAccessMode) {
    if let Some(client) = global_client() {
        unsafe { client.try_handle_open(path, mode) }.unwrap();
    }
}

#[cfg(not(test))]
#[ctor::ctor]
fn init_client() {
    CLIENT.set(Client::from_env()).unwrap();
    unsafe extern "C" fn reset_shm_atfork() {
        let Some(client) = global_client() else {
            return;
        };
        if let Some(shm_cursor) = client.tls_shm_cursor.get() {
            // Move the shm cursor to the end so that the next time it's used it will be reset.
            let mut shm_cursor = shm_cursor.borrow_mut();
            shm_cursor.position = shm_cursor.mmap_mut.len();
        }
    }
    let ret = unsafe { pthread_atfork(None, None, Some(reset_shm_atfork)) };
    if ret != 0 {
        panic!("pthread_atfork failed: {}", ret);
    }
}
