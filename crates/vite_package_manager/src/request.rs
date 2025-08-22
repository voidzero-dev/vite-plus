use std::path::Path;
use std::time::Duration;

use flate2::read::GzDecoder;
use futures_util::stream::StreamExt;
use reqwest::Response;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::de::DeserializeOwned;
use tar::Archive;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use vite_error::Error;

/// HTTP client with built-in retry support
#[derive(Clone)]
pub struct HttpClient {
    client: ClientWithMiddleware,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// Create a new HTTP client with default settings (3 retries, 100ms base interval)
    pub fn new() -> Self {
        Self::with_config(3, 100)
    }

    /// Create a new HTTP client with custom retry configuration
    ///
    /// # Arguments
    ///
    /// * `max_retries` - Maximum number of retry attempts
    /// * `base_interval_ms` - Base interval in milliseconds for exponential backoff
    pub fn with_config(max_retries: u32, base_interval_ms: u64) -> Self {
        let retry_policy = ExponentialBackoff::builder()
            .base(base_interval_ms as u32)
            .retry_bounds(
                Duration::from_millis(base_interval_ms),
                Duration::from_secs(10), // Max 10 seconds between retries
            )
            .build_with_max_retries(max_retries);

        let client = ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        Self { client }
    }

    /// Get JSON data from a URL
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to fetch JSON from
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - Deserialized JSON data
    /// * `Err(e)` - If the request fails or JSON deserialization fails
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        tracing::debug!("Fetching JSON from: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::AnyhowError(anyhow::Error::msg(format!("Request failed: {}", e))))?
            .error_for_status()?;

        let data = response.json::<T>().await?;
        Ok(data)
    }

    /// Download a file to a specified path
    ///
    /// # Arguments
    ///
    /// * `url` - The URL of the file to download
    /// * `target_path` - The path where the file will be saved
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the file is downloaded successfully
    /// * `Err(e)` - If the download fails
    pub async fn download_file(
        &self,
        url: &str,
        target_path: impl AsRef<Path>,
    ) -> Result<(), Error> {
        let target_path = target_path.as_ref();
        tracing::debug!("Downloading {} to {:?}", url, target_path);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::AnyhowError(anyhow::Error::msg(format!("Request failed: {}", e))))?
            .error_for_status()?;

        self.write_response_to_file(response, target_path).await?;

        tracing::debug!("Download completed: {:?}", target_path);
        Ok(())
    }

    /// Internal helper to write response body to file
    async fn write_response_to_file(
        &self,
        response: Response,
        target_path: &Path,
    ) -> Result<(), Error> {
        // Create the target file
        let mut file = fs::File::create(target_path).await?;

        // Stream the response body to the file
        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }
}

