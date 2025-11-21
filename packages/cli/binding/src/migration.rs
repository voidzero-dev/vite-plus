use napi::{anyhow, bindgen_prelude::*};
use napi_derive::napi;

/// Rewrite scripts json content using rules from rules_yaml
///
/// # Arguments
///
/// * `scripts_json` - The scripts section of the package.json file as a JSON string
/// * `rules_yaml` - The ast-grep rules.yaml as a YAML string
///
/// # Returns
///
/// * `updated` - The updated scripts section of the package.json file as a JSON string, or `null` if no updates were made
///
/// # Example
///
/// ```javascript
/// const updated = rewriteScripts("scripts section json content here", "ast-grep rules yaml content here");
/// console.log(`Updated: ${updated}`);
/// ```
#[napi]
pub fn rewrite_scripts(scripts_json: String, rules_yaml: String) -> Result<Option<String>> {
    let updated =
        vite_migration::rewrite_scripts(&scripts_json, &rules_yaml).map_err(anyhow::Error::from)?;
    Ok(updated)
}
