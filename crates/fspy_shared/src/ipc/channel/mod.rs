mod shm_io;

use std::{env::temp_dir, fs::File, io, num::NonZeroUsize, path::PathBuf};

use bincode::{Decode, Encode};
pub use shm_io::{FrameMut, ShmReader, ShmWriter};
use uuid::Uuid;

use super::NativeString;

#[derive(Encode, Decode)]
pub struct ChannelConfig {
    lock_file_path: NativeString,
    shm_name: Box<str>,
}

// impl ChannelConfig {
//     pub fn new() -> Self {
//         Self { lock_file_path: Uuid::new_v4().to_string() }
//     }

//     fn lock_file_path(&self) -> PathBuf {
//         temp_dir().join(format!("fspy_ipc_{}.lock", self.lock_file_path))
//     }

//     pub fn with_name(mut self, name: String) -> Self {
//         self.lock_file_path = name;
//         self
//     }

//     pub fn sender(self) -> Sender {
//         let lock_file_path = format!("/tmp/fspy_ipc_{}.lock", self.lock_file_path);
//         let lock_file =
//             File::create(&lock_file_path).expect("fspy: failed to create ipc lock file");
//         let shm_writer = ShmWriter::new(&self.lock_file_path);
//         Sender { writer: shm_writer, _lock_file: lock_file }
//     }
// }

// pub struct Receiver {
//     reader: ShmReader<memmap2::MmapRaw>,
// }

// pub struct Sender {
//     writer: ShmWriter<memmap2::MmapRaw>,
//     // Holds the file to keep the shared lock alive
//     _lock_file: File,
// }
// impl Sender {
//     pub fn claim_frame(&self, frame_size: NonZeroUsize) -> Option<FrameMut<'_>> {
//         self.writer.claim_frame(frame_size)
//     }
// }
