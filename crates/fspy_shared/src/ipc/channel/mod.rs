//! Fast mpsc IPC channel implementation based on shared memory.

mod shm_io;

use std::{env::temp_dir, fs::File, io, ops::Deref, path::PathBuf, sync::Arc};

use bincode::{Decode, Encode};
use shared_memory::{Shmem, ShmemConf};
pub use shm_io::FrameMut;
use shm_io::{ShmReader, ShmWriter};
use tracing::debug;
use uuid::Uuid;

use super::NativeString;

/// Serializable configuration to create channel senders.
#[derive(Encode, Decode, Clone, Debug)]
pub struct ChannelConf {
    lock_file_path: NativeString,
    shm_id: Arc<str>,
    shm_size: usize,
}

/// Creates a mpsc IPC channel with one receiver and a `ChannelConf` that can be passed around processes and used to create multiple senders
pub fn channel(capacity: usize) -> io::Result<(ChannelConf, Receiver)> {
    // Initialize the lock file with a unique name.
    let lock_file_path = temp_dir().join(format!("fspy_ipc_{}.lock", Uuid::new_v4()));

    // Initialize the shared memory with a unique id.
    // Use a named shared memory so it can be opened from other processes.
    let shm_id = format!("fspy_shm_{}", Uuid::new_v4());
    let shm = ShmemConf::new()
        .size(capacity)
        .os_id(&shm_id)
        .create()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let conf = ChannelConf {
        lock_file_path: lock_file_path.as_os_str().into(),
        shm_id: shm_id.into(),
        shm_size: capacity,
    };

    let receiver = Receiver::new(lock_file_path, shm)?;
    Ok((conf, receiver))
}

impl ChannelConf {
    /// Creates a sender.
    ///
    /// This doesn't block on the file lock. Instead it returns immediately with error if the receiver is locked or dropped.
    pub fn sender(&self) -> io::Result<Sender> {
        let lock_file = File::open(self.lock_file_path.to_cow_os_str())?;
        lock_file.try_lock_shared()?;
        let mut conf = ShmemConf::new().size(self.shm_size).os_id(&self.shm_id);
        // On Windows, allow opening raw shared memory (without backing file) for DLL injection scenarios
        #[cfg(target_os = "windows")]
        {
            conf = conf.allow_raw(true);
        }
        let shm = conf.open().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let writer = unsafe { ShmWriter::new(shm) };
        Ok(Sender { writer, lock_file, lock_file_path: self.lock_file_path.clone() })
    }
}

pub struct Sender {
    writer: ShmWriter<Shmem>,
    lock_file_path: NativeString,
    lock_file: File,
}

impl Drop for Sender {
    fn drop(&mut self) {
        if let Err(err) = self.lock_file.unlock() {
            debug!("Failed to unlock the shared IPC lock {:?}: {}", self.lock_file_path, err);
        }
    }
}

impl Deref for Sender {
    type Target = ShmWriter<Shmem>;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

/// Safety: `Sender` holds a shared file lock that ensures there's no reader, so `shm` can be safely written to.
unsafe impl Send for Sender {}

/// Safety: `Sender` holds a shared file lock that ensures there's no reader, so `shm` can be safely written to.
unsafe impl Sync for Sender {}

/// The unique receiver side of an IPC channel.
/// Owns the lock file and removes it on drop.
pub struct Receiver {
    lock_file_path: PathBuf,
    lock_file: File,
    shm: Shmem,
}

/// Safety: Receiver doesn't read or write `shm`. It only pass it to ReceiverLockGuard under the lock.
unsafe impl Send for Receiver {}

/// Safety: Receiver doesn't read or write `shm`. It only pass it to ReceiverLockGuard under the lock.
unsafe impl Sync for Receiver {}

impl Drop for Receiver {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.lock_file_path) {
            debug!("Failed to remove IPC lock file {:?}: {}", self.lock_file_path, err);
        }
    }
}

