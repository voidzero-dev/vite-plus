#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::print_stdout
)]

mod ast_grep;
mod eslint;
mod file_walker;
mod import_rewriter;
mod package;
mod prettier;
mod script_rewrite;
mod vite_config;

pub use file_walker::{WalkResult, find_ts_files};
pub use import_rewriter::{BatchRewriteResult, rewrite_imports_in_directory};
pub use package::{rewrite_eslint, rewrite_prettier, rewrite_scripts};
pub use vite_config::{MergeResult, merge_json_config, merge_tsdown_config};
