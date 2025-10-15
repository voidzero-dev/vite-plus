use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::format;

pub fn with_unique_cache_path<F, R>(test_name: &str, f: F) -> R
where
    F: FnOnce(AbsolutePathBuf) -> R,
{
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let cache_path =
        AbsolutePath::new(temp_dir.path()).unwrap().join(format!("vite-test-{}", test_name));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(cache_path)));

    // The temp directory and all its contents will be automatically cleaned up
    // when temp_dir goes out of scope

    match result {
        Ok(r) => r,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

pub fn get_fixture_path(rel_path: &str) -> AbsolutePathBuf {
    AbsolutePath::new(env!("CARGO_MANIFEST_DIR")).unwrap().join(rel_path)
}
