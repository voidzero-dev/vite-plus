//! Tracing initialization for vite-plus
//!
//! ## Environment Variables
//! - `VITE_LOG`: Controls log filtering (e.g., `"debug"`, `"vite_task=trace"`)
//! - `VITE_LOG_OUTPUT`: Output format — `"chrome-json"` for Chrome DevTools timeline,
//!   `"readable"` for pretty-printed output, or default stdout.
//! - `VITE_LOG_OUTPUT_DIR`: Directory for chrome-json trace files (default: cwd).

use std::{any::Any, path::PathBuf, sync::atomic::AtomicBool};

use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    fmt::{self, format::FmtSpan},
    prelude::*,
};

use crate::env_vars;

static IS_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize tracing with `VITE_LOG` and `VITE_LOG_OUTPUT` environment variables.
///
/// Returns an optional guard that must be kept alive for the duration of the
/// program when using file-based output (e.g., `chrome-json`). Dropping the
/// guard flushes and finalizes the trace file.
///
/// Uses `AtomicBool` to ensure tracing is only initialized once.
pub fn init_tracing() -> Option<Box<dyn Any + Send>> {
    if IS_INITIALIZED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return None;
    }

    let Ok(env_var) = std::env::var(env_vars::VITE_LOG) else {
        // Tracing is disabled by default (performance sensitive)
        return None;
    };

    let targets = {
        use std::str::FromStr;
        Targets::from_str(&env_var)
            .unwrap_or_default()
            // disable brush-parser tracing
            .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)])
    };

    let output_mode =
        std::env::var(env_vars::VITE_LOG_OUTPUT).unwrap_or_else(|_| "stdout".to_string());

    match output_mode.as_str() {
        "chrome-json" => {
            let mut builder = ChromeLayerBuilder::new()
                .trace_style(tracing_chrome::TraceStyle::Async)
                .include_args(true);
            // Write trace files to VITE_LOG_OUTPUT_DIR if set, to avoid
            // polluting the project directory (formatters may pick them up).
            if let Ok(dir) = std::env::var(env_vars::VITE_LOG_OUTPUT_DIR) {
                let dir = PathBuf::from(dir);
                let _ = std::fs::create_dir_all(&dir);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_micros())
                    .unwrap_or(0);
                builder = builder.file(dir.join(format!("trace-{ts}.json")));
            }
            let (chrome_layer, guard) = builder.build();
            tracing_subscriber::registry().with(targets).with(chrome_layer).init();
            Some(Box::new(guard))
        }
        "readable" => {
            tracing_subscriber::registry()
                .with(targets)
                .with(
                    fmt::layer()
                        .pretty()
                        .with_span_events(FmtSpan::NONE)
                        .with_level(true)
                        .with_target(false),
                )
                .init();
            None
        }
        _ => {
            // Default: stdout with span events
            tracing_subscriber::registry()
                .with(targets)
                .with(fmt::layer().with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER))
                .init();
            None
        }
    }
}
