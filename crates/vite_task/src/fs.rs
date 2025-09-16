use std::{
    ffi::OsStr,
    fs::File,
    hash::Hasher as _,
    io::{self, BufRead, Read},
    sync::Arc,
};

use dashmap::DashMap;
#[cfg(unix)]
use vite_path::AbsolutePath;
use vite_str::Str;

use crate::{
    Error,
    collections::HashMap,
    execute::PathRead,
    fingerprint::{DirEntryKind, PathFingerprint},
};
pub trait FileSystem: Sync {
    fn fingerprint_path(
        &self,
        path: &Arc<AbsolutePath>,
        read: PathRead,
    ) -> Result<PathFingerprint, Error>;
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
        path: &Arc<AbsolutePath>,
        path_read: PathRead,
    ) -> Result<PathFingerprint, Error> {
        use nix::dir::{Dir, Type};

        let file = match File::open(path.as_ref()) {
            Ok(file) => file,
            Err(err) => {
                return if matches!(
                    err.kind(),
                    io::ErrorKind::NotFound |
                    // A component used as a directory in path is not a directory,
                    // e.g. "/foo.txt/bar" where "/foo.txt" is a file
                    io::ErrorKind::NotADirectory
                ) {
                    Ok(PathFingerprint::NotFound)
                } else {
                    Err(Error::IoWithPath { err, path: path.clone() })
                };
            }
        };

        let mut reader = io::BufReader::new(file);
        if let Err(io_err) = reader.fill_buf() {
            if io_err.kind() != io::ErrorKind::IsADirectory {
                return Err(io_err.into());
            }
            // Is a directory
            let dir_entries: Option<std::collections::HashMap<Str, DirEntryKind>> =
                if path_read.read_dir_entries {
                    let mut dir_entries = HashMap::<Str, DirEntryKind>::new();
                    let dir = Dir::from_fd(reader.into_inner().into())?;
                    for entry in dir {
                        use bstr::ByteSlice;

                        let entry = entry?;

                        let entry_kind = match entry.file_type() {
                            None => todo!("handle DT_UNKNOWN (see readdir(3))"),
                            Some(Type::File) => DirEntryKind::File,
                            Some(Type::Directory) => DirEntryKind::Dir,
                            Some(Type::Symlink) => DirEntryKind::Symlink,
                            Some(other_type) => {
                                return Err(Error::UnsupportedFileType(other_type));
                            }
                        };
                        let filename: &[u8] = entry.file_name().to_bytes();
                        if matches!(filename, b"." | b".." | b".DS_Store") {
                            continue;
                        }
                        dir_entries.insert(filename.to_str()?.into(), entry_kind);
                    }
                    Some(dir_entries)
                } else {
                    None
                };
            return Ok(PathFingerprint::Folder(dir_entries));
        }
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
        path: &Arc<AbsolutePath>,
        path_read: PathRead,
    ) -> Result<PathFingerprint, Error> {
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
    #[expect(dead_code)]
    pub fn invalidate_path(&self, path: &OsStr) {
        self.cache.remove(path);
    }
}
