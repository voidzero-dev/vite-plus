use std::{cell::Cell, thread::LocalKey};

pub struct StackOnceToken {
    active: &'static LocalKey<Cell<bool>>,
}

impl StackOnceToken {
    #[doc(hidden)]
    pub const fn new(active: &'static LocalKey<Cell<bool>>) -> StackOnceToken {
        Self { active }
    }
    pub fn try_enter(&self) -> Option<StackOnceGuard> {
        if self.active.get() {
            None
        } else {
            self.active.set(true);
            Some(StackOnceGuard(self.active))
        }
    }
}

pub struct StackOnceGuard(&'static LocalKey<Cell<bool>>);

impl Drop for StackOnceGuard {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

macro_rules! stack_once_token {
    ($name:ident) => {
            static $name: $crate::stack_once::StackOnceToken = {
                ::std::thread_local! { static ACTIVE: ::core::cell::Cell<bool> = ::core::cell::Cell::new(false) }
                $crate::stack_once::StackOnceToken::new(&ACTIVE)
            };
    };
}

pub(crate) use stack_once_token;
