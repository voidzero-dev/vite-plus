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

    /// Custom npm registry URL
    #[arg(long = "registry")]
    pub registry: Option<String>,

    /// Skip Node.js version manager setup
    #[arg(long = "no-node-manager")]
    pub no_node_manager: bool,

    /// Resolved Node.js and package-manager management choice. `run` derives
    /// it from `--no-node-manager`, `VP_NODE_MANAGER` and auto-detection.
    #[arg(skip)]
    pub node_manager: NodeManager,

    /// Do not modify the User PATH
    #[arg(long = "no-modify-path")]
    pub no_modify_path: bool,
}

/// Node.js management choice passed to the installed binary's self-setup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeManager {
    /// Vite+ manages Node.js (`VP_NODE_MANAGER=yes`).
    #[default]
    Enabled,
    /// Prefer the system Node.js (`VP_NODE_MANAGER=no`).
    Disabled,
    /// Use only the system Node.js. Only `VP_NODE_MANAGER=system-only` selects
    /// it, and the interactive menu cannot change it.
    SystemOnly,
}

impl NodeManager {
    /// The `VP_NODE_MANAGER` value for this choice.
    #[must_use]
    pub const fn env_value(self) -> &'static str {
        match self {
            Self::Enabled => "yes",
            Self::Disabled => "no",
            Self::SystemOnly => vp_shared::env_vars::SYSTEM_ONLY,
        }
    }

    /// The menu label for this choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::SystemOnly => "system-only (set by VP_NODE_MANAGER)",
        }
    }
}

/// Parse CLI arguments, merging with environment variables.
/// CLI flags take precedence over environment variables.
pub fn parse() -> Options {
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
