use std::path::Path;

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

/// Result of merging JSON config into vite config
#[napi(object)]
pub struct MergeJsonConfigResult {
    /// The updated vite config content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
    /// Whether the config uses a function callback
    pub uses_function_callback: bool,
}

/// Merge JSON configuration file into vite config file
///
/// This function reads the files from disk and merges the JSON config
/// into the vite configuration file.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
/// * `json_config_path` - Path to the JSON config file (e.g., .oxlintrc, .oxfmtrc)
/// * `config_key` - The key to use in the vite config (e.g., "lint", "fmt")
///
/// # Returns
///
/// Returns a `MergeJsonConfigResult` containing:
/// - `content`: The updated vite config content
/// - `updated`: Whether any changes were made
/// - `usesFunctionCallback`: Whether the config uses a function callback
///
/// # Example
///
/// ```javascript
/// const result = mergeJsonConfig('vite.config.ts', '.oxlintrc', 'lint');
/// if (result.updated) {
///     fs.writeFileSync('vite.config.ts', result.content);
/// }
/// ```
#[napi]
pub fn merge_json_config(
    vite_config_path: String,
    json_config_path: String,
    config_key: String,
) -> Result<MergeJsonConfigResult> {
    let result = vite_migration::merge_json_config(
        Path::new(&vite_config_path),
        Path::new(&json_config_path),
        &config_key,
    )
    .map_err(anyhow::Error::from)?;

    Ok(MergeJsonConfigResult {
        content: result.content,
        updated: result.updated,
        uses_function_callback: result.uses_function_callback,
    })
}

/// Error from batch import rewriting
#[napi(object)]
pub struct BatchRewriteError {
    /// The file path that had an error
    pub path: String,
    /// The error message
    pub message: String,
}

/// Result of rewriting imports in multiple files
#[napi(object)]
pub struct BatchRewriteResult {
    /// Files that were modified
    pub modified_files: Vec<String>,
    /// Files that had errors
    pub errors: Vec<BatchRewriteError>,
}

/// Rewrite imports in all TypeScript/JavaScript files under a directory
///
/// This function finds all TypeScript and JavaScript files in the specified directory
/// (respecting `.gitignore` rules), applies the import rewrite rules to each file,
/// and writes the modified content back to disk.
///
/// # Arguments
///
/// * `root` - The root directory to search for files
///
/// # Returns
///
/// Returns a `BatchRewriteResult` containing:
/// - `modifiedFiles`: Files that were changed
/// - `errors`: Files that had errors during processing
///
/// # Example
///
/// ```javascript
/// const result = rewriteImportsInDirectory('./src');
/// console.log(`Modified ${result.modifiedFiles.length} files`);
/// for (const file of result.modifiedFiles) {
///     console.log(`  ${file}`);
/// }
/// ```
#[napi]
pub fn rewrite_imports_in_directory(root: String) -> Result<BatchRewriteResult> {
    let result = vite_migration::rewrite_imports_in_directory(Path::new(&root))
        .map_err(anyhow::Error::from)?;

    Ok(BatchRewriteResult {
        modified_files: result
            .modified_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        errors: result
            .errors
            .iter()
            .map(|(p, m)| BatchRewriteError {
                path: p.to_string_lossy().to_string(),
                message: m.clone(),
            })
            .collect(),
    })
}
