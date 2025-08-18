use std::path::Path;
use std::time::Duration;

use flate2::read::GzDecoder;
use futures_util::stream::StreamExt;
use tar::Archive;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use vite_error::Error;

/// Download a tgz file with retry logic using exponential backoff.
///
/// # Arguments
///
/// * `url` - The URL of the tgz file to download.
/// * `target_path` - The path where the tgz file will be saved.
/// * `max_retries` - Maximum number of retry attempts (default: 3).
///
/// # Returns
///
/// * `Ok(())` - If the file is downloaded successfully.
/// * `Err(e)` - If all retry attempts fail.
async fn download_file_with_retry(
    url: &str,
    target_path: impl AsRef<Path>,
    max_retries: Option<u32>,
) -> Result<(), Error> {
    let target_path = target_path.as_ref();
    let max_retries = max_retries.unwrap_or(3);
    let mut attempt = 0;
    let mut last_error = None;

    while attempt <= max_retries {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s, 4s, 8s...
            let wait_seconds = 2_u64.pow(attempt - 1);
            let wait_duration = Duration::from_secs(wait_seconds);

            tracing::warn!(
                "Download attempt {} failed, retrying in {} seconds...",
                attempt,
                wait_seconds
            );
            tokio::time::sleep(wait_duration).await;
        }

        tracing::debug!(
            "Downloading {} to {:?} (attempt {}/{})",
            url,
            target_path,
            attempt + 1,
            max_retries + 1
        );

        match download_file_internal(url, target_path).await {
            Ok(()) => {
                if attempt > 0 {
                    tracing::info!("Download succeeded on attempt {}", attempt + 1);
                }
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Download attempt {} failed: {}", attempt + 1, e);
                last_error = Some(e);
                attempt += 1;
            }
        }
    }

    // All retries exhausted
    Err(last_error.unwrap_or_else(|| {
        Error::AnyhowError(anyhow::anyhow!(
            "Failed to download {} after {} attempts",
            url,
            max_retries + 1
        ))
    }))
}

/// Internal function to perform the actual download.
async fn download_file_internal(url: &str, target_path: &Path) -> Result<(), Error> {
    // Make the HTTP request
    let response = reqwest::get(url).await?.error_for_status()?;

    // Create the target file
    let mut file = fs::File::create(target_path).await?;

    // Stream the response body to the file
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    tracing::debug!("Download completed: {:?}", target_path);
    Ok(())
}

fn extract_tgz(tgz_file: impl AsRef<Path>, target_dir: impl AsRef<Path>) -> Result<(), Error> {
    let tgz_file = tgz_file.as_ref();
    let target_dir = target_dir.as_ref();
    tracing::debug!("Extract tgz: {:?} to {:?}", tgz_file, target_dir);

    let file = std::fs::File::open(&tgz_file)?;
    let tar_stream = GzDecoder::new(file);
    let mut archive = Archive::new(tar_stream);
    archive.unpack(&target_dir)?;

    tracing::debug!("Extract tgz finished");

    Ok(())
}

