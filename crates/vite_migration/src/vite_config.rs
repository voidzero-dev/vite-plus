use std::path::Path;

use ast_grep_config::{GlobalRules, RuleConfig, from_yaml_string};
use ast_grep_language::{LanguageExt, SupportLang};
use vite_error::Error;

use crate::ast_grep;

/// Result of merging JSON config into vite config
#[derive(Debug)]
pub struct MergeResult {
    /// The updated vite config content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
    /// Whether the config uses a function callback
    pub uses_function_callback: bool,
}

/// Merge a JSON configuration file into vite.config.ts or vite.config.js
///
/// This function reads a JSON configuration file and merges it into the vite
/// configuration file by adding a section with the specified key to the config.
///
/// Note: TypeScript parser is used for both .ts and .js files since TypeScript
/// syntax is a superset of JavaScript.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
/// * `json_config_path` - Path to the JSON config file (e.g., .oxlintrc.json, .oxfmtrc.json)
/// * `config_key` - The key to use in the vite config (e.g., "lint", "fmt")
///
/// # Returns
///
/// Returns a `MergeResult` containing:
/// - `content`: The updated vite config content
/// - `updated`: Whether any changes were made
/// - `uses_function_callback`: Whether the config uses a function callback
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vite_migration::merge_json_config;
///
/// // Merge oxlint config with "lint" key
/// let result = merge_json_config(
///     Path::new("vite.config.ts"),
///     Path::new(".oxlintrc"),
///     "lint",
/// )?;
/// // Merge oxfmt config with "fmt" key
/// let result = merge_json_config(
///     Path::new("vite.config.ts"),
///     Path::new(".oxfmtrc.json"),
///     "fmt",
///     "format",
/// )?;
///
/// if result.updated {
///     std::fs::write("vite.config.ts", &result.content)?;
/// }
/// ```
pub fn merge_json_config(
    vite_config_path: &Path,
    json_config_path: &Path,
    config_key: &str,
) -> Result<MergeResult, Error> {
    // Read the vite config file
    let vite_config_content = std::fs::read_to_string(vite_config_path)?;

    // Read the JSON/JSONC config file directly
    // JSON/JSONC content is valid JS (comments are valid in JS too)
    let js_config = std::fs::read_to_string(json_config_path)?;

    // Merge the config
    merge_json_config_content(&vite_config_content, &js_config, config_key)
}

/// Merge JSON configuration into vite config content
///
/// This is the internal function that performs the actual merge using ast-grep.
/// It takes the vite config content and the JSON config as a TypeScript object literal string.
///
/// # Arguments
///
/// * `vite_config_content` - The content of the vite.config.ts or vite.config.js file
/// * `ts_config` - The config as a TypeScript object literal string
/// * `config_key` - The key to use in the vite config (e.g., "lint", "fmt")
///
/// # Returns
///
/// Returns a `MergeResult` with the updated content and status flags.
fn merge_json_config_content(
    vite_config_content: &str,
    ts_config: &str,
    config_key: &str,
) -> Result<MergeResult, Error> {
    // Check if the config uses a function callback (for informational purposes)
    let uses_function_callback = check_function_callback(vite_config_content)?;

    // Generate the ast-grep rules with the actual config
    let rule_yaml = generate_merge_rule(ts_config, config_key);

    // Apply the transformation
    let (content, updated) = ast_grep::apply_rules(vite_config_content, &rule_yaml)?;

    Ok(MergeResult { content, updated, uses_function_callback })
}

