use std::{path::PathBuf, process::ExitStatus};

use vp_pm_cli::{download_package_manager_for_cwd, resolve_package_manager_version_for_cwd};
use vt_path::AbsolutePathBuf;

use super::{
    config,
    list::package_manager_version_dirs,
    package_manager,
    spec::{EnvScope, EnvSpecs},
};
use crate::{cli::exit_status, error::Error};

fn is_installable_node_source(source: &str) -> bool {
    matches!(
        source,
        ".node-version"
            | ".nvmrc"
            | "engines.node"
            | "devEngines.runtime"
            | config::VERSION_ENV_VAR
            | config::SESSION_VERSION_FILE
    )
}

pub(crate) async fn install(
    cwd: AbsolutePathBuf,
    requests: Vec<String>,
) -> Result<ExitStatus, Error> {
    let (scope, specs) = EnvSpecs::parse_requests(&requests)?;
    let mut status = ExitStatus::default();

    if scope.includes_node() {
        let resolved = match specs.node {
            Some(version) => {
                let provider = vp_js_runtime::NodeProvider::new();
                Some((config::resolve_version_alias(&version, &provider).await?, false))
            }
            None => {
                let resolution = config::resolve_version(&cwd).await?;
                if !is_installable_node_source(&resolution.source) {
                    eprintln!("No Node.js version found in current project.");
                    eprintln!("Specify a version: vp env install <VERSION>");
                    eprintln!("Or pin one:       vp env pin <VERSION>");
                    status = exit_status(1);
                    None
                } else {
                    let from_session_override = matches!(
                        resolution.source.as_str(),
                        config::VERSION_ENV_VAR | config::SESSION_VERSION_FILE
                    );
                    Some((resolution.version, from_session_override))
                }
            }
        };
        if let Some((version, from_session_override)) = resolved {
            vp_shared::output::print_stdout_line(format_args!("Installing Node.js v{version}..."));
            vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &version).await?;
            vp_shared::output::print_stdout_line(format_args!("Installed Node.js v{version}"));
            if from_session_override {
                eprintln!("Note: Installed from session override.");
                eprintln!("Run `vp env use --unset` to revert to project version resolution.");
            }
        }
    }

    if scope.includes_package_managers() {
        let requested = if let Some((kind, selector, hash)) = specs.package_manager {
            let version = resolve_package_manager_version_for_cwd(&cwd, kind, &selector).await?;
            Some((kind, version, hash))
        } else if let EnvScope::PackageManager(kind) = scope {
            let resolution = package_manager::resolve_current_or_fallback_for(&cwd, kind).await?;
            Some((
                resolution.package_manager_type,
                resolution.version,
                resolution.hash.map(|hash| hash.to_string()),
            ))
        } else {
            package_manager::resolve_current(&cwd).await?.map(|current| {
                (
                    current.package_manager_type,
                    current.version,
                    current.hash.map(|hash| hash.to_string()),
                )
            })
        };
        if let Some((kind, version, hash)) = requested {
            vp_shared::output::print_stdout_line(format_args!("Installing {kind} v{version}..."));
            download_package_manager_for_cwd(&cwd, kind, &version, hash.as_deref()).await?;
            vp_shared::output::print_stdout_line(format_args!("Installed {kind} v{version}"));
        }
    }

    Ok(status)
}

pub(crate) async fn uninstall(specs: Vec<String>) -> Result<ExitStatus, Error> {
    let specs = EnvSpecs::parse(&specs)?;
    let package_manager = specs
        .package_manager
        .map(|(kind, version, _)| {
            js_semver::Version::parse(&version).map(|version| (kind, version.to_string())).map_err(
                |_| Error::Other("uninstall requires exact package-manager versions".into()),
            )
        })
        .transpose()?;
    let node = match specs.node {
        Some(version) => {
            let provider = vp_js_runtime::NodeProvider::new();
            Some(config::resolve_version_alias(&version, &provider).await?)
        }
        None => None,
    };

    let home = vp_shared::EnvConfig::get().dirs.data.clone();
    // Each target is one logical install with the directories that hold it. A Node
    // runtime has one; a package-manager version has an unpinned directory and a
    // verified-namespace directory, both from `package_manager_version_dirs`.
    let mut targets: Vec<(String, Vec<PathBuf>)> = Vec::new();
    if let Some(version) = node {
        targets.push((
            format!("Node.js v{version}"),
            vec![home.join("js_runtime").join("node").join(version).into_path_buf()],
        ));
    }
    if let Some((kind, version)) = package_manager {
        let family = home.join("package_manager").join(kind.to_string());
        targets.push((
            format!("{kind} v{version}"),
            package_manager_version_dirs(family.as_path(), &version),
        ));
    }
    for (label, dirs) in &targets {
        if !dirs.iter().any(|dir| dir.exists()) {
            return Err(Error::Other(format!("{label} is not installed").into()));
        }
    }
    for (label, dirs) in targets {
        for dir in dirs {
            if dir.exists() {
                tokio::fs::remove_dir_all(&dir).await?;
            }
        }
        vp_shared::output::print_stdout_line(format_args!("Uninstalled {label}"));
    }
    Ok(ExitStatus::default())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Create a package-manager install directory so the version directory exists.
    fn seed(install_dir: &std::path::Path) {
        std::fs::create_dir_all(install_dir.join("bin")).unwrap();
        std::fs::write(install_dir.join("bin/pnpm.cjs"), "cli").unwrap();
    }

    async fn run_uninstall(vp_home: &std::path::Path) -> Result<ExitStatus, Error> {
        vp_shared::EnvConfig::with_vars_async(
            [(vp_shared::env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async { uninstall(vec!["pnpm@10.34.7".to_string()]).await },
        )
        .await
    }

    #[tokio::test]
    async fn uninstall_removes_a_pinned_only_install() {
        let temp = TempDir::new().unwrap();
        let verified_version = temp.path().join("package_manager/pnpm/.verified-v1/10.34.7");
        seed(&verified_version.join("0123456789abcdef0123456789abcdef/pnpm"));

        assert!(run_uninstall(temp.path()).await.unwrap().success());
        assert!(!verified_version.exists(), "the pinned install must be removed");
    }

    #[tokio::test]
    async fn uninstall_removes_an_unpinned_only_install() {
        let temp = TempDir::new().unwrap();
        let unpinned_version = temp.path().join("package_manager/pnpm/10.34.7");
        seed(&unpinned_version.join("pnpm"));

        assert!(run_uninstall(temp.path()).await.unwrap().success());
        assert!(!unpinned_version.exists(), "the unpinned install must be removed");
    }

    #[tokio::test]
    async fn uninstall_removes_unpinned_and_every_pinned_key() {
        let temp = TempDir::new().unwrap();
        let unpinned_version = temp.path().join("package_manager/pnpm/10.34.7");
        let verified_version = temp.path().join("package_manager/pnpm/.verified-v1/10.34.7");
        seed(&unpinned_version.join("pnpm"));
        seed(&verified_version.join("00000000000000000000000000000000/pnpm"));
        seed(&verified_version.join("11111111111111111111111111111111/pnpm"));

        assert!(run_uninstall(temp.path()).await.unwrap().success());
        assert!(!unpinned_version.exists(), "the unpinned install must be removed");
        assert!(!verified_version.exists(), "every pinned key must be removed");
    }

    #[tokio::test]
    async fn uninstall_reports_an_absent_manager() {
        let temp = TempDir::new().unwrap();
        assert!(
            run_uninstall(temp.path()).await.is_err(),
            "uninstall must fail when neither install layout exists"
        );
    }
}
