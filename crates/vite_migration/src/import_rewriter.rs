use std::path::{Path, PathBuf};

use vite_error::Error;

use crate::{ast_grep, file_walker};

/// ast-grep rules for rewriting imports to @voidzero-dev/vite-plus
///
/// This rewrites:
/// - `import { ... } from 'vite'` → `import { ... } from '@voidzero-dev/vite-plus'`
/// - `import { ... } from 'vite/{name}'` → `import { ... } from '@voidzero-dev/vite-plus/{name}'`
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
const REWRITE_IMPORT_RULES: &str = r#"---
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
"#;

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

    for file_path in walk_result.files {
        match rewrite_import(&file_path) {
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
///
/// # Arguments
///
/// * `file_path` - Path to the TypeScript/JavaScript file
///
/// # Returns
///
/// Returns a `RewriteResult` containing:
/// - `content`: The updated file content
/// - `updated`: Whether any changes were made
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vite_migration::rewrite_import;
///
/// let result = rewrite_import(Path::new("src/app.ts"))?;
/// if result.updated {
///     std::fs::write("src/app.ts", &result.content)?;
/// }
/// ```
fn rewrite_import(file_path: &Path) -> Result<RewriteResult, Error> {
    // Read the file
    let content = std::fs::read_to_string(file_path)?;

    // Rewrite the imports
    rewrite_import_content(&content)
}

/// Rewrite imports in content from vite/vitest to @voidzero-dev/vite-plus
///
/// This is the internal function that performs the actual rewrite using ast-grep.
fn rewrite_import_content(content: &str) -> Result<RewriteResult, Error> {
    let (new_content, updated) = ast_grep::apply_rules(content, REWRITE_IMPORT_RULES)?;
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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
        let result = rewrite_import(&vite_config_path).unwrap();

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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { startVitest } from '@voidzero-dev/vite-plus/test/node';

export default startVitest;"#
        );

        // Test vitest/plugins/runner subpath
        let vite_config = r#"import { somePlugin } from 'vitest/plugins/runner';

export default somePlugin;"#;

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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

        let result = rewrite_import_content(vite_config).unwrap();
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
}
