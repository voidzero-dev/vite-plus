#![expect(clippy::disallowed_types, reason = "standalone test uses std types")]
#![expect(clippy::disallowed_macros, reason = "standalone test uses std macros")]

#[path = "cli_snapshots/registry_pack.rs"]
mod registry_pack;

use std::{
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

fn repo(root: &Path) {
    for package in ["cli", "core"] {
        let dir = root.join("packages").join(package).join("dist");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.js"), "built output").unwrap();
    }
    for file in ["package.json", "pnpm-workspace.yaml", "pnpm-lock.yaml"] {
        std::fs::write(root.join(file), "fixture").unwrap();
    }
}

fn pack(dest: &Path) -> Result<(), String> {
    for file in ["vite-plus.tgz", "core.tgz"] {
        std::fs::write(dest.join(file), "packed output").unwrap();
    }
    Ok(())
}

#[test]
fn concurrent_workers_prepare_once() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let cache = tmp.path().join("cache");
    let calls = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let prepared = registry_pack::get_or_prepare(&cache, tmp.path(), |dir| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    pack(dir)
                })
                .unwrap();
                assert_eq!(
                    std::fs::read_to_string(prepared.join("core.tgz")).unwrap(),
                    "packed output"
                );
            });
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn rejects_changed_build_and_changed_archives() {
    for change in ["input", "archive", "new-input"] {
        let tmp = tempfile::tempdir().unwrap();
        repo(tmp.path());
        let cache = tmp.path().join("cache");
        let prepared = registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap();
        let path = match change {
            "input" => tmp.path().join("packages/cli/dist/index.js"),
            "new-input" => tmp.path().join("packages/core/dist/new.js"),
            _ => prepared.join("core.tgz"),
        };
        std::fs::write(path, "different package content").unwrap();
        let error = registry_pack::get_or_prepare(&cache, tmp.path(), |_| {
            panic!("must not repack in-use files")
        })
        .unwrap_err();
        assert!(error.contains("stale"), "{error}");
    }
}

#[test]
fn failed_pack_is_not_published_and_can_be_retried() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let cache = tmp.path().join("cache");
    assert!(
        registry_pack::get_or_prepare(&cache, tmp.path(), |dir| {
            std::fs::write(dir.join("partial.tgz"), "incomplete").unwrap();
            Err("pack failed".into())
        })
        .is_err()
    );
    assert!(!cache.join("packages").exists());
    registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap();
}

#[test]
fn rejects_checkout_changes_during_pack() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let cache = tmp.path().join("cache");
    let error = registry_pack::get_or_prepare(&cache, tmp.path(), |dir| {
        pack(dir)?;
        std::fs::write(tmp.path().join("packages/cli/dist/index.js"), "rebuilt while packing")
            .unwrap();
        Ok(())
    })
    .unwrap_err();
    assert!(error.contains("checkout changed"));
    assert!(!cache.join("packages").exists());
}

#[test]
fn installed_dependencies_do_not_invalidate_preparation() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let cache = tmp.path().join("cache");
    registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap();
    let modules = tmp.path().join("packages/cli/node_modules");
    std::fs::create_dir(&modules).unwrap();
    std::fs::write(modules.join("dependency"), "not packed").unwrap();
    registry_pack::get_or_prepare(&cache, tmp.path(), |_| panic!("must reuse packed files"))
        .unwrap();
}

#[test]
fn separate_processes_prepare_once() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let cache = tmp.path().join("cache");
    let workers: Vec<_> = (0..4)
        .map(|_| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--ignored", "--exact", "pack_worker"])
                .env("VP_SNAP_TEST_REPO", tmp.path())
                .env("VP_SNAP_TEST_PACKAGES", &cache)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for worker in workers {
        let output = worker.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(std::fs::read_to_string(cache.join("preparations")).unwrap(), "packed\n");
}

#[test]
#[ignore = "subprocess helper for separate_processes_prepare_once"]
fn pack_worker() {
    use std::io::Write as _;
    let Some(repo) = std::env::var_os("VP_SNAP_TEST_REPO") else { return };
    let cache = std::path::PathBuf::from(std::env::var_os("VP_SNAP_TEST_PACKAGES").unwrap());
    registry_pack::get_or_prepare(&cache, Path::new(&repo), |dir| {
        let mut calls = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(cache.join("preparations"))
            .unwrap();
        calls.write_all(b"packed\n").unwrap();
        pack(dir)
    })
    .unwrap();
}

#[cfg(unix)]
#[test]
fn linked_package_inputs_are_validated_without_following_cycles() {
    let tmp = tempfile::tempdir().unwrap();
    repo(tmp.path());
    let docs = tmp.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("guide.md"), "original guide").unwrap();
    std::os::unix::fs::symlink(&docs, tmp.path().join("packages/cli/docs")).unwrap();
    let cache = tmp.path().join("cache");
    registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap();
    std::fs::write(docs.join("guide.md"), "updated packaged guide").unwrap();
    assert!(registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap_err().contains("stale"));
    std::os::unix::fs::symlink(&docs, docs.join("cycle")).unwrap();
    assert!(registry_pack::get_or_prepare(&cache, tmp.path(), pack).unwrap_err().contains("cycle"));
}
