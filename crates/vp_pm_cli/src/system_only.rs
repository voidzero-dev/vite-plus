//! Run-time `system-only` restriction of package managers.
//!
//! `VP_NODE_MANAGER=system-only` restricts Node.js and every package-manager
//! family to their system installations. Vite+ then neither downloads nor uses
//! a managed package manager, nor asks the registry which release to use.

use vp_error::Error;
use vp_shared::{EnvConfig, env_vars};

use crate::PackageManagerType;

/// Fail when package managers are restricted to the system.
///
/// # Errors
///
/// Returns [`Error::DisabledBySystemOnly`] in system-only mode.
pub fn ensure_managed_package_manager_allowed(
    package_manager_type: PackageManagerType,
) -> Result<(), Error> {
    if EnvConfig::get().node_system_only() {
        return Err(Error::DisabledBySystemOnly {
            tool: package_manager_type.to_string().into(),
            variable: env_vars::VP_NODE_MANAGER,
        });
    }
    Ok(())
}
