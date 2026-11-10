//! Integrity tests for the pinned package manager that setup installs.
//!
//! A project `.npmrc` selects the registry that first-start setup uses to fetch
//! its pinned pnpm. Setup must verify the download against an integrity value the
//! binary carries, not one the registry reports, and that pin must also gate
//! cache reuse. The managed install is key-addressed by what it was verified
//! against, so a different pin, or no pin, resolves to a different directory.
//! These tests confirm:
//!
//! 1. A fresh download whose bytes do not match the pin is rejected.
//! 2. A fresh download whose bytes match the pin installs, so configured
//!    registries and mirrors keep working.
//! 3. A cached install a different pin produced is not reused for this pin.
//! 4. A cached install this pin produced is reused without a new download.
//! 5. The shim path applies the same pin gate as the download path, so a cache an
//!    unpinned download filled is not handed to a project that declared the pin.
//! 6. No archive entry can write outside its install directory, by `..` or
//!    through a symlink.

// This test builds tarball fixtures and changes the working directory, so it
// uses std helpers the workspace restricts in production code.
#![allow(clippy::disallowed_macros, clippy::disallowed_types, clippy::disallowed_methods)]

use std::{ffi::OsStr, future::Future};

use httpmock::prelude::*;
use sha2::{Digest, Sha512};
use vp_pm_cli::{
    PackageManagerType, bootstrap_pnpm, download_package_manager, ensure_package_manager_bin,
    installed_package_manager_dir, package_manager_install_dir,
};
use vp_shared::{EnvConfig, env_vars};

const PINNED_PNPM_VERSION: &str = "10.34.7";
/// Content marker for bytes that do not match the pin, so a test can tell which
/// pnpm an install holds.
const MARKER: &str = "MISMATCHED_PNPM_CONTENT";

/// Build a pnpm npm tarball whose `bin/pnpm.cjs` carries `marker`.
fn pnpm_tgz(marker: &str) -> Vec<u8> {
    pnpm_tgz_with(marker, |_| {})
}

/// Build a pnpm npm tarball whose `bin/pnpm.cjs` carries `marker`, then let
/// `extra` append further entries.
fn pnpm_tgz_with(marker: &str, extra: impl FnOnce(&mut tar::Builder<Vec<u8>>)) -> Vec<u8> {
    let mut tar_builder = tar::Builder::new(Vec::new());

    let package_json = format!(
        r#"{{"name":"pnpm","version":"{PINNED_PNPM_VERSION}","bin":{{"pnpm":"bin/pnpm.cjs"}}}}"#
    );
    append_file(&mut tar_builder, "package/package.json", &package_json);

    let pnpm_cjs = format!("#!/usr/bin/env node\n// {marker}\nconsole.log('pnpm');\n");
    let mut header = tar::Header::new_gnu();
    header.set_size(pnpm_cjs.len() as u64);
    header.set_mode(0o755);
    tar_builder
        .append_data(&mut header, "package/bin/pnpm.cjs", std::io::Cursor::new(pnpm_cjs))
        .unwrap();

    extra(&mut tar_builder);

    let tar_data = tar_builder.into_inner().unwrap();
    let mut gz_data = Vec::new();
    {
        let mut encoder =
            flate2::write::GzEncoder::new(&mut gz_data, flate2::Compression::default());
        std::io::copy(&mut std::io::Cursor::new(tar_data), &mut encoder).unwrap();
    }
    gz_data
}

/// Append a regular file at `name`, which the builder validates.
fn append_file(tar_builder: &mut tar::Builder<Vec<u8>>, name: &str, contents: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    tar_builder.append_data(&mut header, name, std::io::Cursor::new(contents.as_bytes())).unwrap();
}

