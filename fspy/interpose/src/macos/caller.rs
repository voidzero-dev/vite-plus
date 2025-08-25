
macro_rules! caller_dli_fname {
    () => { {
        let mut addrs = [::core::ptr::null_mut::<::core::ffi::c_void>(); 2];
        let backtrace_len = unsafe { ::libc::backtrace(addrs.as_mut_ptr(), addrs.len() as _) } as usize;
        if backtrace_len == 2 {
            let caller_addr = addrs[1];

            let mut dl_info: ::libc::Dl_info = unsafe { ::core::mem::zeroed() };
            let ret = unsafe { ::libc::dladdr(caller_addr, &mut dl_info) };
            if ret == 0 {
                None
            } else {
                Some(unsafe { ::core::ffi::CStr::from_ptr(dl_info.dli_fname) }.to_bytes())
            }
        } else {
            None
        }
    } }
}
pub(crate) use caller_dli_fname;
