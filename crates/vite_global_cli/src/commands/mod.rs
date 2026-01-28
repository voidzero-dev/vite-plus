//! Command implementations for the global CLI.
//!
//! Commands are organized by category:
//!
//! Category A - Package manager commands:
//! - `add`: Add packages to dependencies
//! - `install`: Install all dependencies
//! - `remove`: Remove packages from dependencies
//! - `update`: Update packages to their latest versions
//! - `dedupe`: Deduplicate dependencies
//! - `outdated`: Check for outdated packages
//! - `why`: Show why a package is installed
//! - `link`: Link packages for local development
//! - `unlink`: Unlink packages
//! - `dlx`: Execute a package binary without installing it
//! - `pm`: Forward commands to the package manager
//!
//! Category B - JS Script Commands:
//! - `new`: Project scaffolding
//! - `migrate`: Migration command
//! - `version`: Version display
//!
//! Category C - Local CLI Delegation:
//! - `delegate`: Local CLI delegation

// Category A: Package manager commands
pub mod add;
pub mod dedupe;
pub mod dlx;
pub mod install;
pub mod link;
pub mod outdated;
pub mod pm;
pub mod remove;
pub mod unlink;
pub mod update;
pub mod why;

// Category B: JS Script Commands
pub mod migrate;
pub mod new;
pub mod version;

// Category C: Local CLI Delegation
pub mod delegate;

// Re-export command structs for convenient access
pub use add::AddCommand;
pub use dedupe::DedupeCommand;
pub use dlx::DlxCommand;
pub use install::InstallCommand;
pub use link::LinkCommand;
pub use outdated::OutdatedCommand;
pub use remove::RemoveCommand;
pub use unlink::UnlinkCommand;
pub use update::UpdateCommand;
pub use why::WhyCommand;
