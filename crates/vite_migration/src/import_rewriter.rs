use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use regex::Regex;
use vite_error::Error;

use crate::{ast_grep, file_walker};

/// ast-grep rules for rewriting vite imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'vite'` → `import { ... } from 'vite-plus'`
/// - `import { ... } from 'vite/{name}'` → `import { ... } from 'vite-plus/{name}'`
/// - `declare module 'vite' { ... }` → `declare module 'vite-plus' { ... }`
/// - `declare module 'vite/{name}' { ... }` → `declare module 'vite-plus/{name}' { ... }`
const REWRITE_VITE_RULES: &str = r#"---
id: rewrite-vite-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vite-subpath-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vite/.+['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vite
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vite['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vite-subpath
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vite/.+['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vite/
      by: "vite-plus/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting vitest imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'vitest'` → `import { ... } from 'vite-plus/test'`
/// - `import { ... } from 'vitest/config'` → `import { ... } from 'vite-plus'`
/// - `import { ... } from 'vitest/{name}'` → `import { ... } from 'vite-plus/test/{name}'`
/// - `import { ... } from '@vitest/browser'` → `import { ... } from 'vite-plus/test/browser'`
/// - `import { ... } from '@vitest/browser/{name}'` → `import { ... } from 'vite-plus/test/browser/{name}'`
/// - `import { ... } from '@vitest/browser-playwright'` → `import { ... } from 'vite-plus/test/browser-playwright'`
/// - `import { ... } from '@vitest/browser-playwright/{name}'` → `import { ... } from 'vite-plus/test/browser-playwright/{name}'`
/// - `import { ... } from '@vitest/browser-preview'` → `import { ... } from 'vite-plus/test/browser-preview'`
/// - `import { ... } from '@vitest/browser-preview/{name}'` → `import { ... } from 'vite-plus/test/browser-preview/{name}'`
/// - `import { ... } from '@vitest/browser-webdriverio'` → `import { ... } from 'vite-plus/test/browser-webdriverio'`
/// - `import { ... } from '@vitest/browser-webdriverio/{name}'` → `import { ... } from 'vite-plus/test/browser-webdriverio/{name}'`
/// - `declare module 'vitest' { ... }` → `declare module 'vite-plus/test' { ... }`
/// - `declare module 'vitest/config' { ... }` → `declare module 'vite-plus' { ... }`
/// - `declare module 'vitest/{name}' { ... }` → `declare module 'vite-plus/test/{name}' { ... }`
/// - `declare module '@vitest/browser' { ... }` → `declare module 'vite-plus/test/browser' { ... }`
/// - `declare module '@vitest/browser/{name}' { ... }` → `declare module 'vite-plus/test/browser/{name}' { ... }`
/// - `declare module '@vitest/browser-playwright' { ... }` → `declare module 'vite-plus/test/browser-playwright' { ... }`
/// - `declare module '@vitest/browser-playwright/{name}' { ... }` → `declare module 'vite-plus/test/browser-playwright/{name}' { ... }`
/// - `declare module '@vitest/browser-preview' { ... }` → `declare module 'vite-plus/test/browser-preview' { ... }`
/// - `declare module '@vitest/browser-preview/{name}' { ... }` → `declare module 'vite-plus/test/browser-preview/{name}' { ... }`
/// - `declare module '@vitest/browser-webdriverio' { ... }` → `declare module 'vite-plus/test/browser-webdriverio' { ... }`
/// - `declare module '@vitest/browser-webdriverio/{name}' { ... }` → `declare module 'vite-plus/test/browser-webdriverio/{name}' { ... }`
const REWRITE_VITEST_RULES: &str = r#"---
id: rewrite-vitest-config-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/config['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-vitest-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-vitest-scoped-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]@vitest/(browser-playwright|browser-preview|browser-webdriverio|browser)(/.*)?['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-vitest-subpath-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]vitest/.+['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vitest-config
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vitest/config['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/config
      by: "vite-plus"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vitest
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vitest['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest
      by: "vite-plus/test"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vitest-scoped
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]@vitest/(browser-playwright|browser-preview|browser-webdriverio|browser)(/.*)?['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: "@vitest/"
      by: "vite-plus/test/"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-vitest-subpath
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['\"]vitest/.+['\"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: vitest/
      by: "vite-plus/test/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting tsdown imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'tsdown'` → `import { ... } from 'vite-plus/pack'`
/// - `declare module 'tsdown' { ... }` → `declare module 'vite-plus/pack' { ... }`
const REWRITE_TSDOWN_RULES: &str = r#"---
id: rewrite-tsdown-import
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: import_statement
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
---
id: rewrite-declare-module-tsdown
language: TypeScript
rule:
  pattern: $STR
  kind: string
  regex: ^['"]tsdown['"]$
  inside:
    kind: module
transform:
  NEW_IMPORT:
    replace:
      source: $STR
      replace: tsdown
      by: "vite-plus/pack"
fix: $NEW_IMPORT
"#;

// Regex patterns for rewriting `/// <reference types="..." />` directives.
// These cannot be handled by ast-grep because triple-slash references are parsed as comments.

/// `vitest/config` → `vite-plus` (special case, must be applied before generic vitest subpath)
static RE_REF_VITEST_CONFIG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest/config(["']\s*/>)"#).unwrap()
});

/// bare `vitest` → `vite-plus/test`
static RE_REF_VITEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest(["']\s*/>)"#).unwrap()
});

/// `vitest/{subpath}` → `vite-plus/test/{subpath}`
static RE_REF_VITEST_SUBPATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vitest/(.+?)(["']\s*/>)"#).unwrap()
});

/// `@vitest/{pkg}[/{subpath}]` → `vite-plus/test/{pkg}[/{subpath}]`
/// Only matches packages and subpaths that vite-plus actually exports:
///   - `@vitest/browser` → `vite-plus/test/browser`
///   - `@vitest/browser/context` → `vite-plus/test/browser/context`
///   - `@vitest/browser/providers/{name}` → `vite-plus/test/browser/providers/{name}`
///   - `@vitest/browser-playwright[/{subpath}]` → `vite-plus/test/browser-playwright[/{subpath}]`
///   - `@vitest/browser-preview[/{subpath}]` → `vite-plus/test/browser-preview[/{subpath}]`
///   - `@vitest/browser-webdriverio[/{subpath}]` → `vite-plus/test/browser-webdriverio[/{subpath}]`
static RE_REF_VITEST_SCOPED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])@vitest/((?:browser-playwright|browser-preview|browser-webdriverio)(?:/.+?)?|browser(?:/(?:context|providers/.+?))?)(["']\s*/>)"#).unwrap()
});

/// bare `vite` → `vite-plus`
static RE_REF_VITE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vite(["']\s*/>)"#).unwrap()
});

/// `vite/{subpath}` → `vite-plus/{subpath}`
static RE_REF_VITE_SUBPATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])vite/(.+?)(["']\s*/>)"#).unwrap()
});

/// bare `tsdown` → `vite-plus/pack`
static RE_REF_TSDOWN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^(\s*///\s*<reference\s+types\s*=\s*["'])tsdown(["']\s*/>)"#).unwrap()
});

/// Apply a single regex replacement, updating `content` in place if matched.
/// Uses `Cow::Owned` variant check to avoid O(n) string comparison on no-match.
/// Uses `replace` (not `replace_all`) since each line contains at most one reference directive.
fn apply_regex_replace(content: &mut String, re: &Regex, replacement: &str) -> bool {
    use std::borrow::Cow;
    match re.replace(content, replacement) {
        Cow::Owned(new) => {
            *content = new;
            true
        }
        Cow::Borrowed(_) => false,
    }
}

/// Rewrite `/// <reference types="..." />` directives in place.
///
/// Only processes the file preamble (blank lines and comments before the first statement)
/// to match TypeScript semantics and avoid false positives inside string/template literals.
/// Allocates only for preamble lines, leaving the file body untouched.
/// Returns whether any changes were made.
fn rewrite_reference_types(content: &mut String, skip_packages: &SkipPackages) -> bool {
    // Fast path: skip files with no triple-slash reference directives.
    // Check for "///" which covers all spacing variants (///<ref, /// <ref, ///\t<ref).
    if !content.contains("///") {
        return false;
    }

    // Find the byte offset where the preamble ends.
    // TypeScript allows triple-slash directives after blank lines, single-line comments (//),
    // block comments (/* ... */), a UTF-8 BOM, and a shebang line.
    // The preamble ends at the first non-comment statement.
    let bytes = content.as_bytes();
    let mut preamble_end = 0;
    let mut in_block_comment = false;

    // Advance preamble_end past a line and its terminator (\n or \r\n).
    let advance_past_line = |offset: usize, line_len: usize| -> usize {
        let mut pos = offset + line_len;
        if pos < bytes.len() && bytes[pos] == b'\r' {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b'\n' {
            pos += 1;
        }
        pos
    };

    // Check what follows after a `*/` close, scanning past any additional `/* ... */` pairs.
    // Returns `None` if code follows (caller should break).
    // Returns `Some(true)` if an unclosed `/*` follows (enter block comment).
    // Returns `Some(false)` if the rest is empty, a `//` comment, or only closed block comments.
    let check_after_close = |text: &str| -> Option<bool> {
        let mut remaining = text.trim();
        loop {
            if remaining.is_empty() || remaining.starts_with("//") {
                return Some(false);
            }
            if !remaining.starts_with("/*") {
                return None; // Code follows — end of preamble.
            }
            // Another block comment starts — check if it closes on this line.
            match remaining[2..].find("*/") {
                Some(pos) => remaining = remaining[2 + pos + 2..].trim(),
                None => return Some(true), // Unclosed — enter block comment.
            }
        }
    };

    for line in content.lines() {
        // Strip UTF-8 BOM (U+FEFF) before trimming — Rust's trim() does not remove BOM.
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if in_block_comment {
            if let Some(pos) = trimmed.find("*/") {
                match check_after_close(&trimmed[pos + 2..]) {
                    None => break, // code after */ — end of preamble
                    Some(new_block) => in_block_comment = new_block,
                }
            }
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#!") {
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        if trimmed.starts_with("/*") {
            if let Some(pos) = trimmed.find("*/") {
                match check_after_close(&trimmed[pos + 2..]) {
                    None => break,
                    Some(new_block) => in_block_comment = new_block,
                }
            } else {
                in_block_comment = true;
            }
            preamble_end = advance_past_line(preamble_end, line.len());
            continue;
        }
        break;
    }

    // Guard: unclosed block comment means the file has a syntax error; skip rewriting.
    if in_block_comment {
        return false;
    }

    let preamble = &content[..preamble_end];
    // Check for "///" which covers all spacing variants (///<ref, /// <ref, etc.)
    if !preamble.contains("///") {
        return false;
    }

    // Detect the line ending style used in the preamble for faithful reconstruction.
    let line_ending = if preamble.contains("\r\n") { "\r\n" } else { "\n" };

    let mut changed = false;
    let mut preamble_lines: Vec<String> = preamble.lines().map(|l| l.to_string()).collect();
    // Strip UTF-8 BOM from the first preamble line so the regex `^(\s*///` can match.
    if let Some(first) = preamble_lines.first_mut() {
        if first.starts_with('\u{feff}') {
            *first = first.trim_start_matches('\u{feff}').to_string();
        }
    }

    for line in &mut preamble_lines {
        // The regexes handle flexible spacing (///\s*<reference), so just check for "///"
        // to avoid filtering out valid variants like ///<reference or ///\t<reference.
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with("///") {
            continue;
        }
        // Each line matches at most one pattern; use early exit to skip remaining regexes.
        if !skip_packages.skip_vitest {
            if apply_regex_replace(line, &RE_REF_VITEST_CONFIG, "${1}vite-plus${2}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_VITEST_SCOPED, "${1}vite-plus/test/${2}${3}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_VITEST_SUBPATH, "${1}vite-plus/test/${2}${3}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_VITEST, "${1}vite-plus/test${2}") {
                changed = true;
                continue;
            }
        }
        if !skip_packages.skip_vite {
            if apply_regex_replace(line, &RE_REF_VITE_SUBPATH, "${1}vite-plus/${2}${3}") {
                changed = true;
                continue;
            }
            if apply_regex_replace(line, &RE_REF_VITE, "${1}vite-plus${2}") {
                changed = true;
                continue;
            }
        }
        if !skip_packages.skip_tsdown
            && apply_regex_replace(line, &RE_REF_TSDOWN, "${1}vite-plus/pack${2}")
        {
            changed = true;
        }
    }

    if changed {
        let suffix = &content[preamble_end..];
        let mut result = preamble_lines.join(line_ending);
        // Re-add the line ending between preamble and suffix if the original had one
        if preamble_end < content.len() || preamble.ends_with('\n') {
            result.push_str(line_ending);
        }
        result.push_str(suffix);
        *content = result;
    }

    changed
}

/// Packages to skip rewriting based on peerDependencies or dependencies
#[derive(Debug, Clone, Default)]
struct SkipPackages {
    /// Skip rewriting vite imports (vite is in peerDependencies or dependencies)
    skip_vite: bool,
    /// Skip rewriting vitest imports (vitest is in peerDependencies or dependencies)
    skip_vitest: bool,
    /// Skip rewriting tsdown imports (tsdown is in peerDependencies or dependencies)
    skip_tsdown: bool,
}

impl SkipPackages {
    /// Check if all packages should be skipped (file can be skipped entirely)
    fn all_skipped(&self) -> bool {
        self.skip_vite && self.skip_vitest && self.skip_tsdown
    }
}

/// Find the nearest package.json by walking up from the file's directory.
/// Stops at the root directory.
fn find_nearest_package_json(file_path: &Path, root: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;

    loop {
        let package_json = current.join("package.json");
        if package_json.exists() {
            return Some(package_json);
        }

        // Stop if we've reached the root
        if current == root {
            break;
        }

        // Move to parent directory
        current = current.parent()?;
    }

    None
}

/// Parse package.json and check which packages are in peerDependencies or dependencies.
/// Returns default (no skipping) if package.json doesn't exist or can't be parsed.
fn get_skip_packages_from_package_json(package_json_path: &Path) -> SkipPackages {
    let content = match std::fs::read_to_string(package_json_path) {
        Ok(c) => c,
        Err(_) => return SkipPackages::default(),
    };

    let pkg: serde_json::Value = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(_) => return SkipPackages::default(),
    };

    // Helper to check if a package exists in a dependencies object
    let has_package = |deps_key: &str, package_name: &str| -> bool {
        pkg.get(deps_key)
            .and_then(|v| v.as_object())
            .map(|deps| deps.contains_key(package_name))
            .unwrap_or(false)
    };

    // Check both peerDependencies and dependencies
    SkipPackages {
        skip_vite: has_package("peerDependencies", "vite") || has_package("dependencies", "vite"),
        skip_vitest: has_package("peerDependencies", "vitest")
            || has_package("dependencies", "vitest"),
        skip_tsdown: has_package("peerDependencies", "tsdown")
            || has_package("dependencies", "tsdown"),
    }
}

/// Result of rewriting imports in a file
#[derive(Debug)]
struct RewriteResult {
    /// The updated file content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
}

/// Result of rewriting imports in multiple files
#[derive(Debug)]
pub struct BatchRewriteResult {
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
    /// Files that had no changes
    pub unchanged_files: Vec<PathBuf>,
    /// Files that had errors (path, error message)
    pub errors: Vec<(PathBuf, String)>,
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
/// - `modified_files`: Files that were changed
/// - `unchanged_files`: Files that required no changes
/// - `errors`: Files that had errors during processing
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vite_migration::rewrite_imports_in_directory;
///
/// let result = rewrite_imports_in_directory(Path::new("./src"))?;
/// println!("Modified {} files", result.modified_files.len());
/// for file in &result.modified_files {
///     println!("  {}", file.display());
/// }
/// ```
pub fn rewrite_imports_in_directory(root: &Path) -> Result<BatchRewriteResult, Error> {
    let walk_result = file_walker::find_ts_files(root)?;

    let mut result = BatchRewriteResult {
        modified_files: Vec::new(),
        unchanged_files: Vec::new(),
        errors: Vec::new(),
    };

    // Cache package.json lookups to avoid re-reading the same file
    let mut skip_packages_cache: HashMap<PathBuf, SkipPackages> = HashMap::new();

    for file_path in walk_result.files {
        // Find the nearest package.json for this file
        let skip_packages =
            if let Some(package_json_path) = find_nearest_package_json(&file_path, root) {
                skip_packages_cache
                    .entry(package_json_path.clone())
                    .or_insert_with(|| get_skip_packages_from_package_json(&package_json_path))
                    .clone()
            } else {
                SkipPackages::default()
            };

        // If all packages are in peerDeps for this file's package, skip it
        if skip_packages.all_skipped() {
            result.unchanged_files.push(file_path);
            continue;
        }

        match rewrite_import(&file_path, &skip_packages) {
            Ok(rewrite_result) => {
                if rewrite_result.updated {
                    // Write the modified content back
                    if let Err(e) = std::fs::write(&file_path, &rewrite_result.content) {
                        result.errors.push((file_path, e.to_string()));
                    } else {
                        result.modified_files.push(file_path);
                    }
                } else {
                    result.unchanged_files.push(file_path);
                }
            }
            Err(e) => {
                result.errors.push((file_path, e.to_string()));
            }
        }
    }

    Ok(result)
}

/// Rewrite imports in a TypeScript/JavaScript file from vite/vitest to vite-plus
///
/// This function reads a file and rewrites the import statements
/// to use 'vite-plus' instead of 'vite', 'vitest', or '@vitest/*'.
/// Packages that are in peerDependencies or dependencies will be skipped.
///
/// # Arguments
///
/// * `file_path` - Path to the TypeScript/JavaScript file
/// * `skip_packages` - Which packages to skip based on peerDependencies or dependencies
///
/// # Returns
///
/// Returns a `RewriteResult` containing:
/// - `content`: The updated file content
/// - `updated`: Whether any changes were made
fn rewrite_import(file_path: &Path, skip_packages: &SkipPackages) -> Result<RewriteResult, Error> {
    // Read the file
    let content = std::fs::read_to_string(file_path)?;

    // Rewrite the imports
    rewrite_import_content(&content, skip_packages)
}

/// Rewrite imports in content from vite/vitest to vite-plus
///
/// This is the internal function that performs the actual rewrite using ast-grep.
/// Packages that are in peerDependencies or dependencies will be skipped.
fn rewrite_import_content(
    content: &str,
    skip_packages: &SkipPackages,
) -> Result<RewriteResult, Error> {
    let mut new_content = content.to_string();
    let mut updated = false;

    // Apply vite rules if not skipped
    if !skip_packages.skip_vite {
        let (vite_content, vite_updated) = ast_grep::apply_rules(&new_content, REWRITE_VITE_RULES)?;
        if vite_updated {
            new_content = vite_content;
            updated = true;
        }
    }

    // Apply vitest rules if not skipped
    if !skip_packages.skip_vitest {
        let (vitest_content, vitest_updated) =
            ast_grep::apply_rules(&new_content, REWRITE_VITEST_RULES)?;
        if vitest_updated {
            new_content = vitest_content;
            updated = true;
        }
    }

    // Apply tsdown rules if not skipped
    if !skip_packages.skip_tsdown {
        let (tsdown_content, tsdown_updated) =
            ast_grep::apply_rules(&new_content, REWRITE_TSDOWN_RULES)?;
        if tsdown_updated {
            new_content = tsdown_content;
            updated = true;
        }
    }

    // Apply reference type rewriting (/// <reference types="..." />)
    // These cannot be handled by ast-grep because they are parsed as comments.
    updated |= rewrite_reference_types(&mut new_content, skip_packages);

    Ok(RewriteResult { content: new_content, updated })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_rewrite_import_content_vite() {
        let vite_config = r#"import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus'

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vite_double_quotes() {
        let vite_config = r#"import { defineConfig } from "vite";

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from "vite-plus";

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_config() {
        let vite_config = r#"import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
  },
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    globals: true,
  },
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_multiple_imports() {
        let vite_config = r#"import { defineConfig, loadEnv, type UserWorkspaceConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig, loadEnv, type UserWorkspaceConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_already_vite_plus() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_rewrite_import_with_file() {
        // Create temporary directory (automatically cleaned up when dropped)
        let temp_dir = tempdir().unwrap();

        let vite_config_path = temp_dir.path().join("vite.config.ts");

        // Write test vite config
        let mut vite_file = std::fs::File::create(&vite_config_path).unwrap();
        write!(
            vite_file,
            r#"import {{ defineConfig }} from 'vite';

export default defineConfig({{
  plugins: [],
}});"#
        )
        .unwrap();

        // Run the rewrite
        let result = rewrite_import(&vite_config_path, &SkipPackages::default()).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest() {
        let vite_config = r#"import { describe, it, expect } from 'vitest';

describe('test', () => {
  it('should work', () => {
    expect(true).toBe(true);
  });
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { describe, it, expect } from 'vite-plus/test';

describe('test', () => {
  it('should work', () => {
    expect(true).toBe(true);
  });
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_double_quotes() {
        let vite_config = r#"import { describe, it, expect } from "vitest";

describe('test', () => {});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { describe, it, expect } from "vite-plus/test";

describe('test', () => {});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser() {
        let vite_config = r#"import { page } from '@vitest/browser';

export default page;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { page } from 'vite-plus/test/browser';

export default page;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_double_quotes() {
        let vite_config = r#"import { page } from "@vitest/browser";

export default page;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { page } from "vite-plus/test/browser";

export default page;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright() {
        let vite_config = r#"import { playwright } from '@vitest/browser-playwright';

export default playwright;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { playwright } from 'vite-plus/test/browser-playwright';

export default playwright;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright_double_quotes() {
        let vite_config = r#"import { playwright } from "@vitest/browser-playwright";

export default playwright;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { playwright } from "vite-plus/test/browser-playwright";

export default playwright;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_subpath() {
        let vite_config = r#"import { context } from '@vitest/browser/context';

export default context;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { context } from 'vite-plus/test/browser/context';

export default context;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_playwright_subpath() {
        let vite_config = r#"import { something } from "@vitest/browser-playwright/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser-playwright/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_preview() {
        let vite_config = r#"import { preview } from '@vitest/browser-preview';

export default preview;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { preview } from 'vite-plus/test/browser-preview';

export default preview;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_preview_subpath() {
        let vite_config = r#"import { something } from "@vitest/browser-preview/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser-preview/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_webdriverio() {
        let vite_config = r#"import { webdriverio } from '@vitest/browser-webdriverio';

export default webdriverio;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { webdriverio } from 'vite-plus/test/browser-webdriverio';

export default webdriverio;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_browser_webdriverio_subpath() {
        let vite_config = r#"import { something } from "@vitest/browser-webdriverio/context";

export default something;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { something } from "vite-plus/test/browser-webdriverio/context";

export default something;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vite_subpath() {
        let vite_config = r#"import { ModuleRunner } from 'vite/module-runner';

export default ModuleRunner;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { ModuleRunner } from 'vite-plus/module-runner';

export default ModuleRunner;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vite_subpath_double_quotes() {
        let vite_config = r#"import { ModuleRunner } from "vite/module-runner";

export default ModuleRunner;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { ModuleRunner } from "vite-plus/module-runner";

export default ModuleRunner;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_subpath() {
        // Test vitest/node subpath
        let vite_config = r#"import { startVitest } from 'vitest/node';

export default startVitest;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { startVitest } from 'vite-plus/test/node';

export default startVitest;"#
        );

        // Test vitest/plugins/runner subpath
        let vite_config = r#"import { somePlugin } from 'vitest/plugins/runner';

export default somePlugin;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { somePlugin } from 'vite-plus/test/plugins/runner';

export default somePlugin;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_vitest_subpath_double_quotes() {
        let vite_config = r#"import { startVitest } from "vitest/node";

export default startVitest;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { startVitest } from "vite-plus/test/node";

export default startVitest;"#
        );
    }

    #[test]
    fn test_rewrite_import_content_mixed_imports() {
        // Test multiple different imports in the same file
        let vite_config = r#"import { defineConfig } from 'vite';
import { ModuleRunner } from 'vite/module-runner';
import { describe, it, expect } from 'vitest';
import { startVitest } from 'vitest/node';
import { page } from '@vitest/browser';
import { playwright } from '@vitest/browser-playwright';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { ModuleRunner } from 'vite-plus/module-runner';
import { describe, it, expect } from 'vite-plus/test';
import { startVitest } from 'vite-plus/test/node';
import { page } from 'vite-plus/test/browser';
import { playwright } from 'vite-plus/test/browser-playwright';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_rewrite_imports_in_directory() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create test files with vite/vitest imports
        fs::write(
            temp.path().join("src/config.ts"),
            r#"import { defineConfig } from 'vite';
export default defineConfig({});"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/test.ts"),
            r#"import { describe, it } from 'vitest';
describe('test', () => {});"#,
        )
        .unwrap();

        // Create a file without vite imports (should be unchanged)
        fs::write(
            temp.path().join("src/utils.ts"),
            r#"export function add(a: number, b: number) {
  return a + b;
}"#,
        )
        .unwrap();

        // Create node_modules (should be ignored)
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::write(
            temp.path().join("node_modules/pkg.ts"),
            r#"import { defineConfig } from 'vite';"#,
        )
        .unwrap();

        // Create .gitignore
        fs::write(temp.path().join(".gitignore"), "node_modules/").unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // Should have 2 modified files (config.ts and test.ts)
        assert_eq!(result.modified_files.len(), 2);
        // Should have 1 unchanged file (utils.ts)
        assert_eq!(result.unchanged_files.len(), 1);
        // Should have no errors
        assert!(result.errors.is_empty());

        // Verify the files were actually modified
        let config_content = fs::read_to_string(temp.path().join("src/config.ts")).unwrap();
        assert!(config_content.contains("vite-plus"));

        let test_content = fs::read_to_string(temp.path().join("src/test.ts")).unwrap();
        assert!(test_content.contains("vite-plus/test"));

        // Verify utils.ts was not modified
        let utils_content = fs::read_to_string(temp.path().join("src/utils.ts")).unwrap();
        assert!(!utils_content.contains("vite-plus"));
    }

    #[test]
    fn test_rewrite_imports_in_directory_empty() {
        let temp = tempdir().unwrap();

        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        assert!(result.modified_files.is_empty());
        assert!(result.unchanged_files.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_rewrite_imports_in_directory_nested() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create nested directory structure
        fs::create_dir_all(temp.path().join("src/components/Button")).unwrap();
        fs::create_dir_all(temp.path().join("tests/unit")).unwrap();

        // Create files at various depths
        fs::write(
            temp.path().join("vite.config.ts"),
            r#"import { defineConfig } from 'vite';
export default defineConfig({});"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/index.ts"),
            r#"import { createServer } from 'vite';
export { createServer };"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("src/components/Button/Button.tsx"),
            r#"import React from 'react';
export const Button = () => <button>Click</button>;"#,
        )
        .unwrap();

        fs::write(
            temp.path().join("tests/unit/app.test.ts"),
            r#"import { describe, it, expect } from 'vitest';
import { page } from '@vitest/browser';

describe('app', () => {
  it('works', () => {
    expect(true).toBe(true);
  });
});"#,
        )
        .unwrap();

        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // vite.config.ts, src/index.ts, tests/unit/app.test.ts should be modified
        assert_eq!(result.modified_files.len(), 3);
        // Button.tsx has no vite imports
        assert_eq!(result.unchanged_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify nested file was modified
        let test_content = fs::read_to_string(temp.path().join("tests/unit/app.test.ts")).unwrap();
        assert!(test_content.contains("vite-plus/test"));
        assert!(test_content.contains("vite-plus/test/browser"));
    }

    #[test]
    fn test_rewrite_declare_module_vite() {
        let content = r#"declare module 'vite' {
  interface UserConfig {
    runtimeEnv?: RuntimeEnvConfig;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    runtimeEnv?: RuntimeEnvConfig;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vite_double_quotes() {
        let content = r#"declare module "vite" {
  interface UserConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus" {
  interface UserConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest() {
        let content = r#"declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_config() {
        let content = r#"declare module 'vitest/config' {
  interface UserConfig {
    test?: TestConfig;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    test?: TestConfig;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vite_subpath() {
        let content = r#"declare module 'vite/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_subpath() {
        let content = r#"declare module 'vitest/node' {
  export interface VitestOptions {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/node' {
  export interface VitestOptions {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser() {
        let content = r#"declare module '@vitest/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_subpath() {
        let content = r#"declare module '@vitest/browser/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser/context' {
  export interface Context {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_playwright() {
        let content = r#"declare module '@vitest/browser-playwright' {
  interface PlaywrightContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-playwright' {
  interface PlaywrightContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_preview() {
        let content = r#"declare module '@vitest/browser-preview' {
  interface PreviewContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-preview' {
  interface PreviewContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_webdriverio() {
        let content = r#"declare module '@vitest/browser-webdriverio' {
  interface WebDriverContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-webdriverio' {
  interface WebDriverContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_mixed_imports_and_declare_modules() {
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

declare module 'vite' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

export default defineConfig({});"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vite-plus/test';

declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vite-plus/test' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_already_vite_plus() {
        let content = r#"declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_multiple_declare_modules() {
        let content = r#"declare module 'vite' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vite/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}

declare module 'vitest' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

declare module '@vitest/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module 'vite-plus/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}

declare module 'vite-plus/test' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

declare module 'vite-plus/test/browser' {
  interface BrowserContext {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_double_quotes() {
        let content = r#"declare module "vitest" {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus/test" {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_playwright_subpath() {
        let content = r#"declare module '@vitest/browser-playwright/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-playwright/context' {
  export interface Context {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_preview_subpath() {
        let content = r#"declare module '@vitest/browser-preview/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-preview/context' {
  export interface Context {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_vitest_browser_webdriverio_subpath() {
        let content = r#"declare module '@vitest/browser-webdriverio/context' {
  export interface Context {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/test/browser-webdriverio/context' {
  export interface Context {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_complex_interface() {
        let content = r#"declare module 'vite' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }

  interface Plugin {
    name: string;
    configResolved?: (config: ResolvedConfig) => void;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus' {
  interface UserConfig {
    /**
     * Options for vite-plugin-runtime-env
     */
    runtimeEnv?: RuntimeEnvConfig;
    /**
     * Options for vite-plugin-runtime-html
     */
    runtimeHtml?: RuntimeHtmlConfig;
  }

  interface Plugin {
    name: string;
    configResolved?: (config: ResolvedConfig) => void;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_import_content_tsdown() {
        let tsdown_config = r#"import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: 'src/index.ts',
});"#;

        let result = rewrite_import_content(tsdown_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: 'src/index.ts',
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_tsdown_double_quotes() {
        let tsdown_config = r#"import { defineConfig } from "tsdown";

export default defineConfig({
  entry: "src/index.ts",
});"#;

        let result = rewrite_import_content(tsdown_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from "vite-plus/pack";

export default defineConfig({
  entry: "src/index.ts",
});"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_tsdown() {
        let content = r#"declare module 'tsdown' {
  interface BuildConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module 'vite-plus/pack' {
  interface BuildConfig {
    custom?: boolean;
  }
}"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_tsdown_double_quotes() {
        let content = r#"declare module "tsdown" {
  interface BuildConfig {
    custom?: boolean;
  }
}"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"declare module "vite-plus/pack" {
  interface BuildConfig {
    custom?: boolean;
  }
}"#
        );
    }

    // ========================
    // PeerDependencies Tests
    // ========================

    #[test]
    fn test_skip_vite_when_peer_dependency() {
        // When vite is a peerDependency, vite imports should NOT be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;

        let skip_packages =
            SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: false };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        // vite import should NOT be rewritten, vitest import SHOULD be rewritten
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_skip_vitest_when_peer_dependency() {
        // When vitest is a peerDependency, vitest imports should NOT be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: true, skip_tsdown: false };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        // vite import SHOULD be rewritten, vitest import should NOT be rewritten
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vitest';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_skip_all_when_all_peer_dependencies() {
        // When all packages are peerDependencies, nothing should be rewritten
        let content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
import { build } from 'tsdown';

export default defineConfig({});"#;

        let skip_packages = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };

        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_skip_packages_all_skipped() {
        let skip_all = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };
        assert!(skip_all.all_skipped());

        let skip_some = SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: true };
        assert!(!skip_some.all_skipped());

        let skip_none = SkipPackages::default();
        assert!(!skip_none.all_skipped());
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_vite_peer_dep() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as peerDependency
        let pkg_json = r#"{
  "name": "my-vite-plugin",
  "peerDependencies": {
    "vite": "^5.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_all_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        let pkg_json = r#"{
  "name": "my-plugin",
  "peerDependencies": {
    "vite": "^5.0.0",
    "vitest": "^1.0.0",
    "tsdown": "^1.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite);
        assert!(skip.skip_vitest);
        assert!(skip.skip_tsdown);
        assert!(skip.all_skipped());
    }

    #[test]
    fn test_get_skip_packages_from_package_json_with_vite_dependency() {
        use std::fs;

        let temp = tempdir().unwrap();

        // vite in dependencies should also skip rewriting
        let pkg_json = r#"{
  "name": "my-app",
  "dependencies": {
    "vite": "^5.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite); // NOW skips because vite is in dependencies
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_no_file() {
        let temp = tempdir().unwrap();

        // No package.json created - should return default (no skipping)
        let package_json_path = temp.path().join("package.json");
        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_from_package_json_no_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Package with no dependencies at all
        let pkg_json = r#"{
  "name": "my-app"
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(!skip.skip_vite);
        assert!(!skip.skip_vitest);
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_get_skip_packages_mixed_peer_and_regular_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // vite in dependencies, vitest in peerDependencies
        let pkg_json = r#"{
  "name": "my-package",
  "dependencies": {
    "vite": "^5.0.0"
  },
  "peerDependencies": {
    "vitest": "^1.0.0"
  }
}"#;
        let package_json_path = temp.path().join("package.json");
        fs::write(&package_json_path, pkg_json).unwrap();

        let skip = get_skip_packages_from_package_json(&package_json_path);
        assert!(skip.skip_vite); // in dependencies
        assert!(skip.skip_vitest); // in peerDependencies
        assert!(!skip.skip_tsdown);
    }

    #[test]
    fn test_rewrite_imports_in_directory_with_vite_dependency() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as dependency (not peerDependency)
        let pkg_json = r#"{
  "name": "my-app",
  "dependencies": {
    "vite": "^5.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create source file with vite and vitest imports
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;
        fs::write(temp.path().join("src/index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be modified (vitest was rewritten)
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify vite import NOT rewritten (in dependencies), vitest IS rewritten
        let content = fs::read_to_string(temp.path().join("src/index.ts")).unwrap();
        assert_eq!(
            content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_imports_in_directory_with_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with vite as peerDependency
        let pkg_json = r#"{
  "name": "my-vite-plugin",
  "peerDependencies": {
    "vite": "^5.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create src directory
        fs::create_dir(temp.path().join("src")).unwrap();

        // Create source file with vite and vitest imports
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';

export default defineConfig({});"#;
        fs::write(temp.path().join("src/index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be modified (vitest was rewritten)
        assert_eq!(result.modified_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify vite import NOT rewritten, vitest IS rewritten
        let content = fs::read_to_string(temp.path().join("src/index.ts")).unwrap();
        assert_eq!(
            content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_imports_skips_file_when_all_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create package.json with all packages as peerDependencies
        let pkg_json = r#"{
  "name": "my-plugin",
  "peerDependencies": {
    "vite": "^5.0.0",
    "vitest": "^1.0.0",
    "tsdown": "^1.0.0"
  }
}"#;
        fs::write(temp.path().join("package.json"), pkg_json).unwrap();

        // Create source file
        let original_content = r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
import { build } from 'tsdown';"#;
        fs::write(temp.path().join("index.ts"), original_content).unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // File should be unchanged (all skipped)
        assert!(result.modified_files.is_empty());
        assert_eq!(result.unchanged_files.len(), 1);

        // Verify content unchanged
        let content = fs::read_to_string(temp.path().join("index.ts")).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_find_nearest_package_json() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create monorepo structure
        fs::create_dir_all(temp.path().join("packages/vite-plugin/src")).unwrap();
        fs::create_dir_all(temp.path().join("packages/app/src")).unwrap();

        // Root package.json (no peerDeps)
        fs::write(temp.path().join("package.json"), r#"{"name": "monorepo"}"#).unwrap();

        // vite-plugin package.json (has vite in peerDeps)
        fs::write(
            temp.path().join("packages/vite-plugin/package.json"),
            r#"{"name": "vite-plugin", "peerDependencies": {"vite": "^5.0.0"}}"#,
        )
        .unwrap();

        // app package.json (no peerDeps)
        fs::write(temp.path().join("packages/app/package.json"), r#"{"name": "app"}"#).unwrap();

        // Test finding package.json from vite-plugin/src/index.ts
        let file_path = temp.path().join("packages/vite-plugin/src/index.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("packages/vite-plugin/package.json")));

        // Test finding package.json from app/src/index.ts
        let file_path = temp.path().join("packages/app/src/index.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("packages/app/package.json")));

        // Test finding package.json from root level file
        let file_path = temp.path().join("vite.config.ts");
        let result = find_nearest_package_json(&file_path, temp.path());
        assert_eq!(result, Some(temp.path().join("package.json")));
    }

    #[test]
    fn test_rewrite_imports_monorepo_different_peer_deps() {
        use std::fs;

        let temp = tempdir().unwrap();

        // Create monorepo structure
        fs::create_dir_all(temp.path().join("packages/vite-plugin/src")).unwrap();
        fs::create_dir_all(temp.path().join("packages/app/src")).unwrap();

        // Root package.json (no peerDeps)
        fs::write(temp.path().join("package.json"), r#"{"name": "monorepo"}"#).unwrap();

        // vite-plugin package.json (has vite in peerDeps)
        fs::write(
            temp.path().join("packages/vite-plugin/package.json"),
            r#"{"name": "vite-plugin", "peerDependencies": {"vite": "^5.0.0"}}"#,
        )
        .unwrap();

        // app package.json (no peerDeps)
        fs::write(temp.path().join("packages/app/package.json"), r#"{"name": "app"}"#).unwrap();

        // vite-plugin source file with vite and vitest imports
        fs::write(
            temp.path().join("packages/vite-plugin/src/index.ts"),
            r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
export default defineConfig({});"#,
        )
        .unwrap();

        // app source file with vite and vitest imports
        fs::write(
            temp.path().join("packages/app/src/index.ts"),
            r#"import { defineConfig } from 'vite';
import { describe } from 'vitest';
export default defineConfig({});"#,
        )
        .unwrap();

        // Run the batch rewrite
        let result = rewrite_imports_in_directory(temp.path()).unwrap();

        // Both files should be modified
        assert_eq!(result.modified_files.len(), 2);

        // vite-plugin: vite NOT rewritten (has peerDep), vitest IS rewritten
        let vite_plugin_content =
            fs::read_to_string(temp.path().join("packages/vite-plugin/src/index.ts")).unwrap();
        assert_eq!(
            vite_plugin_content,
            r#"import { defineConfig } from 'vite';
import { describe } from 'vite-plus/test';
export default defineConfig({});"#
        );

        // app: vite IS rewritten (no peerDep), vitest IS rewritten
        let app_content =
            fs::read_to_string(temp.path().join("packages/app/src/index.ts")).unwrap();
        assert_eq!(
            app_content,
            r#"import { defineConfig } from 'vite-plus';
import { describe } from 'vite-plus/test';
export default defineConfig({});"#
        );
    }

    // ====================================
    // Reference Types Rewriting Tests
    // ====================================

    #[test]
    fn test_rewrite_reference_types_vite_client() {
        let content = r#"/// <reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vite_client_single_quotes() {
        let content = r#"/// <reference types='vite/client' />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types='vite-plus/client' />"#);
    }

    #[test]
    fn test_rewrite_reference_types_bare_vite() {
        let content = r#"/// <reference types="vite" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_bare_vitest() {
        let content = r#"/// <reference types="vitest" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_globals() {
        let content = r#"/// <reference types="vitest/globals" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/globals" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_config() {
        let content = r#"/// <reference types="vitest/config" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_browser() {
        let content = r#"/// <reference types="vitest/browser" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/browser" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_matchers_not_rewritten() {
        // @vitest/browser/matchers is NOT exported by vite-plus — should not be rewritten
        let content = r#"/// <reference types="@vitest/browser/matchers" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_context() {
        let content = r#"/// <reference types="@vitest/browser/context" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/test/browser/context" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_playwright() {
        let content = r#"/// <reference types="@vitest/browser/providers/playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_playwright_pkg() {
        let content = r#"/// <reference types="@vitest/browser-playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser-playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_browser_webdriverio() {
        let content = r#"/// <reference types="@vitest/browser/providers/webdriverio" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/webdriverio" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_tsdown_subpath_not_rewritten() {
        // tsdown subpaths should NOT be rewritten because vite-plus only exports ./pack (no subpaths)
        let content = r#"/// <reference types="tsdown/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_vitest_scoped_not_matching() {
        // Non-enumerated @vitest/* packages should NOT be rewritten
        let content = r#"/// <reference types="@vitest/coverage-v8" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_inside_template_literal_not_rewritten() {
        // Reference-like content inside template literals should NOT be rewritten.
        // The preamble ends at the first non-comment line (`const`), so nothing is processed.
        let content = r#"const template = `
/// <reference types="vite/client" />
`;"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_preamble_only() {
        // Only references in the preamble (before first statement) should be rewritten
        let content = r#"/// <reference types="vite/client" />
// A regular comment

const x = 1;
/// <reference types="vitest" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        // First reference (preamble) is rewritten; last one (after code) is not
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
// A regular comment

const x = 1;
/// <reference types="vitest" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_block_comment() {
        // Block comments (/* ... */) should not end the preamble scan
        let content = "/* License: MIT */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* License: MIT */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_multiline_block_comment() {
        // Multi-line block comments should be skipped entirely
        let content =
            "/*\n * License header\n * Copyright 2024\n */\n/// <reference types=\"vitest\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/*\n * License header\n * Copyright 2024\n */\n/// <reference types=\"vite-plus/test\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_block_comment_with_trailing_code() {
        // A single-line block comment followed by code should end the preamble
        let content = "/* banner */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_block_comment_with_trailing_comment() {
        // A block comment followed by a line comment is still preamble
        let content = "/* banner */ // generated\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* banner */ // generated\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_multiline_block_comment_closes_into_code() {
        // Multi-line block comment closing line has code after */ — end of preamble
        let content = "/*\n * License\n */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_multiline_block_comment_closes_into_comment() {
        // Multi-line block comment closing line has only a comment after */ — still preamble
        let content = "/*\n * License\n */ // end\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/*\n * License\n */ // end\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_block_close_into_new_block_comment() {
        // `/* a */ /* b` closes first comment then opens a new multi-line one
        let content = "/* a */ /* b\n * still going */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* a */ /* b\n * still going */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_multiple_inline_block_comments_then_code() {
        // `/* a */ /* b */ const x = 1;` — code after two closed block comments ends preamble
        let content = "/* a */ /* b */ const x = 1;\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_multiple_inline_block_comments_no_code() {
        // `/* a */ /* b */` — only block comments, no trailing code, preamble continues
        let content = "/* a */ /* b */\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* a */ /* b */\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_vitest_browser_providers_playwright() {
        // @vitest/browser/providers/playwright → vite-plus/test/browser/providers/playwright
        let content = r#"/// <reference types="@vitest/browser/providers/playwright" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/test/browser/providers/playwright" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_crlf() {
        // CRLF line endings should be preserved
        let content =
            "/// <reference types=\"vite/client\" />\r\n/// <reference types=\"vitest\" />\r\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/// <reference types=\"vite-plus/client\" />\r\n/// <reference types=\"vite-plus/test\" />\r\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_crlf_with_block_comment() {
        // CRLF + block comment header
        let content =
            "/* License */\r\n/// <reference types=\"vite/client\" />\r\nconst x = 1;\r\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "/* License */\r\n/// <reference types=\"vite-plus/client\" />\r\nconst x = 1;\r\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_no_space_after_slashes() {
        // TypeScript accepts ///<reference without a space
        let content = r#"///<reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"///<reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_tab_after_slashes() {
        // TypeScript accepts ///\t<reference with a tab
        let content = "///\t<reference types=\"vite/client\" />";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, "///\t<reference types=\"vite-plus/client\" />");
    }

    #[test]
    fn test_rewrite_reference_types_after_shebang() {
        // Shebang lines should not end the preamble scan
        let content = "#!/usr/bin/env node\n/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "#!/usr/bin/env node\n/// <reference types=\"vite-plus/client\" />\n"
        );
    }

    #[test]
    fn test_rewrite_reference_types_after_bom() {
        // UTF-8 BOM should not end the preamble scan; BOM is stripped during rewrite
        let content = "\u{feff}/// <reference types=\"vite/client\" />\n";
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, "/// <reference types=\"vite-plus/client\" />\n");
    }

    #[test]
    fn test_rewrite_reference_types_bare_tsdown() {
        let content = r#"/// <reference types="tsdown" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"/// <reference types="vite-plus/pack" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_already_migrated() {
        let content = r#"/// <reference types="vite-plus/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_preserves_non_matching() {
        let content = r#"/// <reference types="node" />
/// <reference lib="es2015" />
/// <reference path="./types.d.ts" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }

    #[test]
    fn test_rewrite_reference_types_with_leading_whitespace() {
        let content = r#"  /// <reference types="vite/client" />"#;
        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(result.content, r#"  /// <reference types="vite-plus/client" />"#);
    }

    #[test]
    fn test_rewrite_reference_types_env_d_ts_style() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="vitest/globals" />
/// <reference types="vitest/config" />
/// <reference types="@vitest/browser/context" />"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
/// <reference types="vite-plus/test" />
/// <reference types="vite-plus/test/globals" />
/// <reference types="vite-plus" />
/// <reference types="vite-plus/test/browser/context" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_mixed_with_imports() {
        let content = r#"/// <reference types="vite/client" />
import { defineConfig } from 'vite';

export default defineConfig({});"#;

        let result = rewrite_import_content(content, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
import { defineConfig } from 'vite-plus';

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_vite() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />"#;

        let skip_packages =
            SkipPackages { skip_vite: true, skip_vitest: false, skip_tsdown: false };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite/client" />
/// <reference types="vite-plus/test" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_vitest() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="@vitest/browser/matchers" />"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: true, skip_tsdown: false };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="vite-plus/client" />
/// <reference types="vitest" />
/// <reference types="@vitest/browser/matchers" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_tsdown() {
        let content = r#"/// <reference types="tsdown/client" />
/// <reference types="vite/client" />"#;

        let skip_packages =
            SkipPackages { skip_vite: false, skip_vitest: false, skip_tsdown: true };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"/// <reference types="tsdown/client" />
/// <reference types="vite-plus/client" />"#
        );
    }

    #[test]
    fn test_rewrite_reference_types_skip_all() {
        let content = r#"/// <reference types="vite/client" />
/// <reference types="vitest" />
/// <reference types="tsdown/client" />"#;

        let skip_packages = SkipPackages { skip_vite: true, skip_vitest: true, skip_tsdown: true };
        let result = rewrite_import_content(content, &skip_packages).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, content);
    }
}
