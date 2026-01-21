use std::process::ExitStatus;

use clap::{CommandFactory, Parser, Subcommand};
use vite_error::Error;
use vite_install::commands::{
    add::SaveDependencyType, install::InstallCommandOptions, outdated::Format,
};
use vite_path::AbsolutePathBuf;

use crate::commands::{
    add::AddCommand, dedupe::DedupeCommand, dlx::DlxCommand, install::InstallCommand,
    link::LinkCommand, outdated::OutdatedCommand, pm::PmCommand, remove::RemoveCommand,
    unlink::UnlinkCommand, update::UpdateCommand, why::WhyCommand,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Args {
    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // package manager commands
    /// Install all dependencies, or add packages if package names are provided
    #[command(alias = "i")]
    Install {
        /// Do not install devDependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only install devDependencies (install) / Save to devDependencies (add)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Do not install optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Fail if lockfile needs to be updated (CI mode)
        #[arg(long, overrides_with = "no_frozen_lockfile")]
        frozen_lockfile: bool,

        /// Allow lockfile updates (opposite of --frozen-lockfile)
        #[arg(long, overrides_with = "frozen_lockfile")]
        no_frozen_lockfile: bool,

        /// Only update lockfile, don't install
        #[arg(long)]
        lockfile_only: bool,

        /// Use cached packages when available
        #[arg(long)]
        prefer_offline: bool,

        /// Only use packages already in cache
        #[arg(long)]
        offline: bool,

        /// Force reinstall all dependencies
        #[arg(short = 'f', long)]
        force: bool,

        /// Do not run lifecycle scripts
        #[arg(long)]
        ignore_scripts: bool,

        /// Don't read or generate lockfile
        #[arg(long)]
        no_lockfile: bool,

        /// Fix broken lockfile entries (pnpm and yarn@2+ only)
        #[arg(long)]
        fix_lockfile: bool,

        /// Create flat node_modules (pnpm only)
        #[arg(long)]
        shamefully_hoist: bool,

        /// Re-run resolution for peer dependency analysis (pnpm only)
        #[arg(long)]
        resolution_only: bool,

        /// Suppress output (silent mode)
        #[arg(long)]
        silent: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Install in workspace root only
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Save exact version (only when adding packages)
        #[arg(short = 'E', long)]
        save_exact: bool,

        /// Save to peerDependencies (only when adding packages)
        #[arg(long)]
        save_peer: bool,

        /// Save to optionalDependencies (only when adding packages)
        #[arg(short = 'O', long)]
        save_optional: bool,

        /// Save the new dependency to the default catalog (only when adding packages)
        #[arg(long)]
        save_catalog: bool,

        /// Install globally (only when adding packages)
        #[arg(short = 'g', long)]
        global: bool,

        /// Packages to add (if provided, acts as `vite add`)
        #[arg(required = false)]
        packages: Option<Vec<String>>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Add packages to dependencies
    Add {
        /// Save to `dependencies` (default)
        #[arg(short = 'P', long)]
        save_prod: bool,
        /// Save to `devDependencies`
        #[arg(short = 'D', long)]
        save_dev: bool,
        /// Save to `peerDependencies` and `devDependencies`
        #[arg(long)]
        save_peer: bool,
        /// Save to `optionalDependencies`
        #[arg(short = 'O', long)]
        save_optional: bool,
        /// Save exact version rather than semver range (e.g., `^1.0.0` -> `1.0.0`)
        #[arg(short = 'E', long)]
        save_exact: bool,

        /// Save the new dependency to the specified catalog name.
        /// Example: `vite add vue --save-catalog-name vue3`
        #[arg(long, value_name = "CATALOG_NAME")]
        save_catalog_name: Option<String>,
        /// Save the new dependency to the default catalog
        #[arg(long)]
        save_catalog: bool,

        /// A list of package names allowed to run postinstall
        #[arg(long, value_name = "NAMES")]
        allow_build: Option<String>,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Add to workspace root (ignore-workspace-root-check)
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only add if package exists in workspace (pnpm-specific)
        #[arg(long)]
        workspace: bool,

        /// Install globally
        #[arg(short = 'g', long)]
        global: bool,

        /// Packages to add
        #[arg(required = true)]
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Remove packages from dependencies
    #[command(alias = "rm", alias = "un", alias = "uninstall")]
    Remove {
        /// Only remove from `devDependencies` (pnpm-specific)
        #[arg(short = 'D', long)]
        save_dev: bool,

        /// Only remove from `optionalDependencies` (pnpm-specific)
        #[arg(short = 'O', long)]
        save_optional: bool,

        /// Only remove from `dependencies` (pnpm-specific)
        #[arg(short = 'P', long)]
        save_prod: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Remove from workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Remove recursively from all workspace packages, including workspace root
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Remove global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Packages to remove
        #[arg(required = true)]
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Update packages to their latest versions
    #[command(alias = "up")]
    Update {
        /// Update to latest version (ignore semver range)
        #[arg(short = 'L', long)]
        latest: bool,

        /// Update global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Update recursively in all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Include workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Update only devDependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Update only dependencies (production)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Interactive mode - show outdated packages and choose which to update
        #[arg(short = 'i', long)]
        interactive: bool,

        /// Don't update optionalDependencies
        #[arg(long)]
        no_optional: bool,

        /// Update lockfile only, don't modify package.json
        #[arg(long)]
        no_save: bool,

        /// Only update if package exists in workspace (pnpm-specific)
        #[arg(long)]
        workspace: bool,

        /// Packages to update (optional - updates all if omitted)
        packages: Vec<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Deduplicate dependencies by removing older versions
    #[command(alias = "ddp")]
    Dedupe {
        /// Check if deduplication would make changes
        #[arg(long)]
        check: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Check for outdated packages
    Outdated {
        /// Package name(s) to check (supports glob patterns in pnpm)
        packages: Vec<String>,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Output format: table (default), list, or json
        #[arg(long, value_name = "FORMAT", value_parser = clap::value_parser!(Format))]
        format: Option<Format>,

        /// Check recursively across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Include workspace root
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production and optional dependencies (pnpm-specific)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies (pnpm-specific)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Exclude optional dependencies (pnpm-specific)
        #[arg(long)]
        no_optional: bool,

        /// Only show compatible versions (pnpm-specific)
        #[arg(long)]
        compatible: bool,

        /// Sort results by field (pnpm-specific)
        #[arg(long, value_name = "FIELD")]
        sort_by: Option<String>,

        /// Check globally installed packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Show why a package is installed
    #[command(alias = "explain")]
    Why {
        /// Package(s) to check
        #[arg(required = true)]
        packages: Vec<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information (pnpm-specific)
        #[arg(long)]
        long: bool,

        /// Show parseable output (pnpm-specific)
        #[arg(long)]
        parseable: bool,

        /// Check recursively across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (pnpm/npm-specific)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Check in workspace root (pnpm-specific)
        #[arg(short = 'w', long)]
        workspace_root: bool,

        /// Only production dependencies (pnpm-specific)
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies (pnpm-specific)
        #[arg(short = 'D', long)]
        dev: bool,

        /// Limit tree depth (pnpm-specific)
        #[arg(long)]
        depth: Option<u32>,

        /// Exclude optional dependencies (pnpm-specific)
        #[arg(long)]
        no_optional: bool,

        /// Check globally installed packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Exclude peer dependencies (pnpm/yarn@2+-specific)
        #[arg(long)]
        exclude_peers: bool,

        /// Use a finder function defined in .pnpmfile.cjs (pnpm-specific)
        #[arg(long, value_name = "FINDER_NAME")]
        find_by: Option<String>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// View package information from the registry
    #[command(alias = "view", alias = "show")]
    Info {
        /// Package name with optional version
        #[arg(required = true)]
        package: String,

        /// Specific field to view
        field: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },
    /// Link packages for local development
    #[command(alias = "ln")]
    Link {
        /// Package name or directory to link
        /// If empty, registers current package globally
        #[arg(value_name = "PACKAGE|DIR")]
        package: Option<String>,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Unlink packages
    Unlink {
        /// Package name to unlink
        /// If empty, unlinks current package globally
        #[arg(value_name = "PACKAGE|DIR")]
        package: Option<String>,

        /// Unlink in every workspace package (pnpm/yarn@2+-specific)
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Forward a command to the package manager.
    #[command(subcommand)]
    Pm(PmCommands),
    /// Execute a package binary without installing it as a dependency
    Dlx {
        /// Package(s) to install before running the command (can be used multiple times)
        #[arg(long, short = 'p', value_name = "NAME")]
        package: Vec<String>,

        /// Execute the command within a shell environment
        #[arg(long = "shell-mode", short = 'c')]
        shell_mode: bool,

        /// Suppress all output except the executed command's output
        #[arg(long, short = 's')]
        silent: bool,

        /// Package to execute (with optional @version) and arguments
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate a new project
    Gen {
        /// Project name
        #[arg(required = true)]
        name: String,
    },
    /// Migrate an existing project to Vite+
    Migrate {
        /// Project directory
        #[arg(required = true)]
        directory: String,
    },

    // below commands only used to show help message, not actually executed
    /// Run the development server
    Dev,
    /// Build application
    Build,
    /// Run test
    Test,
    /// Lint code
    Lint,
    /// Format code
    Fmt,
    /// Build library
    Lib,
    #[command(hide = true)]
    /// Build documentation
    Doc,
    /// Run tasks
    Run,
    /// Manage the task cache
    Cache,
}

#[derive(Subcommand, Debug, Clone)]
pub enum PmCommands {
    /// Remove unnecessary packages
    Prune {
        /// Remove devDependencies
        #[arg(long)]
        prod: bool,

        /// Remove optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Create a tarball of the package
    Pack {
        /// Pack all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages to pack (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Customizes the output path for the tarball. Use %s and %v to include the package name and version (pnpm and yarn@2+ only), e.g., %s.tgz or some-dir/%s-%v.tgz
        #[arg(long)]
        out: Option<String>,

        /// Directory where the tarball will be saved (pnpm and npm only)
        #[arg(long)]
        pack_destination: Option<String>,

        /// Gzip compression level (0-9)
        #[arg(long)]
        pack_gzip_level: Option<u8>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// List installed packages
    #[command(alias = "ls")]
    List {
        /// Package pattern to filter
        pattern: Option<String>,

        /// Maximum depth of dependency tree
        #[arg(long)]
        depth: Option<u32>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Show extended information
        #[arg(long)]
        long: bool,

        /// Parseable output format
        #[arg(long)]
        parseable: bool,

        /// Only production dependencies
        #[arg(short = 'P', long)]
        prod: bool,

        /// Only dev dependencies
        #[arg(short = 'D', long)]
        dev: bool,

        /// Exclude optional dependencies
        #[arg(long)]
        no_optional: bool,

        /// Exclude peer dependencies
        #[arg(long)]
        exclude_peers: bool,

        /// Show only project packages (pnpm-specific)
        #[arg(long)]
        only_projects: bool,

        /// Use a finder function defined in .pnpmfile.cjs (pnpm-specific)
        #[arg(long, value_name = "FINDER_NAME")]
        find_by: Option<String>,

        /// List across all workspaces
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Vec<String>,

        /// List global packages
        #[arg(short = 'g', long)]
        global: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// View package information from the registry
    #[command(alias = "info", alias = "show")]
    View {
        /// Package name with optional version
        #[arg(required = true)]
        package: String,

        /// Specific field to view
        field: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Publish package to registry
    Publish {
        /// Tarball or folder to publish
        #[arg(value_name = "TARBALL|FOLDER")]
        target: Option<String>,

        /// Preview without publishing
        #[arg(long)]
        dry_run: bool,

        /// Publish tag (default: latest)
        #[arg(long)]
        tag: Option<String>,

        /// Access level (public/restricted)
        #[arg(long)]
        access: Option<String>,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Skip git checks (pnpm-specific)
        #[arg(long)]
        no_git_checks: bool,

        /// Set the branch name to publish from (pnpm-specific)
        #[arg(long, value_name = "BRANCH")]
        publish_branch: Option<String>,

        /// Save publish summary to pnpm-publish-summary.json (pnpm-specific)
        #[arg(long)]
        report_summary: bool,

        /// Force publish
        #[arg(long)]
        force: bool,

        /// Output in JSON format (pnpm-specific)
        #[arg(long)]
        json: bool,

        /// Publish all workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo (can be used multiple times)
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage package owners
    #[command(subcommand, alias = "author")]
    Owner(OwnerCommands),

    /// Manage package cache
    Cache {
        /// Subcommand: dir, path, clean
        #[arg(required = true)]
        subcommand: String,

        /// Additional arguments to pass through to the package manager
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Option<Vec<String>>,
    },

    /// Manage package manager configuration
    #[command(subcommand, alias = "c")]
    Config(ConfigCommands),
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCommands {
    /// List all configuration
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Get configuration value
    Get {
        /// Config key
        key: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Set configuration value
    Set {
        /// Config key
        key: String,

        /// Config value
        value: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },

    /// Delete configuration key
    Delete {
        /// Config key
        key: String,

        /// Use global config
        #[arg(short = 'g', long)]
        global: bool,

        /// Config location: project (default) or global
        #[arg(long, value_name = "LOCATION")]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum OwnerCommands {
    /// List package owners
    #[command(alias = "ls")]
    List {
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Add package owner
    Add {
        /// Username
        user: String,
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },

    /// Remove package owner
    Rm {
        /// Username
        user: String,
        /// Package name
        package: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,
    },
}

#[tracing::instrument]
pub async fn main(cwd: AbsolutePathBuf, mut args: Args) -> Result<std::process::ExitStatus, Error> {
    match &mut args.commands {
        // package manager commands
        Commands::Install {
            prod,
            dev,
            no_optional,
            frozen_lockfile,
            no_frozen_lockfile,
            lockfile_only,
            prefer_offline,
            offline,
            force,
            ignore_scripts,
            no_lockfile,
            fix_lockfile,
            shamefully_hoist,
            resolution_only,
            silent,
            filter,
            workspace_root,
            save_exact,
            save_peer,
            save_optional,
            save_catalog,
            global,
            packages,
            pass_through_args,
        } => {
            // If packages are provided, redirect to Add command
            // This allows `vite install <packages>` to work as an alias for `vite add <packages>`
            if let Some(pkgs) = packages {
                if !pkgs.is_empty() {
                    let exit_status = execute_add_command(
                        cwd,
                        pkgs,
                        *prod,             // save_prod (maps from --prod/-P)
                        *dev,              // save_dev (maps from --dev/-D)
                        *save_peer,        // save_peer
                        *save_optional,    // save_optional
                        *save_exact,       // save_exact
                        *save_catalog,     // save_catalog
                        None,              // save_catalog_name
                        filter.as_deref(), // filter
                        *workspace_root,   // workspace_root
                        false,             // workspace (pnpm-specific, not in install)
                        *global,           // global
                        None,              // allow_build
                        pass_through_args.as_deref(),
                    )
                    .await?;
                    return Ok(exit_status);
                }
            }

            // No packages provided, run regular install
            let options = InstallCommandOptions {
                prod: *prod,
                dev: *dev,
                no_optional: *no_optional,
                frozen_lockfile: *frozen_lockfile,
                no_frozen_lockfile: *no_frozen_lockfile,
                lockfile_only: *lockfile_only,
                prefer_offline: *prefer_offline,
                offline: *offline,
                force: *force,
                ignore_scripts: *ignore_scripts,
                no_lockfile: *no_lockfile,
                fix_lockfile: *fix_lockfile,
                shamefully_hoist: *shamefully_hoist,
                resolution_only: *resolution_only,
                silent: *silent,
                filters: filter.as_deref(),
                workspace_root: *workspace_root,
                pass_through_args: pass_through_args.as_deref(),
            };
            let exit_status = InstallCommand::new(cwd).execute(&options).await?;
            return Ok(exit_status);
        }
        Commands::Add {
            filter,
            workspace_root,
            workspace,
            packages,
            save_prod,
            save_dev,
            save_peer,
            save_optional,
            save_exact,
            save_catalog,
            save_catalog_name,
            global,
            allow_build,
            pass_through_args,
        } => {
            let exit_status = execute_add_command(
                cwd,
                packages,
                *save_prod,
                *save_dev,
                *save_peer,
                *save_optional,
                *save_exact,
                *save_catalog,
                save_catalog_name.as_deref(),
                filter.as_deref(),
                *workspace_root,
                *workspace,
                *global,
                allow_build.as_deref(),
                pass_through_args.as_deref(),
            )
            .await?;
            return Ok(exit_status);
        }
        Commands::Remove {
            save_dev,
            save_optional,
            save_prod,
            filter,
            workspace_root,
            recursive,
            global,
            packages,
            pass_through_args,
        } => {
            let exit_status = RemoveCommand::new(cwd)
                .execute(
                    packages,
                    *save_dev,
                    *save_optional,
                    *save_prod,
                    filter.as_deref(),
                    *workspace_root,
                    *recursive,
                    *global,
                    pass_through_args.as_deref(),
                )
                .await?;
            return Ok(exit_status);
        }
        Commands::Update {
            latest,
            global,
            recursive,
            filter,
            workspace_root,
            dev,
            prod,
            interactive,
            no_optional,
            no_save,
            workspace,
            packages,
            pass_through_args,
        } => {
            let exit_status = UpdateCommand::new(cwd)
                .execute(
                    packages,
                    *latest,
                    *global,
                    *recursive,
                    filter.as_deref(),
                    *workspace_root,
                    *dev,
                    *prod,
                    *interactive,
                    *no_optional,
                    *no_save,
                    *workspace,
                    pass_through_args.as_deref(),
                )
                .await?;
            return Ok(exit_status);
        }
        Commands::Dedupe { check, pass_through_args } => {
            let exit_status =
                DedupeCommand::new(cwd).execute(*check, pass_through_args.as_deref()).await?;
            return Ok(exit_status);
        }
        Commands::Outdated {
            packages,
            long,
            format,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            no_optional,
            compatible,
            sort_by,
            global,
            pass_through_args,
        } => {
            let exit_status = OutdatedCommand::new(cwd)
                .execute(
                    packages,
                    *long,
                    *format,
                    *recursive,
                    filter.as_deref(),
                    *workspace_root,
                    *prod,
                    *dev,
                    *no_optional,
                    *compatible,
                    sort_by.as_deref(),
                    *global,
                    pass_through_args.as_deref(),
                )
                .await?;
            return Ok(exit_status);
        }
        Commands::Link { package, args } => {
            let exit_status = LinkCommand::new(cwd).execute(package.as_deref(), Some(args)).await?;
            return Ok(exit_status);
        }
        Commands::Unlink { package, recursive, args } => {
            let exit_status =
                UnlinkCommand::new(cwd).execute(package.as_deref(), *recursive, Some(args)).await?;
            return Ok(exit_status);
        }
        Commands::Why {
            packages,
            json,
            long,
            parseable,
            recursive,
            filter,
            workspace_root,
            prod,
            dev,
            depth,
            no_optional,
            global,
            exclude_peers,
            find_by,
            pass_through_args,
        } => {
            let exit_status = WhyCommand::new(cwd)
                .execute(
                    packages,
                    *json,
                    *long,
                    *parseable,
                    *recursive,
                    filter.as_deref(),
                    *workspace_root,
                    *prod,
                    *dev,
                    *depth,
                    *no_optional,
                    *global,
                    *exclude_peers,
                    find_by.as_deref(),
                    pass_through_args.as_deref(),
                )
                .await?;
            return Ok(exit_status);
        }
        Commands::Info { package, field, json, pass_through_args } => {
            let exit_status = PmCommand::new(cwd)
                .execute(PmCommands::View {
                    package: package.clone(),
                    field: field.clone(),
                    json: *json,
                    pass_through_args: pass_through_args.clone(),
                })
                .await?;
            return Ok(exit_status);
        }
        Commands::Pm(pm_command) => {
            let exit_status = PmCommand::new(cwd).execute(pm_command.clone()).await?;
            return Ok(exit_status);
        }
        Commands::Dlx { package, shell_mode, silent, args } => {
            let exit_status = DlxCommand::new(cwd)
                .execute(package.clone(), *shell_mode, *silent, args.clone())
                .await?;
            return Ok(exit_status);
        }
        _ => unreachable!(),
    };
}

pub fn command_with_help() -> clap::Command {
    let bold = "\x1b[1m";
    let bold_underline = "\x1b[1;4m";
    let reset = "\x1b[0m";
    let version = env!("CARGO_PKG_VERSION");

    let after_help = format!(
        "{bold_underline}Vite+ Commands:{reset}
  {bold}dev{reset}        Run the development server
  {bold}build{reset}      Build for production
  {bold}lint{reset}       Lint code
  {bold}test{reset}       Run tests
  {bold}fmt{reset}        Format code
  {bold}lib{reset}        Build library
  {bold}migrate{reset}    Migrate an existing project to Vite+
  {bold}cache{reset}      Manage the task cache
  {bold}new{reset}        Generate a new project
  {bold}run{reset}        Run tasks

{bold_underline}Package Manager Commands:{reset}
  {bold}install{reset}    Install all dependencies, or add packages if package names are provided
  {bold}add{reset}        Add packages to dependencies
  {bold}remove{reset}     Remove packages from dependencies
  {bold}dedupe{reset}     Deduplicate dependencies by removing older versions
  {bold}dlx{reset}        Execute a package binary without installing it as a dependency
  {bold}info{reset}       View package information from the registry
  {bold}link{reset}       Link packages for local development
  {bold}outdated{reset}   Check for outdated packages
  {bold}pm{reset}         Forward a command to the package manager
  {bold}unlink{reset}     Unlink packages
  {bold}update{reset}     Update packages to their latest versions
  {bold}why{reset}        Show why a package is installed
"
    );
    let help_template = format!(
        "Vite+/{version}

{{usage-heading}} {{usage}}{{after-help}}
{bold_underline}Options:{reset}
{{options}}
"
    );

    Args::command().after_help(after_help).help_template(help_template)
}

pub fn init_tracing() {
    use std::sync::OnceLock;

    use tracing_subscriber::{
        filter::{LevelFilter, Targets},
        prelude::__tracing_subscriber_SubscriberExt,
        util::SubscriberInitExt,
    };

    static TRACING: OnceLock<()> = OnceLock::new();
    TRACING.get_or_init(|| {
        // Usage without the `regex` feature.
        // <https://github.com/tokio-rs/tracing/issues/1436#issuecomment-918528013>
        tracing_subscriber::registry()
            .with(
                std::env::var("VITE_LOG")
                    .map_or_else(
                        |_| Targets::new(),
                        |env_var| {
                            use std::str::FromStr;
                            Targets::from_str(&env_var).unwrap_or_default()
                        },
                    )
                    // disable brush-parser tracing
                    .with_targets([("tokenize", LevelFilter::OFF), ("parse", LevelFilter::OFF)]),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    });
}

/// Execute add command with the given parameters
async fn execute_add_command(
    cwd: AbsolutePathBuf,
    packages: &[String],
    save_prod: bool,
    save_dev: bool,
    save_peer: bool,
    save_optional: bool,
    save_exact: bool,
    save_catalog: bool,
    save_catalog_name: Option<&str>,
    filter: Option<&[String]>,
    workspace_root: bool,
    workspace: bool,
    global: bool,
    allow_build: Option<&str>,
    pass_through_args: Option<&[String]>,
) -> Result<ExitStatus, Error> {
    let save_dependency_type = if save_dev {
        Some(SaveDependencyType::Dev)
    } else if save_peer {
        Some(SaveDependencyType::Peer)
    } else if save_optional {
        Some(SaveDependencyType::Optional)
    } else if save_prod {
        Some(SaveDependencyType::Production)
    } else {
        None
    };

    // empty string means save as `catalog:`
    let save_catalog_name = if save_catalog { Some("") } else { save_catalog_name };

    AddCommand::new(cwd)
        .execute(
            packages,
            save_dependency_type,
            save_exact,
            save_catalog_name,
            filter,
            workspace_root,
            workspace,
            global,
            allow_build,
            pass_through_args,
        )
        .await
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    mod install_command_tests {
        use super::*;

        #[test]
        fn test_args_install_command_basic() {
            let args = Args::try_parse_from(&["vite-plus", "install"]).unwrap();
            if let Commands::Install { prod, dev, frozen_lockfile, filter, .. } = &args.commands {
                assert!(!prod);
                assert!(!dev);
                assert!(!frozen_lockfile);
                assert!(filter.is_none());
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_prod() {
            let args = Args::try_parse_from(&["vite-plus", "install", "--prod"]).unwrap();
            if let Commands::Install { prod, dev, .. } = &args.commands {
                assert!(prod);
                assert!(!dev);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_frozen_lockfile() {
            let args =
                Args::try_parse_from(&["vite-plus", "install", "--frozen-lockfile"]).unwrap();
            if let Commands::Install { frozen_lockfile, no_frozen_lockfile, .. } = &args.commands {
                assert!(frozen_lockfile);
                assert!(!no_frozen_lockfile);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_no_frozen_lockfile() {
            let args =
                Args::try_parse_from(&["vite-plus", "install", "--no-frozen-lockfile"]).unwrap();
            if let Commands::Install { frozen_lockfile, no_frozen_lockfile, .. } = &args.commands {
                assert!(!frozen_lockfile);
                assert!(no_frozen_lockfile);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_frozen_lockfile_override() {
            // --no-frozen-lockfile should override --frozen-lockfile when both are specified
            // Last one wins due to overrides_with
            let args = Args::try_parse_from(&[
                "vite-plus",
                "install",
                "--frozen-lockfile",
                "--no-frozen-lockfile",
            ])
            .unwrap();
            if let Commands::Install { frozen_lockfile, no_frozen_lockfile, .. } = &args.commands {
                // With overrides_with, the last flag wins and resets the other
                assert!(!frozen_lockfile);
                assert!(no_frozen_lockfile);
            } else {
                panic!("Expected Install command");
            }

            // Reverse order: --frozen-lockfile after --no-frozen-lockfile
            let args = Args::try_parse_from(&[
                "vite-plus",
                "install",
                "--no-frozen-lockfile",
                "--frozen-lockfile",
            ])
            .unwrap();
            if let Commands::Install { frozen_lockfile, no_frozen_lockfile, .. } = &args.commands {
                assert!(frozen_lockfile);
                assert!(!no_frozen_lockfile);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_filter() {
            let args = Args::try_parse_from(&["vite-plus", "install", "--filter", "app"]).unwrap();
            if let Commands::Install { filter, .. } = &args.commands {
                assert_eq!(filter.as_ref().unwrap(), &vec!["app".to_string()]);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_multiple_filters() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "install",
                "--filter",
                "app",
                "--filter",
                "web",
            ])
            .unwrap();
            if let Commands::Install { filter, .. } = &args.commands {
                assert_eq!(filter.as_ref().unwrap(), &vec!["app".to_string(), "web".to_string()]);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_alias() {
            let args = Args::try_parse_from(&["vite-plus", "i"]).unwrap();
            assert!(matches!(args.commands, Commands::Install { .. }));
        }

        #[test]
        fn test_args_install_command_with_all_options() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "install",
                "--prod",
                "--frozen-lockfile",
                "--prefer-offline",
                "--ignore-scripts",
                "--filter",
                "app",
                "-w",
            ])
            .unwrap();
            if let Commands::Install {
                prod,
                frozen_lockfile,
                prefer_offline,
                ignore_scripts,
                filter,
                workspace_root,
                ..
            } = &args.commands
            {
                assert!(prod);
                assert!(frozen_lockfile);
                assert!(prefer_offline);
                assert!(ignore_scripts);
                assert_eq!(filter.as_ref().unwrap(), &vec!["app".to_string()]);
                assert!(workspace_root);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages() {
            // vite install <packages> should be parsed as Install with packages
            let args =
                Args::try_parse_from(&["vite-plus", "install", "react", "react-dom"]).unwrap();
            if let Commands::Install { packages, dev, save_exact, .. } = &args.commands {
                assert_eq!(
                    packages.as_ref().unwrap(),
                    &vec!["react".to_string(), "react-dom".to_string()]
                );
                assert!(!dev);
                assert!(!save_exact);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_dev_flag() {
            // vite install -D <packages> should work like vite add -D <packages>
            let args = Args::try_parse_from(&["vite-plus", "install", "-D", "typescript"]).unwrap();
            if let Commands::Install { packages, dev, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["typescript".to_string()]);
                assert!(dev);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_exact_flag() {
            // vite install -E <packages> should work like vite add -E <packages>
            let args =
                Args::try_parse_from(&["vite-plus", "install", "-E", "lodash@4.17.21"]).unwrap();
            if let Commands::Install { packages, save_exact, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["lodash@4.17.21".to_string()]);
                assert!(save_exact);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_global_flag() {
            // vite install -g <packages> should work like vite add -g <packages>
            let args = Args::try_parse_from(&["vite-plus", "install", "-g", "typescript"]).unwrap();
            if let Commands::Install { packages, global, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["typescript".to_string()]);
                assert!(global);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_complex() {
            // Complex example: vite install -D -E --filter app typescript eslint
            let args = Args::try_parse_from(&[
                "vite-plus",
                "install",
                "-D",
                "-E",
                "--filter",
                "app",
                "typescript",
                "eslint",
            ])
            .unwrap();
            if let Commands::Install { packages, dev, save_exact, filter, .. } = &args.commands {
                assert_eq!(
                    packages.as_ref().unwrap(),
                    &vec!["typescript".to_string(), "eslint".to_string()]
                );
                assert!(dev);
                assert!(save_exact);
                assert_eq!(filter.as_ref().unwrap(), &vec!["app".to_string()]);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_save_peer_flag() {
            // vite install --save-peer <packages> should work like vite add --save-peer <packages>
            let args =
                Args::try_parse_from(&["vite-plus", "install", "--save-peer", "react"]).unwrap();
            if let Commands::Install { packages, save_peer, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["react".to_string()]);
                assert!(save_peer);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_save_catalog_flag() {
            // vite install --save-catalog <packages> should work like vite add --save-catalog <packages>
            let args =
                Args::try_parse_from(&["vite-plus", "install", "--save-catalog", "react"]).unwrap();
            if let Commands::Install { packages, save_catalog, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["react".to_string()]);
                assert!(save_catalog);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_packages_and_save_optional_flag() {
            // vite install -O <packages> should work like vite add -O <packages>
            let args = Args::try_parse_from(&["vite-plus", "install", "-O", "fsevents"]).unwrap();
            if let Commands::Install { packages, save_optional, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["fsevents".to_string()]);
                assert!(save_optional);
            } else {
                panic!("Expected Install command");
            }

            // Also test long form
            let args =
                Args::try_parse_from(&["vite-plus", "install", "--save-optional", "fsevents"])
                    .unwrap();
            if let Commands::Install { packages, save_optional, .. } = &args.commands {
                assert_eq!(packages.as_ref().unwrap(), &vec!["fsevents".to_string()]);
                assert!(save_optional);
            } else {
                panic!("Expected Install command");
            }
        }

        #[test]
        fn test_args_install_command_with_silent_flag() {
            let args = Args::try_parse_from(&["vite-plus", "install", "--silent"]).unwrap();
            if let Commands::Install { silent, .. } = &args.commands {
                assert!(silent);
            } else {
                panic!("Expected Install command");
            }
        }
    }

    mod add_command_tests {
        use super::*;

        #[test]
        fn test_args_add_command() {
            let args = Args::try_parse_from(&["vite-plus", "add", "react"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }

            let args = Args::try_parse_from(&["vite-plus", "add", "--save-peer", "react"]).unwrap();
            if let Commands::Add {
                filter, workspace_root, workspace, packages, save_peer, ..
            } = &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!workspace);
                assert!(save_peer);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_workspace_root() {
            let args = Args::try_parse_from(&["vite-plus", "add", "-w", "react"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }
            let args = Args::try_parse_from(&["vite-plus", "add", "react", "-w"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }

            let args =
                Args::try_parse_from(&["vite-plus", "add", "react", "--workspace-root"]).unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, .. } =
                &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(filter.is_none());
                assert!(workspace_root);
                assert!(!workspace);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_multiple_packages() {
            let args =
                Args::try_parse_from(&["vite-plus", "add", "react", "react-dom", "@types/react"])
                    .unwrap();
            if let Commands::Add { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "react-dom", "@types/react"]);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_flags() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "-w",
                "--workspace",
                "typescript",
                "-D",
            ])
            .unwrap();
            if let Commands::Add { filter, workspace_root, workspace, packages, save_dev, .. } =
                &args.commands
            {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert!(workspace_root);
                assert!(workspace);
                assert_eq!(packages, &vec!["typescript"]);
                assert!(save_dev);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_with_allow_build() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "-w",
                "--workspace",
                "typescript",
                "-D",
                "--allow-build=react,napi",
            ])
            .unwrap();
            if let Commands::Add {
                filter,
                workspace_root,
                workspace,
                packages,
                save_dev,
                allow_build,
                ..
            } = &args.commands
            {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert!(workspace_root);
                assert!(workspace);
                assert_eq!(packages, &vec!["typescript"]);
                assert!(save_dev);
                assert_eq!(allow_build, &Some("react,napi".to_string()));
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_multiple_filters() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "--filter",
                "app",
                "--filter",
                "web",
                "react",
            ])
            .unwrap();
            if let Commands::Add { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_args_add_command_invalid_filter() {
            let args = Args::try_parse_from(&["vite-plus", "add", "react", "--filter"]);
            assert!(args.is_err());
        }

        #[test]
        fn test_args_add_command_with_pass_through_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "react",
                "--",
                "--watch",
                "--mode=production",
                "--use-stderr",
            ])
            .unwrap();
            if let Commands::Add { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
                assert_eq!(
                    pass_through_args,
                    &Some(vec![
                        "--watch".to_string(),
                        "--mode=production".to_string(),
                        "--use-stderr".to_string()
                    ])
                );
            } else {
                panic!("Expected Add command");
            }

            let args = Args::try_parse_from(&[
                "vite-plus",
                "add",
                "react",
                "napi",
                "--",
                "--allow-build=react,napi",
            ])
            .unwrap();
            if let Commands::Add { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "napi"]);
                assert_eq!(pass_through_args, &Some(vec!["--allow-build=react,napi".to_string()]));
            } else {
                panic!("Expected Add command");
            }
        }
    }

    mod remove_command_tests {
        use super::*;

        #[test]
        fn test_args_remove_command() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "react"]).unwrap();
            if let Commands::Remove {
                save_dev,
                save_optional,
                save_prod,
                filter,
                workspace_root,
                recursive,
                global,
                packages,
                pass_through_args,
            } = &args.commands
            {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(!save_dev);
                assert!(!save_optional);
                assert!(!save_prod);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!recursive);
                assert!(!global);
                assert!(pass_through_args.is_none());
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_dev_flag() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-D", "typescript"]).unwrap();
            if let Commands::Remove { save_dev, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["typescript".to_string()]);
                assert!(save_dev);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_optional_flag() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-O", "lodash"]).unwrap();
            if let Commands::Remove { save_optional, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["lodash".to_string()]);
                assert!(save_optional);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_prod_flag() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-P", "express"]).unwrap();
            if let Commands::Remove { save_prod, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["express".to_string()]);
                assert!(save_prod);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_workspace_root() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-w", "react"]).unwrap();
            if let Commands::Remove { workspace_root, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(workspace_root);
            } else {
                panic!("Expected Remove command");
            }

            let args = Args::try_parse_from(&["vite-plus", "remove", "react", "--workspace-root"])
                .unwrap();
            if let Commands::Remove { workspace_root, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(workspace_root);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_recursive() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-r", "react"]).unwrap();
            if let Commands::Remove { recursive, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(recursive);
            } else {
                panic!("Expected Remove command");
            }

            let args =
                Args::try_parse_from(&["vite-plus", "remove", "react", "--recursive"]).unwrap();
            if let Commands::Remove { recursive, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react".to_string()]);
                assert!(recursive);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_global() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "-g", "npm"]).unwrap();
            if let Commands::Remove { global, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["npm".to_string()]);
                assert!(global);
            } else {
                panic!("Expected Remove command");
            }

            let args = Args::try_parse_from(&["vite-plus", "remove", "npm", "--global"]).unwrap();
            if let Commands::Remove { global, packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["npm".to_string()]);
                assert!(global);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_multiple_packages() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "remove",
                "react",
                "react-dom",
                "@types/react",
            ])
            .unwrap();
            if let Commands::Remove { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "react-dom", "@types/react"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_single_filter() {
            let args =
                Args::try_parse_from(&["vite-plus", "remove", "--filter", "app", "typescript"])
                    .unwrap();
            if let Commands::Remove { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert_eq!(packages, &vec!["typescript"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_multiple_filters() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "remove",
                "--filter",
                "app",
                "--filter",
                "web",
                "react",
            ])
            .unwrap();
            if let Commands::Remove { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_combined_flags() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "remove",
                "-D",
                "-w",
                "--filter",
                "app",
                "typescript",
                "eslint",
            ])
            .unwrap();
            if let Commands::Remove { save_dev, workspace_root, filter, packages, .. } =
                &args.commands
            {
                assert!(save_dev);
                assert!(workspace_root);
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert_eq!(packages, &vec!["typescript", "eslint"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_with_pass_through_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "remove",
                "react",
                "--",
                "--ignore-scripts",
                "--force",
            ])
            .unwrap();
            if let Commands::Remove { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
                assert_eq!(
                    pass_through_args,
                    &Some(vec!["--ignore-scripts".to_string(), "--force".to_string()])
                );
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_alias_rm() {
            let args = Args::try_parse_from(&["vite-plus", "rm", "react"]).unwrap();
            if let Commands::Remove { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_alias_un() {
            let args = Args::try_parse_from(&["vite-plus", "un", "react"]).unwrap();
            if let Commands::Remove { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_alias_uninstall() {
            let args = Args::try_parse_from(&["vite-plus", "uninstall", "react"]).unwrap();
            if let Commands::Remove { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Remove command");
            }
        }

        #[test]
        fn test_args_remove_command_invalid_filter() {
            let args = Args::try_parse_from(&["vite-plus", "remove", "react", "--filter"]);
            assert!(args.is_err());
        }

        #[test]
        fn test_args_remove_command_complex_scenario() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "remove",
                "-D",
                "-r",
                "--filter",
                "app",
                "--filter",
                "web",
                "typescript",
                "eslint",
                "@types/node",
                "--",
                "--ignore-scripts",
            ])
            .unwrap();
            if let Commands::Remove {
                save_dev,
                recursive,
                filter,
                packages,
                pass_through_args,
                ..
            } = &args.commands
            {
                assert!(save_dev);
                assert!(recursive);
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert_eq!(packages, &vec!["typescript", "eslint", "@types/node"]);
                assert_eq!(pass_through_args, &Some(vec!["--ignore-scripts".to_string()]));
            } else {
                panic!("Expected Remove command");
            }
        }
    }

    mod update_command_tests {
        use super::*;

        #[test]
        fn test_args_update_command_basic() {
            let args = Args::try_parse_from(&["vite-plus", "update"]).unwrap();
            if let Commands::Update {
                latest,
                global,
                recursive,
                filter,
                workspace_root,
                dev,
                prod,
                interactive,
                no_optional,
                no_save,
                workspace,
                packages,
                ..
            } = &args.commands
            {
                assert!(!latest);
                assert!(!global);
                assert!(!recursive);
                assert!(filter.is_none());
                assert!(!workspace_root);
                assert!(!dev);
                assert!(!prod);
                assert!(!interactive);
                assert!(!no_optional);
                assert!(!no_save);
                assert!(!workspace);
                assert!(packages.is_empty());
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_alias() {
            let args = Args::try_parse_from(&["vite-plus", "up"]).unwrap();
            assert!(matches!(args.commands, Commands::Update { .. }));
        }

        #[test]
        fn test_args_update_command_with_packages() {
            let args =
                Args::try_parse_from(&["vite-plus", "update", "react", "react-dom"]).unwrap();
            if let Commands::Update { packages, .. } = &args.commands {
                assert_eq!(packages, &vec!["react", "react-dom"]);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_latest_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-L", "react"]).unwrap();
            if let Commands::Update { latest, packages, .. } = &args.commands {
                assert!(latest);
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--latest", "react"]).unwrap();
            if let Commands::Update { latest, packages, .. } = &args.commands {
                assert!(latest);
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_global_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-g"]).unwrap();
            if let Commands::Update { global, .. } = &args.commands {
                assert!(global);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--global"]).unwrap();
            if let Commands::Update { global, .. } = &args.commands {
                assert!(global);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_recursive_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-r"]).unwrap();
            if let Commands::Update { recursive, .. } = &args.commands {
                assert!(recursive);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--recursive"]).unwrap();
            if let Commands::Update { recursive, .. } = &args.commands {
                assert!(recursive);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_workspace_root_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-w"]).unwrap();
            if let Commands::Update { workspace_root, .. } = &args.commands {
                assert!(workspace_root);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--workspace-root"]).unwrap();
            if let Commands::Update { workspace_root, .. } = &args.commands {
                assert!(workspace_root);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_dev_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-D"]).unwrap();
            if let Commands::Update { dev, .. } = &args.commands {
                assert!(dev);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--dev"]).unwrap();
            if let Commands::Update { dev, .. } = &args.commands {
                assert!(dev);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_prod_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-P"]).unwrap();
            if let Commands::Update { prod, .. } = &args.commands {
                assert!(prod);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--prod"]).unwrap();
            if let Commands::Update { prod, .. } = &args.commands {
                assert!(prod);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_interactive_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "-i"]).unwrap();
            if let Commands::Update { interactive, .. } = &args.commands {
                assert!(interactive);
            } else {
                panic!("Expected Update command");
            }

            let args = Args::try_parse_from(&["vite-plus", "update", "--interactive"]).unwrap();
            if let Commands::Update { interactive, .. } = &args.commands {
                assert!(interactive);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_no_optional_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "--no-optional"]).unwrap();
            if let Commands::Update { no_optional, .. } = &args.commands {
                assert!(no_optional);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_no_save_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "--no-save"]).unwrap();
            if let Commands::Update { no_save, .. } = &args.commands {
                assert!(no_save);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_workspace_flag() {
            let args = Args::try_parse_from(&["vite-plus", "update", "--workspace"]).unwrap();
            if let Commands::Update { workspace, .. } = &args.commands {
                assert!(workspace);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_filter() {
            let args =
                Args::try_parse_from(&["vite-plus", "update", "--filter", "app", "react"]).unwrap();
            if let Commands::Update { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_multiple_filters() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "update",
                "--filter",
                "app",
                "--filter",
                "web",
                "react",
            ])
            .unwrap();
            if let Commands::Update { filter, packages, .. } = &args.commands {
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_combined_flags() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "update",
                "-L",
                "-r",
                "-D",
                "--filter",
                "app",
                "typescript",
                "eslint",
            ])
            .unwrap();
            if let Commands::Update { latest, recursive, dev, filter, packages, .. } =
                &args.commands
            {
                assert!(latest);
                assert!(recursive);
                assert!(dev);
                assert_eq!(filter, &Some(vec!["app".to_string()]));
                assert_eq!(packages, &vec!["typescript", "eslint"]);
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_with_pass_through_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "update",
                "react",
                "--",
                "--registry",
                "https://custom-registry.com",
            ])
            .unwrap();
            if let Commands::Update { packages, pass_through_args, .. } = &args.commands {
                assert_eq!(packages, &vec!["react"]);
                assert_eq!(
                    pass_through_args,
                    &Some(vec![
                        "--registry".to_string(),
                        "https://custom-registry.com".to_string()
                    ])
                );
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_complex_scenario() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "update",
                "-L",
                "-r",
                "-w",
                "-D",
                "--filter",
                "app",
                "--filter",
                "web",
                "--no-optional",
                "react",
                "vue",
                "--",
                "--registry",
                "https://registry.npmjs.org",
            ])
            .unwrap();
            if let Commands::Update {
                latest,
                recursive,
                workspace_root,
                dev,
                filter,
                no_optional,
                packages,
                pass_through_args,
                ..
            } = &args.commands
            {
                assert!(latest);
                assert!(recursive);
                assert!(workspace_root);
                assert!(dev);
                assert_eq!(filter, &Some(vec!["app".to_string(), "web".to_string()]));
                assert!(no_optional);
                assert_eq!(packages, &vec!["react", "vue"]);
                assert_eq!(
                    pass_through_args,
                    &Some(vec!["--registry".to_string(), "https://registry.npmjs.org".to_string()])
                );
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_all_packages() {
            // When no packages are specified, should update all packages
            let args = Args::try_parse_from(&["vite-plus", "update", "-r"]).unwrap();
            if let Commands::Update { recursive, packages, .. } = &args.commands {
                assert!(recursive);
                assert!(packages.is_empty());
            } else {
                panic!("Expected Update command");
            }
        }

        #[test]
        fn test_args_update_command_workspace_combinations() {
            // Test --workspace-root with --recursive
            let args = Args::try_parse_from(&["vite-plus", "update", "-w", "-r"]).unwrap();
            if let Commands::Update { workspace_root, recursive, .. } = &args.commands {
                assert!(workspace_root);
                assert!(recursive);
            } else {
                panic!("Expected Update command");
            }

            // Test --workspace flag
            let args =
                Args::try_parse_from(&["vite-plus", "update", "--workspace", "react"]).unwrap();
            if let Commands::Update { workspace, packages, .. } = &args.commands {
                assert!(workspace);
                assert_eq!(packages, &vec!["react"]);
            } else {
                panic!("Expected Update command");
            }
        }
    }

    mod dedupe_command_tests {
        use super::*;

        #[test]
        fn test_args_dedupe_command_basic() {
            let args = Args::try_parse_from(&["vite-plus", "dedupe"]).unwrap();
            if let Commands::Dedupe { check, .. } = &args.commands {
                assert!(!check);
            } else {
                panic!("Expected Dedupe command");
            }
        }

        #[test]
        fn test_args_dedupe_command_with_alias() {
            let args = Args::try_parse_from(&["vite-plus", "ddp"]).unwrap();
            assert!(matches!(args.commands, Commands::Dedupe { .. }));
        }

        #[test]
        fn test_args_dedupe_command_with_check() {
            let args = Args::try_parse_from(&["vite-plus", "dedupe", "--check"]).unwrap();
            if let Commands::Dedupe { check, .. } = &args.commands {
                assert!(check);
            } else {
                panic!("Expected Dedupe command");
            }
        }

        #[test]
        fn test_args_dedupe_command_with_pass_through_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "dedupe",
                "--",
                "--some-flag",
                "--another-flag",
            ])
            .unwrap();
            if let Commands::Dedupe { pass_through_args, .. } = &args.commands {
                assert_eq!(
                    pass_through_args,
                    &Some(vec!["--some-flag".to_string(), "--another-flag".to_string()])
                );
            } else {
                panic!("Expected Dedupe command");
            }
        }

        #[test]
        fn test_args_dedupe_command_with_check_and_pass_through() {
            let args =
                Args::try_parse_from(&["vite-plus", "dedupe", "--check", "--", "--custom-flag"])
                    .unwrap();
            if let Commands::Dedupe { check, pass_through_args, .. } = &args.commands {
                assert!(check);
                assert_eq!(pass_through_args, &Some(vec!["--custom-flag".to_string()]));
            } else {
                panic!("Expected Dedupe command");
            }
        }
    }

    mod dlx_command_tests {
        use super::*;

        #[test]
        fn test_args_dlx_command_basic() {
            let args = Args::try_parse_from(&["vite-plus", "dlx", "create-vue", "my-app"]).unwrap();
            if let Commands::Dlx { package, shell_mode, silent, args: cmd_args } = &args.commands {
                assert!(package.is_empty());
                assert!(!shell_mode);
                assert!(!silent);
                assert_eq!(cmd_args, &vec!["create-vue", "my-app"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_version() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "typescript@5.5.4", "tsc", "--version"])
                    .unwrap();
            if let Commands::Dlx { args: cmd_args, .. } = &args.commands {
                assert_eq!(cmd_args, &vec!["typescript@5.5.4", "tsc", "--version"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_package_flag() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "dlx",
                "-p",
                "yo",
                "-p",
                "generator-webapp",
                "yo",
                "webapp",
            ])
            .unwrap();
            if let Commands::Dlx { package, args: cmd_args, .. } = &args.commands {
                assert_eq!(package, &vec!["yo", "generator-webapp"]);
                assert_eq!(cmd_args, &vec!["yo", "webapp"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_long_package_flag() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "dlx",
                "--package",
                "cowsay",
                "--package",
                "lolcatjs",
                "cowsay",
                "hello",
            ])
            .unwrap();
            if let Commands::Dlx { package, args: cmd_args, .. } = &args.commands {
                assert_eq!(package, &vec!["cowsay", "lolcatjs"]);
                assert_eq!(cmd_args, &vec!["cowsay", "hello"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_shell_mode() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "-c", "echo hello | cowsay"]).unwrap();
            if let Commands::Dlx { shell_mode, args: cmd_args, .. } = &args.commands {
                assert!(shell_mode);
                assert_eq!(cmd_args, &vec!["echo hello | cowsay"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_shell_mode_long() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "--shell-mode", "echo hello | cowsay"])
                    .unwrap();
            if let Commands::Dlx { shell_mode, .. } = &args.commands {
                assert!(shell_mode);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_silent() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "-s", "create-vue", "my-app"]).unwrap();
            if let Commands::Dlx { silent, .. } = &args.commands {
                assert!(silent);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_silent_long() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "--silent", "create-vue", "my-app"])
                    .unwrap();
            if let Commands::Dlx { silent, .. } = &args.commands {
                assert!(silent);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_combined_flags() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "dlx",
                "-p",
                "cowsay",
                "-c",
                "-s",
                "echo hello | cowsay",
            ])
            .unwrap();
            if let Commands::Dlx { package, shell_mode, silent, args: cmd_args } = &args.commands {
                assert_eq!(package, &vec!["cowsay"]);
                assert!(shell_mode);
                assert!(silent);
                assert_eq!(cmd_args, &vec!["echo hello | cowsay"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_with_hyphen_args() {
            let args = Args::try_parse_from(&[
                "vite-plus",
                "dlx",
                "typescript",
                "tsc",
                "--noEmit",
                "--strict",
            ])
            .unwrap();
            if let Commands::Dlx { args: cmd_args, .. } = &args.commands {
                assert_eq!(cmd_args, &vec!["typescript", "tsc", "--noEmit", "--strict"]);
            } else {
                panic!("Expected Dlx command");
            }
        }

        #[test]
        fn test_args_dlx_command_requires_package() {
            // dlx requires at least one argument (the package)
            let result = Args::try_parse_from(&["vite-plus", "dlx"]);
            assert!(result.is_err());
        }

        #[test]
        fn test_args_dlx_command_scoped_package() {
            let args =
                Args::try_parse_from(&["vite-plus", "dlx", "@vue/cli@5.0.0", "create", "my-app"])
                    .unwrap();
            if let Commands::Dlx { args: cmd_args, .. } = &args.commands {
                assert_eq!(cmd_args, &vec!["@vue/cli@5.0.0", "create", "my-app"]);
            } else {
                panic!("Expected Dlx command");
            }
        }
    }
}
