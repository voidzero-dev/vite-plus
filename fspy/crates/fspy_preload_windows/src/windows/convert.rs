use fspy_shared::ipc::{AccessMode, NativeStr};
use widestring::{U16CStr, U16CString, U16Str};
use winapi::{
    shared::ntdef::{HANDLE, POBJECT_ATTRIBUTES},
    um::winnt::ACCESS_MASK,
};

use crate::windows::winapi_utils::{
    access_mask_to_mode, combine_paths, get_path_name, get_u16_str,
};

pub trait ToAccessMode {
    unsafe fn to_access_mode(self) -> AccessMode;
}

impl ToAccessMode for AccessMode {
    unsafe fn to_access_mode(self) -> AccessMode {
        self
    }
}

impl ToAccessMode for ACCESS_MASK {
    unsafe fn to_access_mode(self) -> AccessMode {
        access_mask_to_mode(self)
    }
}

pub trait ToAbsolutePath {
    unsafe fn to_absolute_path<R, F: FnOnce(Option<&U16Str>) -> winsafe::SysResult<R>>(
        self,
        f: F,
    ) -> winsafe::SysResult<R>;
}

impl ToAbsolutePath for HANDLE {
    unsafe fn to_absolute_path<R, F: FnOnce(Option<&U16Str>) -> winsafe::SysResult<R>>(
        self,
        f: F,
    ) -> winsafe::SysResult<R> {
        let path = unsafe { get_path_name(self) }.ok();
        let path = path.as_ref().map(|path| U16Str::from_slice(&path));
        f(path)
    }
}

impl ToAbsolutePath for POBJECT_ATTRIBUTES {
    unsafe fn to_absolute_path<R, F: FnOnce(Option<&U16Str>) -> winsafe::SysResult<R>>(
        self,
        f: F,
    ) -> winsafe::SysResult<R> {
        let filename_str = unsafe { get_u16_str(&*(*self).ObjectName) };
        let filename_slice = filename_str.as_slice();
        let is_absolute = (filename_slice.get(0) == Some(&b'\\'.into())
        && filename_slice.get(1) == Some(&b'\\'.into())) // \\...
        || filename_slice.get(1) == Some(&b':'.into()); // C:...

        if is_absolute {
            let Ok(mut root_dir) = (unsafe { get_path_name((*self).RootDirectory) }) else {
                return f(None);
            };
            let root_dir_cstr = {
                root_dir.push(0);
                unsafe { U16CStr::from_ptr_str(root_dir.as_ptr()) }
            };
            let filename_cstr = U16CString::from_ustr_truncate(filename_str);
            let abs_path = combine_paths(root_dir_cstr, filename_cstr.as_ucstr()).unwrap();
            f(Some(abs_path.to_u16_str()))
        } else {
            f(Some(filename_str))
        }
    }
}
