use std::path::PathBuf;

use napi::{anyhow, bindgen_prelude::*};
use napi_derive::napi;

/// Rewrite package.json scripts using rules from rules_yaml_path
///
/// # Arguments
///
/// * `package_json_path` - The path to the package.json file
/// * `rules_yaml_path` - The path to the ast-grep rules.yaml file
///
/// # Returns
///
/// * `updated` - Whether the package.json scripts were updated
///
/// # Example
///
/// ```javascript
/// const updated = await rewritePackageJsonScripts("package.json", "rules.yaml");
/// console.log(`Updated: ${updated}`);
/// ```
#[napi]
pub async fn rewrite_package_json_scripts(
    package_json_path: String,
    rules_yaml_path: String,
) -> Result<bool> {
    let package_json_path = PathBuf::from(&package_json_path);
    let rules_yaml_path = PathBuf::from(&rules_yaml_path);
    let updated =
        vite_migration::rewrite_package_json_scripts(&package_json_path, &rules_yaml_path)
            .await
            .map_err(anyhow::Error::from)?;
    Ok(updated)
}
