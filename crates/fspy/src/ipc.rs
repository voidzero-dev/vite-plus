use std::io;

use bincode::borrow_decode_from_slice;
use fspy_shared::ipc::{
    BINCODE_CONFIG, PathAccess,
    channel::{Receiver, ReceiverLockGuard},
};
use tokio::task::spawn_blocking;

// Shared memory size for storing path accesses.
// 4 GiB is large enough to store path accesses in almost any realistic scenario.
// This doesn't allocate physical memory until it's actually used.
pub const SHM_CAPACITY: usize = 4 * 1024 * 1024 * 1024;

#[ouroboros::self_referencing]
pub struct OwnedReceiverLockGuard {
    /// Owns the shared memory
    receiver: Receiver,
    /// Borrows the shared memory and owns the file lock
    #[borrows(receiver)]
    #[covariant]
    lock_guard: ReceiverLockGuard<'this>,
}

impl OwnedReceiverLockGuard {
    pub fn lock(receiver: Receiver) -> io::Result<Self> {
        OwnedReceiverLockGuard::try_new(receiver, |receiver| receiver.lock())
    }

    pub async fn lock_async(receiver: Receiver) -> io::Result<Self> {
        spawn_blocking(move || Self::lock(receiver)).await.expect("lock task panicked")
    }

    pub fn iter_path_accesses(&self) -> impl Iterator<Item = PathAccess<'_>> {
        self.borrow_lock_guard().iter_frames().map(|frame| {
            let (path_access, decoded_size) =
                borrow_decode_from_slice::<PathAccess<'_>, _>(frame, BINCODE_CONFIG).unwrap();
            assert_eq!(decoded_size, frame.len());
            path_access
        })
    }
}
