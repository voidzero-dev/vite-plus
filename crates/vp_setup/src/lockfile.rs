//! The release lockfile travels beside the binary, separately from pnpm's working copy.

use std::io::{Cursor, Read as _};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use vp_shared::EnvConfig;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use crate::{error::Error, integrity, platform, registry};

pub const INSTALL_LOCKFILE: &str = "vp-install-lock.yaml";
pub const INSTALL_RECEIPT: &str = "node_modules/.vp-install-complete";
// This can never match a frozen install's 32-byte SHA-256 digest.
const UNLOCKED_INSTALL_RECEIPT: &[u8] = b"unlocked";

/// Publish only complete files, including when an archive read or disk write fails.
pub(crate) fn write_atomic(
    target: &AbsolutePath,
    mut content: impl std::io::Read,
) -> std::io::Result<()> {
    let parent = target.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "File has no parent")
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    std::io::copy(&mut content, &mut temporary)?;
    temporary.as_file().sync_all()?;
    temporary.persist(target).map_err(|error| error.error)?;
    Ok(())
}

async fn write(target: AbsolutePathBuf, content: Vec<u8>) -> Result<(), Error> {
    tokio::task::spawn_blocking(move || write_atomic(&target, content.as_slice())).await.map_err(
        |error| Error::Setup(format!("Failed to write installation lockfile: {error}").into()),
    )??;
    Ok(())
}

