pub mod convert;
pub mod raw_exec;

use std::{fmt::Debug, num::NonZeroUsize, sync::OnceLock};

use anyhow::Context;
use bincode::{enc::write::SizeWriter, encode_into_slice, encode_into_writer};
use fspy_shared::ipc::{BINCODE_CONFIG, PathAccess, shm_io::ShmWriter};
use fspy_shared_unix::{
    exec::ExecResolveConfig,
    payload::EncodedPayload,
    spawn::{PreExec, handle_exec},
};

use convert::{ToAbsolutePath, ToAccessMode};
use memmap2::MmapRaw;
use raw_exec::RawExec;

pub struct Client {
    encoded_payload: EncodedPayload,
    shm_writer: ShmWriter<MmapRaw>,

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

impl Client {
    #[cfg(not(test))]
    fn from_env() -> Self {
        use fspy_shared_unix::payload::decode_payload_from_env;

        let encoded_payload = decode_payload_from_env().unwrap();

        let shm_mmap_mut = MmapRaw::map_raw(encoded_payload.payload.shm_fd)
            .expect("fspy: failed to mmap shared memory for ipc");

        let shm_writer = unsafe { ShmWriter::new(shm_mmap_mut) };

        Self {
            shm_writer,
            encoded_payload,
            #[cfg(target_os = "macos")]
            posix_spawn_file_actions: OnceLock::new(),
        }
    }

    fn send(&self, path_access: PathAccess<'_>) -> anyhow::Result<()> {
        let path = path_access.path.as_bstr();
        if path.starts_with(b"/dev/")
            || (cfg!(target_os = "linux")
                && (path.starts_with(b"/proc/") || path.starts_with(b"/sys/")))
        {
            return Ok(());
        }
        let mut size_writer = SizeWriter::default();
        encode_into_writer(path_access, &mut size_writer, BINCODE_CONFIG)?;

        let frame_size = NonZeroUsize::new(size_writer.bytes_written)
            .expect("fspy: encoded PathAccess should never be empty");

        let mut frame = self.shm_writer.claim_frame(frame_size).expect("fspy: shm buffer overflow");
        let written_size = encode_into_slice(&path_access, &mut frame, BINCODE_CONFIG)?;
        assert_eq!(written_size, size_writer.bytes_written);

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
                Ok(self.send(PathAccess { mode, path: abs_path.into() }))
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
        unsafe extern "C" {
            unsafe fn posix_spawn_file_actions_addinherit_np(
                actions: *mut libc::posix_spawn_file_actions_t,
                fd: libc::c_int,
            ) -> libc::c_int;
        }

        use core::mem::zeroed;

        use libc::c_short;
        let cloexec_default = if attrp.is_null() {
            false
        } else {
            let mut flags = 0;
            let ret = unsafe { libc::posix_spawnattr_getflags(attrp, &raw mut flags) };
            if ret != 0 {
                return Err(nix::Error::from_raw(ret));
            }
            (flags & (libc::POSIX_SPAWN_CLOEXEC_DEFAULT as c_short)) != 0
        };

        if !cloexec_default {
            return Ok(());
        }

        // ensure ipc fd is inherited when POSIX_SPAWN_CLOEXEC_DEFAULT is set.
        if (*file_actions).is_null() {
            let shared_file_actions = self.posix_spawn_file_actions.get_or_init(|| {
                let mut fa: libc::posix_spawn_file_actions_t = unsafe { zeroed() };
                let ret = unsafe { libc::posix_spawn_file_actions_init(&raw mut fa) };
                assert_eq!(ret, 0);
                let ret = unsafe {
                    posix_spawn_file_actions_addinherit_np(
                        &mut fa,
                        self.encoded_payload.payload.process_exit_sentinel_fd,
                    )
                };
                assert_eq!(ret, 0);
                let ret = unsafe {
                    posix_spawn_file_actions_addinherit_np(
                        &mut fa,
                        self.encoded_payload.payload.shm_fd,
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
                    self.encoded_payload.payload.process_exit_sentinel_fd,
                )
            };
            if ret != 0 {
                return Err(nix::Error::from_raw(ret));
            }
            let ret = unsafe {
                posix_spawn_file_actions_addinherit_np(
                    (*file_actions).cast_mut(),
                    self.encoded_payload.payload.shm_fd,
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
    use libc::pthread_atfork;

    CLIENT.set(Client::from_env()).unwrap();
}
