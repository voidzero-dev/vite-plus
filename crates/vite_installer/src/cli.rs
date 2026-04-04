//! CLI argument parsing for `vp-setup`.

use clap::Parser;

/// Vite+ Installer — standalone installer for the vp CLI.
#[derive(Parser, Debug)]
#[command(name = "vp-setup", about = "Install the Vite+ CLI")]
struct Cli {
    /// Accept defaults without prompting (for CI/unattended installs)
    #[arg(short = 'y', long = "yes")]
    yes: bool,

    /// Suppress all output except errors
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Install a specific version (default: latest)
    #[arg(long = "version")]
    version: Option<String>,

    /// npm dist-tag to install (default: latest)
    #[arg(long = "tag", default_value = "latest")]
    tag: String,

    /// Custom installation directory (default: ~/.vite-plus)
    #[arg(long = "install-dir")]
    install_dir: Option<String>,

    /// Custom npm registry URL
    #[arg(long = "registry")]
    registry: Option<String>,

    /// Skip Node.js version manager setup
    #[arg(long = "no-node-manager")]
    no_node_manager: bool,

    /// Do not modify the User PATH
    #[arg(long = "no-modify-path")]
    no_modify_path: bool,
}

/// Parsed installation options.
pub struct Options {
    pub yes: bool,
    pub quiet: bool,
    pub version: Option<String>,
    pub tag: String,
    pub install_dir: Option<String>,
    pub registry: Option<String>,
    pub no_node_manager: bool,
    pub no_modify_path: bool,
}

/// Parse CLI arguments, merging with environment variables.
///
/// CLI flags take precedence over environment variables.
pub fn parse() -> Options {
    let cli = Cli::parse();

    // Environment variable overrides (CLI flags take precedence)
    let version = cli.version.or_else(|| std::env::var("VP_VERSION").ok());
    let install_dir = cli.install_dir.or_else(|| std::env::var("VP_HOME").ok());
    let registry = cli.registry.or_else(|| std::env::var("NPM_CONFIG_REGISTRY").ok());

    let no_node_manager = cli.no_node_manager
        || std::env::var("VP_NODE_MANAGER")
            .ok()
            .is_some_and(|v| v.eq_ignore_ascii_case("no"));

    // quiet implies yes
    let yes = cli.yes || cli.quiet;

    Options {
        yes,
        quiet: cli.quiet,
        version,
        tag: cli.tag,
        install_dir,
        registry,
        no_node_manager,
        no_modify_path: cli.no_modify_path,
    }
}
