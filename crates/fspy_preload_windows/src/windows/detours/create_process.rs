use std::ffi::CStr;

use crate::windows::{
    client::global_client,
    detour::{Detour, DetourAny},
};

use fspy_shared::{
    ipc::{AccessMode, NativeStr, PathAccess},
};
use ms_detours::{DetourCreateProcessWithDllExA, DetourCreateProcessWithDllExW};
use widestring::U16CStr;
use winapi::{
    shared::{minwindef::{BOOL, DWORD, LPVOID}, ntdef::{LPCSTR, LPSTR}},
    um::{
        minwinbase::LPSECURITY_ATTRIBUTES,
        processthreadsapi::{CreateProcessA, CreateProcessW, ResumeThread, LPPROCESS_INFORMATION, LPSTARTUPINFOA, LPSTARTUPINFOW},
        winbase::CREATE_SUSPENDED,
        winnt::{LPCWSTR, LPWSTR},
    },
};

static DETOUR_CREATE_PROCESS_W: Detour<
    unsafe extern "system" fn(
        LPCWSTR,
        LPWSTR,
        LPSECURITY_ATTRIBUTES,
        LPSECURITY_ATTRIBUTES,
        BOOL,
        DWORD,
        LPVOID,
        LPCWSTR,
        LPSTARTUPINFOW,
        LPPROCESS_INFORMATION,
    ) -> i32,
> = unsafe {
    Detour::new(c"CreateProcessW", CreateProcessW, {
        unsafe extern "system" fn new_fn(
            lp_application_name: LPCWSTR,
            lp_command_line: LPWSTR,
            lp_process_attributes: LPSECURITY_ATTRIBUTES,
            lp_thread_attributes: LPSECURITY_ATTRIBUTES,
            b_inherit_handles: BOOL,
            dw_creation_flags: DWORD,
            lp_environment: LPVOID,
            lp_current_directory: LPCWSTR,
            lp_startup_info: LPSTARTUPINFOW,
            lp_process_information: LPPROCESS_INFORMATION,
        ) -> BOOL {
            let client = unsafe { global_client() };
            let Some(sender) = client.sender() else {
                // Detect re-entrance and avoid double hooking
                return unsafe {
                    (DETOUR_CREATE_PROCESS_W.real())(
                        lp_application_name,
                        lp_command_line,
                        lp_process_attributes,
                        lp_thread_attributes,
                        b_inherit_handles,
                        dw_creation_flags | CREATE_SUSPENDED,
                        lp_environment,
                        lp_current_directory,
                        lp_startup_info,
                        lp_process_information,
                    )
                };
            };

            if !lp_application_name.is_null() {
                unsafe {
                    sender.send(PathAccess {
                        mode: AccessMode::Read,
                        path: NativeStr::from_wide(
                            U16CStr::from_ptr_str(lp_application_name).as_slice(),
                        ),
                    });
                }
            }

            unsafe extern "system" fn CreateProcessWithPayloadW(
                lp_application_name: LPCWSTR,
                lp_command_line: LPWSTR,
                lp_process_attributes: LPSECURITY_ATTRIBUTES,
                lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                b_inherit_handles: BOOL,
                dw_creation_flags: DWORD,
                lp_environment: LPVOID,
                lp_current_directory: LPCWSTR,
                lp_startup_info: LPSTARTUPINFOW,
                lp_process_information: LPPROCESS_INFORMATION,
            ) -> BOOL {
                let ret = unsafe {
                    (DETOUR_CREATE_PROCESS_W.real())(
                        lp_application_name,
                        lp_command_line,
                        lp_process_attributes,
                        lp_thread_attributes,
                        b_inherit_handles,
                        dw_creation_flags | CREATE_SUSPENDED,
                        lp_environment,
                        lp_current_directory,
                        lp_startup_info,
                        lp_process_information,
                    )
                };
                if ret == 0 {
                    return 0;
                }

                let ret = unsafe {
                    global_client().prepare_child_process((*lp_process_information).hProcess)
                };

                if ret == 0 {
                    return 0;
                }
                if dw_creation_flags & CREATE_SUSPENDED == 0 {
                    let ret = unsafe { ResumeThread((*lp_process_information).hThread) };
                    if ret == -1i32 as DWORD {
                        return 0;
                    }
                }
                ret
            }

            unsafe {
                DetourCreateProcessWithDllExW(
                    lp_application_name,
                    lp_command_line,
                    lp_process_attributes,
                    lp_thread_attributes,
                    b_inherit_handles,
                    dw_creation_flags,
                    lp_environment,
                    lp_current_directory,
                    lp_startup_info,
                    lp_process_information,
                    client.asni_dll_path().as_ptr().cast(),
                    Some(CreateProcessWithPayloadW),
                )
            }
        }
        new_fn
    })
};


