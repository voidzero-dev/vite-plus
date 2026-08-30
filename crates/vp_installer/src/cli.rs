//! CLI argument parsing for `vp-setup`.

use clap::Parser;

/// Vite+ Installer — standalone installer for the vp CLI.
#[derive(Parser, Debug)]
#[command(name = "vp-setup", about = "Install the Vite+ CLI")]
pub(crate) struct Options {
    /// Accept defaults without prompting (for CI/unattended installs)
    #[arg(short = 'y', long = "yes")]
    pub(crate) yes: bool,

    /// Suppress all output except errors
    #[arg(short = 'q', long = "quiet")]
    pub(crate) quiet: bool,

    /// Install a specific version (default: latest)
    #[arg(long = "version")]
    pub(crate) version: Option<String>,

    /// npm dist-tag to install (default: latest)
    #[arg(long = "tag", default_value = "latest")]
    pub(crate) tag: String,

    /// Custom npm registry URL
    #[arg(long = "registry")]
    pub(crate) registry: Option<String>,

    /// Skip Node.js version manager setup
    #[arg(long = "no-node-manager")]
    pub(crate) no_node_manager: bool,

    /// Do not modify the User PATH
    #[arg(long = "no-modify-path")]
    pub(crate) no_modify_path: bool,
}

/// Parse CLI arguments, merging with environment variables.
/// CLI flags take precedence over environment variables.
pub(crate) fn parse() -> Options {
    let mut opts = Options::parse();

    // Merge env var overrides (CLI flags already set take precedence)
    if opts.version.is_none() {
        opts.version = std::env::var("VP_VERSION").ok();
    }
    // `VP_HOME` / `VP_*_DIR` / `XDG_*` are owned by [`vp_shared::EnvConfig`].
    if opts.registry.is_none() {
        opts.registry = std::env::var("NPM_CONFIG_REGISTRY").ok();
    }
    // CI and quiet both imply non-interactive (no prompts)
    if opts.quiet || std::env::var_os("CI").is_some() {
        opts.yes = true;
    }

    opts
}
