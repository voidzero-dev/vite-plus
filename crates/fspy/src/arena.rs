use std::{cell::RefCell, path::Path};

use crate::PathAccess;
use allocator_api2::vec::Vec;
use bumpalo::Bump;

#[ouroboros::self_referencing]
#[derive(Debug)]
pub struct PathAccessArena {
    pub bump: Bump,
    #[borrows(bump)]
    #[covariant]
    // TODO(pref): use linked list to avoid realloc & copy. We don't need random access.
    pub accesses: Vec<PathAccess<'this>, &'this Bump>,
}

impl Default for PathAccessArena {
    fn default() -> Self {
        Self::new(Bump::new(), |bump| Vec::new_in(bump))
    }
}

impl PathAccessArena {
    pub fn add(&mut self, access: PathAccess<'_>) {
        self.with_mut(|fields| {
            let path = access.path.clone_in(fields.bump);
            let path_access = PathAccess {
                mode: access.mode,
                path,
            };
            fields.accesses.push(path_access);
        });
    }
}

unsafe impl Send for PathAccessArena {}

// impl PathAccessArena {
//     pub fn as_slice(&self) -> &[PathAccess<'_>] {
//         self.borrow_accesses().as_slice()
//     }
// }
