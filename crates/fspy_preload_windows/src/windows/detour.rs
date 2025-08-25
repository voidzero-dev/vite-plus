use std::{
    cell::{SyncUnsafeCell, UnsafeCell},
    ffi::{CStr, c_char},
    mem::transmute_copy,
    os::{raw::c_void, windows::raw::HANDLE},
    ptr::null_mut,
};

use ms_detours::{DetourAttach, DetourDetach};
use winapi::{
    shared::minwindef::HMODULE,
    um::{
        fileapi::CreateFileW,
        libloaderapi::{GetProcAddress, LoadLibraryA},
    },
};
use winsafe::{HINSTANCE, SysResult};

use crate::windows::winapi_utils::ck_long;

unsafe impl<T: Sync> Sync for Detour<T> {}
pub struct Detour<T> {
    symbol_name: &'static CStr,
    target: UnsafeCell<*mut c_void>,
    new: T,
}

impl<T: Copy> Detour<T> {
    pub const unsafe fn new(symbol_name: &'static CStr, target: T, new: T) -> Self {
        Detour {
            symbol_name: symbol_name,
            target: UnsafeCell::new(unsafe { transmute_copy(&target) }),
            new: new,
        }
    }
    pub const unsafe fn dynamic(symbol_name: &'static CStr, new: T) -> Self {
        Detour {
            symbol_name: symbol_name,
            target: UnsafeCell::new(null_mut()),
            new: new,
        }
    }
    pub fn real(&self) -> &T {
        unsafe { &(*self.target.get().cast::<T>()) }
    }
    pub const fn as_any(&'static self) -> DetourAny
    where
        T: Copy,
    {
        DetourAny {
            symbol_name: &self.symbol_name,
            target: self.target.get(),
            new: ((&self.new) as *const T).cast(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DetourAny {
    symbol_name: *const &'static CStr,
    target: *mut *mut c_void,
    new: *const *mut c_void,
}

pub struct AttachContext {
    kernelbase: HMODULE,
    kernel32: HMODULE,
}

impl AttachContext {
    pub fn new() -> Self {
        let kernelbase = unsafe { LoadLibraryA(c"kernelbase".as_ptr()) };
        let kernel32 = unsafe { LoadLibraryA(c"kernel32".as_ptr()) };
        assert_ne!(kernelbase, null_mut());
        assert_ne!(kernel32, null_mut());
        Self {
            kernelbase,
            kernel32,
        }
    }
}

unsafe impl Sync for DetourAny {}
impl DetourAny {
    pub unsafe fn attach(&self, ctx: &AttachContext) -> SysResult<()> {
        let symbol_name = unsafe { *self.symbol_name }.as_ptr();
        let symbol_in_kernelbase = unsafe { GetProcAddress(ctx.kernelbase, symbol_name) };
        if !symbol_in_kernelbase.is_null() {
            //  stub symbol: https://github.com/microsoft/Detours/issues/328#issuecomment-2494147615
            unsafe { *self.target = symbol_in_kernelbase.cast() };
        } else {
            if unsafe { *self.target }.is_null() {
                // dynamic symbol
                let symbol_in_kernel32 = unsafe { GetProcAddress(ctx.kernel32, symbol_name) };
                unsafe { *self.target = symbol_in_kernel32.cast() };
            }
        }
        if unsafe { *self.target }.is_null() {
            // dynamic symbol not found, skip attaching
            return Ok(());
        }
        ck_long(unsafe { DetourAttach(self.target, *self.new) })?;
        Ok(())
    }
    pub unsafe fn detach(&self) -> SysResult<()> {
        if unsafe { *self.target }.is_null() {
            // dynamic symbol not found, skip detaching
            return Ok(());
        }
        ck_long(unsafe { DetourDetach(self.target, *self.new) })
    }
}
