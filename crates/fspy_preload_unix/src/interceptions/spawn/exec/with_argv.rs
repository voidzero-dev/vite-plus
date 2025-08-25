use std::{
    ffi::VaListImpl,
    mem::{self, MaybeUninit, transmute},
    slice,
};

use libc::{c_char, c_int};
use nix::Error;

// https://github.com/redox-os/relibc/blob/710911febb07a43716a6236cc9e5b864e227e36e/src/header/unistd/mod.rs#L1094
pub unsafe fn with_argv(
    mut va: VaListImpl,
    arg0: *const c_char,
    f: impl FnOnce(&[*const c_char], VaListImpl) -> c_int,
) -> c_int {
    let argc = 1 + unsafe {
        va.with_copy(|mut copy| {
            core::iter::from_fn(|| Some(copy.arg::<*const c_char>()))
                .position(|p| p.is_null())
                .unwrap()
        })
    };

    let mut stack: [MaybeUninit<*const c_char>; 32] = [MaybeUninit::uninit(); 32];

    let out = if argc < 32 {
        stack.as_mut_slice()
    } else if argc < 4096 {
        // TODO: Use ARG_MAX, not this hardcoded constant
        let ptr = unsafe { libc::malloc(argc * mem::size_of::<*const c_char>()) };
        if ptr.is_null() {
            Error::ENOMEM.set();
            return -1;
        }
        unsafe { slice::from_raw_parts_mut(ptr.cast::<MaybeUninit<*const c_char>>(), argc) }
    } else {
        Error::E2BIG.set();
        return -1;
    };
    out[0].write(arg0);

    for i in 1..argc {
        out[i].write(unsafe { va.arg::<*const c_char>() });
    }
    out[argc].write(core::ptr::null());
    // NULL
    unsafe { va.arg::<*const c_char>() };

    f(
        unsafe { transmute::<&[MaybeUninit<*const c_char>], &[*const c_char]>(&*out) },
        va,
    );

    // f only returns if it fails
    if argc >= 32 {
        unsafe { libc::free(out.as_mut_ptr().cast()) };
    }
    -1
}
