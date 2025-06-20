use std::{
    ffi::OsStr,
    fs::File,
    hash::Hasher as _,
    io::{self, Read},
    sync::Arc,
};

use dashmap::DashMap;

use crate::fingerprint::PathFingerprint;

pub trait FileSystem: Sync {
    fn fingerprint_path(&self, path: &Arc<OsStr>) -> io::Result<PathFingerprint>;
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
    fn fingerprint_path(&self, path: &Arc<OsStr>) -> io::Result<PathFingerprint> {
        match File::open(path.as_ref()) {
            Ok(file) => {
                let mut reader = io::BufReader::new(file);
                let hash = hash_content(&mut reader)?;
                Ok(PathFingerprint::FileContentHash(hash))
            }
            Err(err) => {
                if err.kind() == io::ErrorKind::NotFound {
                    Ok(PathFingerprint::NotFound)
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CachedFileSystem<FS = RealFileSystem> {
    underlying: FS,
    cache: DashMap<Arc<OsStr>, PathFingerprint>,
}

impl<FS: FileSystem> FileSystem for CachedFileSystem<FS> {
    fn fingerprint_path(&self, path: &Arc<OsStr>) -> io::Result<PathFingerprint> {
        let fingerprint = self
            .cache
            .entry(path.clone())
            .or_try_insert_with(|| self.underlying.fingerprint_path(path))?;
        Ok(fingerprint.value().clone())
    }
}

impl<FS> CachedFileSystem<FS> {
    pub fn invalidate_path(&self, path: &OsStr) {
        self.cache.remove(path);
    }
}
