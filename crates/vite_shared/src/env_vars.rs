//! Centralized environment variable name constants.
//!
//! Every vite-plus-specific environment variable is defined here as a `&str`
//! constant. Using these constants instead of string literals ensures:
//!
//! - **Single source of truth** — each name defined once.
//! - **Compile-time typo detection** — a misspelled constant name won't compile.
//! - **Easy discoverability** — grep this file to see all env vars.
//!
//! Standard system variables (`PATH`, `HOME`, `CI`, etc.) are intentionally
//! excluded — they're well-known and benefit less from constant definitions.

// ── Config: read once at startup via EnvConfig ──────────────────────────

/// Override for the vite-plus home directory (default: `~/.vite-plus`).
pub const VITE_PLUS_HOME: &str = "VITE_PLUS_HOME";

/// Log filter string for `tracing_subscriber` (e.g. `"debug"`, `"vite_task=trace"`).
pub const VITE_LOG: &str = "VITE_LOG";

/// NPM registry URL (lowercase form, highest priority).
pub const NPM_CONFIG_REGISTRY: &str = "npm_config_registry";

/// NPM registry URL (uppercase fallback).
pub const NPM_CONFIG_REGISTRY_UPPER: &str = "NPM_CONFIG_REGISTRY";

/// Node.js distribution mirror URL for downloads.
pub const VITE_NODE_DIST_MIRROR: &str = "VITE_NODE_DIST_MIRROR";

/// Override Node.js version (takes highest priority in version resolution).
pub const VITE_PLUS_NODE_VERSION: &str = "VITE_PLUS_NODE_VERSION";

/// Enable debug output for shim dispatch.
pub const VITE_PLUS_DEBUG_SHIM: &str = "VITE_PLUS_DEBUG_SHIM";

/// Enable eval mode for `vp env use`.
pub const VITE_PLUS_ENV_USE_EVAL_ENABLE: &str = "VITE_PLUS_ENV_USE_EVAL_ENABLE";

/// Filter for update task types.
pub const VITE_UPDATE_TASK_TYPES: &str = "VITE_UPDATE_TASK_TYPES";

/// Override directory for global CLI JS scripts.
pub const VITE_GLOBAL_CLI_JS_SCRIPTS_DIR: &str = "VITE_GLOBAL_CLI_JS_SCRIPTS_DIR";

// ── Runtime: set/removed during shim dispatch for child processes ────────

/// Bypass the vite-plus shim and use the system tool directly.
///
/// Value is a `PATH`-style list of directories to bypass.
pub const VITE_PLUS_BYPASS: &str = "VITE_PLUS_BYPASS";

/// Recursion guard for `vp env exec` — prevents infinite shim loops.
pub const VITE_PLUS_TOOL_RECURSION: &str = "VITE_PLUS_TOOL_RECURSION";

/// Set by shim dispatch to record the active Node.js version.
pub const VITE_PLUS_ACTIVE_NODE: &str = "VITE_PLUS_ACTIVE_NODE";

/// Set by shim dispatch to record how the Node.js version was resolved.
pub const VITE_PLUS_RESOLVE_SOURCE: &str = "VITE_PLUS_RESOLVE_SOURCE";

/// Set by shell wrapper scripts to indicate which tool is being shimmed.
pub const VITE_PLUS_SHIM_TOOL: &str = "VITE_PLUS_SHIM_TOOL";

/// Set by Windows shim wrappers that route through `vp env exec`.
///
/// When present, `env exec` can normalize wrapper-inserted argument separators
/// before forwarding to the actual tool.
pub const VITE_PLUS_SHIM_WRAPPER: &str = "VITE_PLUS_SHIM_WRAPPER";

/// Path to the vp binary, passed to JS scripts so they can invoke CLI commands.
pub const VITE_PLUS_CLI_BIN: &str = "VITE_PLUS_CLI_BIN";

/// Global CLI version, passed from Rust binary to JS for --version display.
pub const VITE_PLUS_GLOBAL_VERSION: &str = "VITE_PLUS_GLOBAL_VERSION";

// ── Testing / Development ───────────────────────────────────────────────

/// Override the trampoline binary path for tests.
///
/// When set, `get_trampoline_path()` uses this path instead of resolving
/// relative to `current_exe()`. Only used in test environments.
pub const VITE_PLUS_TRAMPOLINE_PATH: &str = "VITE_PLUS_TRAMPOLINE_PATH";
