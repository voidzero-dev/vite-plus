//! JavaScript runtime provider implementations.
//!
//! This module contains implementations of the `JsRuntimeProvider` trait
//! for each supported JavaScript runtime.

mod node;

pub use node::{LtsInfo, NodeProvider, NodeVersionEntry};