/// Append a regular file whose name is written into the header unvalidated. The
/// builder refuses `..`, so a test that needs such a name writes the header bytes
/// directly.
fn append_raw_file(tar_builder: &mut tar::Builder<Vec<u8>>, raw_name: &str, contents: &str) {
    let mut header = tar::Header::new_gnu();
    header.as_gnu_mut().unwrap().name[..raw_name.len()].copy_from_slice(raw_name.as_bytes());
    header.set_entry_type(tar::EntryType::Regular);
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar_builder.append(&header, std::io::Cursor::new(contents.as_bytes())).unwrap();
}

/// Append a symlink at `name` that points at `target`.
fn append_symlink(tar_builder: &mut tar::Builder<Vec<u8>>, name: &str, target: &str) {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    tar_builder.append_link(&mut header, name, target).unwrap();
}

/// The `packageManager`-format integrity (`sha512.<hex>`) of a tarball.
fn pin(tgz: &[u8]) -> String {
    format!("sha512.{}", hex::encode(Sha512::digest(tgz)))
}

/// Answer the pnpm metadata and tarball requests on `server` with `tgz`. The
/// mocks stay active without their handles, like the runner's other cases.
fn mount_pnpm_registry(server: &MockServer, tgz: Vec<u8>) {
    let tarball_url = format!("{}/pnpm/-/pnpm-{PINNED_PNPM_VERSION}.tgz", server.base_url());
    server.mock(|when, then| {
        when.method(GET).path(format!("/pnpm/{PINNED_PNPM_VERSION}"));
        then.status(200).json_body(serde_json::json!({
            "name": "pnpm",
            "version": PINNED_PNPM_VERSION,
            "dist": { "tarball": tarball_url },
        }));
    });
    server.mock(|when, then| {
        when.method(GET).path(format!("/pnpm/-/pnpm-{PINNED_PNPM_VERSION}.tgz"));
        then.status(200).body(tgz);
    });
}

/// Run `f` with the working directory set to a project whose `.npmrc` names
/// `registry_base`, a fresh `VP_HOME`, and the registry env cleared. Return the
/// temporary `VP_HOME` so the caller keeps the install tree alive.
async fn in_project_env<F, Fut, R>(registry_base: &str, f: F) -> (R, tempfile::TempDir)
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = R>,
{
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("package.json"), r#"{"name":"demo","version":"1.0.0"}"#)
        .unwrap();
    std::fs::write(project.path().join(".npmrc"), format!("registry={registry_base}\n")).unwrap();

    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(project.path()).unwrap();

    let vp_home = tempfile::tempdir().unwrap();
    let vp_home_os = vp_home.path().as_os_str().to_owned();

    let result = EnvConfig::with_vars_async(
        [
            (env_vars::VP_HOME, Some(vp_home_os.as_os_str())),
            (env_vars::NPM_CONFIG_REGISTRY, None::<&OsStr>),
            (env_vars::NPM_CONFIG_REGISTRY_UPPER, None::<&OsStr>),
            ("registry", None::<&OsStr>),
        ],
        |_| async { f().await },
    )
    .await;

    std::env::set_current_dir(original_cwd).unwrap();
    (result, vp_home)
}

/// All scenarios run in one test, because each one changes the process working
/// directory. Separate `#[test]` functions would race on that global state.
#[tokio::test(flavor = "current_thread")]
async fn pinned_integrity_gates_setup_pnpm() {
    fresh_download_not_matching_the_pin_is_rejected().await;
    fresh_download_matching_the_pin_installs().await;
    cached_install_from_a_different_pin_is_rejected().await;
    matching_pin_cache_is_reused().await;
    shim_path_does_not_reuse_an_unpinned_cache().await;
    archive_entries_cannot_escape_the_install_directory().await;
}

/// A project `.npmrc` names a registry whose bytes do not match the pin. The
/// bundled pin rejects the fresh download, and nothing is installed.
async fn fresh_download_not_matching_the_pin_is_rejected() {
    let server = MockServer::start();
    let official = pin(&pnpm_tgz("genuine"));
    mount_pnpm_registry(&server, pnpm_tgz(MARKER));

    let (result, _vp_home) = in_project_env(&server.base_url(), || async {
        let result = bootstrap_pnpm(PINNED_PNPM_VERSION, &official, None).await;
        let installed =
            installed_package_manager_dir(PackageManagerType::Pnpm, PINNED_PNPM_VERSION);
        (result, installed)
    })
    .await;

    let (result, installed) = result;
    let error = result.expect_err("a download not matching the pin must fail integrity");
    assert!(error.is_integrity_failure(), "expected an integrity failure, got: {error:?}");
    assert!(installed.is_none(), "no mismatched pnpm must be installed");
}

