
    pub fn with_unique_cache_path<F, R>(test_name: &str, f: F) -> R
    where
        F: FnOnce(&std::path::Path) -> R,
    {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let cache_path = temp_dir.path().join(format!("vite-test-{}.db", test_name));

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&cache_path)));

        // The temp directory and all its contents will be automatically cleaned up
        // when temp_dir goes out of scope

        match result {
            Ok(r) => r,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
