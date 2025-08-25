pub(crate) mod client;
pub(crate) mod detour;
mod detours;
mod winapi_utils;
mod convert;


use std::{env::current_exe, slice};

use detours::DETOURS;
use fspy_shared::windows::PAYLOAD_ID;
use ms_detours::{
    DetourFindPayloadEx, DetourIsHelperProcess, DetourRestoreAfterWith, DetourTransactionBegin,
    DetourTransactionCommit, DetourUpdateThread,
};
use winapi_utils::{ck, ck_long};

use winapi::{
    shared::minwindef::{BOOL, DWORD, FALSE, HINSTANCE, TRUE},
    um::{
        processthreadsapi::GetCurrentThread,
        winnt::{self},
    },
};
use winsafe::SetLastError;

use client::{Client, set_global_client};

use crate::windows::{client::global_client, detour::AttachContext};

fn dll_main(_hinstance: HINSTANCE, reason: u32) -> winsafe::SysResult<()> {
    if unsafe { DetourIsHelperProcess() } == TRUE {
        return Ok(());
    }

    match reason {
        winnt::DLL_PROCESS_ATTACH => {
            // dbg!((current_exe(), std::process::id()));
            ck(unsafe { DetourRestoreAfterWith() })?;

            let mut payload_len: DWORD = 0;
            let payload_ptr =
                unsafe { DetourFindPayloadEx(&PAYLOAD_ID, &mut payload_len).cast::<u8>() };
            let payload_bytes = unsafe {
                slice::from_raw_parts::<'static, u8>(payload_ptr, payload_len.try_into().unwrap())
            };
            let client = Client::from_payload_bytes(payload_bytes);
            unsafe { set_global_client(client) };

            let ctx = AttachContext::new();

            ck_long(unsafe { DetourTransactionBegin() })?;
            ck_long(unsafe { DetourUpdateThread(GetCurrentThread().cast()) })?;

            for d in DETOURS {
                unsafe { d.attach(&ctx) }?;
            }

            ck_long(unsafe { DetourTransactionCommit() })?;
        }
        winnt::DLL_PROCESS_DETACH => {
            unsafe { global_client() }.finish();
            ck(unsafe { DetourTransactionBegin() })?;
            ck(unsafe { DetourUpdateThread(GetCurrentThread().cast()) })?;

            for d in DETOURS {
                unsafe { d.detach() }?;
            }

            ck(unsafe { DetourTransactionCommit() })?;
        }
        _ => {}
    }
    Ok(())
}

#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(hinstance: HINSTANCE, reason: u32, _: *mut std::ffi::c_void) -> BOOL {
    match dll_main(hinstance, reason) {
        Ok(()) => TRUE,
        Err(err) => {
            SetLastError(err);
            FALSE
        }
    }
}