static DETOUR_CREATE_PROCESS_A: Detour<
    unsafe extern "system" fn(
        LPCSTR,
        LPSTR,
        LPSECURITY_ATTRIBUTES,
        LPSECURITY_ATTRIBUTES,
        BOOL,
        DWORD,
        LPVOID,
        LPCSTR,
        LPSTARTUPINFOA,
        LPPROCESS_INFORMATION,
    ) -> i32,
> = unsafe {
    Detour::new(c"CreateProcessA", CreateProcessA, {
        unsafe extern "system" fn new_fn(
            lp_application_name: LPCSTR,
            lp_command_line: LPSTR,
            lp_process_attributes: LPSECURITY_ATTRIBUTES,
            lp_thread_attributes: LPSECURITY_ATTRIBUTES,
            b_inherit_handles: BOOL,
            dw_creation_flags: DWORD,
            lp_environment: LPVOID,
            lp_current_directory: LPCSTR,
            lp_startup_info: LPSTARTUPINFOA,
            lp_process_information: LPPROCESS_INFORMATION,
        ) -> BOOL {
            let client = unsafe { global_client() };
            let Some(sender) = client.sender() else {
                // Detect re-entrance and avoid double hooking
                return unsafe {
                    (DETOUR_CREATE_PROCESS_A.real())(
                        lp_application_name,
                        lp_command_line,
                        lp_process_attributes,
                        lp_thread_attributes,
                        b_inherit_handles,
                        dw_creation_flags | CREATE_SUSPENDED,
                        lp_environment,
                        lp_current_directory,
                        lp_startup_info,
                        lp_process_information,
                    )
                };
            };

            if !lp_application_name.is_null() {
                unsafe {
                    sender.send(PathAccess {
                        mode: AccessMode::Read,
                        path: NativeStr::from_bytes(
                            CStr::from_ptr(lp_application_name).to_bytes(),
                        ),
                    });
                }
            }

            unsafe extern "system" fn CreateProcessWithPayloadA(
                lp_application_name: LPCSTR,
                lp_command_line: LPSTR,
                lp_process_attributes: LPSECURITY_ATTRIBUTES,
                lp_thread_attributes: LPSECURITY_ATTRIBUTES,
                b_inherit_handles: BOOL,
                dw_creation_flags: DWORD,
                lp_environment: LPVOID,
                lp_current_directory: LPCSTR,
                lp_startup_info: LPSTARTUPINFOA,
                lp_process_information: LPPROCESS_INFORMATION,
            ) -> BOOL {
                let ret = unsafe {
                    (DETOUR_CREATE_PROCESS_A.real())(
                        lp_application_name,
                        lp_command_line,
                        lp_process_attributes,
                        lp_thread_attributes,
                        b_inherit_handles,
                        dw_creation_flags | CREATE_SUSPENDED,
                        lp_environment,
                        lp_current_directory,
                        lp_startup_info,
                        lp_process_information,
                    )
                };
                if ret == 0 {
                    return 0;
                }

                let ret = unsafe {
                    global_client().prepare_child_process((*lp_process_information).hProcess)
                };

                if ret == 0 {
                    return 0;
                }
                if dw_creation_flags & CREATE_SUSPENDED == 0 {
                    let ret = unsafe { ResumeThread((*lp_process_information).hThread) };
                    if ret == -1i32 as DWORD {
                        return 0;
                    }
                }
                ret
            }

            unsafe {
                DetourCreateProcessWithDllExA(
                    lp_application_name,
                    lp_command_line,
                    lp_process_attributes,
                    lp_thread_attributes,
                    b_inherit_handles,
                    dw_creation_flags,
                    lp_environment,
                    lp_current_directory,
                    lp_startup_info,
                    lp_process_information,
                    client.asni_dll_path().as_ptr().cast(),
                    Some(CreateProcessWithPayloadA),
                )
            }
        }
        new_fn
    })
};
pub const DETOURS: &[DetourAny] = &[
    DETOUR_CREATE_PROCESS_W.as_any(),
    DETOUR_CREATE_PROCESS_A.as_any(),
];