/// Download tgz file from url and extract it to the target directory.
///
/// # Arguments
///
/// * `url` - The url of the tgz file.
/// * `target_dir` - The directory to extract the tgz file to.
///
/// # Returns
///
/// * `Ok(())` - If the tgz file is downloaded and extracted successfully.
/// * `Err(e)` - If the tgz file is not downloaded or extracted successfully.
pub async fn download_and_extract_tgz(
    url: &str,
    target_dir: impl AsRef<Path>,
) -> Result<(), Error> {
    let target_dir = target_dir.as_ref().to_path_buf();
    tracing::debug!("Start download and extract {} to {:?}", url, target_dir);

    // Create target directory
    fs::create_dir_all(&target_dir).await.map_err(|e| Error::IoWithPathAndOperation {
        err: e,
        path: target_dir.clone(),
        operation: "create_dir_all".into(),
    })?;

    // Download the tgz file with retry logic
    let tgz_file = target_dir.join("package.tgz");
    download_file_with_retry(url, &tgz_file, None).await?;

    // Extract the tgz file to the target directory
    let tgz_file_for_extract = tgz_file.clone();
    let target_dir_for_extract = target_dir.clone();
    tokio::task::spawn_blocking(move || {
        extract_tgz(&tgz_file_for_extract, &target_dir_for_extract)
    })
    .await??;

    // Remove the temp file
    fs::remove_file(&tgz_file).await.map_err(|e| Error::IoWithPathAndOperation {
        err: e,
        path: tgz_file.clone(),
        operation: "remove_file".into(),
    })?;

    tracing::debug!("Download and extract finished");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    #[tokio::test]
    #[test_log::test]
    async fn test_extract_tgz_function() {
        // Test the extract_tgz function directly
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");

        // Create a simple tar.gz file content for testing
        let test_content = b"test file content";
        let mut tar_builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(test_content.len() as u64);
        tar_builder
            .append_data(&mut header, "test.txt", std::io::Cursor::new(test_content))
            .unwrap();
        let tar_data = tar_builder.into_inner().unwrap();

        // Compress with gzip
        let mut gz_data = Vec::new();
        {
            let mut encoder =
                flate2::write::GzEncoder::new(&mut gz_data, flate2::Compression::default());
            std::io::copy(&mut std::io::Cursor::new(tar_data), &mut encoder).unwrap();
        }

        // Write the compressed data to a temporary file
        let tgz_file = temp_dir.path().join("test.tgz");
        fs::write(&tgz_file, gz_data).unwrap();

        // Test extraction
        let result = extract_tgz(&tgz_file, &target_dir);
        assert!(result.is_ok());

        // Verify the file was extracted
        let extracted_file = target_dir.join("test.txt");
        assert!(extracted_file.exists());

        // Verify the content
        let content = fs::read_to_string(extracted_file).unwrap();
        assert_eq!(content, "test file content");
    }

    #[tokio::test]
    async fn test_extract_tgz_large_file() {
        // Test extraction with a larger file
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");

        // Create a larger tar.gz file for testing
        let large_content = vec![b'a'; 1024 * 1024]; // 1MB
        let mut tar_builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(large_content.len() as u64);
        tar_builder
            .append_data(&mut header, "large.txt", std::io::Cursor::new(&large_content))
            .unwrap();
        let tar_data = tar_builder.into_inner().unwrap();

        // Compress with gzip
        let mut gz_data = Vec::new();
        {
            let mut encoder =
                flate2::write::GzEncoder::new(&mut gz_data, flate2::Compression::default());
            std::io::copy(&mut std::io::Cursor::new(tar_data), &mut encoder).unwrap();
        }

        // Write the compressed data to a temporary file
        let tgz_file = temp_dir.path().join("large.tgz");
        fs::write(&tgz_file, gz_data).unwrap();

        // Test extraction
        let result = extract_tgz(&tgz_file, &target_dir);
        assert!(result.is_ok());

        // Verify the file was extracted
        let extracted_file = target_dir.join("large.txt");
        assert!(extracted_file.exists());

        // Verify the content size
        let content = fs::read(extracted_file).unwrap();
        assert_eq!(content.len(), 1024 * 1024);
    }

    #[tokio::test]
    async fn test_extract_tgz_invalid_file() {
        // Test extraction with invalid tar.gz content
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");

        // Create an invalid tar.gz file
        let invalid_content = b"this is not a valid tar.gz file";
        let tgz_file = temp_dir.path().join("invalid.tgz");
        fs::write(&tgz_file, invalid_content).unwrap();

        // Test extraction - should fail gracefully
        let result = extract_tgz(&tgz_file, &target_dir);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_extract_tgz_empty_file() {
        // Test extraction with empty tar.gz
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");

        // Create an empty tar.gz file
        let tgz_file = temp_dir.path().join("empty.tgz");
        fs::write(&tgz_file, Vec::<u8>::new()).unwrap();

        // Test extraction - should handle empty file gracefully
        let result = extract_tgz(&tgz_file, &target_dir);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_and_extract_tgz() {
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");
        let url = "https://registry.npmjs.org/@yarnpkg/cli-dist/-/cli-dist-4.9.2.tgz";

        let result = download_and_extract_tgz(url, &target_dir).await;
        assert!(result.is_ok());

        assert!(target_dir.join("package/bin/yarn").exists());
        assert!(target_dir.join("package/bin/yarn.cmd").exists());

        // TempDir automatically cleans up when it goes out of scope
    }

    #[tokio::test]
    async fn test_download_with_retry_invalid_url() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.tgz");

        // This URL should fail
        let invalid_url =
            "https://registry.npmjs.org/nonexistent-package-that-doesnt-exist/-/package-1.0.0.tgz";

        // Should fail after retries
        let result = download_file_with_retry(invalid_url, &target_file, Some(2)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_download_with_retry_success() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.tgz");

        // Use a small, reliable package for testing
        let url = "https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz";

        // Should succeed
        let result = download_file_with_retry(url, &target_file, Some(2)).await;
        assert!(result.is_ok());

        // Verify file exists and has content
        assert!(target_file.exists());
        let metadata = fs::metadata(&target_file).unwrap();
        assert!(metadata.len() > 0);
    }

    #[tokio::test]
    async fn test_download_with_custom_retries() {
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.tgz");

        // Invalid URL to force retries
        let invalid_url = "https://httpstat.us/500"; // Returns 500 Internal Server Error

        // Should fail after custom number of retries
        let result = download_file_with_retry(invalid_url, &target_file, Some(1)).await;
        assert!(result.is_err());
    }
}
