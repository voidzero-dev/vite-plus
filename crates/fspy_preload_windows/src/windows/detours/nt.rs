use fspy_shared::ipc::{AccessMode, NativeStr, PathAccess};
use ntapi::ntioapi::{
    FILE_INFORMATION_CLASS, NtQueryDirectoryFile, NtQueryFullAttributesFile,
    NtQueryInformationByName, PFILE_BASIC_INFORMATION, PFILE_NETWORK_OPEN_INFORMATION,
    PIO_APC_ROUTINE, PIO_STATUS_BLOCK,
};
use winapi::{
    shared::{
        minwindef::HFILE,
        ntdef::{
            BOOLEAN, HANDLE, NTSTATUS, PHANDLE, PLARGE_INTEGER, POBJECT_ATTRIBUTES,
            PUNICODE_STRING, PVOID, ULONG,
        },
    },
    um::winnt::{ACCESS_MASK, GENERIC_READ},
};

use crate::windows::{
    client::global_client,
    convert::{ToAbsolutePath, ToAccessMode},
    detour::{Detour, DetourAny},
};

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
        file_handle: PHANDLE,
        desired_access: ACCESS_MASK,
        object_attributes: POBJECT_ATTRIBUTES,
        io_status_block: PIO_STATUS_BLOCK,
        share_access: ULONG,
        open_options: ULONG,
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

static DETOUR_NT_QUERY_ATTRIBUTES_FILE: Detour<
    unsafe extern "system" fn(
        object_attributes: POBJECT_ATTRIBUTES,
        file_information: PFILE_BASIC_INFORMATION,
    ) -> HFILE,
> = unsafe {
    Detour::new(c"NtQueryAttributesFile", ntapi::ntioapi::NtQueryAttributesFile, {
        unsafe extern "system" fn new_nt_open_file(
            object_attributes: POBJECT_ATTRIBUTES,
            file_information: PFILE_BASIC_INFORMATION,
        ) -> HFILE {
            unsafe { handle_open(AccessMode::Read, object_attributes) };
            unsafe { (DETOUR_NT_QUERY_ATTRIBUTES_FILE.real())(object_attributes, file_information) }
        }
        new_nt_open_file
    })
};

unsafe fn handle_open(access_mode: impl ToAccessMode, path: impl ToAbsolutePath) {
    let client = unsafe { global_client() };
    unsafe {
        path.to_absolute_path(|path| {
            let Some(path) = path else {
                return Ok(());
            };
            let path = path.as_slice();
            let path_access = if let Some(wildcard_pos) =
                path.iter().rposition(|c| *c == b'*' as u16)
            {
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
                PathAccess { mode: access_mode.to_access_mode(), path: NativeStr::from_wide(path) }
            };
            client.send(path_access);
            Ok(())
        })
    }
    .unwrap();
}

static DETOUR_NT_FULL_QUERY_ATTRIBUTES_FILE: Detour<
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
                (DETOUR_NT_FULL_QUERY_ATTRIBUTES_FILE.real())(object_attributes, file_information)
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
    Detour::new(c"NtOpenSymbolicLinkObject", ntapi::ntobapi::NtOpenSymbolicLinkObject, {
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
    })
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
    DETOUR_NT_QUERY_ATTRIBUTES_FILE.as_any(),
    DETOUR_NT_FULL_QUERY_ATTRIBUTES_FILE.as_any(),
    DETOUR_NT_OPEN_SYMBOLIC_LINK_OBJECT.as_any(),
    DETOUR_NT_QUERY_INFORMATION_BY_NAME.as_any(),
    DETOUR_NT_QUERY_DIRECTORY_FILE.as_any(),
];