/// A configured registry or private mirror serves the pinned bytes, so the fresh
/// download installs and `.npmrc` support is unchanged.
async fn fresh_download_matching_the_pin_installs() {
    let server = MockServer::start();
    let genuine = pnpm_tgz("genuine");
    let official = pin(&genuine);
    mount_pnpm_registry(&server, genuine);

    let (result, _vp_home) = in_project_env(&server.base_url(), || async {
        bootstrap_pnpm(PINNED_PNPM_VERSION, &official, None).await
    })
    .await;

    let install_dir = result.expect("the pinned pnpm bytes must install");
    assert!(
        std::fs::read_to_string(install_dir.join("bin").join("pnpm.cjs").as_path())
            .unwrap()
            .contains("genuine"),
        "the genuine pnpm.cjs must be installed",
    );
}

/// A prior operation caches a pnpm of the pinned version under a different pin.
/// That cache lives under a different key, so setup with the bundled pin does not
/// reuse it. It downloads again and verifies, which the mismatched bytes fail.
async fn cached_install_from_a_different_pin_is_rejected() {
    let server = MockServer::start();
    let mismatched = pnpm_tgz(MARKER);
    let mismatched_pin = pin(&mismatched);
    let official = pin(&pnpm_tgz("genuine"));
    mount_pnpm_registry(&server, mismatched);

    let (result, _vp_home) = in_project_env(&server.base_url(), || async {
        let (seeded, _, _) = download_package_manager(
            PackageManagerType::Pnpm,
            PINNED_PNPM_VERSION,
            Some(mismatched_pin.as_str()),
        )
        .await
        .expect("seed a pnpm into the shared cache under a different pin");
        let seeded_mismatched =
            std::fs::read_to_string(seeded.join("bin").join("pnpm.cjs").as_path())
                .unwrap()
                .contains(MARKER);
        (bootstrap_pnpm(PINNED_PNPM_VERSION, &official, None).await, seeded_mismatched)
    })
    .await;

    let (result, seeded_mismatched) = result;
    assert!(seeded_mismatched, "the cache must really hold the mismatched pnpm");
    let error = result.expect_err("a cache under a different pin must not satisfy the bundled pin");
    assert!(error.is_integrity_failure(), "expected an integrity failure, got: {error:?}");
}

/// A cache that the bundled pin installed is reused without a new download or a
/// metadata request, so the cache fast path still serves matching pins.
async fn matching_pin_cache_is_reused() {
    let server = MockServer::start();
    let genuine = pnpm_tgz("genuine");
    let official = pin(&genuine);
    let tarball_url = format!("{}/pnpm/-/pnpm-{PINNED_PNPM_VERSION}.tgz", server.base_url());
    let metadata = server.mock(|when, then| {
        when.method(GET).path(format!("/pnpm/{PINNED_PNPM_VERSION}"));
        then.status(200).json_body(serde_json::json!({
            "name": "pnpm",
            "version": PINNED_PNPM_VERSION,
            "dist": { "tarball": tarball_url },
        }));
    });
    let tarball = server.mock(|when, then| {
        when.method(GET).path(format!("/pnpm/-/pnpm-{PINNED_PNPM_VERSION}.tgz"));
        then.status(200).body(genuine);
    });
    let metadata_ref = &metadata;
    let tarball_ref = &tarball;

    let (reused_genuine, _vp_home) = in_project_env(&server.base_url(), || async move {
        let (seeded, _, _) = download_package_manager(
            PackageManagerType::Pnpm,
            PINNED_PNPM_VERSION,
            Some(official.as_str()),
        )
        .await
        .expect("seed a genuine pnpm into the shared cache");
        let downloads_after_seed = tarball_ref.hits();

        let dir = bootstrap_pnpm(PINNED_PNPM_VERSION, &official, None)
            .await
            .expect("a matching pin must reuse the cache");

        assert_eq!(dir, seeded, "the reused install is the seeded one");
        assert_eq!(tarball_ref.hits(), downloads_after_seed, "reuse must not download again");
        assert_eq!(metadata_ref.hits(), 0, "reuse must not fetch metadata");
        std::fs::read_to_string(dir.join("bin").join("pnpm.cjs").as_path())
            .unwrap()
            .contains("genuine")
    })
    .await;

    assert!(reused_genuine, "the reused install must be the genuine pnpm");
}