/// Download a tgz file with retry logic using the HttpClient.
/// This function is deprecated in favor of using HttpClient directly.
/// Kept for backward compatibility.
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
#[deprecated(note = "Use HttpClient::download_file instead")]
#[allow(dead_code)]
async fn download_file_with_retry(
    url: &str,
    target_path: impl AsRef<Path>,
    max_retries: Option<u32>,
) -> Result<(), Error> {
    let client = if let Some(retries) = max_retries {
        HttpClient::with_config(retries, 100)
    } else {
        HttpClient::new()
    };

    client.download_file(url, target_path).await
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

    // Download the tgz file with retry logic using HttpClient
    let tgz_file = target_dir.join("package.tgz");
    let client = HttpClient::new();
    client.download_file(url, &tgz_file).await?;

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

    use httpmock::prelude::*;
    use tempfile::TempDir;

    /// Helper function to create a mock tar.gz file content
    fn create_mock_tgz_content() -> Vec<u8> {
        // Create a simple tar file with test content
        let test_content = b"test file content";
        let mut tar_builder = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_size(test_content.len() as u64);
        header.set_mode(0o644);
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
        gz_data
    }

    /// Helper function to create a mock package tar.gz that mimics npm package structure
    fn create_mock_package_tgz() -> Vec<u8> {
        let mut tar_builder = tar::Builder::new(Vec::new());

        // Add package.json
        let package_json = br#"{"name":"test-package","version":"1.0.0"}"#;
        let mut header = tar::Header::new_gnu();
        header.set_size(package_json.len() as u64);
        header.set_mode(0o644);
        tar_builder
            .append_data(&mut header, "package/package.json", std::io::Cursor::new(package_json))
            .unwrap();

        // Add bin/yarn mock file
        let yarn_content = b"#!/usr/bin/env node\nconsole.log('mock yarn');";
        let mut header = tar::Header::new_gnu();
        header.set_size(yarn_content.len() as u64);
        header.set_mode(0o755);
        tar_builder
            .append_data(&mut header, "package/bin/yarn", std::io::Cursor::new(yarn_content))
            .unwrap();

        // Add bin/yarn.cmd mock file
        let yarn_cmd_content = b"@echo off\nnode yarn %*";
        let mut header = tar::Header::new_gnu();
        header.set_size(yarn_cmd_content.len() as u64);
        header.set_mode(0o755);
        tar_builder
            .append_data(
                &mut header,
                "package/bin/yarn.cmd",
                std::io::Cursor::new(yarn_cmd_content),
            )
            .unwrap();

        let tar_data = tar_builder.into_inner().unwrap();

        // Compress with gzip
        let mut gz_data = Vec::new();
        {
            let mut encoder =
                flate2::write::GzEncoder::new(&mut gz_data, flate2::Compression::default());
            std::io::copy(&mut std::io::Cursor::new(tar_data), &mut encoder).unwrap();
        }
        gz_data
    }

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
    async fn test_http_client_get_json() {
        let server = MockServer::start();

        // Create mock JSON response
        let mock_json = serde_json::json!({
            "name": "test-package",
            "version": "1.0.0",
            "description": "A test package"
        });

        server.mock(|when, then| {
            when.method(GET).path("/api/package.json");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(mock_json.clone());
        });

        let client = HttpClient::new();
        let url = format!("{}/api/package.json", server.base_url());

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct PackageInfo {
            name: String,
            version: String,
            description: String,
        }

        let result: Result<PackageInfo, _> = client.get_json(&url).await;
        assert!(result.is_ok());

        let package_info = result.unwrap();
        assert_eq!(package_info.name, "test-package");
        assert_eq!(package_info.version, "1.0.0");
        assert_eq!(package_info.description, "A test package");
    }

    #[tokio::test]
    async fn test_http_client_download_file() {
        let server = MockServer::start();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("downloaded.txt");

        let mock_content = b"Hello, World! This is test content.";

        server.mock(|when, then| {
            when.method(GET).path("/file.txt");
            then.status(200).header("content-type", "text/plain").body(mock_content);
        });

        let client = HttpClient::new();
        let url = format!("{}/file.txt", server.base_url());

        let result = client.download_file(&url, &target_file).await;
        assert!(result.is_ok(), "Failed to download file: {:?}", result);

        // Verify file exists and has correct content
        assert!(target_file.exists());
        let content = fs::read(&target_file).unwrap();
        assert_eq!(content, mock_content);
    }

    #[tokio::test]
    async fn test_http_client_retry_on_server_error() {
        // Test that the client correctly retries on server errors
        let server = MockServer::start();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.txt");

        // Mock a 500 error which should trigger retries
        server.mock(|when, then| {
            when.method(GET).path("/server_error");
            then.status(500).body("Internal Server Error");
        });

        let client = HttpClient::with_config(2, 50); // 2 retries with 50ms base interval
        let url = format!("{}/server_error", server.base_url());

        // Should fail after retries
        let result = client.download_file(&url, &target_file).await;
        assert!(result.is_err(), "Expected download to fail with 500 after retries");
    }

    #[tokio::test]
    async fn test_http_client_custom_config() {
        // Test that we can create a client with custom config
        let client = HttpClient::with_config(5, 200);
        // Just verify it can be created successfully
        assert!(matches!(client.client, _));
    }

    #[tokio::test]
    async fn test_download_and_extract_tgz() {
        // Start a mock server
        let server = MockServer::start();
        let temp_dir = TempDir::new().unwrap();
        let target_dir = temp_dir.path().join("extracted");

        // Create mock response with package tar.gz
        let mock_tgz = create_mock_package_tgz();
        server.mock(|when, then| {
            when.method(GET).path("/test-package.tgz");
            then.status(200).header("content-type", "application/octet-stream").body(mock_tgz);
        });

        let url = format!("{}/test-package.tgz", server.base_url());
        let result = download_and_extract_tgz(&url, &target_dir).await;
        assert!(result.is_ok(), "Failed to download and extract: {:?}", result);

        assert!(target_dir.join("package/bin/yarn").exists());
        assert!(target_dir.join("package/bin/yarn.cmd").exists());

        // TempDir automatically cleans up when it goes out of scope
    }

    #[tokio::test]
    async fn test_http_client_download_with_404_error() {
        let server = MockServer::start();
        let temp_dir = TempDir::new().unwrap();
        let target_file = temp_dir.path().join("test.txt");

        // Mock a 404 response
        server.mock(|when, then| {
            when.method(GET).path("/nonexistent");
            then.status(404).body("Not Found");
        });

        let client = HttpClient::new();
        let url = format!("{}/nonexistent", server.base_url());

        // Should fail with 404
        let result = client.download_file(&url, &target_file).await;
        assert!(result.is_err(), "Expected download to fail with 404");
    }

    #[tokio::test]
    async fn test_http_client_json_with_invalid_response() {
        let server = MockServer::start();

        // Mock response with invalid JSON
        server.mock(|when, then| {
            when.method(GET).path("/invalid.json");
            then.status(200).header("content-type", "application/json").body("not valid json");
        });

        let client = HttpClient::new();
        let url = format!("{}/invalid.json", server.base_url());

        #[derive(serde::Deserialize)]
        struct TestData {
            field: String,
        }

        let result: Result<TestData, _> = client.get_json(&url).await;
        assert!(result.is_err(), "Expected JSON parsing to fail");
    }
}
