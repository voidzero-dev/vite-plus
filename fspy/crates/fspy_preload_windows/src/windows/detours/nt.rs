use std::{cell::Cell, ffi::CStr, ops::Deref, slice, sync::LazyLock};

use arrayvec::ArrayVec;
use fspy_shared::ipc::{AccessMode, NativeStr, PathAccess};
use ntapi::ntioapi::{
    FILE_INFORMATION_CLASS, NtQueryDirectoryFile, NtQueryFullAttributesFile,
    NtQueryInformationByName, PFILE_BASIC_INFORMATION, PFILE_NETWORK_OPEN_INFORMATION,
    PIO_APC_ROUTINE, PIO_STATUS_BLOCK,
};
use smallvec::SmallVec;
use widestring::{U16CStr, U16CString, U16Str};
use winapi::{
    shared::{
        minwindef::{BOOL, DWORD, HFILE, MAX_PATH, UINT},
        ntdef::{
            BOOLEAN, HANDLE, LPCSTR, LPCWSTR, NTSTATUS, PHANDLE, PLARGE_INTEGER,
            POBJECT_ATTRIBUTES, PUNICODE_STRING, PVOID, ULONG, UNICODE_STRING,
        },
    },
    um::winnt::{ACCESS_MASK, GENERIC_READ},
};

use crate::windows::{
    client::global_client,
    convert::{ToAbsolutePath, ToAccessMode},
    detour::{Detour, DetourAny},
    winapi_utils::{access_mask_to_mode, combine_paths, get_path_name, get_u16_str},
};

unsafe fn to_path_access<F: FnOnce(PathAccess<'_>)>(
    mode: AccessMode,
    object_attributes: POBJECT_ATTRIBUTES,
    f: F,
) {
    let filename_str = unsafe { get_u16_str(&*(*object_attributes).ObjectName) };
    let filename_slice = filename_str.as_slice();
    let is_absolute = (filename_slice.get(0) == Some(&b'\\'.into())
        && filename_slice.get(1) == Some(&b'\\'.into())) // \\...
        || filename_slice.get(1) == Some(&b':'.into()); // C:...
    if is_absolute {
        let Ok(mut root_dir) = (unsafe { get_path_name((*object_attributes).RootDirectory) })
        else {
            return;
        };
        let root_dir_cstr = {
            root_dir.push(0);
            unsafe { U16CStr::from_ptr_str(root_dir.as_ptr()) }
        };
        let filename_cstr = U16CString::from_ustr_truncate(filename_str);
        let abs_path = combine_paths(root_dir_cstr, filename_cstr.as_ucstr()).unwrap();
        f(PathAccess {
            mode,
            path: NativeStr::from_wide(abs_path.to_u16_str().as_slice()),
        })
    } else {
        f(PathAccess {
            mode,
            path: NativeStr::from_wide(filename_slice),
        })
    }
}

thread_local! { pub static IS_DETOURING: Cell<bool> = const { Cell::new(false) }; }

struct DetourGuard {
    active: bool,
}

impl DetourGuard {
    pub fn new() -> Self {
        let active = !IS_DETOURING.get();
        if active {
            IS_DETOURING.set(true);
        }
        Self { active }
    }
    pub fn active(&self) -> bool {
        self.active
    }
}

impl Drop for DetourGuard {
    fn drop(&mut self) {
        if self.active {
            IS_DETOURING.set(false);
        }
    }
}

static DETOUR_NT_CREATE_FILE: Detour<
    unsafe extern "system" fn(
        file_handle: PHANDLE,
        desired_access: ACCESS_MASK,
        object_attributes: POBJECT_ATTRIBUTES,
        io_status_block: PIO_STATUS_BLOCK,
        allocation_size: PLARGE_INTEGER,
        file_attributes: ULONG,
        share_access: ULONG,
        create_disposition: ULONG,
        create_options: ULONG,
        ea_buffer: PVOID,
        ea_length: ULONG,
    ) -> HFILE,
