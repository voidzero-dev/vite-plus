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
/// - `current`'s FLOOR (the minimum version it permits) already satisfies
///   `supported_range` — i.e. the lowest version the pin allows is itself
///   supported, so it is left untouched (e.g. `24.18.0`, `>=24.11.0`,
///   `^22.18.0`, the partial `24.11`).
///
/// The check is FLOOR-based, not overlap-based, because engine-strict installers
/// (pnpm) evaluate the native optional dependency's `engines.node` against the
/// FLOOR of the project's declared range. `>=24` overlaps `>=24.11.0` yet its
/// floor 24.0.0 fails `>=24.11.0`, so pnpm would still skip the native package —
/// such a pin must be lifted.
///
/// Otherwise returns a constrained range like `>=24.0.0 <25.0.0` (the floor's
/// whole major) that, when resolved against the Node.js release index, yields
/// the latest release of that major. The resolved version is verified against
/// `supported_range` separately, which is what rejects unsupported majors (e.g.
/// 21 or 23) whose floor is also below the (nonexistent) supported minimum.
fn supported_node_requirement(current: &str, supported_range: &Range) -> Option<String> {
    let normalized = current.strip_prefix('v').unwrap_or(current);

    // Treat the pin as a range so partials ("24", "24.2") are accepted. A true
    // alias ("lts/*") or non-version string fails to parse and is left as-is.
    let current_range = Range::parse(normalized).ok()?;

    // FLOOR-based check: the pin is already OK iff the minimum version it permits
    // is itself supported. (Overlap is not enough — see the doc comment.)
    let floor = current_range.min_version()?;
    if supported_range.satisfies(&floor) {
        return None;
    }

    // Below the supported minimum: target the latest release of the floor's
    // major (e.g. floor 24.0.0 → ">=24.0.0 <25.0.0").
    let major = floor.major;
    Some(format!(">={major}.0.0 <{}.0.0", major + 1))
}

/// Lift a below-floor Node.js pin up to the lowest *supported* release of its
/// major(s) WITHOUT pinning a concrete version.
///
/// Used to rewrite the constraint fields `engines.node` and
/// `devEngines.runtime[node].version`, where an exact pin would wrongly reject
/// newer supported releases. The supported minimum is derived from
/// `supported_range` (e.g. `^20.19.0 || ^22.18.0 || >=24.11.0` yields `20.19.0`
/// / `22.18.0` / `24.11.0`): there are no hardcoded per-major floors.
///
/// - **Single disjunct** (`>=24`, `^24`, `24.3.0`, …) → the open-ended
///   `>=<supported-minimum>` form, so the constraint keeps accepting newer
///   supported releases of that major.
/// - **Multi-major disjunct** (`^20 || ^22`) → each disjunct's floor is lifted
///   to that major's supported minimum while its per-major upper bound is
///   preserved, so EVERY supported disjunct survives and the unsupported gaps
///   (21, 23) are excluded — e.g.
///   `>=20.19.0 <21.0.0 || >=22.18.0 <23.0.0`. Disjuncts whose entire major has
///   no supported release (e.g. a lone `^21`, or the `^18` in `^18 || ^20`)
///   drop out.
///
/// Returns `None` when the input is unparsable, no disjunct has a supported
/// release in its major (e.g. `^21 || ^23`), or nothing needed lifting. For a
/// single disjunct that means its floor is already supported, and for a union it
/// means every branch is already supported and survives verbatim (e.g.
/// `^20.19.0 || ^22.18.0`). Pure range math: never touches the network / release
/// index.
///
/// # Example
///
/// `supported_node_floor_range(">=24", &range)` → `Some(">=24.11.0")`;
/// `supported_node_floor_range("^20 || ^22", &range)`
/// → `Some(">=20.19.0 <21.0.0 || >=22.18.0 <23.0.0")`.
fn supported_node_floor_range(current: &str, supported_range: &Range) -> Option<String> {
    let normalized = current.strip_prefix('v').unwrap_or(current);

    // Multi-major pin (`^20 || ^22`): each disjunct must be lifted INDEPENDENTLY,
    // regardless of the overall floor. The overall floor (the minimum across all
    // disjuncts) cannot gate a union: a union whose lowest branch is supported
    // can still carry a LATER below-floor branch (e.g. `^20.19.0 || ^22.0.0`,
    // whose 22 branch admits 22.0 to 22.17 below the `^22.18.0` floor).
    // Collapsing to a single open-ended `>=<supported-minimum>` would also
    // (a) DROP the other disjuncts and (b) WIDEN the constraint across the gaps
    // (21, 23). So lift each disjunct on its own: every supported major survives,
    // each below-floor disjunct is bounded to its own major (excluding 21 / 23),
    // and fully-unsupported disjuncts drop out. Return `None` only when no
    // disjunct survives (`^21 || ^23`) or every disjunct survives verbatim, i.e.
    // nothing needed lifting and nothing was dropped (`^20.19.0 || ^22.18.0`).
    if normalized.contains("||") {
        let originals: Vec<&str> = normalized.split("||").map(str::trim).collect();
        let lifted: Vec<String> = originals
            .iter()
            .filter_map(|disjunct| lift_disjunct(disjunct, supported_range))
            .collect();
        if lifted.is_empty() {
            return None;
        }
        // Unchanged union: same arity AND every branch kept verbatim → no rewrite.
        let unchanged = lifted.len() == originals.len()
            && lifted.iter().zip(&originals).all(|(lift, original)| lift.as_str() == *original);
        if unchanged {
            return None;
        }
        return Some(lifted.join(" || "));
    }

    // Single disjunct: reuse the floor-based decision as the gate. `None` here
    // means "leave the pin alone" because the floor is already supported.
    let requirement = supported_node_requirement(current, supported_range)?;

    // Single disjunct: keep the open-ended `>=<supported-minimum>` form so the
    // constraint keeps accepting newer supported releases of the same major
    // (e.g. `^24` / `>=24` → `>=24.11.0`). The supported minimum for this major
    // is the floor of the major bracket intersected with the supported range.
    // An unsupported major (e.g. 21) yields an empty intersection → `None`,
    // matching the resolve-then-verify rejection on the concrete path.
    let requirement_range = Range::parse(&requirement).ok()?;
    let supported_minimum = requirement_range.intersect(supported_range)?.min_version()?;
    Some(format!(">={supported_minimum}"))
}

