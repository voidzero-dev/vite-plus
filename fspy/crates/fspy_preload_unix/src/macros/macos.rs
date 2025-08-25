use std::os::raw::c_void;

// $crate::macros::
macro_rules! intercept {
    ($name: ident $((64))? : $fn_sig: ty) => {
        const _: () = {
            const _: $fn_sig = $name;
            const _: $fn_sig = $crate::libc::$name;

            #[used]
            #[allow(dead_code)]
            #[unsafe(link_section = "__DATA,__interpose")]
            static mut _INTERPOSE_ENTRY: $crate::macros::InterposeEntry =
                $crate::macros::InterposeEntry {
                    _new: $name as _,
                    _old: $crate::libc::$name as _,
                };
        };

        mod $name {
            #[allow(unused)]
            use super::*;
            pub fn original() -> $fn_sig {
                $crate::libc::$name
            }
        }
    };
}

pub(crate) use intercept;

#[doc(hidden)]
#[repr(C)]
pub struct InterposeEntry {
    pub _new: *const c_void,
    pub _old: *const c_void,
}
