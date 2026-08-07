//! CLI argument parsing for `vp-setup`.

use clap::Parser;

/// Vite+ Installer — standalone installer for the vp CLI.
#[derive(Parser, Debug)]
#[command(name = "vp-setup", about = "Install the Vite+ CLI")]
pub struct Options {
    /// Accept defaults without prompting (for CI/unattended installs)
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Suppress all output except errors
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Install a specific version (default: latest)
    #[arg(long = "version")]
    pub version: Option<String>,

    /// npm dist-tag to install (default: latest)
    #[arg(long = "tag", default_value = "latest")]
    pub tag: String,

    /// Custom installation directory: selects the legacy monolithic layout
    /// rooted at this directory (default: split platform layout, or the
    /// legacy root when `~/.vite-plus` already exists). Equivalent to setting
    /// the deprecated `VP_HOME` override for this process.
    #[arg(long = "install-dir")]
    pub install_dir: Option<String>,

    /// Custom npm registry URL
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// Skip Node.js version manager setup
    #[arg(long = "no-node-manager")]
    pub no_node_manager: bool,

    /// Do not modify the User PATH
    #[arg(long = "no-modify-path")]
    pub no_modify_path: bool,
}

/// Parse CLI arguments, merging with environment variables.
/// CLI flags take precedence over environment variables.
pub fn parse() -> Options {
    let mut opts = Options::parse();

    // Merge env var overrides (CLI flags already set take precedence)
    if opts.version.is_none() {
        opts.version = std::env::var("VP_VERSION").ok();
    }
    if opts.install_dir.is_none() {
        // Installer-only: honor deprecated `VP_HOME` as a custom legacy root
        // (same as install.sh). Prefer `VP_*_DIR` / XDG for split overrides;
        // those are read by VpDirs without going through this flag.
        opts.install_dir = std::env::var(vp_shared::env_vars::DEPRECATED_VP_HOME).ok();
    }
    if opts.registry.is_none() {
        opts.registry = std::env::var("NPM_CONFIG_REGISTRY").ok();
    }
    // CI and quiet both imply non-interactive (no prompts)
    if opts.quiet || std::env::var_os("CI").is_some() {
        opts.yes = true;
    }

    opts
}
