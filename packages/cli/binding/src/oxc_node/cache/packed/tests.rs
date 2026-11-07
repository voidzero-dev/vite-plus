use std::fs;

use oxc_node::TransformCache;

use super::{super::DiskTransformCache, *};

fn store(temp: &tempfile::TempDir) -> DiskTransformCache {
    DiskTransformCache::new(AbsolutePathBuf::new(temp.path().join("vpx")).unwrap(), "24")
}

fn key<'a>(file: &'a [u8], source: &'a [u8]) -> [&'a [u8]; 6] {
    [b"oxc-node-transform-v1", b"transform", file, source, b"options", b"0"]
}

fn value(code: &str) -> CachedTransform {
    CachedTransform { code: code.into(), source_map: None, module: true }
}

fn pack_path(store: &DiskTransformCache) -> AbsolutePathBuf {
    store.directory.join(vt_str::format!("{}.pack", blake3::hash(b"/app").to_hex()).as_str())
}

#[test]
fn related_modules_share_a_pack_and_keep_distinct_keys() {
    let temp = tempfile::tempdir().unwrap();
    let store1 = store(&temp);
    let a = key(b"/app/a.ts", b"before");
    let b = key(b"/app/b.ts", b"before");
    store1.put(&a, &value("A"));
    store1.put(&b, &value("B"));
    assert_eq!(fs::read_dir(&store1.directory).unwrap().count(), 1);
    assert!(pack_path(&store1).as_path().is_file());

    let store2 = store(&temp);
    assert_eq!(store2.get(&a).unwrap().code, "A");
    assert_eq!(store2.get(&b).unwrap().code, "B");
    assert!(store2.get(&key(b"/app/a.ts", b"after")).is_none());
    let mut changed_options = a;
    changed_options[4] = b"new-options";
    assert!(store2.get(&changed_options).is_none());
}

#[test]
fn large_input_or_output_uses_the_loose_store() {
    let temp = tempfile::tempdir().unwrap();
    let store1 = store(&temp);
    let large_input = vec![0; MAX_SOURCE + 1];
    let a = key(b"/app/a.ts", &large_input);
    let b = key(b"/app/b.ts", b"short");
    let large = "X".repeat(MAX_PAYLOAD);
    store1.put(&a, &value("small"));
    store1.put(&b, &value(&large));
    assert!(!pack_path(&store1).as_path().exists());
    let store2 = store(&temp);
    assert_eq!(store2.get(&a).unwrap().code, "small");
    assert_eq!(store2.get(&b).unwrap().code, large);
}

#[test]
fn incomplete_append_preserves_prior_records_and_is_repaired() {
    let temp = tempfile::tempdir().unwrap();
    let store1 = store(&temp);
    let a = key(b"/app/a.ts", b"a");
    let b = key(b"/app/b.ts", b"b");
    store1.put(&a, &value("A"));
    let path = pack_path(&store1);
    let prefix = fs::read(&path).unwrap();
    OpenOptions::new().append(true).open(&path).unwrap().write_all(&[0xff; 3]).unwrap();
    let store2 = store(&temp);
    assert_eq!(store2.get(&a).unwrap().code, "A");
    assert!(store2.get(&b).is_none());
    assert_eq!(store2.stats.snapshot().errors, 1);
    store2.put(&b, &value("B"));
    assert!(fs::read(path).unwrap().starts_with(&prefix));
    let store3 = store(&temp);
    assert_eq!(store3.get(&a).unwrap().code, "A");
    assert_eq!(store3.get(&b).unwrap().code, "B");
    assert_eq!(store3.stats.snapshot().errors, 0);
}

#[test]
fn checksum_binds_the_key_to_the_payload() {
    let temp = tempfile::tempdir().unwrap();
    let store1 = store(&temp);
    let a = key(b"/app/a.ts", b"a");
    let b = key(b"/app/b.ts", b"b");
    store1.put(&a, &value("A"));
    let path = pack_path(&store1);
    let mut bytes = fs::read(&path).unwrap();
    bytes[LENGTH_BYTES + 32..LENGTH_BYTES + FRAME_HEADER]
        .copy_from_slice(DiskTransformCache::key(&b).as_bytes());
    fs::write(&path, bytes).unwrap();
    let store2 = store(&temp);
    assert!(store2.get(&b).is_none());
    assert_eq!(store2.stats.snapshot().errors, 1);
    store2.put(&b, &value("B"));
    assert_eq!(store(&temp).get(&b).unwrap().code, "B");
}