/// Rewrite a single disjunct of a multi-major constraint pin (a `||`-separated
/// branch of `engines.node` / `devEngines.runtime[node].version`).
///
/// Returns:
/// - the disjunct verbatim when its floor is already supported (e.g. the
///   `^20.19.0` branch of `^18 || ^20.19.0`),
/// - `>=<supported-minimum> <<next-major>.0.0` when the floor is below the
///   major's supported minimum but bounded to that major (`^20`, `20`,
///   `>=20 <21` → `>=20.19.0 <21.0.0`), so the unsupported next major is
///   excluded,
/// - the open `>=<supported-minimum>` when the disjunct itself extends past its
///   major (an open `>=24` branch stays open), or
/// - `None` when the major has no supported release (a lone `^21`, or the `^18`
///   of `^18 || ^20`), so the disjunct drops out of the rewritten union.
///
/// Unlike the single-disjunct path, a bounded caret branch (`^24`) is kept
/// bounded here: in a union the user enumerated specific majors, so the rewrite
/// must not widen past them.
fn lift_disjunct(disjunct: &str, supported_range: &Range) -> Option<String> {
    let parsed = Range::parse(disjunct).ok()?;
    let floor = parsed.min_version()?;

    // Floor already supported → keep the user's branch exactly as written.
    if supported_range.satisfies(&floor) {
        return Some(disjunct.to_string());
    }

    // Below floor: lift to this major's supported minimum. An unsupported major
    // (e.g. 21) has an empty intersection with the supported range → drop it.
    let major = floor.major;
    let bracket = Range::parse(format!(">={major}.0.0 <{}.0.0", major + 1)).ok()?;
    let supported_minimum = bracket.intersect(supported_range)?.min_version()?;

    // A branch that admits the start of the next major is open-ended (`>=24`);
    // keep it open so it accepts newer releases. A per-major branch (`^24`,
    // `24`, `>=20 <21`) is bounded to its major so 21 / 23 stay excluded.
    if parsed.satisfies(&Version::from((major + 1, 0, 0))) {
        Some(format!(">={supported_minimum}"))
    } else {
        Some(format!(">={supported_minimum} <{}.0.0", major + 1))
    }
}

/// Return `resolved` only when it satisfies `supported_range`. An unsupported
/// major (e.g. 21 or 23) resolves to a concrete release of its own major but
/// must not be returned. Shared by the NAPI entry point and the unit tests so
/// the resolve-then-verify contract lives in one place.
fn resolved_if_supported(resolved: String, supported_range: &Range) -> Option<String> {
    Version::parse(resolved.as_str())
        .ok()
        .filter(|version| supported_range.satisfies(version))
        .map(|_| resolved)
}

