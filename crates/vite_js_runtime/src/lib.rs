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
//!
//! # Project-Based Runtime Download
//!
//! You can also download a runtime based on a project's `devEngines.runtime` configuration:
//!
//! ```rust,ignore
//! use vite_js_runtime::download_runtime_for_project;
//! use vite_path::AbsolutePathBuf;
//!
//! let project_path = AbsolutePathBuf::new("/path/to/project".into()).unwrap();
//! let runtime = download_runtime_for_project(&project_path).await?;
//! ```
//!
//! # Adding a New Runtime
//!
//! To add support for a new JavaScript runtime (e.g., Bun, Deno):
//!
//! 1. Create a new provider in `src/providers/` implementing `JsRuntimeProvider`
//! 2. Add the runtime type to `JsRuntimeType` enum
//! 3. Add a match arm in `download_runtime()` to use the new provider

mod cache;
mod dev_engines;
mod download;
mod error;
mod platform;
mod provider;
mod providers;
mod runtime;

pub use dev_engines::{
    parse_node_version_content, read_node_version_file, write_node_version_file,
};
pub use error::Error;
pub use platform::{Arch, Os, Platform};
pub use provider::{ArchiveFormat, DownloadInfo, HashVerification, JsRuntimeProvider};
pub use providers::{LtsInfo, NodeProvider, NodeVersionEntry};
pub use runtime::{
    JsRuntime, JsRuntimeType, VersionResolution, VersionSource, download_runtime,
    download_runtime_for_project, download_runtime_with_provider, is_valid_version,
    normalize_version, read_package_json, resolve_node_version,
};
