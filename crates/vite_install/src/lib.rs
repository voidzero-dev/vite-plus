#![allow(
    clippy::allow_attributes,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    clippy::print_stdout
)]

pub mod commands;
pub mod config;
pub mod package_manager;
pub mod request;
mod shim;

pub use package_manager::{
    PackageManager, PackageManagerType, download_package_manager,
    get_package_manager_type_and_version,
};
