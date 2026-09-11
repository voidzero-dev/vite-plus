//! Generic download utilities for JavaScript runtime management.
//!
//! This module provides platform-agnostic utilities for downloading,
//! verifying, and extracting runtime archives.

use std::{fs::File, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use tokio::{
    fs,
    io::{AsyncSeekExt, AsyncWriteExt},
};
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;

use crate::{Error, provider::ArchiveFormat};

/// Response from a cached fetch operation
pub struct CachedFetchResponse<T> {
    /// Deserialized response body (None if 304 Not Modified)
    pub body: Option<T>,
    /// `ETag` header value
    pub etag: Option<Str>,
    /// Cache max-age in seconds (from Cache-Control header)
    pub max_age: Option<u64>,
    /// Whether this was a 304 Not Modified response
    pub not_modified: bool,
}

/// Download a file with retry logic and progress bar
///
/// The `message` parameter is displayed to the user to indicate what is being downloaded
/// (e.g., "Downloading Node.js v22.13.1").
pub async fn download_file(
    url: &str,
    target_path: &AbsolutePath,
    message: &str,
) -> Result<(), Error> {
    let client = vp_shared::shared_http_client()?;

    tracing::debug!("Downloading {url} to {target_path:?}");

    // Create progress bar (only in TTY and not in CI). Built once and reused
    // across retry attempts; its position is reset at the start of every
    // attempt so a retried download doesn't double-count bytes.
    let is_ci = vp_shared::EnvConfig::get().is_ci;
    let progress = if vp_shared::is_stderr_terminal() && !is_ci {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] {bytes} ({bytes_per_sec})")
                .expect("valid spinner template"),
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(message.to_string());
        Some(pb)
    } else {
        None
    };

    // Make the request *and* the body stream a single retried unit, so a
    // truncated download (bytes written != advertised Content-Length) triggers
    // a re-download instead of surfacing as a corrupt archive later.
    //
    // Runtime archives are tens of megabytes, so the request gets the longer,
    // configurable download budget instead of the shared client's 2-minute
    // default — a slow-but-steady transfer must be allowed to finish.
    //
    // Keep partial bytes between attempts, but append only after the server
    // proves it honored the exact requested range.
    let timeout = vp_shared::download_timeout();
    let result = (|| async {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(target_path)
            .await?;
        let resume_from = file.metadata().await?.len();

        let (response, total_size, is_resumed) = if resume_from > 0 {
            let ranged = client
                .get(url)
                .timeout(timeout)
                .header(reqwest::header::RANGE, format!("bytes={resume_from}-"))
                .send()
                .await?;

            // A range-capable server answers `Range: bytes=<len>-` on an
            // already-complete file with `416 Range Not Satisfiable` (there's
            // nothing left to send). `error_for_status()` would surface that
            // as a hard failure and every retry would resend the same doomed
            // Range request against the same full-length file, permanently
            // failing instead of restarting. Treat it like an inconsistent
            // partial response: drop it and fall back to a plain full request.
            if ranged.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                drop(ranged);
                let plain = full_response(client.get(url).timeout(timeout).send().await?)?;
                let total = plain.content_length();
                (plain, total, false)
            } else {
                let ranged = ranged.error_for_status()?;
                match classify_range_response(&ranged, resume_from) {
                    RangeOutcome::Resume { total } => (ranged, Some(total), true),
                    RangeOutcome::Restart => {
                        let total = ranged.content_length();
                        (ranged, total, false)
                    }
                    RangeOutcome::Reject => {
                        // Never consume an inconsistent partial response. Retry
                        // once without Range so the outer retry loop can progress.
                        drop(ranged);
                        let plain = full_response(client.get(url).timeout(timeout).send().await?)?;
                        let total = plain.content_length();
                        (plain, total, false)
                    }
                }
            }
        } else {
            let plain = full_response(client.get(url).timeout(timeout).send().await?)?;
            let total = plain.content_length();
            (plain, total, false)
        };

        if let Some(ref pb) = progress {
            pb.set_position(if is_resumed { resume_from } else { 0 });
            if let Some(size) = total_size {
                pb.set_length(size);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:40.blue/white}] \
                             {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )
                        .expect("valid progress bar template")
                        .progress_chars("#>-"),
                );
            }
        }

        let mut bytes_written = if is_resumed {
            let actual_len = file.seek(std::io::SeekFrom::End(0)).await?;
            if actual_len != resume_from {
                return Err(Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "partial download changed while resuming",
                )));
            }
            actual_len
        } else {
            file.set_len(0).await?;
            file.seek(std::io::SeekFrom::Start(0)).await?;
            0
        };

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            bytes_written += chunk.len() as u64;
            if let Some(ref pb) = progress {
                pb.inc(chunk.len() as u64);
            }
            file.write_all(&chunk).await?;
        }

        file.flush().await?;

        // A short read triggers another attempt, which resumes from disk.
        if let Some(expected_len) = total_size
            && bytes_written != expected_len
        {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                vt_str::format!(
                    "incomplete download: expected {expected_len} bytes, got {bytes_written}"
                )
                .to_string(),
            )));
        }

        Ok(())
    })
    .retry(
        ExponentialBuilder::default()
            .with_jitter()
            .with_min_delay(Duration::from_millis(500))
            .with_max_times(3),
    )
    .await
    .map_err(|e| Error::DownloadFailed {
        url: url.into(),
        reason: vp_shared::format_error_chain(&e).into(),
    });

    if let Some(pb) = progress {
        pb.finish_and_clear();
    }

    result?;

    tracing::debug!("Download completed: {target_path:?}");

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangeOutcome {
    Resume { total: u64 },
    Restart,
    Reject,
}

