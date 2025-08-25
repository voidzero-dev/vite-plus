use std::{
    cell::{SyncUnsafeCell, UnsafeCell},
    ffi::{CStr, OsStr, OsString},
    fs::File,
    mem::MaybeUninit,
    os::fd::{FromRawFd as _, RawFd},
    sync::LazyLock,
};

use bincode::encode_into_std_write;
use fspy_shared::{
    ipc::{AccessMode, BINCODE_CONFIG, NativeStr, NativeString, PathAccess},
    linux::{
        PAYLOAD_ENV_NAME, Payload,
        inject::{PayloadWithEncodedString, inject},
    },
    unix::{cmdinfo::RawCommand, env::decode_env},
};

use libc::{c_char, c_int};
use nix::sys::socket::MsgFlags;
use thread_local::ThreadLocal;

use crate::linux::{
    path::resolve_path,
};

pub struct Client {
    payload_with_str: PayloadWithEncodedString,
    tls_shm: ThreadLocal<&'static mut [u8]>,
}

const SHM_CHUNK_SIZE: usize = 65535;

impl Client {
    fn from_env() -> Self {
        let payload_string = std::env::var_os(PAYLOAD_ENV_NAME).unwrap();
        let payload = decode_env::<Payload>(&payload_string);
        Self {
            payload_with_str: PayloadWithEncodedString {
                payload,
                payload_string,
            },
            tls_shm: ThreadLocal::new(),
        }
    }


    pub unsafe fn handle_open(
        &self,
        dirfd: c_int,
        path: *const c_char,
        flags: c_int,
    ) -> nix::Result<()> {
        let path = unsafe { CStr::from_ptr(path) };
        let acc_mode = match flags & libc::O_ACCMODE {
            libc::O_RDWR => AccessMode::ReadWrite,
            libc::O_WRONLY => AccessMode::Write,
            _ => AccessMode::Read,
        };

        let abs_path = resolve_path(dirfd, path)?;

        let path_access = PathAccess {
            mode: acc_mode,
            path: NativeStr::from_bytes(abs_path.to_bytes()),
        };
        let mut msg = std::vec::Vec::<u8>::new();
        // msg.extend_from_slice(&[12,32,32]);
        // msg.clear();
        encode_into_std_write(&path_access, &mut msg, BINCODE_CONFIG).unwrap();
        nix::sys::socket::send(
            self.payload_with_str.payload.ipc_fd,
            &msg,
            MsgFlags::empty(),
        )?;

        Ok(())
    }
}

static CLIENT: SyncUnsafeCell<MaybeUninit<Client>> = SyncUnsafeCell::new(MaybeUninit::uninit());

pub unsafe fn global_client() -> &'static Client {
    static CLIENT: LazyLock<Client> = LazyLock::new(|| Client::from_env());
    &CLIENT
}
