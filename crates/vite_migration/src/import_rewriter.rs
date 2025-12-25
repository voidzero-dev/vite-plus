use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use vite_error::Error;

use crate::{ast_grep, file_walker};

/// ast-grep rules for rewriting vite imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'vite'` → `import { ... } from '@voidzero-dev/vite-plus'`
/// - `import { ... } from 'vite/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/{name}'`
/// - `declare module 'vite' { ... }` → `declare module '@voidzero-dev/vite-plus' { ... }`
/// - `declare module 'vite/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/{name}' { ... }`
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
      by: "@voidzero-dev/vite-plus"
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
      by: "@voidzero-dev/vite-plus/"
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
      by: "@voidzero-dev/vite-plus"
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
      by: "@voidzero-dev/vite-plus/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting vitest imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'vitest'` → `import { ... } from '@voidzero-dev/vite-plus/test'`
/// - `import { ... } from 'vitest/config'` → `import { ... } from '@voidzero-dev/vite-plus'`
/// - `import { ... } from 'vitest/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/test/{name}'`
/// - `import { ... } from '@vitest/browser'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser'`
/// - `import { ... } from '@vitest/browser/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser/{name}'`
/// - `import { ... } from '@vitest/browser-playwright'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-playwright'`
/// - `import { ... } from '@vitest/browser-playwright/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-playwright/{name}'`
/// - `import { ... } from '@vitest/browser-preview'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-preview'`
/// - `import { ... } from '@vitest/browser-preview/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-preview/{name}'`
/// - `import { ... } from '@vitest/browser-webdriverio'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-webdriverio'`
/// - `import { ... } from '@vitest/browser-webdriverio/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/test/browser-webdriverio/{name}'`
/// - `declare module 'vitest' { ... }` → `declare module '@voidzero-dev/vite-plus/test' { ... }`
/// - `declare module 'vitest/config' { ... }` → `declare module '@voidzero-dev/vite-plus' { ... }`
/// - `declare module 'vitest/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/test/{name}' { ... }`
/// - `declare module '@vitest/browser' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser' { ... }`
/// - `declare module '@vitest/browser/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser/{name}' { ... }`
/// - `declare module '@vitest/browser-playwright' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-playwright' { ... }`
/// - `declare module '@vitest/browser-playwright/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-playwright/{name}' { ... }`
/// - `declare module '@vitest/browser-preview' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-preview' { ... }`
/// - `declare module '@vitest/browser-preview/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-preview/{name}' { ... }`
/// - `declare module '@vitest/browser-webdriverio' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-webdriverio' { ... }`
/// - `declare module '@vitest/browser-webdriverio/{name}' { ... }` → `declare module '@voidzero-dev/vite-plus/test/browser-webdriverio/{name}' { ... }`
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
      by: "@voidzero-dev/vite-plus"
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
      by: "@voidzero-dev/vite-plus/test"
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
      by: "@voidzero-dev/vite-plus/test/"
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
      by: "@voidzero-dev/vite-plus/test/"
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
      by: "@voidzero-dev/vite-plus"
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
      by: "@voidzero-dev/vite-plus/test"
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
      by: "@voidzero-dev/vite-plus/test/"
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
      by: "@voidzero-dev/vite-plus/test/"
fix: $NEW_IMPORT
"#;

/// ast-grep rules for rewriting tsdown imports and declare module statements
///
/// This rewrites:
/// - `import { ... } from 'tsdown'` → `import { ... } from '@voidzero-dev/vite-plus/lib'`
/// - `declare module 'tsdown' { ... }` → `declare module '@voidzero-dev/vite-plus/lib' { ... }`
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
      by: "@voidzero-dev/vite-plus/lib"
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
      by: "@voidzero-dev/vite-plus/lib"
fix: $NEW_IMPORT
"#;

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

/// Rewrite imports in a TypeScript/JavaScript file from vite/vitest to @voidzero-dev/vite-plus
///
/// This function reads a file and rewrites the import statements
/// to use '@voidzero-dev/vite-plus' instead of 'vite', 'vitest', or '@vitest/*'.
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

