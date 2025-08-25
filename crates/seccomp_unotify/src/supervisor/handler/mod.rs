pub mod arg;

use std::io;
use libc::seccomp_notif;

pub trait SeccompNotifyHandler {
    fn syscalls() -> &'static [syscalls::Sysno];
    fn handle_notify(&mut self, notify: &seccomp_notif) -> io::Result<()>;
}

#[macro_export]
macro_rules! impl_handler {
    ($type: ty, $($syscall:ident)*) => {

    impl $crate::supervisor::handler::SeccompNotifyHandler for $type {
        fn syscalls() -> &'static [::syscalls::Sysno] {
            &[ $( ::syscalls::Sysno:: $syscall ),* ]
        }
        fn handle_notify(&mut self, notify: &::libc::seccomp_notif) -> ::std::io::Result<()> {
            $(
                if notify.data.nr == ::syscalls::Sysno::$syscall as _ {
                    return self.$syscall($crate::supervisor::handler::arg::FromNotify::from_notify(notify)?)
                }
            )*
            Ok(())
        }
    }
    };
}
