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

/// Rewrite ESLint scripts: rename `eslint` → `vp lint` and strip ESLint-only flags.
///
/// Uses brush-parser to parse shell commands, so it correctly handles env var prefixes,
/// compound commands (`&&`, `||`, `|`), and quoted arguments.
///
/// # Arguments
///
/// * `scripts_json` - The scripts section as a JSON string
///
/// # Returns
///
/// * `updated` - The updated scripts JSON string, or `null` if no changes were made
#[napi]
pub fn rewrite_eslint(scripts_json: String) -> Result<Option<String>> {
    let updated = vite_migration::rewrite_eslint(&scripts_json).map_err(anyhow::Error::from)?;
    Ok(updated)
}

/// Rewrite Prettier scripts: rename `prettier` → `vp fmt` and strip Prettier-only flags.
///
/// Uses brush-parser to parse shell commands, so it correctly handles env var prefixes,
/// compound commands (`&&`, `||`, `|`), and quoted arguments.
///
/// # Arguments
///
/// * `scripts_json` - The scripts section as a JSON string
///
/// # Returns
///
/// * `updated` - The updated scripts JSON string, or `null` if no changes were made
#[napi]
pub fn rewrite_prettier(scripts_json: String) -> Result<Option<String>> {
    let updated = vite_migration::rewrite_prettier(&scripts_json).map_err(anyhow::Error::from)?;
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

/// Set the value of a top-level config key in a vite config file (upsert)
///
/// Unlike `mergeJsonConfig`, which prepends a new key (and duplicates it when
/// the key already exists), this targets only direct config objects
/// (`defineConfig({...})`, `export default {...}`, direct callback returns):
/// it replaces the value of an existing `config_key` (pair or shorthand
/// property) or inserts the key when absent. Unrecognized shapes (e.g.
/// `module.exports`, `return someVar`) report `updated: false` instead of
/// being corrupted. The splice is raw, the JS caller is expected to reformat
/// afterwards.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
/// * `json_config_path` - Path to the JSON config file whose contents become the new value
/// * `config_key` - The top-level key whose value should be set
///
/// # Returns
///
/// Returns a `MergeJsonConfigResult`. `updated` is `true` only when at least
/// one direct config object was updated; otherwise the original content is
/// returned unchanged.
///
/// # Example
///
/// ```javascript
/// const result = upsertJsonConfig('vite.config.ts', 'create.json', 'create');
/// if (result.updated) {
///     fs.writeFileSync('vite.config.ts', result.content);
/// }
/// ```
#[napi]
pub fn upsert_json_config(
    vite_config_path: String,
    json_config_path: String,
    config_key: String,
) -> Result<MergeJsonConfigResult> {
    let result = vite_migration::upsert_json_config(
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

/// Whether `config_key` is already declared as a top-level property in the
/// vite config's `defineConfig({...})` (or equivalent) object literal.
///
/// AST-based check covering the six shapes the merger understands; ignores
/// comments, string literal occurrences, and nested keys. Returns `false`
/// for unrecognized shapes (e.g. `return $VAR` from a callback).
#[napi]
pub fn has_config_key(vite_config_path: String, config_key: String) -> Result<bool> {
    let content = std::fs::read_to_string(&vite_config_path).map_err(anyhow::Error::from)?;
    Ok(vite_migration::has_config_key(&content, &config_key).map_err(anyhow::Error::from)?)
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

/// Merge tsdown config into vite config by importing it
///
/// This function adds an import statement for the tsdown config file
/// and adds `pack: packConfig` to the defineConfig.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
/// * `tsdown_config_path` - Relative path to the tsdown.config.ts file (e.g., "./tsdown.config.ts")
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
/// const result = mergeTsdownConfig('vite.config.ts', './tsdown.config.ts');
/// if (result.updated) {
///     fs.writeFileSync('vite.config.ts', result.content);
/// }
/// ```
#[napi]
pub fn merge_tsdown_config(
    vite_config_path: String,
    tsdown_config_path: String,
) -> Result<MergeJsonConfigResult> {
    let result =
        vite_migration::merge_tsdown_config(Path::new(&vite_config_path), &tsdown_config_path)
            .map_err(anyhow::Error::from)?;

    Ok(MergeJsonConfigResult {
        content: result.content,
        updated: result.updated,
        uses_function_callback: result.uses_function_callback,
    })
}

/// Wrap safe inline `plugins: [...]` arrays in recognized Vite config objects
/// with `lazyPlugins(() => [...])` and add a `lazyPlugins` import from
/// `vite-plus` when needed.
#[napi]
pub fn wrap_lazy_plugins(vite_config_path: String) -> Result<MergeJsonConfigResult> {
    let result = vite_migration::wrap_lazy_plugins(Path::new(&vite_config_path))
        .map_err(anyhow::Error::from)?;

    Ok(MergeJsonConfigResult {
        content: result.content,
        updated: result.updated,
        uses_function_callback: result.uses_function_callback,
    })
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