/// Resolve the latest supported Node.js release matching `current`'s major from
/// an explicit version list, verifying the result against `supported_range`.
/// Test-only mirror of [`resolve_supported_node_version`] that takes a fixed
/// version list instead of hitting the Node.js release index.
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
    resolved_if_supported(resolved, &supported)
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
///   (e.g. `24.18.0`) when `current`'s FLOOR is below the major's supported
///   minimum but the major has a supported release (e.g. `24.3.0`, `24`, `>=24`,
///   `^24`)
/// * `None` - When `current`'s FLOOR is already supported (e.g. `24.18.0`,
///   `24.11`, `>=24.11.0`), cannot be parsed (e.g. `lts/*`), or belongs to an
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

    Ok(resolved_if_supported(latest.to_string(), &supported))
}

/// Compute the open-ended `>=<supported-minimum>` range that lifts a below-floor
/// Node.js pin up to the lowest supported release of the same major, for
/// rewriting the `engines.node` / `devEngines.runtime[node].version` constraint
/// fields.
///
/// Unlike [`resolve_supported_node_version`] (which pins a concrete release for
/// the single-version `.node-version` file), this returns an open-ended range so
/// the constraint keeps accepting newer supported releases. Pure range math — it
/// never hits the network.
///
/// # Arguments
///
/// * `current` - The pinned Node.js spec, treated as a semver range so partials
///   and ranges are accepted (e.g. `>=24`, `^24`, `24`, `24.3.0`,
///   optionally `v`-prefixed)
/// * `supported_range` - The Vite+-supported Node.js range, sourced from the
///   `engines.node` field in `package.json` (e.g.
///   `^20.19.0 || ^22.18.0 || >=24.11.0`)
///
/// # Returns
///
/// * `Some(range)` - e.g. `>=24.11.0` when `current`'s floor is below the major's
///   supported minimum but the major has a supported release
/// * `None` - when `current`'s floor is already supported, cannot be parsed
///   (e.g. `lts/*`), or belongs to an unsupported major (e.g. `21`, `23`)
///
/// # Example
///
/// ```javascript
/// const range = resolveSupportedNodeRange('>=24', '^20.19.0 || ^22.18.0 || >=24.11.0');
/// // range === '>=24.11.0'
/// ```
#[napi]
pub fn resolve_supported_node_range(
    current: String,
    supported_range: String,
) -> Result<Option<String>> {
    let Ok(supported) = Range::parse(&supported_range) else {
        return Ok(None);
    };

    Ok(supported_node_floor_range(&current, &supported))
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
        // True aliases / garbage fail to parse as a range and are left as-is.
        // (`^24.3.0` is valid semver with a below-floor minimum, so it now
        // upgrades — see `floor_based_open_and_caret_ranges_upgrade`.)
        assert_eq!(
            resolve_supported_node_version_from_list("lts/*", SUPPORTED_RANGE, &mock_versions()),
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

    // NOTE: bare-major upgrades (`24` → 24.18.0, `20` → 20.19.0) are covered by
    // `floor_based_bare_major_upgrades`; under FLOOR-based logic a bare major's
    // floor (e.g. 24.0.0) is below the supported minimum, so it is lifted.

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

    // ---------------------------------------------------------------------
    // FLOOR-BASED behavior (new): a pin is OK iff the *minimum* version it
    // permits is itself supported. Overlap is not enough — `>=24` overlaps
    // `>=24.11.0` but its floor 24.0.0 is below the supported minimum, so
    // pnpm's engine check against the floor would skip the native dep.
    // ---------------------------------------------------------------------

    #[test]
    fn floor_based_bare_major_upgrades() {
        // "24" floor is 24.0.0 (< 24.11.0) → upgrade to latest 24.x.
        assert_eq!(
            resolve_supported_node_version_from_list("24", SUPPORTED_RANGE, &mock_versions())
                .as_deref(),
            Some("24.18.0")
        );
        // "20" floor is 20.0.0 (< 20.19.0) → upgrade to latest 20.x.
        assert_eq!(
            resolve_supported_node_version_from_list("20", SUPPORTED_RANGE, &mock_versions())
                .as_deref(),
            Some("20.19.0")
        );
    }

    #[test]
    fn floor_based_open_and_caret_ranges_upgrade() {
        // All of these have a floor below the major's supported minimum even
        // though the range overlaps the supported range.
        for spec in [">=24", ">=24.0.0", "^24", "^24.3.0", "^24.10.0"] {
            assert_eq!(
                resolve_supported_node_version_from_list(spec, SUPPORTED_RANGE, &mock_versions())
                    .as_deref(),
                Some("24.18.0"),
                "spec {spec} should upgrade to latest supported 24.x"
            );
        }
    }

    #[test]
    fn floor_based_already_supported_ranges_left() {
        // Floor already supported → leave alone (regression guard).
        for spec in [">=24.11.0", "^22.18.0", ">=22.18.0", "24.18.0", "24.11", ">=25"] {
            assert_eq!(
                resolve_supported_node_version_from_list(spec, SUPPORTED_RANGE, &mock_versions()),
                None,
                "spec {spec} should be left unchanged"
            );
        }
    }

    // ---------------------------------------------------------------------
    // supported_node_floor_range: open-ended `>=<supported-minimum>` for the
    // constraint fields (engines.node / devEngines.runtime). Pure range math,
    // derived from the supported range (no hardcoded per-major floors).
    // ---------------------------------------------------------------------

    #[test]
    fn floor_range_targets_supported_minimum() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // Below-floor major-24 pins (bare/open/caret/partial/exact) all lift to
        // the major's supported minimum, 24.11.0.
        for spec in ["24", ">=24", ">=24.0.0", "^24", "^24.3.0", "24.3.0", "24.2"] {
            assert_eq!(
                supported_node_floor_range(spec, &range).as_deref(),
                Some(">=24.11.0"),
                "spec {spec} should lift to >=24.11.0"
            );
        }
        // Other majors derive their own supported minimum from the range.
        assert_eq!(supported_node_floor_range("22.0.0", &range).as_deref(), Some(">=22.18.0"));
        assert_eq!(supported_node_floor_range(">=22", &range).as_deref(), Some(">=22.18.0"));
        assert_eq!(supported_node_floor_range("20.5", &range).as_deref(), Some(">=20.19.0"));
        assert_eq!(supported_node_floor_range("20", &range).as_deref(), Some(">=20.19.0"));
    }

    #[test]
    fn floor_range_leaves_supported_and_unsupported_alone() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // Already-supported floor → None.
        for spec in [">=24.11.0", "24.18.0", "^22.18.0", "24.11", ">=25"] {
            assert_eq!(
                supported_node_floor_range(spec, &range),
                None,
                "spec {spec} floor is already supported"
            );
        }
        // Unsupported major (no supported release in that major) → None.
        for spec in ["21", "21.5", "23.5.0", "18", ">=18"] {
            assert_eq!(
                supported_node_floor_range(spec, &range),
                None,
                "spec {spec} has no supported release in its major"
            );
        }
        // Unparsable input → None.
        assert_eq!(supported_node_floor_range("lts/*", &range), None);
        assert_eq!(supported_node_floor_range("", &range), None);
    }

    // ---------------------------------------------------------------------
    // Multi-major (disjunct) constraint fields: a `^20 || ^22`-style pin must
    // keep EVERY supported disjunct and lift each below-floor disjunct to its
    // own supported minimum, without (a) dropping later disjuncts or (b)
    // widening open-ended across the unsupported gaps (21 / 23).
    // ---------------------------------------------------------------------

    #[test]
    fn floor_range_multi_major_preserves_supported_disjuncts() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();

        // `^20 || ^22`: both floors (20.0.0, 22.0.0) are below their major's
        // supported minimum, but both majors are supported. The rewrite must
        // lift each floor and keep BOTH disjuncts.
        let result = supported_node_floor_range("^20 || ^22", &range)
            .expect("multi-major below-floor pin should be rewritten");
        assert_eq!(result, ">=20.19.0 <21.0.0 || >=22.18.0 <23.0.0");

        let rewritten = Range::parse(&result).unwrap();
        assert!(rewritten.satisfies(&Version::parse("20.19.0").unwrap()), "keeps supported 20.x");
        assert!(rewritten.satisfies(&Version::parse("22.18.0").unwrap()), "keeps supported 22.x");
        assert!(!rewritten.satisfies(&Version::parse("21.5.0").unwrap()), "must not admit 21.x");
        assert!(!rewritten.satisfies(&Version::parse("23.5.0").unwrap()), "must not admit 23.x");
    }

    #[test]
    fn floor_range_multi_major_drops_unsupported_disjunct() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();

        // `^18 || ^20 || ^22`: 18 has no supported release, so its disjunct is
        // dropped; 20 and 22 are lifted to their supported minimums.
        let result = supported_node_floor_range("^18 || ^20 || ^22", &range)
            .expect("multi-major pin with supported majors should be rewritten");
        assert_eq!(result, ">=20.19.0 <21.0.0 || >=22.18.0 <23.0.0");

        let rewritten = Range::parse(&result).unwrap();
        assert!(!rewritten.satisfies(&Version::parse("18.20.0").unwrap()), "18 dropped");
        assert!(rewritten.satisfies(&Version::parse("20.19.0").unwrap()));
        assert!(rewritten.satisfies(&Version::parse("22.18.0").unwrap()));
        assert!(!rewritten.satisfies(&Version::parse("21.5.0").unwrap()));
        assert!(!rewritten.satisfies(&Version::parse("23.5.0").unwrap()));
    }

    #[test]
    fn floor_range_multi_major_explicit_bounds() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();

        // Explicit `>=X <Y` disjuncts get their floors lifted while keeping the
        // per-major upper bound, still excluding 21 / 23.
        let result = supported_node_floor_range(">=20 <21 || >=22 <23", &range)
            .expect("explicit-bound multi-major pin should be rewritten");
        assert_eq!(result, ">=20.19.0 <21.0.0 || >=22.18.0 <23.0.0");

        let rewritten = Range::parse(&result).unwrap();
        // Regression guard: the supported 22.x portion still resolves.
        assert!(
            rewritten.satisfies(&Version::parse("22.18.0").unwrap()),
            "supported 22.x resolves"
        );
        assert!(!rewritten.satisfies(&Version::parse("21.5.0").unwrap()));
        assert!(!rewritten.satisfies(&Version::parse("23.5.0").unwrap()));
    }

    #[test]
    fn floor_range_multi_major_all_unsupported_is_none() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // No disjunct has a supported release in its major → leave the pin alone.
        assert_eq!(supported_node_floor_range("^21 || ^23", &range), None);
    }

    #[test]
    fn floor_range_multi_major_lifts_later_below_floor_disjunct() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();

        // `^20.19.0 || ^22.0.0`: the FIRST/lowest branch (20.19.0) is already
        // supported, so the OVERALL floor is supported, but the LATER `^22.0.0`
        // branch accepts 22.0 to 22.17, below the `^22.18.0` floor. The 22 branch
        // must be lifted independently while the supported `^20.19.0` branch
        // stays verbatim, even though the overall floor would gate the rewrite
        // out.
        let result = supported_node_floor_range("^20.19.0 || ^22.0.0", &range)
            .expect("a below-floor later disjunct must be lifted");
        assert_eq!(result, "^20.19.0 || >=22.18.0 <23.0.0");

        let rewritten = Range::parse(&result).unwrap();
        assert!(
            rewritten.satisfies(&Version::parse("20.19.0").unwrap()),
            "keeps the supported 20.x branch"
        );
        assert!(
            rewritten.satisfies(&Version::parse("22.18.0").unwrap()),
            "admits the lifted 22.18.0"
        );
        assert!(
            !rewritten.satisfies(&Version::parse("22.17.0").unwrap()),
            "must not admit below-floor 22.17.x"
        );
        assert!(!rewritten.satisfies(&Version::parse("23.0.0").unwrap()), "must not admit 23.x");
    }

    #[test]
    fn floor_range_multi_major_all_supported_is_none() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // Every branch is already supported → no rewrite, even though it is a
        // union (regression guard for the gate-removal change).
        assert_eq!(supported_node_floor_range("^20.19.0 || ^22.18.0", &range), None);
    }

    #[test]
    fn floor_range_multi_major_drops_unsupported_lower_keeps_supported_higher() {
        let range = Range::parse(SUPPORTED_RANGE).unwrap();
        // `^18 || ^20.19.0`: the lower branch's major (18) has no supported
        // release and drops out; the higher `^20.19.0` branch is already
        // supported and stays verbatim. Dropping a branch is still a rewrite.
        let result = supported_node_floor_range("^18 || ^20.19.0", &range)
            .expect("dropping an unsupported lower branch is still a rewrite");
        assert_eq!(result, "^20.19.0");

        let rewritten = Range::parse(&result).unwrap();
        assert!(
            rewritten.satisfies(&Version::parse("20.19.0").unwrap()),
            "keeps the supported 20.x branch"
        );
        assert!(
            !rewritten.satisfies(&Version::parse("18.20.0").unwrap()),
            "the unsupported 18 branch is dropped"
        );
    }
}
