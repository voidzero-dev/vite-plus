//! Opt in to a compatible Node runtime for first-start dependency installation.

use js_semver::{Range, Version};
use vp_js_runtime::NodeProvider;
use vp_shared::EnvConfig;

use crate::{commands::env::config, error::Error};

pub(super) async fn resolve() -> Result<Option<String>, Error> {
    let env = EnvConfig::get();
    let Some(selector) = env.setup_node_version.as_deref().map(str::trim).filter(|v| !v.is_empty())
    else {
        // Keep the installer on LTS unless a caller explicitly opts in.
        // Project discovery and normal Node overrides do not apply here.
        return Ok(None);
    };
    config::resolve_version_alias(selector, &NodeProvider::new()).await.map(Some)
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

    async fn with_selector(selector: Option<&str>) -> Option<String> {
        let root = tempfile::tempdir().unwrap();
        EnvConfig::with_vars_async(
            [
                (env_vars::VP_HOME, root.path().to_str()),
                (env_vars::VP_SETUP_NODE_VERSION, selector),
                (env_vars::VP_NODE_VERSION, Some("20.18.0")),
            ],
            |_| async move {
                config::write_session_version("20.18.0").await.unwrap();
                let mut defaults = config::load_config().await.unwrap();
                defaults.default_node_version = Some("20.18.0".into());
                config::save_config(&defaults).await.unwrap();
                resolve().await.unwrap()
            },
        )
        .await
    }

    #[tokio::test]
    async fn explicit_selector_overrides_normal_node_configuration() {
        assert_eq!(with_selector(Some(" v22.23.1 ")).await.as_deref(), Some("22.23.1"));
    }

    #[tokio::test]
    async fn absent_or_empty_selector_keeps_the_lts_default() {
        for selector in [None, Some(""), Some("  ")] {
            assert_eq!(with_selector(selector).await, None);
        }
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
        let version = with_selector(Some("20.18.0")).await.unwrap();
        assert!(!supports_cli(&version));
        EnvConfig::with_vars_async([(env_vars::VP_SETUP_NODE_VERSION_FILE, &file)], |_| async {
            write_resolution(&version).await.unwrap();
        })
        .await;
        assert_eq!(tokio::fs::read_to_string(file).await.unwrap(), "20.18.0");
    }
}
