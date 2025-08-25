pub use libc::*;


unsafe extern "C" {
    pub unsafe fn scandir(
        dirname: *const c_char,
        namelist: *mut c_void,
        select: *const c_void,
        compar: *const c_void,
    ) -> c_int;

    #[cfg(target_os = "macos")]
    pub unsafe fn scandir_b(
        dirname: *const c_char,
        namelist: *mut c_void,
        select: *const c_void,
        compar: *const c_void,
    ) -> c_int;

    pub unsafe fn getdirentries(fd: c_int, buf: *mut c_char, nbytes: c_int, basep: *mut c_long) -> c_int;
}
