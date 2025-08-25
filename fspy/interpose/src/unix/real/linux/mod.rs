use libc::{c_char, c_int};

pub unsafe extern "C" fn openat(
    dirfd: c_int,
    path_ptr: *const c_char,
    flags: c_int,
    args: ...
) -> c_int {
    2
}