fn classify_range_response(response: &reqwest::Response, resume_from: u64) -> RangeOutcome {
    if response.status() == reqwest::StatusCode::OK {
        return RangeOutcome::Restart;
    }
    if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return RangeOutcome::Reject;
    }

    let Some(parsed) = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|h| h.to_str().ok())
        .and_then(parse_content_range)
    else {
        return RangeOutcome::Reject;
    };

    if parsed.start != resume_from || parsed.end < parsed.start || parsed.end >= parsed.total {
        return RangeOutcome::Reject;
    }

    let expected_len = parsed.end - parsed.start + 1;
    if response.content_length() != Some(expected_len) {
        return RangeOutcome::Reject;
    }

    RangeOutcome::Resume { total: parsed.total }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedContentRange {
    start: u64,
    end: u64,
    total: u64,
}

fn parse_content_range(header: &str) -> Option<ParsedContentRange> {
    let rest = header.strip_prefix("bytes ")?;
    let (range, total) = rest.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    let parsed = ParsedContentRange {
        start: start.trim().parse().ok()?,
        end: end.trim().parse().ok()?,
        total: total.trim().parse().ok()?,
    };
    (parsed.end >= parsed.start && parsed.end < parsed.total).then_some(parsed)
}

fn full_response(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    let response = response.error_for_status()?;
    if response.status() == reqwest::StatusCode::OK {
        Ok(response)
    } else {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            vt_str::format!("expected a full HTTP response, got {}", response.status()).to_string(),
        )))
    }
}

/// Download text content from a URL with retry logic
#[expect(clippy::disallowed_types, reason = "HTTP response body is a String")]
pub async fn download_text(url: &str) -> Result<String, Error> {
    let client = vp_shared::shared_http_client()?;

    tracing::debug!("Downloading text from {url}");

    let content = (|| async { client.get(url).send().await?.error_for_status()?.text().await })
        .retry(
            ExponentialBuilder::default()
                .with_jitter()
                .with_min_delay(Duration::from_millis(500))
                .with_max_times(3),
        )
        .await
        .map_err(|e| Error::DownloadFailed {
            url: url.into(),
            reason: vp_shared::format_error_chain(&e).into(),
        })?;

    Ok(content)
}