/// The global shim resolves a package manager through `ensure_package_manager_bin`
/// with the project's declared `packageManager` hash. An unpinned download caches
/// under a different key, so the pinned shim path does not reuse it. It downloads
/// again and verifies, which the mismatched bytes fail.
async fn shim_path_does_not_reuse_an_unpinned_cache() {
    let server = MockServer::start();
    let official = pin(&pnpm_tgz("genuine"));
    mount_pnpm_registry(&server, pnpm_tgz(MARKER));

    let (result, _vp_home) = in_project_env(&server.base_url(), || async {
        let (seeded, _, _) =
            download_package_manager(PackageManagerType::Pnpm, PINNED_PNPM_VERSION, None)
                .await
                .expect("seed the mismatched archive through an unpinned download");
        let seeded_mismatched =
            std::fs::read_to_string(seeded.join("bin").join("pnpm.cjs").as_path())
                .unwrap()
                .contains(MARKER);
        let resolved = ensure_package_manager_bin(
            PackageManagerType::Pnpm,
            PINNED_PNPM_VERSION,
            Some(official.as_str()),
            "pnpm",
        )
        .await;
        (resolved, seeded_mismatched)
    })
    .await;

    let (resolved, seeded_mismatched) = result;
    assert!(seeded_mismatched, "the cache must really hold the mismatched pnpm");
    let error = resolved.expect_err("the shim must not reuse a cache this pin did not install");
    assert!(error.is_integrity_failure(), "expected an integrity failure, got: {error:?}");
}

/// The staging directory sits in the version directory's parent, so trusting the
/// managed tree depends on extraction refusing to leave its destination, whether
/// by a `..` entry or through a symlink. Either kind of entry may be skipped or
/// may fail the extraction, but neither may write a file beside the version
/// directory.
async fn archive_entries_cannot_escape_the_install_directory() {
    let escape_name = "escaped.txt";
    let escape_marker = "ESCAPED";
    let traversal = pnpm_tgz_with(MARKER, |tar_builder| {
        append_raw_file(tar_builder, &format!("../{escape_name}"), escape_marker);
    });
    let through_symlink = pnpm_tgz_with(MARKER, |tar_builder| {
        append_symlink(tar_builder, "escape", "..");
        append_file(tar_builder, &format!("escape/{escape_name}"), escape_marker);
    });

    for (vector, archive) in [("traversal", traversal), ("symlink", through_symlink)] {
        let server = MockServer::start();
        mount_pnpm_registry(&server, archive);
        let ((escaped, extraction), _vp_home) = in_project_env(&server.base_url(), || async {
            let extraction =
                download_package_manager(PackageManagerType::Pnpm, PINNED_PNPM_VERSION, None).await;
            // The managed directory holding this version's installs.
            let install =
                package_manager_install_dir(PackageManagerType::Pnpm, PINNED_PNPM_VERSION, None);
            let manager_dir = install.parent().unwrap().parent().unwrap();
            (manager_dir.join(escape_name).as_path().exists(), format!("{extraction:?}"))
        })
        .await;
        assert!(
            !escaped,
            "a {vector} entry wrote a file beside the version directory (extraction: {extraction})"
        );
    }
}
