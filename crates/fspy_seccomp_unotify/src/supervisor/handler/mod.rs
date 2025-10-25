pub mod arg;

use std::io;

use libc::seccomp_notif;

pub trait SeccompNotifyHandler {
    fn syscalls() -> &'static [syscalls::Sysno];
    fn handle_notify(&mut self, notify: &seccomp_notif) -> io::Result<()>;
}

#[macro_export]
macro_rules! impl_handler {
    ($type:ty: $(
        $(#[$attr:meta])?
        $syscall:ident,
    )* ) => {

    impl $crate::supervisor::handler::SeccompNotifyHandler for $type {
        fn syscalls() -> &'static [::syscalls::Sysno] {
            &[ $(
                $(#[$attr])?
                ::syscalls::Sysno::$syscall
            ),* ]
        }
        fn handle_notify(&mut self, notify: &::libc::seccomp_notif) -> ::std::io::Result<()> {
            $crate::supervisor::handler::arg::Caller::with_pid(notify.pid as _, |caller| {
                $(
                    $(#[$attr])?
                    if notify.data.nr == ::syscalls::Sysno::$syscall as _ {
                        return self.$syscall(caller, $crate::supervisor::handler::arg::FromNotify::from_notify(notify)?)
                    }
                )*
                Ok(())
            })
        }
    }
    };
}
