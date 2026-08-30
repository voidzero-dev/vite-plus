use vt_workspace::package_filter::PackageQueryArgs;

/// Parsed exec arguments (clap-derived).
#[derive(Debug, clap::Args)]
#[command(
    about = "Execute a command from local node_modules/.bin",
    after_help = "\
Examples:
  vp exec node --version                             # Run local node
  vp exec tsc --noEmit                               # Run local TypeScript compiler
  vp exec -c 'tsc --noEmit && prettier --check .'    # Shell mode
  vp exec -r -- tsc --noEmit                         # Run in all workspace packages
  vp exec --filter 'app...' -- tsc                   # Run in filtered packages"
)]
pub(crate) struct ExecArgs {
    #[clap(flatten)]
    pub(crate) packages: PackageQueryArgs,

    /// Execute the command within a shell environment
    #[clap(short = 'c', long = "shell-mode")]
    pub(crate) shell_mode: bool,

    /// Run concurrently without topological ordering
    #[clap(long)]
    pub(crate) parallel: bool,

    /// Reverse execution order
    #[clap(long)]
    pub(crate) reverse: bool,

    /// Resume from a specific package
    #[clap(long = "resume-from")]
    pub(crate) resume_from: Option<String>,

    /// Save results to vp-exec-summary.json
    #[clap(long = "report-summary")]
    pub(crate) report_summary: bool,

    /// Command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub(crate) command: Vec<String>,
}