/// Check if the vite config uses a function callback pattern
fn check_function_callback(vite_config_content: &str) -> Result<bool, Error> {
    // Match both sync and async arrow functions
    let check_rule = r#"
---
id: check-function-callback
language: TypeScript
rule:
  any:
    - pattern: defineConfig(($PARAMS) => $BODY)
    - pattern: defineConfig(async ($PARAMS) => $BODY)
"#;

    let globals = GlobalRules::default();
    let rules: Vec<RuleConfig<SupportLang>> =
        from_yaml_string::<SupportLang>(check_rule, &globals)?;

    for rule in &rules {
        if rule.language != SupportLang::TypeScript {
            continue;
        }

        let grep = rule.language.ast_grep(vite_config_content);
        let root = grep.root();
        let matcher = &rule.matcher;

        if root.find(matcher).is_some() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generate the ast-grep rules YAML for merging JSON config
///
/// This generates six rules:
/// 1. For object literal: `defineConfig({ ... })`
/// 2. For arrow function with direct return: `defineConfig((env) => ({ ... }))`
/// 3. For return object literal inside defineConfig callback: `return { ... }`
/// 4. For return variable inside defineConfig callback: `return configObj` -> `return { ..., ...configObj }`
/// 5. For plain object export: `export default { ... }`
/// 6. For satisfies pattern: `export default { ... } satisfies Type`
///
/// The config is placed first to avoid trailing comma issues.
fn generate_merge_rule(ts_config: &str, config_key: &str) -> String {
    // Indent the config to match the YAML structure
    let indented_config = indent_multiline(ts_config, 4);

    let template = r#"---
id: merge-json-config-object
language: TypeScript
rule:
  pattern: |
    defineConfig({
      $$$CONFIG
    })
fix: |-
  defineConfig({
    __CONFIG_KEY__: __JSON_CONFIG__,
    $$$CONFIG
  })
---
id: merge-json-config-function
language: TypeScript
rule:
  pattern: |
    defineConfig(($PARAMS) => ({
      $$$CONFIG
    }))
fix: |-
  defineConfig(($PARAMS) => ({
    __CONFIG_KEY__: __JSON_CONFIG__,
    $$$CONFIG
  }))
---
id: merge-json-config-return
language: TypeScript
rule:
  pattern: |
    return {
      $$$CONFIG
    }
  inside:
    stopBy: end
    pattern: defineConfig($$$ARGS)
fix: |-
  return {
    __CONFIG_KEY__: __JSON_CONFIG__,
    $$$CONFIG
  }
---
id: merge-json-config-return-var
language: TypeScript
rule:
  pattern: return $VAR
  has:
    pattern: $VAR
    kind: identifier
  inside:
    stopBy: end
    pattern: defineConfig($$$ARGS)
fix: |-
  return {
    __CONFIG_KEY__: __JSON_CONFIG__,
    ...$VAR,
  }
---
id: merge-json-config-plain-export
language: TypeScript
rule:
  pattern: |
    export default {
      $$$CONFIG
    }
fix: |-
  export default {
    __CONFIG_KEY__: __JSON_CONFIG__,
    $$$CONFIG
  }
---
id: merge-json-config-satisfies
language: TypeScript
rule:
  pattern: |
    export default {
      $$$CONFIG
    } satisfies $TYPE
fix: |-
  export default {
    __CONFIG_KEY__: __JSON_CONFIG__,
    $$$CONFIG
  } satisfies $TYPE
"#;

    template.replace("__CONFIG_KEY__", config_key).replace("__JSON_CONFIG__", &indented_config)
}

/// Indent each line of a multiline string
fn indent_multiline(s: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    let lines: Vec<&str> = s.lines().collect();

    if lines.len() <= 1 {
        return s.to_string();
    }

    // First line doesn't get indented (it's on the same line as the key)
    // Subsequent lines get the specified indent
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| if i == 0 { line.to_string() } else { format!("{indent}{line}") })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Merge tsdown config into vite.config.ts by importing it
///
/// This function adds an import statement for the tsdown config file
/// and adds `lib: tsdownConfig` to the defineConfig.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
/// * `tsdown_config_path` - Path to the tsdown.config.ts file (relative path like "./tsdown.config.ts")
///
/// # Returns
///
/// Returns a `MergeResult` with the updated content
pub fn merge_tsdown_config(
    vite_config_path: &Path,
    tsdown_config_path: &str,
) -> Result<MergeResult, Error> {
    let vite_config_content = std::fs::read_to_string(vite_config_path)?;
    merge_tsdown_config_content(&vite_config_content, tsdown_config_path)
}

/// Merge tsdown config into vite config content
///
/// This adds:
/// 1. An import statement: `import tsdownConfig from './tsdown.config.ts'`
/// 2. The lib config in defineConfig: `lib: tsdownConfig`
///
/// This function is idempotent - running it multiple times will not create duplicates.
fn merge_tsdown_config_content(
    vite_config_content: &str,
    tsdown_config_path: &str,
) -> Result<MergeResult, Error> {
    let uses_function_callback = check_function_callback(vite_config_content)?;

    // Check if already migrated (idempotency check)
    if vite_config_content.contains("import tsdownConfig from") {
        return Ok(MergeResult {
            content: vite_config_content.to_string(),
            updated: false,
            uses_function_callback,
        });
    }

    // Step 1: Add import statement at the beginning
    // Use JavaScript extensions for TypeScript files (TypeScript module resolution convention)
    // .ts → .js, .mts → .mjs, .cts → .cjs
    let import_path = if tsdown_config_path.ends_with(".mts") {
        tsdown_config_path.replace(".mts", ".mjs")
    } else if tsdown_config_path.ends_with(".cts") {
        tsdown_config_path.replace(".cts", ".cjs")
    } else if tsdown_config_path.ends_with(".ts") {
        tsdown_config_path.replace(".ts", ".js")
    } else {
        tsdown_config_path.to_string()
    };
    let content_with_import =
        format!("import tsdownConfig from '{import_path}';\n\n{vite_config_content}");

    // Step 2: Add lib: tsdownConfig to defineConfig
    let lib_rule = generate_merge_rule("tsdownConfig", "lib");
    let (final_content, _) = ast_grep::apply_rules(&content_with_import, &lib_rule)?;

    Ok(MergeResult { content: final_content, updated: true, uses_function_callback })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_check_function_callback() {
        let simple_config = r#"
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [],
});
"#;
        assert!(!check_function_callback(simple_config).unwrap());

        let function_config = r#"
import { defineConfig } from 'vite';

export default defineConfig((env) => ({
  plugins: [],
  server: {
    port: env.mode === 'production' ? 8080 : 3000,
  },
}));
"#;
        assert!(check_function_callback(function_config).unwrap());
    }

    #[test]
    fn test_merge_json_config_content_simple() {
        let vite_config = r#"import { defineConfig } from 'vite';

export default defineConfig({});"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'warn',
    },
  },
  
});"#
        );
        assert!(result.updated);
        assert!(!result.uses_function_callback);
    }

    #[test]
    fn test_merge_json_config_content_with_existing_config() {
        let vite_config = r#"import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
  },
});"#;

        let oxlint_config = r#"{
  rules: {
    'no-unused-vars': 'error',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("plugins: [react()]"));
        assert!(result.content.contains("port: 3000"));
        assert!(result.content.contains("lint:"));
        assert!(result.content.contains("'no-unused-vars': 'error'"));
    }

    #[test]
    fn test_merge_json_config_content_function_callback() {
        let vite_config = r#"import { defineConfig } from 'vite';

export default defineConfig((env) => ({
  plugins: [],
}));"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert!(result.uses_function_callback);
        // Function callbacks are now supported
        assert!(result.updated);
        assert!(result.content.contains("lint:"));
        assert!(result.content.contains("'no-console': 'warn'"));
        // Verify the function callback structure is preserved
        assert!(result.content.contains("(env) =>"));
        println!("result: {}", result.content);
    }

    #[test]
    fn test_merge_json_config_content_complex_function_callback() {
        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;
        // Complex function callback with conditional returns
        // https://vite.dev/config/#conditional-config
        let vite_config = r#"import { defineConfig } from 'vite';

export default defineConfig(({ command, mode, isSsrBuild, isPreview }) => {
  if (command === 'serve') {
    return {
      // dev specific config
    }
  } else {
    // command === 'build'
    return {
      // build specific config
    }
  }
});"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        // Detected as function callback
        assert!(result.uses_function_callback);
        // Now can be auto-migrated using return statement matching
        assert!(result.updated);
        // Both return statements should have lint config added
        assert_eq!(
            result.content.matches("lint: {").count(),
            2,
            "Expected 2 lint configs, one for each return statement"
        );
        assert!(result.content.contains("'no-console': 'warn'"));

        // https://vite.dev/config/#using-environment-variables-in-config
        let vite_config = r#"
import { defineConfig, loadEnv } from 'vite'

export default defineConfig(({ mode }) => {
  // Load env file based on `mode` in the current working directory.
  // Set the third parameter to '' to load all env regardless of the
  // `VITE_` prefix.
  const env = loadEnv(mode, process.cwd(), '')
  return {
    define: {
      // Provide an explicit app-level constant derived from an env var.
      __APP_ENV__: JSON.stringify(env.APP_ENV),
    },
    // Example: use an env var to set the dev server port conditionally.
    server: {
      port: env.APP_PORT ? Number(env.APP_PORT) : 5173,
    },
  }
})
"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        // Detected as function callback
        assert!(result.uses_function_callback);
        // Now can be auto-migrated using return statement matching
        assert!(result.updated);
        assert!(result.content.contains("'no-console': 'warn'"));

        // https://vite.dev/config/#async-config
        let vite_config = r#"
export default defineConfig(async ({ command, mode }) => {
  const data = await asyncFunction()
  return {
    // vite config
  }
})
"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        // Detected as function callback
        assert!(result.uses_function_callback);
        // Now can be auto-migrated using return statement matching
        assert!(result.updated);
        assert!(result.content.contains("'no-console': 'warn'"));
    }

    #[test]
    fn test_generate_merge_rule() {
        let config = "{ rules: { 'no-console': 'warn' } }";

        // Test with "lint" key
        let rule = generate_merge_rule(config, "lint");
        assert!(rule.contains("id: merge-json-config-object"));
        assert!(rule.contains("id: merge-json-config-function"));
        assert!(rule.contains("id: merge-json-config-return"));
        assert!(rule.contains("id: merge-json-config-return-var"));
        assert!(rule.contains("id: merge-json-config-plain-export"));
        assert!(rule.contains("id: merge-json-config-satisfies"));
        assert!(rule.contains("language: TypeScript"));
        assert!(rule.contains("defineConfig"));
        assert!(rule.contains("lint:"));
        assert!(rule.contains("'no-console': 'warn'"));
        assert!(rule.contains("($PARAMS) =>"));
        assert!(rule.contains("inside:"));
        assert!(rule.contains("defineConfig($$$ARGS)"));
        assert!(rule.contains("export default {"));
        assert!(rule.contains("...$VAR,"));

        // Test with "format" key
        let rule = generate_merge_rule(config, "format");
        assert!(rule.contains("format:"));
        assert!(!rule.contains("lint:"));
    }

    #[test]
    fn test_merge_json_config_content_arrow_wrapper() {
        // Arrow function that wraps defineConfig
        let vite_config = r#"import { defineConfig } from "vite";

export default () =>
  defineConfig({
    root: "./",
    build: {
      outDir: "./build/app",
    },
  });"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        assert!(result.content.contains("lint: {"));
        assert!(result.content.contains("'no-console': 'warn'"));
    }

    #[test]
    fn test_merge_json_config_content_plain_export() {
        // Plain object export without defineConfig
        // https://vite.dev/config/#config-intellisense
        let vite_config = r#"export default {
  server: {
    port: 5173,
  },
}"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        assert!(result.content.contains("lint: {"));
        assert!(result.content.contains("'no-console': 'warn'"));
        assert!(result.content.contains("server: {"));

        let vite_config = r#"
import type { UserConfig } from 'vite'

export default {
  server: {
    port: 5173,
  },
} satisfies UserConfig
        "#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        assert!(result.content.contains("lint: {"));
        assert!(result.content.contains("'no-console': 'warn'"));
        assert!(result.content.contains("server: {"));
    }

    #[test]
    fn test_merge_json_config_content_return_variable() {
        // Return a variable instead of object literal
        let vite_config = r#"import { defineConfig, loadEnv } from 'vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const configObject = {
    define: {
      __APP_ENV__: JSON.stringify(env.APP_ENV),
    },
    server: {
      port: env.APP_PORT ? Number(env.APP_PORT) : 5173,
    },
  }

  return configObject
})"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert_eq!(
            result.content,
            r#"import { defineConfig, loadEnv } from 'vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  const configObject = {
    define: {
      __APP_ENV__: JSON.stringify(env.APP_ENV),
    },
    server: {
      port: env.APP_PORT ? Number(env.APP_PORT) : 5173,
    },
  }

  return {
    lint: {
      rules: {
        'no-console': 'warn',
      },
    },
    ...configObject,
  }
})"#
        );
        assert!(result.updated);
        assert!(result.uses_function_callback);
    }

    #[test]
    fn test_merge_json_config_content_with_format_key() {
        // Test merge_json_config_content with "format" key (for oxfmt)
        let vite_config = r#"import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [],
});"#;

        let format_config = r#"{
  indentWidth: 2,
  lineWidth: 100,
}"#;

        let result = merge_json_config_content(vite_config, format_config, "format").unwrap();
        println!("result: {}", result.content);
        assert!(result.updated);
        assert!(result.content.contains("format: {"));
        assert!(result.content.contains("indentWidth: 2"));
        assert!(result.content.contains("lineWidth: 100"));
        assert!(!result.content.contains("lint:"));
    }

    #[test]
    fn test_merge_json_config_with_files() {
        // Create temporary directory (automatically cleaned up when dropped)
        let temp_dir = tempdir().unwrap();

        let vite_config_path = temp_dir.path().join("vite.config.ts");
        let oxlint_config_path = temp_dir.path().join(".oxlintrc");

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

        // Write test oxlint config
        let mut oxlint_file = std::fs::File::create(&oxlint_config_path).unwrap();
        write!(
            oxlint_file,
            r#"{{
  "rules": {{
    "no-unused-vars": "error",
    "no-console": "warn"
  }},
  "ignorePatterns": ["dist", "node_modules"]
}}"#
        )
        .unwrap();

        // Run the merge
        let result = merge_json_config(&vite_config_path, &oxlint_config_path, "lint").unwrap();

        // Verify the result - JSON content is used directly (double quotes preserved)
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    "rules": {
      "no-unused-vars": "error",
      "no-console": "warn"
    },
    "ignorePatterns": ["dist", "node_modules"]
  },
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_merge_json_config_with_jsonc_file() {
        // Test JSONC support with single-line and block comments
        let temp_dir = tempdir().unwrap();

        let vite_config_path = temp_dir.path().join("vite.config.ts");
        let jsonc_config_path = temp_dir.path().join(".oxfmtrc.jsonc");

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

        // Write test JSONC config with comments
        let mut jsonc_file = std::fs::File::create(&jsonc_config_path).unwrap();
        write!(
            jsonc_file,
            r#"{{
  // Formatting options
  "indentWidth": 2,
  /*
   * Line width configuration
   */
  "lineWidth": 100
}}"#
        )
        .unwrap();

        // Run the merge
        let result = merge_json_config(&vite_config_path, &jsonc_config_path, "fmt").unwrap();

        // Verify the result - JSONC content used directly (comments preserved)
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';

