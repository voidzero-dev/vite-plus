//! Run-time `system-only` package-manager preference.
//!
//! `VP_<FAMILY>_MANAGER=system-only` (or `VP_PM_MANAGER=system-only` without a
//! family-specific value) restricts a family to its system installation. Vite+
//! then neither downloads nor uses a managed release of that family, nor asks
//! the registry which release to use.

use vp_error::Error;
use vp_shared::{EnvConfig, ManagerPreference};

use crate::PackageManagerType;

/// The explicit management preference for `package_manager_type`, with the
/// variable that supplied it. See [`EnvConfig::package_manager_preference`].
#[must_use]
pub fn package_manager_preference(
    package_manager_type: PackageManagerType,
) -> Option<(ManagerPreference, &'static str)> {
    EnvConfig::get().package_manager_preference(&package_manager_type.to_string())
}

/// The variable that restricts `package_manager_type` to the system, if any.
#[must_use]
pub fn system_only_variable(package_manager_type: PackageManagerType) -> Option<&'static str> {
    match package_manager_preference(package_manager_type)? {
        (ManagerPreference::SystemOnly, variable) => Some(variable),
        _ => None,
    }
}

/// Fail when `package_manager_type` is restricted to the system.
///
/// # Errors
///
/// Returns [`Error::DisabledBySystemOnly`] in system-only mode.
pub fn ensure_managed_package_manager_allowed(
    package_manager_type: PackageManagerType,
) -> Result<(), Error> {
    match system_only_variable(package_manager_type) {
        Some(variable) => Err(Error::DisabledBySystemOnly {
            tool: package_manager_type.to_string().into(),
            variable,
        }),
        None => Ok(()),
    }
}
