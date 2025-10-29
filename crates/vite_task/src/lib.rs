mod cache;
mod cmd;
mod collections;
mod config;
mod error;
mod execute;
mod fingerprint;
mod fs;
mod maybe_str;
mod schedule;
mod types;
mod ui;

#[cfg(test)]
mod test_utils;

// Public exports for vite-plus-cli to use
pub use cache::TaskCache;
pub use config::{ResolvedTask, Workspace};
pub use error::Error;
pub use execute::{CURRENT_EXECUTION_ID, EXECUTION_SUMMARY_DIR};
pub use schedule::{ExecutionPlan, ExecutionStatus, ExecutionSummary};
pub use types::ResolveCommandResult;
