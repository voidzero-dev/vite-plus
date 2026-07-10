//! Package-manager infrastructure for `vp`.
//!
//! [`PackageManager`] detects and downloads the selected package manager.
//! [`PackageManagerCommand`] provides the shared clap surface, and [`dispatch`]
//! resolves and executes it. Managed Node.js runtimes and managed global
//! packages remain owned by the global CLI.

#![allow(clippy::allow_attributes, clippy::disallowed_types)]

mod cli;
mod config;
mod dispatch;
mod error;
mod helpers;
mod package_manager;
mod request;
pub(crate) mod resolution;
mod shim;

pub use cli::{ManagedGlobalCommand, PackageManagerCommand, PmCommand};
pub use config::npm_registry;
pub use dispatch::dispatch;
pub use error::Error;
pub use package_manager::{
    PackageManager, PackageManagerBuilder, PackageManagerResolution, PackageManagerSource,
    PackageManagerType, download_package_manager, get_package_manager_type_and_version,
    package_manager_bin_path, package_manager_install_dir,
    resolve_package_manager_from_package_json,
};
pub use request::HttpClient;
pub use resolution::{
    AddArgs, ApproveBuildsArgs, AuditArgs, CacheArgs, ConfigCommand, DedupeArgs, DeprecateArgs,
    DistTagCommand, DlxArgs, FundArgs, InstallArgs, LinkArgs, ListArgs, LoginArgs, LogoutArgs,
    OutdatedArgs, OutdatedFormat, OwnerCommand, PackArgs, PingArgs, PruneArgs, PublishArgs,
    RebuildArgs, RemoveArgs, SearchArgs, StageCommand, TokenCommand, UnlinkArgs, UpdateArgs,
    VersionArgs, ViewArgs, WhoamiArgs, WhyArgs,
};