#[test]
fn competing_publishers_merge_or_use_loose_records() {
    let temp = tempfile::tempdir().unwrap();
    let first = store(&temp);
    let second = store(&temp);
    let a = key(b"/app/a.ts", b"a");
    let b = key(b"/app/b.ts", b"b");
    let c = key(b"/app/c.ts", b"c");
    first.put(&a, &value("A"));
    assert!(second.get(&b).is_none());
    first.put(&b, &value("B"));
    let path = pack_path(&first);
    let length = fs::metadata(&path).unwrap().len();
    second.put(&b, &value("B"));
    assert_eq!(second.stats.snapshot().writes, 0);
    assert_eq!(fs::metadata(&path).unwrap().len(), length);
    let locked = OpenOptions::new().read(true).write(true).open(&path).unwrap();
    locked.lock().unwrap();
    second.put(&c, &value("C"));
    drop(locked);
    assert_eq!(fs::metadata(path).unwrap().len(), length);
    let reader = store(&temp);
    assert_eq!(reader.get(&b).unwrap().code, "B");
    assert_eq!(reader.get(&c).unwrap().code, "C");
    assert!(reader.path(&DiskTransformCache::key(&c)).as_path().is_file());
}

#[test]
fn hardlinked_pack_is_never_modified() {
    let temp = tempfile::tempdir().unwrap();
    let first = store(&temp);
    let a = key(b"/app/a.ts", b"a");
    let b = key(b"/app/b.ts", b"b");
    first.put(&a, &value("A"));
    let path = pack_path(&first);
    let before = fs::read(&path).unwrap();
    let external = temp.path().join("external");
    fs::hard_link(&path, &external).unwrap();
    let second = store(&temp);
    assert!(second.get(&a).is_none());
    second.put(&b, &value("B"));
    assert_eq!(fs::read(external).unwrap(), before);
    assert_eq!(store(&temp).get(&b).unwrap().code, "B");
}

#[cfg(unix)]
#[test]
fn symlink_pack_is_never_followed() {
    let temp = tempfile::tempdir().unwrap();
    let first = store(&temp);
    let a = key(b"/app/a.ts", b"a");
    first.put(&a, &value("A"));
    let path = pack_path(&first);
    let external = temp.path().join("external");
    fs::rename(&path, &external).unwrap();
    std::os::unix::fs::symlink(&external, &path).unwrap();
    let before = fs::read(&external).unwrap();
    let second = store(&temp);
    assert!(second.get(&a).is_none());
    second.put(&a, &value("A"));
    assert_eq!(fs::read(external).unwrap(), before);
    assert_eq!(store(&temp).get(&a).unwrap().code, "A");
}

#[test]
fn full_pack_falls_back_for_new_keys_and_corrupt_records() {
    let temp = tempfile::tempdir().unwrap();
    let first = store(&temp);
    let value = value(&"x".repeat(MAX_PACK / 128 - LENGTH_BYTES - FRAME_HEADER - 17));
    for i in 0_u8..128 {
        first.put(&key(b"/app/a.ts", &[i]), &value);
    }
    let path = pack_path(&first);
    assert_eq!(fs::metadata(&path).unwrap().len(), MAX_PACK as u64);
    // Damage the first frame's checksum. A full pack has no room to append
    // its replacement, so a reader must still look for a valid loose record.
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(LENGTH_BYTES as u64)).unwrap();
    file.write_all(&[0; 32]).unwrap();
    drop(file);
    let second = store(&temp);
    let broken = key(b"/app/a.ts", &[0]);
    let new = key(b"/app/a.ts", &[200]);
    assert!(second.get(&broken).is_none());
    second.put(&broken, &value);
    second.put(&new, &value);
    assert_eq!(fs::metadata(&path).unwrap().len(), MAX_PACK as u64);
    let third = store(&temp);
    assert_eq!(third.get(&broken).unwrap().code, value.code);
    assert_eq!(third.get(&new).unwrap().code, value.code);
    assert_eq!(third.stats.snapshot().errors, 0);
}