/// Rewrite imports in content from vite/vitest to @voidzero-dev/vite-plus
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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus'

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
            r#"import { defineConfig } from "@voidzero-dev/vite-plus";

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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';

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
            r#"import { defineConfig, loadEnv, type UserWorkspaceConfig } from '@voidzero-dev/vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_rewrite_import_content_already_vite_plus() {
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';

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
            r#"import { describe, it, expect } from '@voidzero-dev/vite-plus/test';

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
            r#"import { describe, it, expect } from "@voidzero-dev/vite-plus/test";

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
            r#"import { page } from '@voidzero-dev/vite-plus/test/browser';

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
            r#"import { page } from "@voidzero-dev/vite-plus/test/browser";

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
            r#"import { playwright } from '@voidzero-dev/vite-plus/test/browser-playwright';

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
            r#"import { playwright } from "@voidzero-dev/vite-plus/test/browser-playwright";

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
            r#"import { context } from '@voidzero-dev/vite-plus/test/browser/context';

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
            r#"import { something } from "@voidzero-dev/vite-plus/test/browser-playwright/context";

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
            r#"import { preview } from '@voidzero-dev/vite-plus/test/browser-preview';

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
            r#"import { something } from "@voidzero-dev/vite-plus/test/browser-preview/context";

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
            r#"import { webdriverio } from '@voidzero-dev/vite-plus/test/browser-webdriverio';

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
            r#"import { something } from "@voidzero-dev/vite-plus/test/browser-webdriverio/context";

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
            r#"import { ModuleRunner } from '@voidzero-dev/vite-plus/module-runner';

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
            r#"import { ModuleRunner } from "@voidzero-dev/vite-plus/module-runner";

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
            r#"import { startVitest } from '@voidzero-dev/vite-plus/test/node';

export default startVitest;"#
        );

        // Test vitest/plugins/runner subpath
        let vite_config = r#"import { somePlugin } from 'vitest/plugins/runner';

export default somePlugin;"#;

        let result = rewrite_import_content(vite_config, &SkipPackages::default()).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { somePlugin } from '@voidzero-dev/vite-plus/test/plugins/runner';

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
            r#"import { startVitest } from "@voidzero-dev/vite-plus/test/node";

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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';
import { ModuleRunner } from '@voidzero-dev/vite-plus/module-runner';
import { describe, it, expect } from '@voidzero-dev/vite-plus/test';
import { startVitest } from '@voidzero-dev/vite-plus/test/node';
import { page } from '@voidzero-dev/vite-plus/test/browser';
import { playwright } from '@voidzero-dev/vite-plus/test/browser-playwright';
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
        assert!(config_content.contains("@voidzero-dev/vite-plus"));

        let test_content = fs::read_to_string(temp.path().join("src/test.ts")).unwrap();
        assert!(test_content.contains("@voidzero-dev/vite-plus/test"));

        // Verify utils.ts was not modified
        let utils_content = fs::read_to_string(temp.path().join("src/utils.ts")).unwrap();
        assert!(!utils_content.contains("@voidzero-dev/vite-plus"));
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
        assert!(test_content.contains("@voidzero-dev/vite-plus/test"));
        assert!(test_content.contains("@voidzero-dev/vite-plus/test/browser"));
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
            r#"declare module '@voidzero-dev/vite-plus' {
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
            r#"declare module "@voidzero-dev/vite-plus" {
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
            r#"declare module '@voidzero-dev/vite-plus/test' {
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
            r#"declare module '@voidzero-dev/vite-plus' {
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
            r#"declare module '@voidzero-dev/vite-plus/module-runner' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/node' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser/context' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-playwright' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-preview' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-webdriverio' {
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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';
import { describe } from '@voidzero-dev/vite-plus/test';

declare module '@voidzero-dev/vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module '@voidzero-dev/vite-plus/test' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

export default defineConfig({});"#
        );
    }

    #[test]
    fn test_rewrite_declare_module_already_vite_plus() {
        let content = r#"declare module '@voidzero-dev/vite-plus' {
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
            r#"declare module '@voidzero-dev/vite-plus' {
  interface UserConfig {
    custom?: boolean;
  }
}

declare module '@voidzero-dev/vite-plus/module-runner' {
  export interface ModuleRunnerOptions {
    custom?: boolean;
  }
}

declare module '@voidzero-dev/vite-plus/test' {
  interface JestAssertion<T = any> {
    toBeCustom(): void;
  }
}

declare module '@voidzero-dev/vite-plus/test/browser' {
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
            r#"declare module "@voidzero-dev/vite-plus/test" {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-playwright/context' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-preview/context' {
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
            r#"declare module '@voidzero-dev/vite-plus/test/browser-webdriverio/context' {
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
            r#"declare module '@voidzero-dev/vite-plus' {
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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus/lib';

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
            r#"import { defineConfig } from "@voidzero-dev/vite-plus/lib";

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
            r#"declare module '@voidzero-dev/vite-plus/lib' {
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
            r#"declare module "@voidzero-dev/vite-plus/lib" {
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
import { describe } from '@voidzero-dev/vite-plus/test';

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
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';
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
import { describe } from '@voidzero-dev/vite-plus/test';

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
import { describe } from '@voidzero-dev/vite-plus/test';

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
import { describe } from '@voidzero-dev/vite-plus/test';
export default defineConfig({});"#
        );

        // app: vite IS rewritten (no peerDep), vitest IS rewritten
        let app_content =
            fs::read_to_string(temp.path().join("packages/app/src/index.ts")).unwrap();
        assert_eq!(
            app_content,
            r#"import { defineConfig } from '@voidzero-dev/vite-plus';
import { describe } from '@voidzero-dev/vite-plus/test';
export default defineConfig({});"#
        );
    }
}