> = unsafe {
    Detour::new(c"NtCreateFile", ntapi::ntioapi::NtCreateFile, {
        unsafe extern "system" fn new_nt_create_file(
            file_handle: PHANDLE,
            desired_access: ACCESS_MASK,
            object_attributes: POBJECT_ATTRIBUTES,
            io_status_block: PIO_STATUS_BLOCK,
            allocation_size: PLARGE_INTEGER,
            file_attributes: ULONG,
            share_access: ULONG,
            create_disposition: ULONG,
            create_options: ULONG,
            ea_buffer: PVOID,
            ea_length: ULONG,
        ) -> HFILE {
            unsafe { handle_open(desired_access, object_attributes) };

            unsafe {
                (DETOUR_NT_CREATE_FILE.real())(
                    file_handle,
                    desired_access,
                    object_attributes,
                    io_status_block,
                    allocation_size,
                    file_attributes,
                    share_access,
                    create_disposition,
                    create_options,
                    ea_buffer,
                    ea_length,
                )
            }
        }
        new_nt_create_file
    })
};

static DETOUR_NT_OPEN_FILE: Detour<
    unsafe extern "system" fn(
        FileHandle: PHANDLE,
        DesiredAccess: ACCESS_MASK,
        ObjectAttributes: POBJECT_ATTRIBUTES,
        IoStatusBlock: PIO_STATUS_BLOCK,
        ShareAccess: ULONG,
        OpenOptions: ULONG,
    ) -> HFILE,
> = unsafe {
    Detour::new(c"NtOpenFile", ntapi::ntioapi::NtOpenFile, {
        unsafe extern "system" fn new_nt_open_file(
            file_handle: PHANDLE,
            desired_access: ACCESS_MASK,
            object_attributes: POBJECT_ATTRIBUTES,
            io_status_block: PIO_STATUS_BLOCK,
            share_access: ULONG,
            open_options: ULONG,
        ) -> HFILE {
            unsafe {
                handle_open(desired_access, object_attributes);
            }

            unsafe {
                (DETOUR_NT_OPEN_FILE.real())(
                    file_handle,
                    desired_access,
                    object_attributes,
                    io_status_block,
                    share_access,
                    open_options,
                )
            }
        }
        new_nt_open_file
    })
};

static DETOUR_NT_QUERY_ATRRIBUTES_FILE: Detour<
    unsafe extern "system" fn(
        object_attributes: POBJECT_ATTRIBUTES,
        file_information: PFILE_BASIC_INFORMATION,
    ) -> HFILE,
> = unsafe {
    Detour::new(
        c"NtQueryAttributesFile",
        ntapi::ntioapi::NtQueryAttributesFile,
        {
            unsafe extern "system" fn new_nt_open_file(
                object_attributes: POBJECT_ATTRIBUTES,
                file_information: PFILE_BASIC_INFORMATION,
            ) -> HFILE {
                unsafe { handle_open(AccessMode::Read, object_attributes) };
                unsafe {
                    (DETOUR_NT_QUERY_ATRRIBUTES_FILE.real())(object_attributes, file_information)
                }
            }
            new_nt_open_file
        },
    )
};

unsafe fn handle_open(acces_mode: impl ToAccessMode, path: impl ToAbsolutePath) {
    let client = unsafe { global_client() };
    unsafe {
        path.to_absolute_path(|path| {
            let Some(path) = path else {
                return Ok(());
            };
            let path = path.as_slice();
            let path_access =
                if let Some(wildcard_pos) = path.iter().rposition(|c| *c == b'*' as u16) {
                    let path_before_wildcard = &path[..wildcard_pos];
                    let slash_pos = path_before_wildcard
                        .iter()
                        .rposition(|c| *c == b'\\' as u16 || *c == b'/' as u16)
                        .unwrap_or(0);
                    PathAccess {
                        mode: AccessMode::ReadDir,
                        path: NativeStr::from_wide(&path[..slash_pos]),
                    }
                } else {
                    PathAccess {
                        mode: acces_mode.to_access_mode(),
                        path: NativeStr::from_wide(path),
                    }
                };
            client.send(path_access);
            Ok(())
        })
    }
    .unwrap();
}

static DETOUR_NT_FULL_QUERY_ATRRIBUTES_FILE: Detour<
    unsafe extern "system" fn(
        object_attributes: POBJECT_ATTRIBUTES,
        file_information: PFILE_NETWORK_OPEN_INFORMATION,
    ) -> HFILE,
> = unsafe {
    Detour::new(c"NtQueryFullAttributesFile", NtQueryFullAttributesFile, {
        unsafe extern "system" fn new_fn(
            object_attributes: POBJECT_ATTRIBUTES,
            file_information: PFILE_NETWORK_OPEN_INFORMATION,
        ) -> HFILE {
            unsafe { handle_open(GENERIC_READ, object_attributes) };
            unsafe {
                (DETOUR_NT_FULL_QUERY_ATRRIBUTES_FILE.real())(object_attributes, file_information)
            }
        }
        new_fn
    })
};

