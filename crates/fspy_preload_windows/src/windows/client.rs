use std::{cell::SyncUnsafeCell, ffi::CStr, mem::MaybeUninit};

use bincode::{borrow_decode_from_slice, encode_to_vec};
use fspy_detours_sys::DetourCopyPayloadToProcess;
use fspy_shared::{
    ipc::{BINCODE_CONFIG, PathAccess, channel::Sender},
    windows::{PAYLOAD_ID, Payload},
};
use winapi::{shared::minwindef::BOOL, um::winnt::HANDLE};

use crate::stack_once::{StackOnceGuard, stack_once_token};

pub struct Client<'a> {
    payload: Payload<'a>,
    ipc_sender: Option<Sender>,
}

stack_once_token!(PATH_ACCESS_ONCE);

pub struct PathAccessSender<'a> {
    ipc_sender: &'a Option<Sender>,
    _once_guard: StackOnceGuard,
}

impl<'a> PathAccessSender<'a> {
    pub fn send(&self, access: PathAccess<'_>) {
        let Some(sender) = &self.ipc_sender else {
            return;
        };
        sender.write_encoded(&access, BINCODE_CONFIG).expect("failed to send path access");
    }
}

impl<'a> Client<'a> {
    pub fn from_payload_bytes(payload_bytes: &'a [u8]) -> Self {
        let (payload, decoded_len) =
            borrow_decode_from_slice::<'a, Payload, _>(payload_bytes, BINCODE_CONFIG).unwrap();
        assert_eq!(decoded_len, payload_bytes.len());

        let ipc_sender = match payload.channel_conf.sender() {
            Ok(sender) => Some(sender),
            Err(err) => {
                // this can happen if the process is started after the root target process has exited.
                // By that time the channel would have been closed in the receiver side.
                // In this case we just leave a message and skip sending any path accesses.
                eprintln!("fspy: failed to create ipc sender: {}", err);
                None
            }
        };

        Self { payload, ipc_sender }
    }

    pub fn send(&self, access: PathAccess<'_>) {
        let Some(sender) = &self.ipc_sender else {
            return;
        };
        sender.write_encoded(&access, BINCODE_CONFIG).expect("failed to send path access");
    }

    pub fn sender(&self) -> Option<PathAccessSender<'_>> {
        let guard = PATH_ACCESS_ONCE.try_enter()?;
        Some(PathAccessSender { ipc_sender: &self.ipc_sender, _once_guard: guard })
    }

    pub unsafe fn prepare_child_process(&self, child_handle: HANDLE) -> BOOL {
        let payload_bytes = encode_to_vec(&self.payload, BINCODE_CONFIG).unwrap();
        unsafe {
            DetourCopyPayloadToProcess(
                child_handle,
                &PAYLOAD_ID,
                payload_bytes.as_ptr().cast(),
                payload_bytes.len().try_into().unwrap(),
            )
        }
    }

    pub fn asni_dll_path(&self) -> &'a CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(self.payload.asni_dll_path_with_nul) }
    }
}

static CLIENT: SyncUnsafeCell<MaybeUninit<Client<'static>>> =
    SyncUnsafeCell::new(MaybeUninit::uninit());

pub unsafe fn set_global_client(client: Client<'static>) {
    unsafe { *CLIENT.get() = MaybeUninit::new(client) }
}

pub unsafe fn global_client() -> &'static Client<'static> {
    unsafe { (*CLIENT.get()).assume_init_ref() }
}
