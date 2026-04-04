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

    /// Custom installation directory (default: ~/.vite-plus)
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
        opts.install_dir = std::env::var("VP_HOME").ok();
    }
    if opts.registry.is_none() {
        opts.registry = std::env::var("NPM_CONFIG_REGISTRY").ok();
    }
    // VP_NODE_MANAGER env var is handled in should_enable_node_manager()
    // to keep both "yes" and "no" logic in one place.

    // quiet implies yes
    if opts.quiet {
        opts.yes = true;
    }

    opts
}
