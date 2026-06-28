use std::path::Path;

use napi::{anyhow, bindgen_prelude::*};
use napi_derive::napi;
use node_semver::{Range, Version};
use vite_js_runtime::{NodeProvider, VersionSource, resolve_node_version};
use vite_path::AbsolutePathBuf;

/// Compute the semver requirement that selects the latest release of the same
/// major as `current`.
///
/// `supported_range` is the Vite+-supported Node.js range, sourced from the
/// `engines.node` field in `package.json` (e.g.
/// `^20.19.0 || ^22.18.0 || >=24.11.0`). It is the only source of truth: there
/// are no hardcoded per-major floors here.
///
/// `current` is treated as a semver **range**, so partial pins like `24` or
/// `24.2` are accepted (a leading `v` is tolerated). `node-semver` expands a
/// bare partial to its implied range: `24` → `>=24.0.0 <25.0.0`, `24.2` →
/// `>=24.2.0 <24.3.0`, an exact `24.3.0` → just `24.3.0`.
///
/// Returns `None` when:
/// - `current` cannot be parsed as a range (a true alias like `lts/*` or
///   garbage), or
/// - `current`'s range overlaps `supported_range` — i.e. the pin can already
///   resolve to a supported version, so it is left untouched. This covers a
///   bare major such as `24` (`>=24.0.0 <25.0.0` overlaps `>=24.11.0`) and a
///   partial that is already in range such as `24.11`.
///
/// Otherwise returns a constrained range like `>=24.0.0 <25.0.0` that, when
/// resolved against the Node.js release index, yields the latest release of that
/// major. The resolved version is verified against `supported_range` separately,
/// which is what rejects unsupported majors (e.g. 21 or 23).
fn supported_node_requirement(current: &str, supported_range: &Range) -> Option<String> {
    let normalized = current.strip_prefix('v').unwrap_or(current);

    // Treat the pin as a range so partials ("24", "24.2") are accepted. A true
    // alias ("lts/*") or non-version string fails to parse and is left as-is.
    let current_range = Range::parse(normalized).ok()?;

    // The pin can resolve to a supported version (its range overlaps the
    // supported range) — nothing to upgrade (and never hits the network).
    if supported_range.allows_any(&current_range) {
        return None;
    }

    // Below/outside the supported range: target the latest release of the same
    // major, taken from the leading numeric component (e.g. "24.2" → 24).
    let major: u64 = normalized.split('.').next()?.parse().ok()?;
    Some(format!(">={major}.0.0 <{}.0.0", major + 1))
}

/// Resolve the latest supported Node.js release matching `current`'s major from
/// an explicit version list, verifying the result against `supported_range`.
/// Shared by the NAPI entry point and unit tests.
#[cfg(test)]
fn resolve_supported_node_version_from_list(
    current: &str,
    supported_range: &str,
    versions: &[vite_js_runtime::NodeVersionEntry],
) -> Option<String> {
    let supported = Range::parse(supported_range).ok()?;
    let requirement = supported_node_requirement(current, &supported)?;
    let resolved =
        vite_js_runtime::resolve_version_from_list(&requirement, versions).ok()?.to_string();
    // Verify the resolved version actually satisfies the supported range. An
    // unsupported major (e.g. 21 or 23) resolves to a concrete release but must
    // not be returned.
    Version::parse(resolved.as_str()).ok().filter(|v| supported.satisfies(v)).map(|_| resolved)
}

