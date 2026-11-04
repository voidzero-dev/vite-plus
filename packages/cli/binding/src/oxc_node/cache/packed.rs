//! Small transforms from one source directory usually load together. A process
//! indexes that directory's pack once, then looks up complete content keys in RAM.

use std::{
    fs::{File, OpenOptions, TryLockError},
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Range,
};

use oxc_node::CachedTransform;
use rustc_hash::FxHashMap;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::decode_payload;

const MAX_SOURCE: usize = 64 * 1024;
pub(super) const MAX_PAYLOAD: usize = 64 * 1024;
const MAX_PACK: usize = 4 * 1024 * 1024;
const RESIDENT_PACKS: usize = 4;
const FRAME_HEADER: usize = 32 + 32; // BLAKE3(key || payload), key
const LENGTH_BYTES: usize = 4;

pub(super) fn source_directory<'a>(parts: &'a [&[u8]]) -> Option<&'a [u8]> {
    // The loader supplies tag, request kind, filename, source, options and mode.
    let [b"oxc-node-transform-v1", _, path, source, _, _] = parts else { return None };
    if source.len() > MAX_SOURCE {
        return None;
    }
    let split = path.iter().rposition(|&c| c == b'/' || (cfg!(windows) && c == b'\\'))?;
    Some(&path[..split])
}

#[derive(Default)]
pub(super) struct SmallPacks {
    entries: Vec<(Vec<u8>, Pack)>,
}

impl SmallPacks {
    pub(super) fn open(&mut self, root: &AbsolutePath, group: &[u8]) -> io::Result<&mut Pack> {
        // Keep the most recently used directory at the end.
        if let Some(pos) = self.entries.iter().rposition(|(dir, _)| dir == group) {
            self.entries[pos..].rotate_left(1);
        } else {
            let name = vt_str::format!("{}.pack", blake3::hash(group).to_hex());
            let pack = Pack::load(root.join(name.as_str()))?;
            if self.entries.len() >= RESIDENT_PACKS {
                self.entries.remove(0);
            }
            self.entries.push((group.to_vec(), pack));
        }
        Ok(&mut self.entries.last_mut().expect("loaded pack").1)
    }
}

pub(super) struct Pack {
    path: AbsolutePathBuf,
    bytes: Vec<u8>,
    index: FxHashMap<[u8; 32], Range<usize>>,
    damaged_tail: bool,
}

impl Pack {
    fn load(path: AbsolutePathBuf) -> io::Result<Self> {
        let mut pack =
            Self { path, bytes: Vec::new(), index: FxHashMap::default(), damaged_tail: false };
        match open_file(&pack.path, false) {
            Ok((mut file, size)) => {
                pack.bytes.reserve_exact(size);
                pack.extend_from(&mut file)?;
                pack.damaged_tail |= pack.bytes.is_empty();
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
        Ok(pack)
    }

    // Only index complete frames. Readers can race a writer or find a partial
    // append after a crash. The next writer rechecks the suffix under its lock.
    fn extend_from(&mut self, file: &mut File) -> io::Result<()> {
        let mut offset = self.bytes.len();
        file.seek(SeekFrom::Start(offset as u64))?;
        let limit = MAX_PACK - offset;
        if let Err(error) = file.take(limit as u64 + 1).read_to_end(&mut self.bytes) {
            self.bytes.truncate(offset);
            return Err(error);
        }
        if self.bytes.len() > MAX_PACK {
            self.bytes.truncate(offset);
            return Err(io::ErrorKind::InvalidData.into());
        }
        while let Some(len) = self.bytes.get(offset..offset + LENGTH_BYTES) {
            let len = u32::from_le_bytes(len.try_into().expect("frame length")) as usize;
            if !(FRAME_HEADER + 17..=FRAME_HEADER + MAX_PAYLOAD).contains(&len) {
                break;
            }
            let start = offset + LENGTH_BYTES;
            let end = start + len;
            let Some(frame) = self.bytes.get(start..end) else { break };
            let key = frame[32..FRAME_HEADER].try_into().expect("frame key");
            self.index.insert(key, start..end);
            offset = end;
        }
        self.damaged_tail = offset != self.bytes.len();
        self.bytes.truncate(offset);
        Ok(())
    }

    pub(super) fn get(&mut self, key: &blake3::Hash) -> io::Result<Option<CachedTransform>> {
        let Some(range) = self.index.get(key.as_bytes()) else {
            return if std::mem::take(&mut self.damaged_tail) {
                Err(io::ErrorKind::InvalidData.into())
            } else {
                Ok(None)
            };
        };
        let frame = &self.bytes[range.clone()];
        if blake3::hash(&frame[32..]).as_bytes() != &frame[..32] {
            return Err(io::ErrorKind::InvalidData.into());
        }
        decode_payload(&frame[FRAME_HEADER..])
            .map(Some)
            .ok_or_else(|| io::ErrorKind::InvalidData.into())
    }

    pub(super) fn put(&mut self, key: &blake3::Hash, payload: &[u8]) -> io::Result<Publication> {
        let (mut file, _) = open_file(&self.path, true)?;
        // A paused writer must not stall a script. A contending writer can
        // publish this key through the individually atomic loose-file path.
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Ok(Publication::Loose),
            Err(TryLockError::Error(err)) => return Err(err),
        }
        let length = file.metadata()?.len();
        if length < self.bytes.len() as u64 {
            self.bytes.clear();
            self.index.clear();
        }
        if length != self.bytes.len() as u64 {
            self.extend_from(&mut file)?;
            if self.damaged_tail {
                file.set_len(self.bytes.len() as u64)?;
            }
        }
        self.damaged_tail = false;
        if let Some(range) = self.index.get(key.as_bytes()) {
            let frame = &self.bytes[range.clone()];
            if &frame[FRAME_HEADER..] == payload
                && blake3::hash(&frame[32..]).as_bytes() == &frame[..32]
            {
                return Ok(Publication::Existing);
            }
        }
        let start = self.bytes.len();
        let frame_len = FRAME_HEADER + payload.len();
        if start + LENGTH_BYTES + frame_len > MAX_PACK {
            return Ok(Publication::Loose);
        }
        let mut frame = Vec::with_capacity(LENGTH_BYTES + frame_len);
        frame.extend_from_slice(&(frame_len as u32).to_le_bytes());
        frame.extend_from_slice(&[0; 32]);
        frame.extend_from_slice(key.as_bytes());
        frame.extend_from_slice(payload);
        let checksum = blake3::hash(&frame[LENGTH_BYTES + 32..]);
        frame[LENGTH_BYTES..LENGTH_BYTES + 32].copy_from_slice(checksum.as_bytes());
        file.seek(SeekFrom::Start(start as u64))?;
        file.write_all(&frame)?;
        self.bytes.extend_from_slice(&frame);
        self.index.insert(*key.as_bytes(), start + LENGTH_BYTES..self.bytes.len());
        Ok(Publication::Added)
    }
}

pub(super) enum Publication {
    Added,
    Existing,
    Loose,
}

// An append mutates the inode, unlike the loose store's atomic rename. Open
// without following links and check the handle before reading, appending or repairing.
fn open_file(path: &AbsolutePath, write: bool) -> io::Result<(File, usize)> {
    let mut options = OpenOptions::new();
    options.read(true).write(write).create(write).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600).custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > MAX_PACK as u64 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if meta.nlink() != 1 {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            || winapi_util::file::information(&file)?.number_of_links() != 1
        {
            return Err(io::ErrorKind::InvalidData.into());
        }
    }
    Ok((file, meta.len() as usize))
}

#[cfg(test)]
mod tests;
