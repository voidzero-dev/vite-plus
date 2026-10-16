//! Share a compatible project runtime with first-start dependency installation.

use js_semver::{Range, Version};
use vp_js_runtime::NodeProvider;
use vp_shared::EnvConfig;
use vt_path::AbsolutePath;

use crate::{commands::env::config, error::Error};

pub(super) async fn resolve(cwd: &AbsolutePath) -> Result<String, Error> {
    let env = EnvConfig::get();
    let selector = match env.setup_node_version.as_deref().filter(|value| !value.trim().is_empty())
    {
        Some(selector) => selector.trim().to_string(),
        None => config::resolve_version(cwd).await?.version,
    };
    // Session overrides and explicit CI inputs can still contain a selector.
    // File-based resolution already returns an exact version, which is reused.
    config::resolve_version_alias(&selector, &NodeProvider::new()).await
}

pub(super) fn supports_cli(version: &str) -> bool {
    // This binary installs its own release. Read that release's requirement
    // from the same package manifest used to publish it.
    let package: serde_json::Value =
        serde_json::from_str(include_str!("../../../../packages/cli/package.json"))
            .expect("CLI package.json must be valid JSON");
    let range = package["engines"]["node"].as_str().expect("CLI must declare engines.node");
    Version::parse(version).is_ok_and(|version| {
        Range::parse(range).expect("CLI Node range must be valid").satisfies(&version)
    })
}

pub(super) async fn write_resolution(version: &str) -> Result<(), Error> {
    if let Some(file) =
        EnvConfig::get().setup_node_version_file.as_deref().filter(|file| !file.is_empty())
    {
        tokio::fs::write(file, version).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use vp_shared::env_vars;

    use super::*;

    async fn with_project(selector: &str, project_version: &str) -> String {
        let root = tempfile::tempdir().unwrap();
        let root = vt_path::AbsolutePathBuf::new(root.path().to_path_buf()).unwrap();
        let project = root.join("project");
        let nested = project.join("packages/app");
        tokio::fs::create_dir_all(&nested).await.unwrap();
        tokio::fs::write(project.join(".node-version"), project_version).await.unwrap();
        tokio::fs::write(project.join(".nvmrc"), "24.11.0").await.unwrap();
        EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, root.as_path().to_str().unwrap()),
                (env_vars::VP_SETUP_NODE_VERSION, selector),
                (env_vars::VP_NODE_VERSION, ""),
            ],
            |_| async move { resolve(&nested).await.unwrap() },
        )
        .await
    }

    #[tokio::test]
    async fn explicit_selector_precedes_project() {
        assert_eq!(with_project("v22.23.1", "26.0.0").await, "22.23.1");
    }

    #[tokio::test]
    async fn project_selection_uses_existing_precedence_and_parent_lookup() {
        assert_eq!(with_project("", "22.23.1").await, "22.23.1");
    }

    #[test]
    fn accepts_supported_project_versions() {
        assert!(supports_cli("22.23.1"));
        assert!(supports_cli("24.11.0"));
        assert!(supports_cli("26.0.0"));
        assert!(!supports_cli("20.18.0"));
        assert!(!supports_cli("22.17.0"));
        assert!(!supports_cli("24.10.0"));
        assert!(!supports_cli("invalid"));
    }

    #[tokio::test]
    async fn returns_the_project_version_even_when_bootstrap_needs_a_fallback() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("selected-node");
        let version = with_project("20.18.0", "26.0.0").await;
        assert!(!supports_cli(&version));
        EnvConfig::with_vars_async([(env_vars::VP_SETUP_NODE_VERSION_FILE, &file)], |_| async {
            write_resolution(&version).await.unwrap();
        })
        .await;
        assert_eq!(tokio::fs::read_to_string(file).await.unwrap(), "20.18.0");
    }
}
