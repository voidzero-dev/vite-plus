//! Command implementations for the global CLI.
//!
//! Commands are organized by category:
//! - `pm`: Package manager commands (Category A)
//! - `new`: Project scaffolding (Category B)
//! - `migrate`: Migration command (Category B)
//! - `version`: Version display (Category B)
//! - `delegate`: Local CLI delegation (Category C)

pub mod delegate;
pub mod migrate;
pub mod new;
pub mod pm;
pub mod version;