/// Resolve a Node.js version that is below Vite+'s supported range to the
/// concrete latest release of the same major.
///
/// Engine-strict installers skip the native optional dependency under an
/// unsupported Node.js version (causing "Cannot find native binding"), so
/// `vp migrate` uses this to bump a too-old pin up to a supported release of the
/// same major line.
///
/// # Arguments
///
/// * `current` - The pinned Node.js version, treated as a semver range so
///   partials are accepted (e.g. `24.3.0`, `24.2`, `24`, optionally `v`-prefixed)
/// * `supported_range` - The Vite+-supported Node.js range, sourced from the
///   `engines.node` field in `package.json` (e.g.
///   `^20.19.0 || ^22.18.0 || >=24.11.0`). This is the only source of truth for
///   what is supported.
///
/// # Returns
///
/// * `Some(latest)` - The concrete latest supported release of `current`'s major
///   (e.g. `24.18.0`) when `current`'s range cannot resolve to any supported
///   version but its major has a supported release
/// * `None` - When `current`'s range can already resolve to a supported version
///   (e.g. `24`, `24.11`), cannot be parsed (e.g. `lts/*`), or belongs to an
///   unsupported major (e.g. `21`, `23`)
///
/// # Example
///
/// ```javascript
/// const upgraded = await resolveSupportedNodeVersion('24.3.0', '^20.19.0 || ^22.18.0 || >=24.11.0');
/// // upgraded === '24.18.0' (latest 24.x at the time of resolution)
/// ```
#[napi]
pub async fn resolve_supported_node_version(
    current: String,
    supported_range: String,
) -> Result<Option<String>> {
    let Ok(supported) = Range::parse(&supported_range) else {
        return Ok(None);
    };
    let Some(requirement) = supported_node_requirement(&current, &supported) else {
        return Ok(None);
    };

    let provider = NodeProvider::new();
    let latest = provider.resolve_version(&requirement).await.map_err(anyhow::Error::from)?;
    let latest = latest.to_string();

    // Verify the resolved version is actually supported. An unsupported major
    // (e.g. 21 or 23) resolves to a concrete release but must not be returned.
    match Version::parse(latest.as_str()) {
        Ok(version) if supported.satisfies(&version) => Ok(Some(latest)),
        _ => Ok(None),
    }
}

/// Stable string label for a [`VersionSource`], used as the `source` field of
/// [`resolve_project_node_version`]'s result so the JS migrator can branch on a
/// fixed value instead of the human-facing `Display` string.
fn version_source_label(source: VersionSource) -> &'static str {
    match source {
        VersionSource::NodeVersionFile => "node-version-file",
        VersionSource::DevEnginesRuntime => "dev-engines-runtime",
        VersionSource::EnginesNode => "engines-node",
    }
}

/// The effective Node.js version pin resolved from a project's configuration.
#[napi(object)]
pub struct ProjectNodeVersion {
    /// The pinned version string, exactly as written in the source.
    pub version: String,
    /// Which source the pin came from: `"node-version-file"`,
    /// `"dev-engines-runtime"`, or `"engines-node"`.
    pub source: String,
    /// Absolute path to the file the pin was read from (the `.node-version`
    /// file or the `package.json`).
    pub source_path: String,
}

