use std::{
    fs::File,
    hash::Hasher as _,
    io::{self, Read},
    sync::Arc,
};

use crate::Error;
use crate::{execute::PathRead, fingerprint::PathFingerprint};
use dashmap::DashMap;
use std::io::BufRead;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;

use crate::collections::HashMap;
use crate::fingerprint::DirEntryKind;
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
    fn fingerprint_path(
        &self,
        path: &Arc<AbsolutePath>,
        path_read: PathRead,
    ) -> Result<PathFingerprint, Error> {
        let file = match File::open(path.as_ref()) {
            Ok(file) => file,
            Err(err) => {
                // On Windows, File::open fails for directories, so we need to check
                // if this might be a directory before giving up
                #[cfg(windows)]
                {
                    // On Windows, opening a directory can fail with various error codes
                    // Try to check if it's a directory first
                    let path_ref: &std::path::Path = path.as_ref();
                    if let Ok(dir_iter) = std::fs::read_dir(path_ref) {
                        return RealFileSystem::process_directory(dir_iter, path_read);
                    }
                }

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
            // Is a directory on Unix - use the optimized nix implementation first
            #[cfg(unix)]
            {
                return RealFileSystem::process_directory_unix(reader, path_read);
            }
            #[cfg(windows)]
            {
                // This shouldn't happen on Windows since File::open should have failed
                // But if it does, fallback to std::fs::read_dir
                let path_ref: &std::path::Path = path.as_ref();
                let dir_iter = std::fs::read_dir(path_ref)?;
                return RealFileSystem::process_directory(dir_iter, path_read);
            }
        }
        Ok(PathFingerprint::FileContentHash(hash_content(reader)?))
    }
}