impl Receiver {
    fn new(lock_file_path: PathBuf, shm: Shmem) -> io::Result<Self> {
        let lock_file = File::create(&lock_file_path)?;
        Ok(Self { lock_file_path, lock_file, shm })
    }

    /// Lock the shared memory for unique read access.
    /// Blocks until all the senders have dropped (or processes owning them have all exited) so the shared memory can be safely read.
    /// During the lifetime of returned `ReceiverReadGuard`, no new senders can be created (ChannelConf::sender would fail).
    pub fn lock(&self) -> io::Result<ReceiverLockGuard<'_>> {
        self.lock_file.lock()?;
        let reader = ShmReader::new(unsafe { self.shm.as_slice() });
        Ok(ReceiverLockGuard { reader, lock_file: &self.lock_file })
    }
}

pub struct ReceiverLockGuard<'a> {
    reader: ShmReader<&'a [u8]>,
    lock_file: &'a File,
}

impl<'a> Drop for ReceiverLockGuard<'a> {
    fn drop(&mut self) {
        if let Err(err) = self.lock_file.unlock() {
            debug!("Failed to unlock IPC lock file: {}", err);
        }
    }
}
impl<'a> Deref for ReceiverLockGuard<'a> {
    type Target = ShmReader<&'a [u8]>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, str::from_utf8};

    use bstr::B;
    use fspy_test_utils::command_executing;

    use super::*;

    #[test]
    fn smoke() {
        let (conf, receiver) = channel(100).unwrap();
        let mut cmd = command_executing!(conf, |conf: ChannelConf| {
            let sender = conf.sender().unwrap();
            let frame_size = NonZeroUsize::new(2).unwrap();
            let mut frame = sender.claim_frame(frame_size).unwrap();
            frame.copy_from_slice(&[4, 2]);
        });
        assert!(cmd.status().unwrap().success());

        let lock = receiver.lock().unwrap();
        let mut frames = lock.iter_frames();

        let received_frame = frames.next().unwrap();
        assert_eq!(received_frame, &[4, 2]);

        assert!(frames.next().is_none());
    }

    #[test]
    fn forbid_new_senders_after_locked() {
        let (conf, receiver) = channel(42).unwrap();
        let _lock = receiver.lock().unwrap();

        let mut cmd = command_executing!(conf, |conf: ChannelConf| {
            print!("{}", conf.sender().is_ok());
        });
        let output = cmd.output().unwrap();
        assert_eq!(B(&output.stdout), B("false"));
    }

    #[test]
    fn forbid_new_senders_after_receiver_dropped() {
        let (conf, receiver) = channel(42).unwrap();
        drop(receiver);

        let mut cmd = command_executing!(conf, |conf: ChannelConf| {
            print!("{}", conf.sender().is_ok());
        });
        let output = cmd.output().unwrap();
        assert_eq!(B(&output.stdout), B("false"));
    }

    #[test]
    fn concurrent_senders() {
        let (conf, receiver) = channel(8192).unwrap();
        for i in 0u16..200 {
            let mut cmd = command_executing!((conf.clone(), i), |(conf, i): (ChannelConf, u16)| {
                let sender = conf.sender().unwrap();
                let data_to_send = i.to_string();
                sender
                    .claim_frame(NonZeroUsize::new(data_to_send.len()).unwrap())
                    .unwrap()
                    .copy_from_slice(data_to_send.as_bytes());
            });
            let output = cmd.output().unwrap();
            assert!(
                output.status.success(),
                "Failed to send in iteration {}: {:?}",
                i,
                B(&output.stderr)
            );
        }
        let lock = receiver.lock().unwrap();
        let mut received_values: Vec<u16> = lock
            .iter_frames()
            .map(|frame| from_utf8(frame).unwrap().parse::<u16>().unwrap())
            .collect();
        received_values.sort_unstable();
        assert_eq!(received_values, (0u16..200).collect::<Vec<u16>>());
    }
}
