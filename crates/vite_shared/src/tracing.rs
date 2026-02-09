//! Tracing initialization for vite-plus

use std::sync::OnceLock;

use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    prelude::*,
};

use crate::env_vars;

/// Initialize tracing with VITE_LOG environment variable.
///
/// Uses `OnceLock` to ensure tracing is only initialized once,
/// even if called multiple times.
///
/// # Environment Variables
/// - `VITE_LOG`: Controls log filtering (e.g., "debug", "vite_task=trace")
pub fn init_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        tracing_subscriber::registry()
            .with(
                std::env::var(env_vars::VITE_LOG)
                    .map_or_else(
                        |_| Targets::new(),
                        |env_var| {
                            use std::str::FromStr;
                            Targets::from_str(&env_var).unwrap_or_default()
                        },
                    )
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}
