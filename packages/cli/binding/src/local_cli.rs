use napi::{Result, Status};
use napi_derive::napi;
use vt_path::AbsolutePathBuf;

/// Metadata for the project-local vite-plus installation.
#[napi(object, object_from_js = false)]
pub struct LocalVitePlusMetadata {
    pub version: String,
    pub path: String,
}

/// Resolve local vite-plus with the same workspace boundary as the global CLI.
#[napi]
pub fn resolve_local_vite_plus(cwd: String) -> Result<Option<LocalVitePlusMetadata>> {
    let cwd = AbsolutePathBuf::new(cwd.into()).ok_or_else(|| {
        napi::Error::new(Status::InvalidArg, "Working directory must be absolute")
    })?;
    Ok(vp_local_cli::resolve_local_vite_plus_package(&cwd).and_then(|resolved| {
        Some(LocalVitePlusMetadata {
            version: resolved.package_json()?.version()?.to_owned(),
            path: resolved.path().parent()?.to_str()?.to_owned(),
        })
    }))
}
