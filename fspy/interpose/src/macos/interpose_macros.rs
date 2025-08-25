
macro_rules! interpose_no_check {
    ($old:expr, $new:expr) => {
        const _: () = {
            #[repr(C)]
            struct InterposeEntry {
                _new: *const ::core::ffi::c_void,
                _old: *const ::core::ffi::c_void,
            }
            #[used]
            #[allow(dead_code)]
            #[unsafe(link_section = "__DATA,__interpose")]
            static mut _INTERPOSE_ENTRY: InterposeEntry = InterposeEntry { _new: $new as _, _old: $old as _ };
        };
    };
}

macro_rules! interpose {
    ($old:expr, $new:expr) => {
        const _: () = {
            let _f = if true { $old } else { $new };
        };
        $crate::macos::interpose_macros::interpose_no_check!($old, $new);
    };
}

macro_rules! interpose_libc {
    ($fn_name:ident) => {
        $crate::macos::interpose_macros::interpose!(::libc::$fn_name, $fn_name);
    };
}

pub(crate) use interpose_no_check;
pub(crate) use interpose;
pub(crate) use interpose_libc;
