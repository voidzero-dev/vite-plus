//! Tracing initialization for vite-plus

use std::{str::FromStr, sync::OnceLock};

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
/// Only sets the global default subscriber when `VITE_LOG` is set.
/// When unset, the global default slot is left free so that other
/// subscribers (e.g., rolldown devtools) can claim it without panicking.
///
/// # Environment Variables
/// - `VITE_LOG`: Controls log filtering (e.g., "debug", "vite_task=trace")
pub fn init_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        let Ok(env_var) = std::env::var(env_vars::VITE_LOG) else {
            return;
        };

        tracing_subscriber::registry()
            .with(
                Targets::from_str(&env_var)
                    .unwrap_or_default()
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .ok();
    });
}
