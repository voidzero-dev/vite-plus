use std::{
    alloc::{self, Layout},
    cmp::max,
    ops::Deref,
    ptr::NonNull,
    sync::LazyLock,
};
use super::get_notif_sizes;

#[derive(Debug)]
struct BufSizes {
    req_layout: Layout,
    resp_layout: Layout,
}

static BUF_SIZES: LazyLock<BufSizes> = LazyLock::new(|| {
    const MAX_ALIGN: usize = align_of::<libc::max_align_t>();

    let sizes = get_notif_sizes().unwrap();
    BufSizes {
        req_layout: Layout::from_size_align(
            max(sizes.seccomp_notif.into(), size_of::<libc::seccomp_notif>()),
            MAX_ALIGN,
        )
        .unwrap(),
        resp_layout: Layout::from_size_align(
            max(
                sizes.seccomp_notif_resp.into(),
                size_of::<libc::seccomp_notif_resp>(),
            ),
            MAX_ALIGN,
        )
        .unwrap(),
    }
});

pub struct Alloced<T> {
    ptr: NonNull<T>,
    layout: Layout,
}

impl<T> Alloced<T> {
    pub(crate) unsafe fn alloc(layout: Layout) -> Self {
        let ptr = unsafe { alloc::alloc_zeroed(layout) };

        let ptr = NonNull::new(ptr).unwrap();
        Self {
            ptr: ptr.cast(),
            layout,
        }
    }
    pub(crate) fn zeroed(&mut self) -> &mut T {
        unsafe { self.ptr.cast::<u8>().write_bytes(0, self.layout.size()) };
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Deref for Alloced<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Drop for Alloced<T> {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.ptr.as_ptr().cast(), self.layout);
        }
    }
}

unsafe impl<T: Send + Sync> Send for Alloced<T> {}
unsafe impl<T: Send + Sync> Sync for Alloced<T> {}

pub fn alloc_seccomp_notif() -> Alloced<libc::seccomp_notif> {
    unsafe { Alloced::alloc(BUF_SIZES.req_layout) }
}

pub fn alloc_seccomp_notif_resp() -> Alloced<libc::seccomp_notif_resp> {
    unsafe { Alloced::alloc(BUF_SIZES.resp_layout) }
}