impl RealFileSystem {
    #[cfg(unix)]
    fn process_directory_unix(
        reader: io::BufReader<File>,
        path_read: PathRead,
    ) -> Result<PathFingerprint, Error> {
        use bstr::ByteSlice;
        use nix::dir::{Dir, Type};

        let dir_entries: Option<HashMap<Str, DirEntryKind>> = if path_read.read_dir_entries {
            let mut dir_entries = HashMap::<Str, DirEntryKind>::new();
            let dir = Dir::from_fd(reader.into_inner().into())?;
            for entry in dir {
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
        Ok(PathFingerprint::Folder(dir_entries))
    }

    #[cfg(any(unix, windows))]
    fn process_directory(
        dir_iter: std::fs::ReadDir,
        path_read: PathRead,
    ) -> Result<PathFingerprint, Error> {
        let dir_entries: Option<HashMap<Str, DirEntryKind>> = if path_read.read_dir_entries {
            let mut dir_entries = HashMap::<Str, DirEntryKind>::new();

            for entry in dir_iter {
                let entry = entry?;
                let file_name = entry.file_name();

                // Skip special entries (same as Unix version)
                // Convert OsStr to bytes for comparison to avoid to_string_lossy
                let file_name_bytes = file_name.as_encoded_bytes();
                if matches!(file_name_bytes, b"." | b".." | b".DS_Store") {
                    continue;
                }

                // Get file type with minimal additional syscalls
                let entry_kind = match entry.file_type() {
                    Ok(file_type) => {
                        if file_type.is_file() {
                            DirEntryKind::File
                        } else if file_type.is_dir() {
                            DirEntryKind::Dir
                        } else if file_type.is_symlink() {
                            DirEntryKind::Symlink
                        } else {
                            // For any other file type, we'll treat it as a file
                            // This is a conservative approach
                            DirEntryKind::File
                        }
                    }
                    Err(_) => {
                        // If we can't determine the file type, treat as file
                        DirEntryKind::File
                    }
                };

                // Convert filename to Str - need to ensure it's valid UTF-8
                match file_name.to_str() {
                    Some(filename_str) => {
                        dir_entries.insert(filename_str.into(), entry_kind);
                    }
                    None => {
                        // Skip files with invalid UTF-8 names
                        continue;
                    }
                }
            }
            Some(dir_entries)
        } else {
            None
        };
        Ok(PathFingerprint::Folder(dir_entries))
    }
}

#[derive(Debug, Default)]
pub struct CachedFileSystem<FS = RealFileSystem> {
    underlying: FS,
    #[expect(dead_code)]
    cache: DashMap<AbsolutePathBuf, PathFingerprint>,
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
    pub fn invalidate_path(&self, path: &AbsolutePath) {
        self.cache.remove(&path.to_absolute_path_buf());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute::PathRead;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_fingerprint_nonexistent_file() {
        let fs = RealFileSystem::default();
        let nonexistent_path = Arc::<AbsolutePath>::from(
            AbsolutePathBuf::new(if cfg!(windows) {
                "C:\\nonexistent\\path".into()
            } else {
                "/nonexistent/path".into()
            })
            .unwrap(),
        );
        let path_read = PathRead { read_dir_entries: false };

        let result = fs.fingerprint_path(&nonexistent_path, path_read).unwrap();
        assert!(matches!(result, PathFingerprint::NotFound));
    }

    #[test]
    fn test_fingerprint_temp_file() {
        let fs = RealFileSystem::default();
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("test_file.txt");

        // Create a test file with known content
        std::fs::write(&temp_file, "Hello, World!").unwrap();

        let file_path =
            Arc::<AbsolutePath>::from(AbsolutePathBuf::new(temp_file.to_path_buf()).unwrap());
        let path_read = PathRead { read_dir_entries: false };

        let result = fs.fingerprint_path(&file_path, path_read).unwrap();
        assert!(matches!(result, PathFingerprint::FileContentHash(_)));

        // Verify that the same file gives the same hash
        let result2 = fs.fingerprint_path(&file_path, path_read).unwrap();
        assert_eq!(result, result2);
    }

    #[test]
    fn test_fingerprint_temp_directory() {
        let fs = RealFileSystem::default();
        let temp_dir = TempDir::new().unwrap();

        // Create some files in the directory
        std::fs::write(temp_dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "content2").unwrap();

        let dir_path =
            Arc::<AbsolutePath>::from(AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap());
        let path_read = PathRead { read_dir_entries: true };

        let result = match fs.fingerprint_path(&dir_path, path_read) {
            Ok(result) => result,
            Err(err) => {
                // On Windows CI, temporary directories might have permission issues
                // Skip the test if we get a permission denied error
                if cfg!(windows) && err.to_string().contains("Access is denied") {
                    eprintln!("Skipping test due to Windows permission issue: {}", err);
                    return;
                }
                panic!("Unexpected error: {}", err);
            }
        };

        match result {
            PathFingerprint::Folder(Some(entries)) => {
                // Should contain our test files (but not . or .. or .DS_Store)
                assert!(entries.contains_key("file1.txt"));
                assert!(entries.contains_key("file2.txt"));
                assert_eq!(entries.len(), 2);
            }
            _ => panic!("Expected folder with entries, got: {:?}", result),
        }

        // Test without reading entries
        let path_read_no_entries = PathRead { read_dir_entries: false };
        let result_no_entries = match fs.fingerprint_path(&dir_path, path_read_no_entries) {
            Ok(result) => result,
            Err(err) => {
                // On Windows CI, temporary directories might have permission issues
                // Skip the test if we get a permission denied error
                if cfg!(windows) && err.to_string().contains("Access is denied") {
                    eprintln!("Skipping test due to Windows permission issue: {}", err);
                    return;
                }
                panic!("Unexpected error: {}", err);
            }
        };
        assert!(matches!(result_no_entries, PathFingerprint::Folder(None)));
    }

    #[test]
    fn test_fingerprint_consistency_across_calls() {
        let fs = RealFileSystem::default();
        let temp_dir = TempDir::new().unwrap();
        let temp_file = temp_dir.path().join("consistent_test.txt");

        std::fs::write(&temp_file, "consistent content").unwrap();

        let file_path =
            Arc::<AbsolutePath>::from(AbsolutePathBuf::new(temp_file.to_path_buf()).unwrap());
        let path_read = PathRead { read_dir_entries: false };

        // Get multiple fingerprints
        let results: Vec<_> = (0..5)
            .map(|_| fs.fingerprint_path(&file_path, path_read))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // All results should be identical
        for result in &results[1..] {
            assert_eq!(&results[0], result);
        }
    }
}
