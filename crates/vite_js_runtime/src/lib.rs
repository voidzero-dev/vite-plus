//! JavaScript Runtime Management Library
//!
//! This crate provides functionality to download and cache JavaScript runtimes
//! like Node.js. It supports automatic platform detection, integrity verification
//! via SHASUMS256.txt, and atomic operations for concurrent-safe caching.
//!
//! # Example
//!
//! ```rust,ignore
//! use vite_js_runtime::{JsRuntimeType, download_runtime};
//!
//! let runtime = download_runtime(JsRuntimeType::Node, "22.13.1").await?;
//! println!("Node.js installed at: {}", runtime.get_binary_path());
//! ```

mod error;
mod node;
mod platform;
mod runtime;

pub use error::Error;
pub use platform::Platform;
pub use runtime::{JsRuntime, JsRuntimeType, download_runtime};
