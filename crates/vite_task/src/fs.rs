use std::{
    ffi::OsStr,
    fs::File,
    hash::Hasher as _,
    io::{self, Read},
    sync::Arc,
};

use crate::str::Str;
use crate::{execute::PathRead, fingerprint::PathFingerprint};
use dashmap::DashMap;
use std::io::BufRead;
pub trait FileSystem: Sync {
    fn fingerprint_path(
        &self,
        path: &Arc<OsStr>,
        read: PathRead,
    ) -> anyhow::Result<PathFingerprint>;
}

#[derive(Debug, Default)]
pub struct RealFileSystem(());

fn hash_content(mut stream: impl Read) -> io::Result<u64> {
    let mut hasher = twox_hash::XxHash3_64::default();
    let mut buf = [0u8; 8192];
    loop {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
    }
    Ok(hasher.finish())
}

impl FileSystem for RealFileSystem {
    #[cfg(unix)] // TODO: windows
    fn fingerprint_path(
        &self,
        path: &Arc<OsStr>,
        path_read: PathRead,
    ) -> anyhow::Result<PathFingerprint> {
        use nix::dir::Dir;
        use std::str::from_utf8;

        let file = match File::open(path.as_ref()) {
            Ok(file) => file,
            Err(err) => {
                return if err.kind() == io::ErrorKind::NotFound {
                    Ok(PathFingerprint::NotFound)
                } else {
                    Err(err.into())
                };
            }
        };

        let mut reader = io::BufReader::new(file);
        if let Err(io_err) = reader.fill_buf() {
            if io_err.kind() != io::ErrorKind::IsADirectory {
                return Err(io_err.into());
            };
            // Is a directory
            let entries = if path_read.read_dir_entries {
                let mut entries = Vec::new();
                let dir = Dir::from_fd(reader.into_inner().into())?;
                for entry in dir {
                    let entry = entry?;
                    let filename = entry.file_name().to_bytes();
                    if matches!(filename, b"." | b".." | b".DS_Store") {
                        continue;
                    }
                    entries.push(Str::from(from_utf8(filename)?));
                }
                entries.sort_unstable();
                Some(Arc::<[Str]>::from(entries))
            } else {
                None
            };
            return Ok(PathFingerprint::Folder(entries));
        };
        Ok(PathFingerprint::FileContentHash(hash_content(reader)?))
    }
}

#[derive(Debug, Default)]
pub struct CachedFileSystem<FS = RealFileSystem> {
    underlying: FS,
    cache: DashMap<Arc<OsStr>, PathFingerprint>,
}

impl<FS: FileSystem> FileSystem for CachedFileSystem<FS> {
    fn fingerprint_path(
        &self,
        path: &Arc<OsStr>,
        path_read: PathRead,
    ) -> anyhow::Result<PathFingerprint> {
        self.underlying.fingerprint_path(path, path_read)

        // TODO: fingerprint memory cache

        // Ok(match self
        //     .cache
        //     .entry(path.clone()) {
        //         Entry::Occupied(occupied_entry) => {
        //             match (occupied_entry.get(), path_read.read_dir_entries) {

        //             }
        //         },
        //         Entry::Vacant(vacant_entry) => {
        //             vacant_entry.insert(self.underlying.fingerprint_path(path, path_read)?).value().clone()
        //         },
        //     })
        // Ok(fingerprint.value().clone())
    }
}

impl<FS> CachedFileSystem<FS> {
    pub fn invalidate_path(&self, path: &OsStr) {
        self.cache.remove(path);
    }
}
