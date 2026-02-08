//! Tracing initialization for vite-plus

use std::sync::OnceLock;

use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    prelude::*,
};

use crate::EnvConfig;

/// Initialize tracing with the current `EnvConfig`.
///
/// Uses `EnvConfig::get().vite_log` for the log filter.
///
/// Uses `OnceLock` to ensure tracing is only initialized once,
/// even if called multiple times.
pub fn init_tracing() {
    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        let config = EnvConfig::get();
        let targets = match config.vite_log {
            Some(ref env_var) => {
                use std::str::FromStr;
                Targets::from_str(env_var).unwrap_or_default()
            }
            None => Targets::new(),
        };

        tracing_subscriber::registry()
            .with(
                targets
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}
