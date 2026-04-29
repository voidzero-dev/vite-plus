//! Shared clap surface and dispatcher for `vp`'s package-manager
//! subcommands (`install`, `add`, `remove`, `update`, `dlx`, `pm …`, …).
//!
//! Both the global CLI and the local NAPI binding flatten
//! [`PackageManagerCommand`] into their top-level argument parser and call
//! [`dispatch`] to execute the parsed command. The crate does not do any
//! managed-Node-runtime or managed-global-install handling — those stay in
//! the global CLI; PM operations always go through whichever package
//! manager (pnpm/npm/yarn/bun) is detected for the project.

pub mod cli;
pub mod dispatch;
pub mod error;
pub mod handlers;
pub mod helpers;

pub use cli::PackageManagerCommand;
pub use dispatch::dispatch;
pub use error::Error;
