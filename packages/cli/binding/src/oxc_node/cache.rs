//! Local artifact storage for the opt-in script-cache experiment. Vite Task's
//! current public client coordinates ignored paths; it has no artifact store.
//! Keep this adapter independent so a future shared store can replace it.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use oxc_node::{CachedTransform, TransformCache};
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::TransformCacheStats;

const MAGIC: &[u8; 8] = b"VPXTC01\0";
const HEADER_BYTES: usize = 8 + 32 + 32;
const MAX_RECORD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Default)]
pub(super) struct CacheStats {
    hits: AtomicU32,
    misses: AtomicU32,
    writes: AtomicU32,
    errors: AtomicU32,
}

impl CacheStats {
    pub(super) fn snapshot(&self) -> TransformCacheStats {
        TransformCacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

pub(super) struct DiskTransformCache {
    root: AbsolutePathBuf,
    directory: AbsolutePathBuf,
    namespace_ready: AtomicBool,
    pub(super) stats: Arc<CacheStats>,
}

impl DiskTransformCache {
    pub(super) fn new(root: AbsolutePathBuf, runtime_version: &str) -> Self {
        let runtime = blake3::hash(runtime_version.as_bytes()).to_hex();
        let directory =
            root.join("transform").join(env!("VPX_CACHE_BUILD_ID")).join(runtime.as_str());
        Self { root, directory, namespace_ready: AtomicBool::new(false), stats: Arc::default() }
    }

    fn key(parts: &[&[u8]]) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        for part in parts {
            hasher.update(&(part.len() as u64).to_le_bytes());
            hasher.update(part);
        }
        hasher.finalize()
    }

    fn path(&self, key: &blake3::Hash) -> AbsolutePathBuf {
        let hex = key.to_hex();
        self.directory.join(&hex[..2]).join(hex.as_str())
    }

    fn read(&self, key: &blake3::Hash) -> io::Result<Option<CachedTransform>> {
        if !self.namespace_ready.load(Ordering::Relaxed) {
            for directory in self.directories() {
                match fs::symlink_metadata(&directory) {
                    Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                    Ok(_) => return Err(io::Error::from(io::ErrorKind::InvalidInput)),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                    Err(error) => return Err(error),
                }
            }
            // The private namespace is stable for this process. Do not repeat
            // four ancestor stats for every imported module.
            self.namespace_ready.store(true, Ordering::Relaxed);
        }
        let path = self.path(key);
        match fs::symlink_metadata(path.parent().expect("artifact shard")) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(io::Error::from(io::ErrorKind::InvalidInput)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if !metadata.is_file() || metadata.len() > MAX_RECORD_BYTES {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(&path)?.take(MAX_RECORD_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            return Err(io::Error::from(io::ErrorKind::InvalidData));
        }
        decode(&bytes, key).map(Some).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidData))
    }

    fn write(&self, key: &blake3::Hash, value: &CachedTransform) -> io::Result<()> {
        let size =
            value.code.len().saturating_add(value.source_map.as_ref().map_or(0, String::len));
        if size.saturating_add(HEADER_BYTES + 17) as u64 > MAX_RECORD_BYTES {
            return Ok(());
        }
        let payload = encode(value);
        let path = self.path(key);
        let parent = path.parent().expect("artifact path has a shard parent");
        // Every directory below the owned root is ours. Refuse symlinks rather
        // than publishing through a cache path redirected outside that root.
        if !self.namespace_ready.load(Ordering::Relaxed) {
            for directory in self.directories() {
                create_private_directory(&directory)?;
            }
            self.namespace_ready.store(true, Ordering::Relaxed);
        }
        create_private_directory(parent)?;
        let temporary = parent.join(vt_str::format!(".{}.tmp", uuid::Uuid::new_v4()).as_str());
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt as _;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(MAGIC)?;
            file.write_all(key.as_bytes())?;
            file.write_all(blake3::hash(&payload).as_bytes())?;
            file.write_all(&payload)?;
            drop(file);
            // Replacement also heals a corrupt record. Concurrent producers
            // publish equivalent values; a reader keeps its open old inode.
            fs::rename(&temporary, &path)?;
            self.stats.writes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn directories(&self) -> [AbsolutePathBuf; 4] {
        [
            self.root.clone(),
            self.root.join("transform"),
            self.directory.parent().expect("build namespace").to_absolute_path_buf(),
            self.directory.clone(),
        ]
    }
}

impl TransformCache for DiskTransformCache {
    fn get(&self, parts: &[&[u8]]) -> Option<CachedTransform> {
        match self.read(&Self::key(parts)) {
            Ok(Some(value)) => {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(value)
            }
            Ok(None) => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            Err(_) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    fn put(&self, parts: &[&[u8]], value: &CachedTransform) {
        if self.write(&Self::key(parts), value).is_err() {
            self.stats.errors.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn create_private_directory(path: &AbsolutePath) -> io::Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        return if metadata.is_dir() && !metadata.file_type().is_symlink() {
            Ok(())
        } else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        };
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn encode(value: &CachedTransform) -> Vec<u8> {
    let map_bytes = value.source_map.as_deref().map_or(&[][..], str::as_bytes);
    let mut payload = Vec::with_capacity(17 + value.code.len() + map_bytes.len());
    payload.push(u8::from(value.module));
    payload.extend_from_slice(&(value.code.len() as u64).to_le_bytes());
    let map_len = value.source_map.as_ref().map_or(u64::MAX, |map| map.len() as u64);
    payload.extend_from_slice(&map_len.to_le_bytes());
    payload.extend_from_slice(value.code.as_bytes());
    payload.extend_from_slice(map_bytes);
    payload
}

fn decode(bytes: &[u8], key: &blake3::Hash) -> Option<CachedTransform> {
    if bytes.get(..8)? != MAGIC || bytes.get(8..40)? != key.as_bytes() {
        return None;
    }
    let payload = bytes.get(HEADER_BYTES..)?;
    if bytes.get(40..HEADER_BYTES)? != blake3::hash(payload).as_bytes() {
        return None;
    }
    let module = match *payload.first()? {
        0 => false,
        1 => true,
        _ => return None,
    };
    let code_len = usize::try_from(u64::from_le_bytes(payload.get(1..9)?.try_into().ok()?)).ok()?;
    let map_len = u64::from_le_bytes(payload.get(9..17)?.try_into().ok()?);
    let code_end = 17_usize.checked_add(code_len)?;
    let code = str::from_utf8(payload.get(17..code_end)?).ok()?.to_owned();
    let map_bytes = payload.get(code_end..)?;
    let source_map = if map_len == u64::MAX {
        if !map_bytes.is_empty() {
            return None;
        }
        None
    } else {
        if map_bytes.len() as u64 != map_len {
            return None;
        }
        Some(str::from_utf8(map_bytes).ok()?.to_owned())
    };
    Some(CachedTransform { code, source_map, module })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(directory: &tempfile::TempDir, runtime: &str) -> DiskTransformCache {
        DiskTransformCache::new(
            AbsolutePathBuf::new(directory.path().join("vpx")).unwrap(),
            runtime,
        )
    }

    fn artifact() -> CachedTransform {
        CachedTransform {
            code: "export const answer = 42;".into(),
            source_map: Some("{\"version\":3,\"mappings\":\"AAAA\"}".into()),
            module: true,
        }
    }

    #[test]
    fn persists_across_instances_and_partitions_inputs_and_runtime() {
        let directory = tempfile::tempdir().unwrap();
        let first = store(&directory, "22.23.4");
        assert!(!first.root.as_path().exists(), "initialization must not write before ignoreInput");
        let key: &[&[u8]] = &[b"/project/main.ts", b"source", b"options"];
        first.put(key, &artifact());
        let second = store(&directory, "22.23.4");
        let hit = second.get(key).unwrap();
        assert_eq!(hit.code, artifact().code);
        assert_eq!(hit.source_map, artifact().source_map);
        assert!(hit.module);
        assert!(second.get(&[b"/project/main.ts", b"edited", b"options"]).is_none());
        assert!(second.get(&[b"/project/main.ts", b"source", b"new-options"]).is_none());
        assert!(store(&directory, "24.15.0").get(key).is_none());
        assert_ne!(
            DiskTransformCache::key(&[b"ab", b"c"]),
            DiskTransformCache::key(&[b"a", b"bc"])
        );
    }

    #[test]
    fn corrupt_and_truncated_records_miss_and_are_repaired() {
        let directory = tempfile::tempdir().unwrap();
        let cache = store(&directory, "22");
        let key: &[&[u8]] = &[b"key"];
        cache.put(key, &artifact());
        let path = cache.path(&DiskTransformCache::key(key));
        let bytes = fs::read(&path).unwrap();
        for broken in [bytes[..20].to_vec(), {
            let mut changed = bytes.clone();
            *changed.last_mut().unwrap() ^= 1;
            changed
        }] {
            fs::write(&path, broken).unwrap();
            assert!(cache.get(key).is_none());
            cache.put(key, &artifact());
            assert_eq!(cache.get(key).unwrap().code, artifact().code);
        }
        assert_eq!(cache.stats.snapshot().errors, 2);
    }

    #[test]
    fn concurrent_publication_never_exposes_partial_records() {
        let directory = tempfile::tempdir().unwrap();
        let cache = Arc::new(store(&directory, "22"));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let cache = Arc::clone(&cache);
                scope.spawn(move || {
                    for _ in 0..20 {
                        cache.put(&[b"shared"], &artifact());
                        assert_eq!(cache.get(&[b"shared"]).unwrap().code, artifact().code);
                    }
                });
            }
        });
        assert_eq!(cache.stats.snapshot().errors, 0);
    }

    #[test]
    fn unavailable_directory_is_a_nonfatal_miss() {
        let directory = tempfile::tempdir().unwrap();
        let cache = store(&directory, "22");
        fs::write(&cache.root, b"not a directory").unwrap();
        cache.put(&[b"key"], &artifact());
        assert!(cache.get(&[b"key"]).is_none());
        assert!(cache.stats.snapshot().errors > 0);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_record_is_not_read() {
        let directory = tempfile::tempdir().unwrap();
        let cache = store(&directory, "22");
        cache.put(&[b"key"], &artifact());
        let path = cache.path(&DiskTransformCache::key(&[b"key"]));
        let target = directory.path().join("target");
        fs::rename(&path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(cache.get(&[b"key"]).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_namespace_and_shards_are_not_read() {
        let directory = tempfile::tempdir().unwrap();
        let cache = store(&directory, "22");
        cache.put(&[b"key"], &artifact());
        let shard = cache
            .path(&DiskTransformCache::key(&[b"key"]))
            .parent()
            .unwrap()
            .to_absolute_path_buf();
        let target = directory.path().join("shard-target");
        fs::rename(&shard, &target).unwrap();
        std::os::unix::fs::symlink(&target, &shard).unwrap();
        assert!(cache.get(&[b"key"]).is_none());
        fs::remove_file(&shard).unwrap();
        fs::rename(&target, &shard).unwrap();
        let namespace_target = directory.path().join("namespace-target");
        fs::rename(&cache.directory, &namespace_target).unwrap();
        std::os::unix::fs::symlink(&namespace_target, &cache.directory).unwrap();
        assert!(store(&directory, "22").get(&[b"key"]).is_none());
    }
}
