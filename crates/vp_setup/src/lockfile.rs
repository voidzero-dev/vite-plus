//! The release lockfile travels beside the binary, separately from pnpm's working copy.

use std::io::{Cursor, Read as _};

use flate2::read::GzDecoder;
use tar::Archive;
use vt_path::AbsolutePath;

use crate::{error::Error, integrity, platform, registry};

pub const INSTALL_LOCKFILE: &str = "vp-install-lock.yaml";

/// Restore the release's lockfile before a frozen install. Returns whether it changed.
pub async fn restore(version_dir: &AbsolutePath) -> Result<bool, Error> {
    let content = tokio::fs::read(version_dir.join("bin").join(INSTALL_LOCKFILE)).await?;
    let target = version_dir.join("pnpm-lock.yaml");
    match tokio::fs::read(&target).await {
        Ok(previous) if previous == content => return Ok(false),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    tokio::fs::write(target, content).await?;
    Ok(true)
}

/// Copy the bootstrap companion, or recover it for a standalone executable.
/// Returns whether this release requires a frozen dependency installation.
/// Older releases without a companion retain their existing installation behavior.
pub async fn prepare(
    source: &AbsolutePath,
    version_dir: &AbsolutePath,
    version: &str,
    registry_override: Option<&str>,
) -> Result<bool, Error> {
    let target = version_dir.join("bin").join(INSTALL_LOCKFILE);
    let sibling = source
        .parent()
        .ok_or_else(|| Error::Setup("Binary has no parent".into()))?
        .join(INSTALL_LOCKFILE);
    if sibling != target && tokio::fs::try_exists(&sibling).await? {
        // Read before writing: different paths may alias the same file through
        // a symlinked installation root. Copying it onto itself would truncate it.
        let content = tokio::fs::read(sibling).await?;
        tokio::fs::write(&target, content).await?;
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
        tokio::fs::write(&target, content).await?;
    }
    restore(version_dir).await?;
    Ok(true)
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