async fn read_optional(path: &AbsolutePath) -> Result<Option<Vec<u8>>, Error> {
    match tokio::fs::read(path).await {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// An interrupted install must not reuse a receipt from an earlier attempt.
pub(crate) async fn invalidate_install(version_dir: &AbsolutePath) -> Result<(), Error> {
    match tokio::fs::remove_file(version_dir.join(INSTALL_RECEIPT)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Record a successful frozen or explicitly unlocked install, independently of self-setup.
pub(crate) async fn record_install(version_dir: &AbsolutePath) -> Result<(), Error> {
    if EnvConfig::get().setup_skip_install_lockfile {
        return write(version_dir.join(INSTALL_RECEIPT), UNLOCKED_INSTALL_RECEIPT.to_vec()).await;
    }
    let Some(content) = read_optional(&version_dir.join("bin").join(INSTALL_LOCKFILE)).await?
    else {
        return Ok(());
    };
    write(version_dir.join(INSTALL_RECEIPT), Sha256::digest(content).to_vec()).await
}

/// Whether the completion receipt records a frozen install of this companion lockfile.
pub fn matches_install_receipt(lockfile: &[u8], receipt: &[u8]) -> bool {
    receipt == Sha256::digest(lockfile).as_slice()
}

/// Restore the release's lockfile before a frozen install. Returns whether it changed.
pub async fn restore(version_dir: &AbsolutePath) -> Result<bool, Error> {
    let content = tokio::fs::read(version_dir.join("bin").join(INSTALL_LOCKFILE)).await?;
    let target = version_dir.join("pnpm-lock.yaml");
    if read_optional(&target).await?.as_deref() == Some(content.as_slice()) {
        return Ok(false);
    }
    // Invalidate before restoring: a failure after this write must still retry the install.
    invalidate_install(version_dir).await?;
    write(target, content).await?;
    Ok(true)
}

/// Copy the bootstrap companion, or recover it for a standalone executable.
/// Returns whether this release still needs a dependency installation.
/// Older releases without a companion retain their existing installation behavior.
pub async fn prepare(
    source: &AbsolutePath,
    version_dir: &AbsolutePath,
    version: &str,
    registry_override: Option<&str>,
) -> Result<bool, Error> {
    if EnvConfig::get().setup_skip_install_lockfile {
        // Only reuse a completed bypass install while the bypass is explicitly enabled.
        // Do not read, restore, or recover the companion, even when it is missing or broken.
        let receipt = read_optional(&version_dir.join(INSTALL_RECEIPT)).await?;
        return Ok(receipt.as_deref() != Some(UNLOCKED_INSTALL_RECEIPT));
    }
    let target = version_dir.join("bin").join(INSTALL_LOCKFILE);
    let sibling = source
        .parent()
        .ok_or_else(|| Error::Setup("Binary has no parent".into()))?
        .join(INSTALL_LOCKFILE);
    if sibling != target && tokio::fs::try_exists(&sibling).await? {
        // Read before writing: different paths may alias the same file through
        // a symlinked installation root. Copying it onto itself would truncate it.
        let content = tokio::fs::read(sibling).await?;
        write(target.clone(), content).await?;
    }
    if !tokio::fs::try_exists(&target).await? {
        // A copied executable, including one extracted by an older vp upgrade,
        // may have lost its companion. Only fetch archives that advertise it.
        let suffix = platform::detect_platform_suffix()?;
        let resolved =
            registry::resolve_platform_package(version, &suffix, registry_override).await?;
        if !resolved.has_install_lockfile {
            return Ok(false);
        }
        let data = vp_pm_cli::HttpClient::new()
            .get_bytes(&resolved.platform_tarball_url)
            .await
            .map_err(|error| {
                Error::Setup(format!("Failed to download installation lockfile: {error}").into())
            })?;
        integrity::verify_integrity(&data, &resolved.platform_integrity)?;
        let content =
            tokio::task::spawn_blocking(move || from_archive(&data)).await.map_err(|error| {
                Error::Setup(format!("Failed to read installation lockfile: {error}").into())
            })??;
        let content = content.ok_or_else(|| {
            Error::Setup(format!("Platform package is missing {INSTALL_LOCKFILE}").into())
        })?;
        write(target.clone(), content).await?;
    }
    restore(version_dir).await?;
    let Some(receipt) = read_optional(&version_dir.join(INSTALL_RECEIPT)).await? else {
        return Ok(true);
    };
    let content = tokio::fs::read(target).await?;
    Ok(!matches_install_receipt(&content, &receipt))
}

fn from_archive(data: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    let mut archive = Archive::new(GzDecoder::new(Cursor::new(data)));
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.as_ref() == std::path::Path::new("package").join(INSTALL_LOCKFILE)
            && entry.header().entry_type().is_file()
        {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            return Ok(Some(content));
        }
    }
    Ok(None)
}

#[cfg(test)]
pub(crate) fn test_archive(requires_lockfile: bool, content: Option<&[u8]>) -> Vec<u8> {
    use flate2::{Compression, write::GzEncoder};

    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    let manifest = serde_json::json!({ "vpInstallLockfile": requires_lockfile }).to_string();
    let mut files = vec![("package/package.json".to_owned(), manifest.as_bytes())];
    if let Some(content) = content {
        files.push((format!("package/{INSTALL_LOCKFILE}"), content));
    }
    for (path, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive.append_data(&mut header, path, data).unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap()
}

#[cfg(test)]
mod tests {
    use httpmock::{Method::GET, MockServer};
    use sha2::{Digest, Sha512};
    use vt_path::AbsolutePathBuf;

    use super::*;

    #[test]
    fn failed_writes_preserve_the_previous_file_and_allow_retry() {
        struct FailedRead;
        impl std::io::Read for FailedRead {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("interrupted archive"))
            }
        }

        for previous in [None, Some(b"previous lock".as_slice())] {
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            let target = directory.join(INSTALL_LOCKFILE);
            if let Some(content) = previous {
                std::fs::write(&target, content).unwrap();
            }
            let partial = Cursor::new(b"partial lock").chain(FailedRead);
            assert!(write_atomic(&target, partial).is_err());
            assert_eq!(std::fs::read(&target).ok().as_deref(), previous);
            // The incomplete temporary file is discarded, never published as the companion.
            assert_eq!(
                std::fs::read_dir(&directory).unwrap().count(),
                usize::from(previous.is_some())
            );
            write_atomic(&target, b"complete lock".as_slice()).unwrap();
            assert_eq!(std::fs::read(target).unwrap(), b"complete lock");
        }
    }

    #[tokio::test]
    async fn only_completed_installs_of_the_current_lockfile_are_reused() {
        let temp = tempfile::tempdir().unwrap();
        let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let source = directory.join("bin/vp");
        tokio::fs::create_dir(directory.join("bin")).await.unwrap();
        tokio::fs::create_dir(directory.join("node_modules")).await.unwrap();
        tokio::fs::write(directory.join("bin").join(INSTALL_LOCKFILE), "release lock")
            .await
            .unwrap();
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());

        record_install(&directory).await.unwrap();
        tokio::fs::write(directory.join("bin").join(crate::SELF_SETUP_MARKER), "").await.unwrap();
        crate::install::clear_self_setup_marker(&directory).await.unwrap();
        assert!(!prepare(&source, &directory, "1.4.0", None).await.unwrap());

        // A new attempt that fails must not retain an earlier success receipt.
        invalidate_install(&directory).await.unwrap();
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());

        record_install(&directory).await.unwrap();
        tokio::fs::write(directory.join("pnpm-lock.yaml"), "modified lock").await.unwrap();
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());
        // Restoring the lockfile alone must not make a subsequent retry skip pnpm.
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());

        record_install(&directory).await.unwrap();
        for path in ["bin/vp-install-lock.yaml", "pnpm-lock.yaml"] {
            tokio::fs::write(directory.join(path), "different release lock").await.unwrap();
        }
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());
        record_install(&directory).await.unwrap();
        assert!(!prepare(&source, &directory, "1.4.0", None).await.unwrap());

        tokio::fs::remove_dir_all(directory.join("node_modules")).await.unwrap();
        assert!(prepare(&source, &directory, "1.4.0", None).await.unwrap());
    }

    #[tokio::test]
    async fn bootstrap_copies_the_release_lock_and_replaces_an_old_clients_resolution() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        let source = root.join("download/vp");
        let version_dir = root.join("installed");
        tokio::fs::create_dir_all(root.join("download")).await.unwrap();
        tokio::fs::create_dir_all(version_dir.join("bin")).await.unwrap();
        tokio::fs::write(root.join("download").join(INSTALL_LOCKFILE), "release lock")
            .await
            .unwrap();
        tokio::fs::write(version_dir.join("pnpm-lock.yaml"), "freshly resolved tree")
            .await
            .unwrap();

        assert!(prepare(&source, &version_dir, "1.4.0", Some("http://127.0.0.1:1")).await.unwrap());
        assert_eq!(
            tokio::fs::read_to_string(version_dir.join("pnpm-lock.yaml")).await.unwrap(),
            "release lock"
        );
        // A retry still requires a frozen install even when a failed attempt
        // already restored the working lockfile. No registry request is needed.
        assert!(
            prepare(&version_dir.join("bin/vp"), &version_dir, "1.4.0", Some("http://127.0.0.1:1"))
                .await
                .unwrap()
        );
        tokio::fs::write(version_dir.join("pnpm-lock.yaml"), "modified working copy")
            .await
            .unwrap();
        assert!(restore(&version_dir).await.unwrap());
        #[cfg(unix)]
        {
            let alias = root.join("alias");
            std::os::unix::fs::symlink(&version_dir, &alias).unwrap();
            assert!(prepare(&alias.join("bin/vp"), &version_dir, "1.4.0", None).await.unwrap());
            assert_eq!(
                tokio::fs::read_to_string(version_dir.join("bin").join(INSTALL_LOCKFILE))
                    .await
                    .unwrap(),
                "release lock"
            );
        }
    }

    #[tokio::test]
    async fn standalone_and_old_upgrade_clients_recover_the_verified_lockfile() {
        for version in ["1.4.0", "0.0.0-commit.0123456789abcdef0123456789abcdef01234567"] {
            let server = MockServer::start();
            let data = test_archive(true, Some(b"published lock"));
            let suffix = platform::detect_platform_suffix().unwrap();
            let metadata = server.mock(|when, then| {
                when.method(GET).path(format!("/@voidzero-dev/vite-plus-cli-{suffix}/{version}"));
                then.status(200).json_body(serde_json::json!({
                    "version": version, "vpInstallLockfile": true,
                    "dist": {
                        "tarball": format!("{}/platform.tgz", server.base_url()),
                        "integrity": format!("sha512-{}", base64_simd::STANDARD.encode_to_string(Sha512::digest(&data))),
                        "attestations": { "provenance": { "predicateType": "https://slsa.dev/provenance/v1" } }
                    }
                }));
            });
            let tarball = server.mock(|when, then| {
                when.method(GET).path("/platform.tgz");
                then.status(200).body(data);
            });
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            tokio::fs::create_dir(directory.join("bin")).await.unwrap();
            assert!(
                prepare(&directory.join("vp"), &directory, version, Some(&server.base_url()))
                    .await
                    .unwrap()
            );
            assert_eq!(
                tokio::fs::read(directory.join("pnpm-lock.yaml")).await.unwrap(),
                b"published lock"
            );
            metadata.assert();
            tarball.assert();
        }
    }

    #[tokio::test]
    async fn missing_or_modified_published_lockfiles_fail_without_falling_back() {
        for corrupt in [false, true] {
            let server = MockServer::start();
            let data = test_archive(true, None);
            let suffix = platform::detect_platform_suffix().unwrap();
            server.mock(|when, then| {
                when.method(GET).path(format!("/@voidzero-dev/vite-plus-cli-{suffix}/1.4.0"));
                then.status(200).json_body(serde_json::json!({
                    "version": "1.4.0", "vpInstallLockfile": true,
                    "dist": {
                        "tarball": format!("{}/platform.tgz", server.base_url()),
                        "integrity": format!("sha512-{}", base64_simd::STANDARD.encode_to_string(Sha512::digest(if corrupt { b"different".as_slice() } else { &data }))),
                        "attestations": { "provenance": { "predicateType": "https://slsa.dev/provenance/v1" } }
                    }
                }));
            });
            server.mock(|when, then| {
                when.method(GET).path("/platform.tgz");
                then.status(200).body(data);
            });
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            tokio::fs::create_dir(directory.join("bin")).await.unwrap();
            let error =
                prepare(&directory.join("vp"), &directory, "1.4.0", Some(&server.base_url()))
                    .await
                    .unwrap_err();
            assert!(error.to_string().contains(if corrupt {
                "Integrity mismatch"
            } else {
                "missing vp-install-lock.yaml"
            }));
            assert!(!directory.join("pnpm-lock.yaml").as_path().exists());
        }
    }

    #[tokio::test]
    async fn legacy_releases_and_previews_without_companions_keep_existing_behavior() {
        for version in ["1.3.0", "0.0.0-commit.0123456789abcdef0123456789abcdef01234567"] {
            let server = MockServer::start();
            let suffix = platform::detect_platform_suffix().unwrap();
            let metadata = server.mock(|when, then| {
                when.method(GET).path(format!("/@voidzero-dev/vite-plus-cli-{suffix}/{version}"));
                then.status(200).json_body(serde_json::json!({
                    "version": version,
                    "dist": {
                        "tarball": format!("{}/unused.tgz", server.base_url()), "integrity": "sha512-old",
                        "attestations": { "provenance": { "predicateType": "https://slsa.dev/provenance/v1" } }
                    }
                }));
            });
            let tarball = server.mock(|when, then| {
                when.method(GET).path("/unused.tgz");
                then.status(500);
            });
            let temp = tempfile::tempdir().unwrap();
            let directory = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
            assert!(
                !prepare(&directory.join("vp"), &directory, version, Some(&server.base_url()))
                    .await
                    .unwrap()
            );
            metadata.assert();
            tarball.assert_calls(0);
        }
    }
}
