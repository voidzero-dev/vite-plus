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
//! The `XDG_*_HOME` base-directory variables are the exception: they
//! participate in `VpDirs` path resolution, so they get constants too.

// ── Config: read once at startup via EnvConfig ──────────────────────────

/// Override pinning every category root under one directory.
///
/// This is the highest-priority layout rule. When set, `bin` uses
/// `<root>/bin`, and `cache` uses `<root>/cache`. Data, config, and state use
/// `<root>`. Old environment scripts and custom-location installs export this
/// variable. Fresh installs do not set it. Prefer `VP_*_DIR` or `XDG_*`.
pub const VP_HOME: &str = "VP_HOME";

/// Override directory for executables and shims.
///
/// Applies only when `VP_DATA_DIR` and `VP_CACHE_DIR` are also absolute. A
/// single-root install from `VP_HOME` or the existing-install check ignores
/// the complete group.
pub const VP_BIN_DIR: &str = "VP_BIN_DIR";

/// Override directory for CLI versions, Node.js runtimes, and package managers.
/// These files use most of the disk space. Applies only as part of the complete
/// `VP_BIN_DIR`, `VP_DATA_DIR`, and `VP_CACHE_DIR` group.
pub const VP_DATA_DIR: &str = "VP_DATA_DIR";

/// Override directory for the disposable cache. Applies only as part of the
/// complete `VP_BIN_DIR`, `VP_DATA_DIR`, and `VP_CACHE_DIR` group.
pub const VP_CACHE_DIR: &str = "VP_CACHE_DIR";

// ── XDG base directories: read by VpDirs resolution ────────────────────

/// XDG base directory for user configuration.
pub const XDG_CONFIG_HOME: &str = "XDG_CONFIG_HOME";

/// XDG base directory for user data.
pub const XDG_DATA_HOME: &str = "XDG_DATA_HOME";

/// XDG base directory for user state.
pub const XDG_STATE_HOME: &str = "XDG_STATE_HOME";

/// XDG base directory for disposable caches.
pub const XDG_CACHE_HOME: &str = "XDG_CACHE_HOME";

/// All environment variables that the `VpDirs` resolution chain reads. Tests
/// clear them to isolate resolution from the developer shell. The Vite+
/// environment script usually exports `VP_HOME` in that shell.
pub const LAYOUT_OVERRIDE_VARS: &[&str] = &[
    VP_HOME,
    VP_BIN_DIR,
    VP_DATA_DIR,
    VP_CACHE_DIR,
    XDG_DATA_HOME,
    XDG_CACHE_HOME,
    XDG_CONFIG_HOME,
    XDG_STATE_HOME,
];

/// Log filter string for `tracing_subscriber` (e.g. `"debug"`, `"vt=trace"`).
pub const VP_LOG: &str = "VP_LOG";

/// NPM registry URL (lowercase form, highest priority).
pub const NPM_CONFIG_REGISTRY: &str = "npm_config_registry";

/// NPM registry URL (uppercase fallback).
pub const NPM_CONFIG_REGISTRY_UPPER: &str = "NPM_CONFIG_REGISTRY";

/// Node.js distribution mirror URL for downloads.
pub const VP_NODE_DIST_MIRROR: &str = "VP_NODE_DIST_MIRROR";

/// Skip PGP signature verification of `SHASUMS256.txt` when set (escape hatch).
/// The SHA-256 checksum is still verified.
pub const VP_NODE_SKIP_SIGNATURE_VERIFY: &str = "VP_NODE_SKIP_SIGNATURE_VERIFY";

/// Override Node.js version (takes highest priority in version resolution).
pub const VP_NODE_VERSION: &str = "VP_NODE_VERSION";

/// Enable debug output for shim dispatch.
pub const VP_DEBUG_SHIM: &str = "VP_DEBUG_SHIM";

/// Enable eval mode for `vp env use`.
pub const VP_ENV_USE_EVAL_ENABLE: &str = "VP_ENV_USE_EVAL_ENABLE";

/// Explicitly specify the current shell.
pub const VP_SHELL: &str = "VP_SHELL";

/// Filter for update task types.
pub const VP_UPDATE_TASK_TYPES: &str = "VP_UPDATE_TASK_TYPES";

/// Override directory for global CLI JS scripts.
pub const VP_GLOBAL_CLI_JS_SCRIPTS_DIR: &str = "VP_GLOBAL_CLI_JS_SCRIPTS_DIR";

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

/// Set (to `1`) only by the PTY snapshot runner to enable render-milestone
/// emission (invisible window-title updates the tests synchronize on).
///
/// The value is a cross-process contract: the runner and the JS prompts
/// package (`packages/prompts/src/milestone.ts`) spell the literal
/// independently.
pub const VP_EMIT_MILESTONES: &str = "VP_EMIT_MILESTONES";

/// Set by Windows shim wrappers that route through `vp env exec`.
///
/// When present, `env exec` can normalize wrapper-inserted argument separators
/// before forwarding to the actual tool.
pub const VP_SHIM_WRAPPER: &str = "VP_SHIM_WRAPPER";

/// The subcommand as the user wrote it, passed from the global CLI to the local
/// one.
///
/// A command runs under its canonical name (`vp format` runs `fmt`), which loses
/// the spelling. This carries the original alongside it.
pub const VP_RAW_SUBCOMMAND: &str = "VP_RAW_SUBCOMMAND";

/// Set (to `1`) when the global CLI delegates a command that used `-C`.
///
/// The local CLI uses this marker to keep the explicit target after the global
/// CLI changes the child process directory and removes `-C` from its arguments.
pub const VP_EXPLICIT_CHDIR: &str = "VP_EXPLICIT_CHDIR";

/// Path to the vp binary, passed to JS scripts so they can invoke CLI commands.
pub const VP_CLI_BIN: &str = "VP_CLI_BIN";

/// Global CLI version, passed from Rust binary to JS for --version display.
pub const VP_GLOBAL_VERSION: &str = "VP_GLOBAL_VERSION";

// ── HTTP client configuration ───────────────────────────────────────────

/// Override the per-request timeout (in seconds) for large file downloads
/// (Node.js runtimes, package-manager tarballs).
///
/// Must be a positive integer number of seconds no larger than 86400
/// (24 hours); an invalid value warns and is ignored. Default: 600
/// (10 minutes). Duration formats like `10m` may be supported later.
pub const VP_DOWNLOAD_TIMEOUT: &str = "VP_DOWNLOAD_TIMEOUT";

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

/// When set to `1`, the global CLI prints the layout mode and the five category
/// roots from [`crate::EnvConfig`], then exits. It prints one value on each
/// line. Installers use this variable when they have a `vp` binary. Thus, they
/// do not implement directory resolution again.
pub const VP_DUMP_DIRS: &str = "VP_DUMP_DIRS";

/// Keys in [`VP_DUMP_DIRS`] output. Each value uses one `<key>\t<value>` line.
/// The `vp_global_cli` printer and `vp-setup` parser share these values.
/// `install.sh` and `install.ps1` use the same keys.
pub mod dump_dirs {
    pub const LAYOUT: &str = "layout";
    pub const DATA: &str = "data";
    pub const BIN: &str = "bin";
    pub const CACHE: &str = "cache";
    pub const CONFIG: &str = "config";
    pub const STATE: &str = "state";
}

/// Override the trampoline binary path for tests.
///
/// When set, `get_trampoline_path()` uses this path instead of resolving
/// relative to `current_exe()`. Only used in test environments.
pub const VP_TRAMPOLINE_PATH: &str = "VP_TRAMPOLINE_PATH";
