//! Fast mpsc IPC channel implementation based on shared memory.

mod shm_io;

use std::{env::temp_dir, fs::File, io, num::NonZeroUsize, path::PathBuf, sync::Arc};

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

pub fn channel(capacity: usize) -> io::Result<(ChannelConf, Receiver)> {
    let lock_file_path = temp_dir().join(format!("fspy_ipc_{}.lock", Uuid::new_v4()));
    let shm = ShmemConf::new()
        .size(capacity)
        .create()
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let conf = ChannelConf {
        lock_file_path: lock_file_path.as_os_str().into(),
        shm_id: shm.get_os_id().into(),
        shm_size: capacity,
    };

    let receiver = Receiver::new(lock_file_path, shm)?;
    Ok((conf, receiver))
}

impl ChannelConf {
    pub fn sender(&self) -> io::Result<Sender> {
        let lock_file = File::open(self.lock_file_path.to_cow_os_str())?;
        lock_file.try_lock_shared()?;
        let shm = ShmemConf::new()
            .size(self.shm_size)
            .os_id(&self.shm_id)
            .open()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let writer = unsafe { ShmWriter::new(shm) };
        Ok(Sender { writer, _lock_file: lock_file })
    }
}

pub struct Sender {
    writer: ShmWriter<Shmem>,
    // Holds the file to keep the shared lock alive
    _lock_file: File,
}
impl Sender {
    pub fn claim_frame(&self, frame_size: NonZeroUsize) -> Option<FrameMut<'_>> {
        self.writer.claim_frame(frame_size)
    }
}

/// The unique receiver side of an IPC channel.
/// Owns the lock file and removes it on drop.
pub struct Receiver {
    lock_file_path: PathBuf,
    lock_file: File,
    shm: Shmem,
}

/// Safety: `shm` is only read under the receiver lock
unsafe impl Send for Receiver {}

/// Safety: `shm` is only read under the receiver lock
unsafe impl Sync for Receiver {}

/// Safety: `shm` is only written under the sender lock
unsafe impl Send for Sender {}

/// Safety: `shm` is only written using a thread-safe algorithm under the sender lock.
unsafe impl Sync for Sender {}

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
    /// During the lifetime of returned `ReceiverLock`, no new senders can be created (ChannelConf::sender would fail).
    pub fn lock(self) -> io::Result<ReceiverLock> {
        self.lock_file.lock()?;
        let reader = ShmReader::new(unsafe {
            std::slice::from_raw_parts(self.shm.as_ptr(), self.shm.len())
        });
        Ok(ReceiverLock { reader, receiver: self })
    }
}

pub struct ReceiverLock {
    // The order here is important to ensure the reader is dropped before the receiver.
    // The reader holds a reference to the shared memory owned by the receiver.
    reader: ShmReader<&'static [u8]>,
    receiver: Receiver,
}

impl Drop for ReceiverLock {
    fn drop(&mut self) {
        if let Err(err) = self.receiver.lock_file.unlock() {
            debug!("Failed to unlock IPC lock file: {}", err);
        }
    }
}
impl<'a> ReceiverLock {
    pub fn iter_frames(&self) -> impl Iterator<Item = &[u8]> {
        self.reader.iter_frames()
    }
}

#[cfg(test)]
mod tests {

    use std::str::from_utf8;

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