/// Resolve the single effective Node.js version pin for a project, reusing the
/// shared Rust resolver so the JS migrator does not re-implement source
/// detection.
///
/// Checks, in priority order (see `rfcs/dev-engines.md`):
/// 1. `.node-version`
/// 2. `package.json#devEngines.runtime[name="node"].version`
/// 3. `package.json#engines.node`
///
/// Does not walk up to parent directories: the migrator operates on the project
/// root it was given.
///
/// # Arguments
///
/// * `project_path` - Absolute path to the project directory
///
/// # Returns
///
/// * `Some(ProjectNodeVersion)` - the effective pin, its source label, and the
///   absolute source path
/// * `None` - when no version source is found
///
/// # Example
///
/// ```javascript
/// const pin = await resolveProjectNodeVersion('/path/to/project');
/// // pin === { version: '24.3.0', source: 'node-version-file', sourcePath: '/path/to/project/.node-version' }
/// ```
#[napi]
pub async fn resolve_project_node_version(
    project_path: String,
) -> Result<Option<ProjectNodeVersion>> {
    let project_path = AbsolutePathBuf::new(project_path.into())
        .ok_or_else(|| napi::Error::from_reason("invalid project path"))?;

    let resolution =
        resolve_node_version(&project_path, false).await.map_err(anyhow::Error::from)?;

    Ok(resolution.map(|r| ProjectNodeVersion {
        version: r.version.to_string(),
        source: version_source_label(r.source).to_string(),
        source_path: r
            .source_path
            .map(|p| p.as_path().to_string_lossy().to_string())
            .unwrap_or_default(),
    }))
}

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
    /// Files in Nuxt test-utils packages where upstream `vitest` imports were preserved
    pub preserved_vitest_files: Vec<String>,
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
/// * `preserve_vitest_in_nuxt_packages` - Preserve `vitest` and `vitest/*`
///   specifiers throughout packages that declare `@nuxt/test-utils`
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
pub fn rewrite_imports_in_directory(
    root: String,
    preserve_vitest_in_nuxt_packages: Option<bool>,
) -> Result<BatchRewriteResult> {
    let result = vite_migration::rewrite_imports_in_directory_with_options(
        Path::new(&root),
        vite_migration::RewriteImportsOptions {
            preserve_vitest_in_nuxt_packages: preserve_vitest_in_nuxt_packages.unwrap_or(false),
        },
    )
    .map_err(anyhow::Error::from)?;

    Ok(BatchRewriteResult {
        modified_files: result
            .modified_files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
        preserved_vitest_files: result
            .preserved_vitest_files
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

#[cfg(test)]
mod tests {
    use vite_js_runtime::{LtsInfo, NodeVersionEntry};

    use super::*;

    /// The Vite+-supported Node.js range used as test input. Mirrors the
    /// `engines.node` field shipped in `packages/cli/package.json`.
    const SUPPORTED_RANGE: &str = "^20.19.0 || ^22.18.0 || >=24.11.0";

    /// A mock Node.js release index spanning several majors, mirroring the
    /// shape used in `vite_js_runtime`'s own `resolve_version_from_list` tests.
    fn mock_versions() -> Vec<NodeVersionEntry> {
        vec![
            NodeVersionEntry {
                version: "v24.18.0".into(),
                lts: LtsInfo::Codename("Krypton".into()),
            },
            NodeVersionEntry {
                version: "v24.11.0".into(),
                lts: LtsInfo::Codename("Krypton".into()),
            },
            NodeVersionEntry { version: "v24.3.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v23.5.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v22.18.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v22.10.0".into(), lts: LtsInfo::Codename("Jod".into()) },
            NodeVersionEntry { version: "v21.5.0".into(), lts: LtsInfo::Boolean(false) },
            NodeVersionEntry { version: "v20.19.0".into(), lts: LtsInfo::Codename("Iron".into()) },
            NodeVersionEntry { version: "v20.11.0".into(), lts: LtsInfo::Codename("Iron".into()) },
        ]
    }

    #[test]
    fn version_source_label_is_stable() {
        // These labels are part of the JS<->Rust contract; the JS migrator
        // branches on them, so they must stay fixed.
        assert_eq!(version_source_label(VersionSource::NodeVersionFile), "node-version-file");
        assert_eq!(version_source_label(VersionSource::DevEnginesRuntime), "dev-engines-runtime");
        assert_eq!(version_source_label(VersionSource::EnginesNode), "engines-node");
    }

    #[test]
    fn upgrades_below_range_major_24() {
        // 24.3.0 is below the 24.11.0 floor → latest 24.x (24.18.0).
        let result =
            resolve_supported_node_version_from_list("24.3.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result.as_deref(), Some("24.18.0"));
    }

    #[test]
    fn leaves_supported_major_24_unchanged() {
        // 24.11.0 already satisfies `>=24.11.0`.
        let result =
            resolve_supported_node_version_from_list("24.11.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result, None);
    }

    #[test]
    fn leaves_supported_major_22_unchanged() {
        // 22.18.0 already satisfies `^22.18.0`.
        let result =
            resolve_supported_node_version_from_list("22.18.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result, None);
    }

    #[test]
    fn upgrades_below_range_major_20() {
        // 20.10.0 is below the 20.19.0 floor → latest 20.x (20.19.0).
        let result =
            resolve_supported_node_version_from_list("20.10.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result.as_deref(), Some("20.19.0"));
    }

    #[test]
    fn skips_unsupported_major_21() {
        // Major 21 is not part of the supported range; the resolved release
        // fails the verify-against-range step, so it is never upgraded.
        let result =
            resolve_supported_node_version_from_list("21.5.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result, None);
    }

    #[test]
    fn skips_unsupported_major_23() {
        // Major 23 is not part of the supported range; the resolved release
        // fails the verify-against-range step, so it is never upgraded.
        let result =
            resolve_supported_node_version_from_list("23.5.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result, None);
    }

    #[test]
    fn skips_non_semver_input() {
        assert_eq!(
            resolve_supported_node_version_from_list("lts/*", SUPPORTED_RANGE, &mock_versions()),
            None
        );
        assert_eq!(
            resolve_supported_node_version_from_list("^24.3.0", SUPPORTED_RANGE, &mock_versions()),
            None
        );
        assert_eq!(
            resolve_supported_node_version_from_list(
                "not-a-version",
                SUPPORTED_RANGE,
                &mock_versions()
            ),
            None
        );
        assert_eq!(
            resolve_supported_node_version_from_list("", SUPPORTED_RANGE, &mock_versions()),
            None
        );
    }

    #[test]
    fn tolerates_leading_v_prefix() {
        // A `v`-prefixed exact version is normalized before resolving.
        let result =
            resolve_supported_node_version_from_list("v24.3.0", SUPPORTED_RANGE, &mock_versions());
        assert_eq!(result.as_deref(), Some("24.18.0"));
    }

    #[test]
    fn partial_pin_bare_major_left_unchanged() {
        // "24" → >=24.0.0 <25.0.0 overlaps the supported >=24.11.0, so it can
        // resolve to a supported version → leave it.
        assert_eq!(
            resolve_supported_node_version_from_list("24", SUPPORTED_RANGE, &mock_versions()),
            None
        );
        // "20" → >=20.0.0 <21.0.0 overlaps ^20.19.0 → leave it.
        assert_eq!(
            resolve_supported_node_version_from_list("20", SUPPORTED_RANGE, &mock_versions()),
            None
        );
    }

    #[test]
    fn partial_pin_below_range_upgrades_to_latest_of_major() {
        // "24.2" → >=24.2.0 <24.3.0 cannot reach >=24.11.0 → latest 24.x.
        assert_eq!(
            resolve_supported_node_version_from_list("24.2", SUPPORTED_RANGE, &mock_versions())
                .as_deref(),
            Some("24.18.0")
        );
        // "20.5" → >=20.5.0 <20.6.0 cannot reach ^20.19.0 → latest 20.x.
        assert_eq!(
            resolve_supported_node_version_from_list("20.5", SUPPORTED_RANGE, &mock_versions())
                .as_deref(),
            Some("20.19.0")
        );
    }

    #[test]
    fn partial_pin_in_range_left_unchanged() {
        // "24.11" → >=24.11.0 <24.12.0 is a subset of >=24.11.0 → leave it.
        assert_eq!(
            resolve_supported_node_version_from_list("24.11", SUPPORTED_RANGE, &mock_versions()),
            None
        );
    }

    #[test]
    fn partial_pin_unsupported_major_left_unchanged() {
        // "21.5" → >=21.5.0 <21.6.0 has no supported release → None.
        assert_eq!(
            resolve_supported_node_version_from_list("21.5", SUPPORTED_RANGE, &mock_versions()),
            None
        );
        // Bare unsupported major "21" → resolves latest 21.x, fails verify → None.
        assert_eq!(
            resolve_supported_node_version_from_list("21", SUPPORTED_RANGE, &mock_versions()),
            None
        );
    }

    #[test]
    fn exact_pin_below_range_upgrades_and_already_supported_left() {
        // exact "24.3.0" → no overlap → latest 24.x.
        assert_eq!(
            resolve_supported_node_version_from_list("24.3.0", SUPPORTED_RANGE, &mock_versions())
                .as_deref(),
            Some("24.18.0")
        );
        // exact already-supported "24.18.0" → overlaps → leave it.
        assert_eq!(
            resolve_supported_node_version_from_list("24.18.0", SUPPORTED_RANGE, &mock_versions()),
            None
        );
    }

    #[test]
    fn requirement_targets_same_major_bracket() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // The requirement brackets the whole major; verification against the
        // range happens after resolution, not here.
        assert_eq!(
            supported_node_requirement("24.3.0", &range).as_deref(),
            Some(">=24.0.0 <25.0.0")
        );
        assert_eq!(
            supported_node_requirement("20.10.0", &range).as_deref(),
            Some(">=20.0.0 <21.0.0")
        );
        assert_eq!(
            supported_node_requirement("22.5.0", &range).as_deref(),
            Some(">=22.0.0 <23.0.0")
        );
        // Unsupported majors still produce a major bracket; only the later
        // verify-against-range step rejects them.
        assert_eq!(
            supported_node_requirement("21.5.0", &range).as_deref(),
            Some(">=21.0.0 <22.0.0")
        );
        assert_eq!(
            supported_node_requirement("23.5.0", &range).as_deref(),
            Some(">=23.0.0 <24.0.0")
        );
        // Majors above 24 already satisfy `>=24.11.0`, so they are reported as
        // supported (no upgrade) before a requirement is computed.
        assert_eq!(supported_node_requirement("26.0.0", &range), None);
        assert_eq!(supported_node_requirement("25.0.0", &range), None);
    }
}
