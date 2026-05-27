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
pub const VP_HOME: &str = "VP_HOME";

/// Log filter string for `tracing_subscriber` (e.g. `"debug"`, `"vite_task=trace"`).
pub const VITE_LOG: &str = "VITE_LOG";

/// NPM registry URL (lowercase form, highest priority).
pub const NPM_CONFIG_REGISTRY: &str = "npm_config_registry";

/// NPM registry URL (uppercase fallback).
pub const NPM_CONFIG_REGISTRY_UPPER: &str = "NPM_CONFIG_REGISTRY";

/// Node.js distribution mirror URL for downloads.
pub const VP_NODE_DIST_MIRROR: &str = "VP_NODE_DIST_MIRROR";

/// Override Node.js version (takes highest priority in version resolution).
pub const VP_NODE_VERSION: &str = "VP_NODE_VERSION";

/// Enable debug output for shim dispatch.
pub const VP_DEBUG_SHIM: &str = "VP_DEBUG_SHIM";

/// Enable eval mode for `vp env use`.
pub const VP_ENV_USE_EVAL_ENABLE: &str = "VP_ENV_USE_EVAL_ENABLE";

/// Explicitly specify the current shell.
pub const VP_SHELL: &str = "VP_SHELL";

/// Filter for update task types.
pub const VITE_UPDATE_TASK_TYPES: &str = "VITE_UPDATE_TASK_TYPES";

/// Override directory for global CLI JS scripts.
pub const VITE_GLOBAL_CLI_JS_SCRIPTS_DIR: &str = "VITE_GLOBAL_CLI_JS_SCRIPTS_DIR";

// ── Runtime: set/removed during shim dispatch for child processes ────────

/// Bypass the vite-plus shim and use the system tool directly.
///
/// Value is a `PATH`-style list of directories to bypass.
pub const VP_BYPASS: &str = "VP_BYPASS";

/// Recursion guard for `vp env exec` — prevents infinite shim loops.
pub const VP_TOOL_RECURSION: &str = "VP_TOOL_RECURSION";

/// Set by shim dispatch to record the active Node.js version.
pub const VP_ACTIVE_NODE: &str = "VP_ACTIVE_NODE";

/// Set by shim dispatch to record how the Node.js version was resolved.
pub const VP_RESOLVE_SOURCE: &str = "VP_RESOLVE_SOURCE";

/// Set by shell wrapper scripts to indicate which tool is being shimmed.
pub const VP_SHIM_TOOL: &str = "VP_SHIM_TOOL";

/// Set by Windows shim wrappers that route through `vp env exec`.
///
/// When present, `env exec` can normalize wrapper-inserted argument separators
/// before forwarding to the actual tool.
pub const VP_SHIM_WRAPPER: &str = "VP_SHIM_WRAPPER";

/// Path to the vp binary, passed to JS scripts so they can invoke CLI commands.
pub const VP_CLI_BIN: &str = "VP_CLI_BIN";

/// Global CLI version, passed from Rust binary to JS for --version display.
pub const VP_GLOBAL_VERSION: &str = "VP_GLOBAL_VERSION";

// ── HTTP client TLS / CA configuration ──────────────────────────────────

/// Path to a PEM bundle of extra CA certificates to trust for HTTPS.
///
/// Industry-standard env var also set by tools like Socket Firewall Free.
///
/// Note on semantics: vp treats this as **additive** to the system trust
/// store (matches Node.js's `NODE_EXTRA_CA_CERTS`), not as a replacement.
/// This differs from OpenSSL/curl/git, which use `SSL_CERT_FILE` as the
/// *sole* trusted bundle. Users who want strict isolation should also
/// restrict outbound traffic at the network layer.
pub const SSL_CERT_FILE: &str = "SSL_CERT_FILE";

/// Path to a PEM bundle of extra CA certificates to trust for HTTPS.
///
/// Node.js convention; honored alongside `SSL_CERT_FILE` for setups that only
/// configure the Node-flavored variable. Always additive to the system trust
/// store.
pub const NODE_EXTRA_CA_CERTS: &str = "NODE_EXTRA_CA_CERTS";

/// Disable HTTPS certificate verification in vp's shared HTTP client.
///
/// Diagnostic escape hatch only. Setting this to any value triggers a loud
/// startup warning. Do not use in production.
pub const VP_INSECURE_TLS: &str = "VP_INSECURE_TLS";

// ── Testing / Development ───────────────────────────────────────────────

/// Override the trampoline binary path for tests.
///
/// When set, `get_trampoline_path()` uses this path instead of resolving
/// relative to `current_exe()`. Only used in test environments.
pub const VP_TRAMPOLINE_PATH: &str = "VP_TRAMPOLINE_PATH";
