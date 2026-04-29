//! Shared helpers used by every PM handler.

use vite_install::package_manager::{PackageManager, PackageManagerType};
use vite_path::AbsolutePath;

use crate::error::Error;

/// Build a `PackageManager`, converting `PackageJsonNotFound` into a
/// friendly error message.
pub async fn build_package_manager(cwd: &AbsolutePath) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build_with_default().await {
        Ok(pm) => Ok(pm),
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_))) => {
            Err(Error::UserMessage("No package.json found.".into()))
        }
        Err(e) => Err(Error::Install(e)),
    }
}

/// Build a `PackageManager`, falling back to a default npm instance when no
/// package.json is found. Uses `build()` instead of `build_with_default()`
/// to skip the interactive package manager selection prompt on the fallback path.
///
/// Callers should ensure npm is on PATH before invoking commands that hit
/// this fallback (the global CLI does this via its managed Node runtime;
/// the local CLI relies on the system Node).
pub async fn build_package_manager_or_npm_default(
    cwd: &AbsolutePath,
) -> Result<PackageManager, Error> {
    match PackageManager::builder(cwd).build().await {
        Ok(pm) => Ok(pm),
        Err(vite_error::Error::WorkspaceError(vite_workspace::Error::PackageJsonNotFound(_)))
        | Err(vite_error::Error::UnrecognizedPackageManager) => {
            Ok(default_npm_package_manager(cwd))
        }
        Err(e) => Err(Error::Install(e)),
    }
}

fn default_npm_package_manager(cwd: &AbsolutePath) -> PackageManager {
    PackageManager {
        client: PackageManagerType::Npm,
        package_name: "npm".into(),
        version: "latest".into(),
        hash: None,
        bin_name: "npm".into(),
        workspace_root: cwd.to_absolute_path_buf(),
        is_monorepo: false,
        install_dir: cwd.to_absolute_path_buf(),
    }
}

/// Ensure a package.json exists in the given directory.
/// If it doesn't exist, create a minimal one with `{ "type": "module" }`.
pub async fn ensure_package_json(project_path: &AbsolutePath) -> Result<(), Error> {
    use tokio::io::AsyncWriteExt;

    let package_json_path = project_path.join("package.json");
    let content = serde_json::to_string_pretty(&serde_json::json!({ "type": "module" }))?;
    match tokio::fs::OpenOptions::new().write(true).create_new(true).open(&package_json_path).await
    {
        Ok(mut file) => {
            file.write_all(content.as_bytes()).await?;
            file.write_all(b"\n").await?;
            tracing::info!("Created package.json in {:?}", project_path);
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(Error::CommandExecution(err)),
    }
}