/// Fetch JSON with conditional request support.
///
/// If `if_none_match` is provided, sends `If-None-Match` header for conditional request.
/// The request, response body, and JSON decoding are retried as one operation so a
/// truncated body cannot escape the retry boundary as a deserialization error.
pub async fn fetch_json_with_cache_headers<T: DeserializeOwned>(
    url: &str,
    if_none_match: Option<&str>,
) -> Result<CachedFetchResponse<T>, Error> {
    let client = vp_shared::shared_http_client()?;

    tracing::debug!("Fetching with cache headers from {url}");

    (|| async {
        let mut request = client.get(url);

        if let Some(etag) = if_none_match {
            request = request.header("If-None-Match", etag);
        }

        let response = request.send().await?.error_for_status()?;

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            tracing::debug!("Received 304 Not Modified for {url}");
            return Ok(CachedFetchResponse {
                body: None,
                etag: None,
                max_age: None,
                not_modified: true,
            });
        }

        // Extract headers before consuming the response.
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(std::convert::Into::into);

        let max_age = response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_max_age);

        let bytes = response.bytes().await?;
        let body = serde_json::from_slice(&bytes)?;

        Ok::<CachedFetchResponse<T>, Error>(CachedFetchResponse {
            body: Some(body),
            etag,
            max_age,
            not_modified: false,
        })
    })
    .retry(
        ExponentialBuilder::default()
            .with_jitter()
            .with_min_delay(Duration::from_millis(500))
            .with_max_times(3),
    )
    .await
    .map_err(|e| Error::DownloadFailed {
        url: url.into(),
        reason: vp_shared::format_error_chain(&e).into(),
    })
}

/// Parse max-age from Cache-Control header value
/// Example: "public, max-age=300" -> Some(300)
fn parse_max_age(cache_control: &str) -> Option<u64> {
    for directive in cache_control.split(',') {
        let directive = directive.trim();
        if let Some(value) = directive.strip_prefix("max-age=") {
            return value.trim().parse().ok();
        }
    }
    None
}

/// Verify file hash against expected SHA256 hash
pub async fn verify_file_hash(
    file_path: &AbsolutePath,
    expected_hash: &str,
    filename: &str,
) -> Result<(), Error> {
    tracing::debug!("Verifying hash for {filename}");

    let content = fs::read(file_path).await?;

    let mut hasher = Sha256::new();
    hasher.update(&content);
    let actual_hash: Str = hex::encode(hasher.finalize()).into();

    if actual_hash != expected_hash {
        return Err(Error::HashMismatch {
            filename: filename.into(),
            expected: expected_hash.into(),
            actual: actual_hash,
        });
    }

    tracing::debug!("Hash verification successful for {filename}");
    Ok(())
}

/// Extract archive based on format
pub async fn extract_archive(
    archive_path: &AbsolutePath,
    target_dir: &AbsolutePath,
    format: ArchiveFormat,
) -> Result<(), Error> {
    let archive_path = AbsolutePathBuf::new(archive_path.as_path().to_path_buf()).unwrap();
    let target_dir = AbsolutePathBuf::new(target_dir.as_path().to_path_buf()).unwrap();

    tokio::task::spawn_blocking(move || match format {
        ArchiveFormat::Zip => extract_zip(&archive_path, &target_dir),
        ArchiveFormat::TarGz => extract_tar_gz(&archive_path, &target_dir),
    })
    .await??;

    Ok(())
}

/// Extract a tar.gz archive
fn extract_tar_gz(archive_path: &AbsolutePath, target_dir: &AbsolutePath) -> Result<(), Error> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    tracing::debug!("Extracting tar.gz: {archive_path:?} to {target_dir:?}");

    let file = File::open(archive_path)?;
    let tar_stream = GzDecoder::new(file);
    let mut archive = Archive::new(tar_stream);
    archive.unpack(target_dir)?;

    tracing::debug!("Extraction completed");
    Ok(())
}

