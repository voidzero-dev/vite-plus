//! Run-time restriction of package managers by `VP_ENV_FORBIDDEN`.
//!
//! `VP_ENV_FORBIDDEN=1` stops Vite+ from managing Node.js and every
//! package-manager family. Vite+ then neither downloads nor uses a managed
//! package manager, nor asks the registry which release to use.

use vp_error::Error;
use vp_shared::EnvConfig;

use crate::PackageManagerType;

/// Fail when `VP_ENV_FORBIDDEN=1` stops Vite+ from managing package managers.
///
/// # Errors
///
/// Returns [`Error::DisabledByEnvForbidden`] when `VP_ENV_FORBIDDEN=1`.
pub fn ensure_managed_package_manager_allowed(
    package_manager_type: PackageManagerType,
) -> Result<(), Error> {
    if EnvConfig::get().env_forbidden {
        return Err(Error::DisabledByEnvForbidden {
            tool: package_manager_type.to_string().into(),
        });
    }
    Ok(())
}
