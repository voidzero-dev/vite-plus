use core::slice;
use std::{
    cell::{RefCell, SyncUnsafeCell},
    convert::identity,
    env,
    ffi::{CStr, OsStr},
    io::{IoSlice, PipeWriter, Write},
    mem::zeroed,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        unix::{ffi::OsStrExt as _, net::UnixDatagram},
    },
    path::Path,
    ptr::null,
    sync::{LazyLock, OnceLock, atomic::AtomicBool},
};

use allocator_api2::vec::Vec;
use bincode::config;
use bstr::BStr;
use bumpalo::Bump;
use libc::c_short;
use nix::{errno::Errno, fcntl::AtFlags};
use passfd::FdPassingExt;
use smallvec::SmallVec;

use fspy_shared::{
    ipc::{AccessMode, NativeStr, PathAccess},
    macos::{
        PAYLOAD_ENV_NAME, decode_payload,
        inject::{PayloadWithEncodedString, inject},
    },
    unix::cmdinfo::CommandInfo,
};

// use super::command::{CommandInfo, Context, inject};

pub struct Client {
    payload_with_str: PayloadWithEncodedString,
    posix_spawn_file_actions: OnceLock<libc::posix_spawn_file_actions_t>,
}

unsafe impl Sync for Client {}

impl Client {
    fn from_env() -> Self {
        let payload_string = env::var_os(PAYLOAD_ENV_NAME).unwrap();
        let payload = decode_payload(&payload_string);
        Self {
            payload_with_str: PayloadWithEncodedString {
                payload,
                payload_string,
            },
            posix_spawn_file_actions: OnceLock::new(),
        }
    }
    pub unsafe fn handle_exec(&self, bump: &Bump, raw_command: &mut RawCommand) -> nix::Result<()> {
        let mut cmd = unsafe { raw_command.into_command(bump) };
        inject(bump, &mut cmd, &self.payload_with_str)?;
        *raw_command = RawCommand::from_command(bump, &cmd);
        Ok(())
    }
    pub unsafe fn handle_posix_spawn_opts(
        &self,
        file_actions: &mut *const libc::posix_spawn_file_actions_t,
        attrp: *const libc::posix_spawnattr_t,
    ) -> nix::Result<()> {
        let cloexec_default = if attrp.is_null() {
            false
        } else {
            let mut flags = 0;
            let ret = unsafe { libc::posix_spawnattr_getflags(attrp, &mut flags) };
            if ret != 0 {
                return Err(Errno::from_raw(ret));
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
                        self.payload_with_str.payload.ipc_fd,
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
                    self.payload_with_str.payload.ipc_fd,
                )
            };
            if ret != 0 {
                return Err(Errno::from_raw(ret));
            }
        }
        Ok(())
        //  posix_spawn_file_actions_addclose(actions, fd, path, oflag, mode)
    }
    pub fn send(&self, mode: AccessMode, path: &BStr) {
        if path.starts_with(b"/dev/") {
            return;
        }
        let mut msg_buf = SmallVec::<u8, 256>::new();
        msg_buf.extend_from_slice(&0u32.to_be_bytes());

        let msg = PathAccess {
            mode,
            path: NativeStr::from_bytes(&path),
            // dir: None,
        };

        let msg_size =
            bincode::encode_into_std_write(msg, &mut msg_buf, config::standard()).unwrap();
        let msg_size = u32::try_from(msg_size).unwrap().to_be_bytes();

        msg_buf[..msg_size.len()].copy_from_slice(&msg_size);

        TLS_CHANNEL.with_borrow_mut(|writer| writer.write_all(&msg_buf).unwrap());
    }
}

thread_local! { static TLS_CHANNEL: RefCell<PipeWriter> = {
        let (channel_reader, channel_writer) = std::io::pipe().unwrap();
        let ipc_fd = global_client().unwrap().payload_with_str.payload.ipc_fd;
        ipc_fd.send_fd(channel_reader.as_raw_fd()).unwrap();
        RefCell::new(channel_writer)
    }
}

// static IS_NODE: LazyLock<bool> = LazyLock::new(|| {
//     std::env::current_exe().unwrap().as_os_str()
//         == "/Users/patr0nus/.local/share/mise/installs/node/24.1.0/bin/node"
// });

static CLIENT: SyncUnsafeCell<Option<Client>> = SyncUnsafeCell::new(None);

pub fn global_client() -> Option<&'static Client> {
    unsafe { (*CLIENT.get()).as_ref() }
}

#[used]
#[doc(hidden)]
#[cfg_attr(
    target_vendor = "apple",
    unsafe(link_section = "__DATA,__mod_init_func")
)]
pub static _CTOR: unsafe extern "C" fn() = {
    unsafe extern "C" fn ctor() {
        unsafe { *CLIENT.get() = Some(Client::from_env()) };
    }
    ctor
};
