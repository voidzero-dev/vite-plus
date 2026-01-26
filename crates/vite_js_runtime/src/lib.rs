//! JavaScript Runtime Management Library
//!
//! This crate provides functionality to download and cache JavaScript runtimes
//! like Node.js. It supports automatic platform detection, integrity verification
//! via SHASUMS256.txt, and atomic operations for concurrent-safe caching.
//!
//! # Example
//!
//! ```rust,ignore
//! use vite_js_runtime::{JsRuntimeType, download_runtime, parse_runtime_spec};
//!
//! // Option 1: Direct download with known runtime type
//! let runtime = download_runtime(JsRuntimeType::Node, "22.13.1").await?;
//! println!("Node.js installed at: {}", runtime.get_binary_path());
//!
//! // Option 2: Parse spec string first
//! let (runtime_type, version) = parse_runtime_spec("node@22.13.1")?;
//! let runtime = download_runtime(runtime_type, &version).await?;
//! ```

mod error;
mod node;
mod platform;
mod runtime;

pub use error::Error;
pub use platform::Platform;
pub use runtime::{JsRuntime, JsRuntimeType, download_runtime, parse_runtime_spec};
