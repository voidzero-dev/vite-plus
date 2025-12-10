use std::process::ExitStatus;

use clap::{Parser, Subcommand};
use vite_error::Error;
use vite_install::commands::{add::SaveDependencyType, outdated::Format};
use vite_path::AbsolutePathBuf;
use vite_task::{ExecutionSummary, TaskCache, Workspace};

use crate::commands::{
    add::AddCommand, dedupe::DedupeCommand, install::InstallCommand, link::LinkCommand,
    outdated::OutdatedCommand, pm::PmCommand, remove::RemoveCommand, unlink::UnlinkCommand,
    update::UpdateCommand, why::WhyCommand,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[command(
    disable_help_subcommand = true,
    help_template = "\
vite+/{version}

{usage-heading} {usage}

{all-args}{after-help}
"
)]
pub struct Args {
    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // package manager commands
    /// Install command.
    /// It will be passed to the package manager's install command currently.
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        /// Arguments to pass to vite install
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
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
    /// Package manager utilities
    #[command(subcommand)]
    Pm(PmCommands),
    /// Generate a new project
    Gen {
        /// Project name
        #[arg(required = true)]
        name: String,
    },
    /// Migrate an existing project to vite+
    Migrate {
        /// Project directory
        #[arg(required = true)]
        directory: String,
    },
    /// Manage the task cache
    Cache {
        #[clap(subcommand)]
        subcmd: CacheSubcommand,
    },

    // below commands only used to show help message, not actually executed
    /// Run development server
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
    /// Build documentation
    Doc,
    /// Run tasks
    Run,
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

    /// View package information from registry
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

impl Commands {
    /// Check if this command is a package manager command that should skip auto-install
    pub fn is_package_manager_command(&self) -> bool {
        matches!(
            self,
            Commands::Install { .. }
                | Commands::Add { .. }
                | Commands::Remove { .. }
                | Commands::Dedupe { .. }
                | Commands::Outdated { .. }
                | Commands::Why { .. }
                | Commands::Link { .. }
                | Commands::Unlink { .. }
                | Commands::Pm(..)
        )
    }
}

#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// Clean up all the cache
    Clean,
    /// View the cache entries in json for debugging purpose
    View,
}

/// Automatically run install command
async fn auto_install(workspace_root: &AbsolutePathBuf) -> Result<(), Error> {
    // Skip if we're already running inside a vite_task execution to prevent nested installs
    if std::env::var("VITE_TASK_EXECUTION_ENV").is_ok_and(|v| v == "1") {
        tracing::debug!("Skipping auto-install: already running inside vite_task execution");
        return Ok(());
    }

    tracing::debug!("Running install automatically...");
    let _exit_status = InstallCommand::builder(workspace_root.clone())
        .ignore_replay()
        .build()
        .execute(&vec![])
        .await?;
    // For auto-install, we don't propagate exit failures to avoid breaking the main command
    Ok(())
}

#[tracing::instrument]
pub async fn main(cwd: AbsolutePathBuf, mut args: Args) -> Result<std::process::ExitStatus, Error> {
    // Auto-install dependencies if needed, but skip for package manager commands, or if `VITE_DISABLE_AUTO_INSTALL=1` is set.
    if !args.commands.is_package_manager_command()
        && std::env::var_os("VITE_DISABLE_AUTO_INSTALL") != Some("1".into())
    {
        auto_install(&cwd).await?;
    }

    let summary: ExecutionSummary = match &mut args.commands {
        Commands::Cache { subcmd } => {
            let cache_path = Workspace::get_cache_path(&cwd)?;
            match subcmd {
                CacheSubcommand::Clean => {
                    std::fs::remove_dir_all(&cache_path)?;
                }
                CacheSubcommand::View => {
                    let cache = TaskCache::load_from_path(cache_path)?;
                    cache.list(std::io::stdout()).await?;
                }
            }
            return Ok(ExitStatus::default());
        }

        // package manager commands
        Commands::Install { args } => {
            // Check if args contain packages - if yes, redirect to Add command
            // This allows `vite install <packages>` to work as an alias for `vite add <packages>`
            if let Some(Commands::Add {
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
            }) = parse_install_as_add(args)
            {
                let exit_status = execute_add_command(
                    cwd,
                    &packages,
                    save_prod,
                    save_dev,
                    save_peer,
                    save_optional,
                    save_exact,
                    save_catalog,
                    save_catalog_name.as_deref(),
                    filter.as_deref(),
                    workspace_root,
                    workspace,
                    global,
                    allow_build.as_deref(),
                    pass_through_args.as_deref(),
                )
                .await?;
                return Ok(exit_status);
            } else {
                InstallCommand::builder(cwd).build().execute(args).await?
            }
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
        Commands::Pm(pm_command) => {
            let exit_status = PmCommand::new(cwd).execute(pm_command.clone()).await?;
            return Ok(exit_status);
        }
        _ => unreachable!(),
    };

    // Return the first non-zero exit status, or zero if all succeeded
    Ok(summary
        .execution_statuses
        .iter()
        .find_map(|status| {
            #[cfg(unix)]
            use std::os::unix::process::ExitStatusExt;
            #[cfg(windows)]
            use std::os::windows::process::ExitStatusExt;

            // Err(ExecutionFailure) can be skipped because currently the only variant of `ExecutionFailure` is
            // `SkippedDueToFailedDependency`, which means there must be at least one task with non-zero exit status.
            if let Ok(exit_status) = status.execution_result
                && let exit_status = ExitStatus::from_raw(exit_status as _)
                && !exit_status.success()
            {
                Some(exit_status)
            } else {
                None
            }
        })
        .unwrap_or_default())
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

/// Check if install args contain packages (non-flag arguments).
/// If packages are detected, reparse as Add command.
fn parse_install_as_add(args: &[String]) -> Option<Commands> {
    // Check if there are any non-flag arguments (potential package names)
    let has_packages = args.iter().any(|arg| !arg.starts_with('-'));

    if !has_packages {
        return None;
    }

    // Reconstruct command line with "add" subcommand
    let mut cmd_args = vec!["vite".to_string(), "add".to_string()];
    cmd_args.extend_from_slice(args);

    // Try to parse as Add command
    match Args::try_parse_from(&cmd_args) {
        Ok(parsed_args) => Some(parsed_args.commands),
        Err(_) => None, // If parsing fails, fall back to regular install
    }
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

    #[tokio::test]
    async fn test_auto_install_skipped_conditions() {
        use vite_path::AbsolutePathBuf;

        // Test auto_install function directly
        let test_workspace = if cfg!(windows) {
            AbsolutePathBuf::new("C:\\test-workspace-not-exists".into()).unwrap()
        } else {
            AbsolutePathBuf::new("/test-workspace-not-exists".into()).unwrap()
        };

        // Without the environment variable, auto_install should attempt to run
        // (it may fail due to invalid workspace, but that's expected)
        unsafe {
            std::env::remove_var("VITE_TASK_EXECUTION_ENV");
        }
        let result_without_env = auto_install(&test_workspace).await;
        // Should attempt to run (and likely fail with workspace error, which is fine)
        assert!(result_without_env.is_err());

        // With environment variable set to different value, auto_install should still attempt to run
        unsafe {
            std::env::set_var("VITE_TASK_EXECUTION_ENV", "0");
        }
        let result_with_wrong_value = auto_install(&test_workspace).await;
        // Should attempt to run (and likely fail with workspace error, which is fine)
        assert!(result_with_wrong_value.is_err());

        // With environment variable set to "1", auto_install should be skipped (return Ok)
        unsafe {
            std::env::set_var("VITE_TASK_EXECUTION_ENV", "1");
        }
        let result_with_correct_value = auto_install(&test_workspace).await;
        assert!(result_with_correct_value.is_ok());

        // Clean up
        unsafe {
            std::env::remove_var("VITE_TASK_EXECUTION_ENV");
        }
    }

    mod install_as_add_tests {
        use super::*;

        #[test]
        fn test_parse_install_as_add_with_packages() {
            let args = vec!["react".to_string(), "react-dom".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, save_exact, .. }) = result {
                assert_eq!(packages, vec!["react", "react-dom"]);
                assert!(!save_dev);
                assert!(!save_exact);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_parse_install_as_add_with_dev_flag() {
            let args = vec!["-D".to_string(), "typescript".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, .. }) = result {
                assert_eq!(packages, vec!["typescript"]);
                assert!(save_dev);
            } else {
                panic!("Expected Add command");
            }
        }

        #[test]
        fn test_parse_install_as_add_without_packages() {
            let args = vec![];
            let result = parse_install_as_add(&args);
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_install_as_add_with_only_flags() {
            let args = vec!["--some-install-flag".to_string()];
            let result = parse_install_as_add(&args);
            assert!(result.is_none());
        }

        #[test]
        fn test_parse_install_as_add_complex() {
            let args = vec![
                "-D".to_string(),
                "-E".to_string(),
                "--filter".to_string(),
                "app".to_string(),
                "typescript".to_string(),
                "eslint".to_string(),
            ];
            let result = parse_install_as_add(&args);
            assert!(result.is_some());
            if let Some(Commands::Add { packages, save_dev, save_exact, filter, .. }) = result {
                assert_eq!(packages, vec!["typescript", "eslint"]);
                assert!(save_dev);
                assert!(save_exact);
                assert_eq!(filter.unwrap(), vec!["app"]);
            } else {
                panic!("Expected Add command");
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
}