export default defineConfig({
  fmt: {
    // Formatting options
    "indentWidth": 2,
    /*
     * Line width configuration
     */
    "lineWidth": 100
  },
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_merge_json_config_with_inline_comments() {
        // Test JSONC with inline comments
        let temp_dir = tempdir().unwrap();

        let vite_config_path = temp_dir.path().join("vite.config.ts");
        let jsonc_config_path = temp_dir.path().join(".oxlintrc.jsonc");

        let mut vite_file = std::fs::File::create(&vite_config_path).unwrap();
        write!(
            vite_file,
            r#"import {{ defineConfig }} from 'vite';

export default defineConfig({{
  plugins: [],
}});"#
        )
        .unwrap();

        // JSONC with inline comments
        let mut jsonc_file = std::fs::File::create(&jsonc_config_path).unwrap();
        write!(
            jsonc_file,
            r#"{{
  "rules": {{
    "no-console": "warn" // warn about console.log usage
  }}
}}"#
        )
        .unwrap();

        let result = merge_json_config(&vite_config_path, &jsonc_config_path, "lint").unwrap();

        // Verify the result - JSONC content used directly (comments preserved)
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    "rules": {
      "no-console": "warn" // warn about console.log usage
    }
  },
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_indent_multiline() {
        // Single line - no change
        assert_eq!(indent_multiline("single line", 4), "single line");

        // Empty string
        assert_eq!(indent_multiline("", 4), "");

        // Multiple lines
        let input = "first\nsecond\nthird";
        let expected = "first\n    second\n    third";
        assert_eq!(indent_multiline(input, 4), expected);
    }

    #[test]
    fn test_merge_json_config_content_no_trailing_comma() {
        // Config WITHOUT trailing comma - lint is placed first to avoid comma issues
        let vite_config = r#"import { defineConfig } from 'vite';
export default defineConfig({
  plugins: []
});"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            "import { defineConfig } from 'vite';
export default defineConfig({
  lint: {
    rules: {
      'no-console': 'warn',
    },
  },
  plugins: []
});"
        );
    }

    #[test]
    fn test_merge_json_config_content_with_trailing_comma() {
        // Config WITH trailing comma - no issues since lint is placed first
        let vite_config = r#"import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [],
})"#;

        let oxlint_config = r#"{
  rules: {
    'no-console': 'warn',
  },
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        println!("result: {}", result.content);
        assert!(result.updated);
        assert_eq!(
            result.content,
            "import { defineConfig } from 'vite'

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'warn',
    },
  },
  plugins: [],
})"
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_simple() {
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  plugins: [],
});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        // TypeScript files use .js extension in imports
        assert_eq!(
            result.content,
            r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  lib: tsdownConfig,
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_with_existing_imports() {
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        assert_eq!(
            result.content,
            r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from '@voidzero-dev/vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  lib: tsdownConfig,
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_function_callback() {
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig((env) => ({
  plugins: [],
}));"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(result.uses_function_callback);
        assert_eq!(
            result.content,
            r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig((env) => ({
  lib: tsdownConfig,
  plugins: [],
}));"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_idempotent() {
        // Already migrated config - import at the beginning
        let already_migrated = r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  lib: tsdownConfig,
  plugins: [],
});"#;

        let result = merge_tsdown_config_content(already_migrated, "./tsdown.config.ts").unwrap();
        assert!(!result.updated, "Should not update already migrated config");
        assert_eq!(result.content, already_migrated);

        // Run migration twice and verify no duplicates
        let fresh_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  plugins: [],
});"#;

        let expected_migrated = r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  lib: tsdownConfig,
  plugins: [],
});"#;

        let first_result = merge_tsdown_config_content(fresh_config, "./tsdown.config.ts").unwrap();
        assert!(first_result.updated);
        assert_eq!(first_result.content, expected_migrated);

        // Run again on the result - should return unchanged
        let second_result =
            merge_tsdown_config_content(&first_result.content, "./tsdown.config.ts").unwrap();
        assert!(!second_result.updated, "Second migration should not update");
        assert_eq!(second_result.content, expected_migrated);
    }

    #[test]
    fn test_merge_tsdown_config_content_no_imports() {
        // vite.config.ts without any import statements
        let vite_config = r#"export default {
  server: { port: 3000 }
}"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(!result.uses_function_callback);
        assert_eq!(
            result.content,
            r#"import tsdownConfig from './tsdown.config.js';

export default {
  lib: tsdownConfig,
  server: { port: 3000 }
}"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_no_false_positive_stdlib() {
        // "stdlib:" should not be detected as "lib:" key
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  stdlib: 'some-value',
});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.js'"));
        assert!(result.content.contains("lib: tsdownConfig"));
        assert!(result.content.contains("stdlib: 'some-value'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_mts_extension() {
        // .mts files should use .mjs extension in imports
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.mts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.mjs'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_cts_extension() {
        // .cts files should use .cjs extension in imports
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.cts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.cjs'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_js_extension_unchanged() {
        // .js, .mjs, .cjs files should keep their extensions unchanged
        let vite_config = r#"import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.js").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.js'"));

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.mjs").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.mjs'"));

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.cjs").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.cjs'"));
    }
}