static DETOUR_NT_OPEN_SYMBOLIC_LINK_OBJECT: Detour<
    unsafe extern "system" fn(
        link_handle: PHANDLE,
        desired_access: ACCESS_MASK,
        object_attributes: POBJECT_ATTRIBUTES,
    ) -> HFILE,
> = unsafe {
    Detour::new(
        c"NtOpenSymbolicLinkObject",
        ntapi::ntobapi::NtOpenSymbolicLinkObject,
        {
            unsafe extern "system" fn new_fn(
                link_handle: PHANDLE,
                desired_access: ACCESS_MASK,
                object_attributes: POBJECT_ATTRIBUTES,
            ) -> HFILE {
                unsafe { handle_open(desired_access, object_attributes) };
                unsafe {
                    (DETOUR_NT_OPEN_SYMBOLIC_LINK_OBJECT.real())(
                        link_handle,
                        desired_access,
                        object_attributes,
                    )
                }
            }
            new_fn
        },
    )
};

static DETOUR_NT_QUERY_INFORMATION_BY_NAME: Detour<
    unsafe extern "system" fn(
        object_attributes: POBJECT_ATTRIBUTES,
        io_status_block: PIO_STATUS_BLOCK,
        file_information: PVOID,
        length: ULONG,
        file_information_class: FILE_INFORMATION_CLASS,
    ) -> HFILE,
> = unsafe {
    Detour::new(c"NtQueryInformationByName", NtQueryInformationByName, {
        unsafe extern "system" fn new_fn(
            object_attributes: POBJECT_ATTRIBUTES,
            io_status_block: PIO_STATUS_BLOCK,
            file_information: PVOID,
            length: ULONG,
            file_information_class: FILE_INFORMATION_CLASS,
        ) -> HFILE {
            unsafe { handle_open(GENERIC_READ, object_attributes) };
            unsafe {
                (DETOUR_NT_QUERY_INFORMATION_BY_NAME.real())(
                    object_attributes,
                    io_status_block,
                    file_information,
                    length,
                    file_information_class,
                )
            }
        }
        new_fn
    })
};

static DETOUR_NT_QUERY_DIRECTORY_FILE: Detour<
    unsafe extern "system" fn(
        file_handle: HANDLE,
        event: HANDLE,
        apc_routine: PIO_APC_ROUTINE,
        apc_context: PVOID,
        io_status_block: PIO_STATUS_BLOCK,
        file_information: PVOID,
        length: ULONG,
        file_information_class: FILE_INFORMATION_CLASS,
        return_single_entry: BOOLEAN,
        file_name: PUNICODE_STRING,
        restart_scan: BOOLEAN,
    ) -> NTSTATUS,
> = unsafe {
    Detour::new(c"NtQueryDirectoryFile", NtQueryDirectoryFile, {
        unsafe extern "system" fn new_fn(
            file_handle: HANDLE,
            event: HANDLE,
            apc_routine: PIO_APC_ROUTINE,
            apc_context: PVOID,
            io_status_block: PIO_STATUS_BLOCK,
            file_information: PVOID,
            length: ULONG,
            file_information_class: FILE_INFORMATION_CLASS,
            return_single_entry: BOOLEAN,
            file_name: PUNICODE_STRING,
            restart_scan: BOOLEAN,
        ) -> NTSTATUS {
            unsafe { handle_open(AccessMode::ReadDir, file_handle) };
            unsafe {
                (DETOUR_NT_QUERY_DIRECTORY_FILE.real())(
                    file_handle,
                    event,
                    apc_routine,
                    apc_context,
                    io_status_block,
                    file_information,
                    length,
                    file_information_class,
                    return_single_entry,
                    file_name,
                    restart_scan,
                )
            }
        }
        new_fn
    })
};

pub const DETOURS: &[DetourAny] = &[
    DETOUR_NT_CREATE_FILE.as_any(),
    DETOUR_NT_OPEN_FILE.as_any(),
    DETOUR_NT_QUERY_ATRRIBUTES_FILE.as_any(),
    DETOUR_NT_FULL_QUERY_ATRRIBUTES_FILE.as_any(),
    DETOUR_NT_OPEN_SYMBOLIC_LINK_OBJECT.as_any(),
    DETOUR_NT_QUERY_INFORMATION_BY_NAME.as_any(),
    DETOUR_NT_QUERY_DIRECTORY_FILE.as_any(),
];