/// Extract a zip archive
fn extract_zip(archive_path: &AbsolutePath, target_dir: &AbsolutePath) -> Result<(), Error> {
    tracing::debug!("Extracting zip: {archive_path:?} to {target_dir:?}");

    let file = File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| Error::ExtractionFailed { reason: vt_str::format!("{e}") })?;

    archive
        .extract(target_dir)
        .map_err(|e| Error::ExtractionFailed { reason: vt_str::format!("{e}") })?;

    tracing::debug!("Extraction completed");
    Ok(())
}

/// Move extracted directory to cache location with atomic operations and file-based locking
///
/// Uses a file-based lock to ensure atomicity when multiple processes/threads
/// try to install the same runtime version concurrently.
pub async fn move_to_cache(
    source: &AbsolutePath,
    target: &AbsolutePathBuf,
    binary_path: &AbsolutePath,
    version: &str,
) -> Result<(), Error> {
    // Create parent directory
    let parent = target.parent().ok_or_else(|| Error::ExtractionFailed {
        reason: "Target path has no parent directory".into(),
    })?;
    fs::create_dir_all(&parent).await?;

    // Use a file-based lock to ensure atomicity of the move operation.
    // This prevents race conditions when multiple processes/threads
    // try to install the same runtime version concurrently.
    let lock_path = parent.join(vt_str::format!("{version}.lock"));
    tracing::debug!("Acquiring lock file: {lock_path:?}");

    // Acquire file lock in a blocking task to avoid blocking the async runtime.
    // The lock() call blocks until the lock is acquired.
    let lock_path_clone = lock_path.clone();
    // Store the lock file to keep it alive until end of function
    let _lock_guard = tokio::task::spawn_blocking(move || {
        let lock_file = File::create(lock_path_clone.as_path())?;
        // Acquire exclusive lock (blocks until available)
        lock_file.lock()?;
        tracing::debug!("Lock acquired: {lock_path_clone:?}");
        Ok::<_, std::io::Error>(lock_file)
    })
    .await??;
    tracing::debug!("Lock acquired: {lock_path:?}");

    // Under the lock, decide by the binary (not just the directory) and clean up
    // here, so it can't delete a peer's install mid-race:
    //   - binary present     -> a peer's complete install, keep it and skip
    //   - dir without binary -> a stale/partial install, remove then move
    if fs::try_exists(binary_path.as_path()).await.unwrap_or(false) {
        tracing::debug!("Complete install already present at {target:?}, skipping move");
        return Ok(());
    }
    if fs::try_exists(target.as_path()).await.unwrap_or(false) {
        tracing::debug!("Removing incomplete install at {target:?} before move");
        fs::remove_dir_all(target.as_path()).await?;
    }

    // Atomic rename (lock is still held)
    fs::rename(source.as_path(), target.as_path()).await?;
    tracing::debug!("Atomic rename successful: {source:?} -> {target:?}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use tempfile::TempDir;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use super::*;

    fn temp_target(dir: &TempDir) -> AbsolutePathBuf {
        AbsolutePathBuf::new(dir.path().join("archive.tar.gz")).unwrap()
    }

    async fn read_request(socket: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
        }
        String::from_utf8(request).unwrap()
    }

    #[tokio::test]
    async fn resumes_after_a_real_truncated_connection() {
        static FULL_BODY: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
        let split_at = 10usize;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut first, _) = listener.accept().await.unwrap();
            let request = read_request(&mut first).await;
            assert!(!request.to_ascii_lowercase().contains("\r\nrange:"));
            first
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        FULL_BODY.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            first.write_all(&FULL_BODY[..split_at]).await.unwrap();
            first.shutdown().await.unwrap();

            let (mut second, _) = listener.accept().await.unwrap();
            let request = read_request(&mut second).await.to_ascii_lowercase();
            assert!(request.contains(&format!("\r\nrange: bytes={split_at}-\r\n")));
            let remaining = &FULL_BODY[split_at..];
            second
                .write_all(
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {split_at}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        FULL_BODY.len() - 1,
                        FULL_BODY.len(),
                        remaining.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            second.write_all(remaining).await.unwrap();
            second.shutdown().await.unwrap();
        });
        let dir = TempDir::new().unwrap();
        let target = temp_target(&dir);
        download_file(&format!("http://{addr}/archive.bin"), &target, "Downloading test")
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(tokio::fs::read(target.as_path()).await.unwrap(), FULL_BODY);
    }

    #[tokio::test]
    async fn server_ignoring_range_with_200_does_not_append() {
        let server = MockServer::start();
        let dir = TempDir::new().unwrap();
        let target = temp_target(&dir);

        let full_body = b"the-quick-brown-fox-jumps-over-the-lazy-dog-1234567890";
        let split_at = 12usize;
        tokio::fs::write(target.as_path(), &full_body[..split_at]).await.unwrap();

        server.mock(|when, then| {
            when.method(GET).path("/archive.bin").header("range", format!("bytes={split_at}-"));
            then.status(200)
                .header("accept-ranges", "bytes")
                .header("content-length", full_body.len().to_string())
                .body(full_body);
        });

        let url = format!("{}/archive.bin", server.base_url());
        download_file(&url, &target, "Downloading test").await.unwrap();

        assert_eq!(tokio::fs::read(target.as_path()).await.unwrap(), full_body);
    }

    // Regression for a retry after content-integrity failure: the file on
    // disk is already the *full* archive (a complete-but-corrupt previous
    // attempt), so the resume offset equals the file's total length. A
    // range-capable server answers that with `416 Range Not Satisfiable`
    // instead of `206`. Before the fix, `error_for_status()` turned that into
    // a hard error and every retry re-sent the same doomed Range request
    // against the same full-length file, so the download could never recover.
    #[tokio::test]
    async fn full_length_file_gets_416_and_restarts_with_a_plain_request() {
        let server = MockServer::start();
        let dir = TempDir::new().unwrap();
        let target = temp_target(&dir);

        let full_body = b"the-full-archive-bytes-already-on-disk-from-a-prior-attempt";
        // Simulate a complete-but-corrupt archive already on disk (e.g. one
        // that failed hash verification and is being retried by the outer
        // content-integrity loop in `runtime.rs`).
        tokio::fs::write(target.as_path(), full_body).await.unwrap();

        let range_mock = server.mock(|when, then| {
            when.method(GET)
                .path("/archive.bin")
                .header("range", format!("bytes={}-", full_body.len()));
            then.status(416);
        });
        let plain_mock = server.mock(|when, then| {
            when.method(GET).path("/archive.bin");
            then.status(200).header("content-length", full_body.len().to_string()).body(full_body);
        });

        let url = format!("{}/archive.bin", server.base_url());
        download_file(&url, &target, "Downloading test").await.unwrap();

        range_mock.assert_hits(1);
        plain_mock.assert_hits(1);
        assert_eq!(tokio::fs::read(target.as_path()).await.unwrap(), full_body);
    }

    #[tokio::test]
    async fn mismatched_content_range_falls_back_to_a_plain_request() {
        let server = MockServer::start();
        let dir = TempDir::new().unwrap();
        let target = temp_target(&dir);

        let full_body = b"another-test-body-with-enough-bytes-to-split-in-the-middle";
        let split_at = 15usize;
        tokio::fs::write(target.as_path(), &full_body[..split_at]).await.unwrap();

        let bad_range_mock = server.mock(|when, then| {
            when.method(GET).path("/archive.bin").header("range", format!("bytes={split_at}-"));
            then.status(206)
                .header(
                    "content-range",
                    format!("bytes 0-{}/{}", full_body.len() - 1, full_body.len()),
                )
                .header("content-length", full_body.len().to_string())
                .body(full_body);
        });
        server.mock(|when, then| {
            when.method(GET).path("/archive.bin");
            then.status(200).header("content-length", full_body.len().to_string()).body(full_body);
        });

        let url = format!("{}/archive.bin", server.base_url());
        download_file(&url, &target, "Downloading test").await.unwrap();

        bad_range_mock.assert_hits(1);
        assert_eq!(tokio::fs::read(target.as_path()).await.unwrap(), full_body);
    }

    #[test]
    fn parses_content_range() {
        assert_eq!(
            parse_content_range("bytes 100-199/1000"),
            Some(ParsedContentRange { start: 100, end: 199, total: 1000 })
        );
        for invalid in [
            "bytes */1000",
            "bytes 5-4/10",
            "bytes 0-10/10",
            "not-bytes 100-199/1000",
            "bytes 100/1000",
            "",
        ] {
            assert_eq!(parse_content_range(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn test_parse_max_age() {
        assert_eq!(parse_max_age("max-age=300"), Some(300));
        assert_eq!(parse_max_age("public, max-age=300"), Some(300));
        assert_eq!(parse_max_age("public, max-age=3600, immutable"), Some(3600));
        assert_eq!(parse_max_age("no-cache"), None);
        assert_eq!(parse_max_age(""), None);
        assert_eq!(parse_max_age("max-age=invalid"), None);
    }

    fn cache_root(dir: &TempDir) -> AbsolutePathBuf {
        AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap()
    }

    /// A fresh extracted download containing the binary.
    async fn make_source(cache: &AbsolutePathBuf, contents: &[u8]) -> AbsolutePathBuf {
        let source = cache.join("extract");
        tokio::fs::create_dir_all(source.join("bin").as_path()).await.unwrap();
        tokio::fs::write(source.join("bin").join("node").as_path(), contents).await.unwrap();
        source
    }

    #[tokio::test]
    async fn move_to_cache_moves_into_absent_target() {
        let root = TempDir::new().unwrap();
        let cache = cache_root(&root);
        let source = make_source(&cache, b"new").await;

        let target = cache.join("node").join("1.0.0");
        let binary = target.join("bin").join("node");

        move_to_cache(&source, &target, &binary, "1.0.0").await.unwrap();

        assert_eq!(tokio::fs::read(binary.as_path()).await.unwrap(), b"new");
    }

    #[tokio::test]
    async fn move_to_cache_replaces_stale_incomplete_target() {
        let root = TempDir::new().unwrap();
        let cache = cache_root(&root);
        let source = make_source(&cache, b"new").await;

        // Stale install: directory exists but the binary is missing.
        let target = cache.join("node").join("1.0.0");
        tokio::fs::create_dir_all(target.join("leftover").as_path()).await.unwrap();
        let binary = target.join("bin").join("node");

        move_to_cache(&source, &target, &binary, "1.0.0").await.unwrap();

        assert_eq!(tokio::fs::read(binary.as_path()).await.unwrap(), b"new");
        assert!(!tokio::fs::try_exists(target.join("leftover").as_path()).await.unwrap());
    }

    // Regression for the concurrent-install TOCTOU: a peer's complete install
    // (dir + binary) must be kept, not clobbered by our own fresh copy.
    #[tokio::test]
    async fn move_to_cache_keeps_a_complete_target_intact() {
        let root = TempDir::new().unwrap();
        let cache = cache_root(&root);
        let source = make_source(&cache, b"ours").await;

        // A peer's complete install already at the target.
        let target = cache.join("node").join("1.0.0");
        let binary = target.join("bin").join("node");
        tokio::fs::create_dir_all(target.join("bin").as_path()).await.unwrap();
        tokio::fs::write(binary.as_path(), b"peer").await.unwrap();

        move_to_cache(&source, &target, &binary, "1.0.0").await.unwrap();

        // The peer's binary is preserved, not clobbered.
        assert_eq!(tokio::fs::read(binary.as_path()).await.unwrap(), b"peer");
        // Our source download is left untouched (not moved over the peer's install).
        assert!(tokio::fs::try_exists(source.as_path()).await.unwrap());
    }
}
