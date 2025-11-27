use std::path::Path;

use ast_grep_config::{GlobalRules, RuleConfig, from_yaml_string};
use ast_grep_language::{LanguageExt, SupportLang};
use serde_json::Value;
use vite_error::Error;

use crate::ast_grep;

/// ast-grep rules for rewriting imports to @voidzero-dev/vite-plus
///
/// This rewrites:
/// - `import { ... } from 'vite'` → `import { ... } from '@voidzero-dev/vite-plus'`
/// - `import { ... } from 'vitest/config'` → `import { ... } from '@voidzero-dev/vite-plus'`
const REWRITE_IMPORT_RULES: &str = r#"---
id: rewrite-vitest-config-import
language: TypeScript
rule:
  pattern: "'vitest/config'"
  inside:
    kind: import_statement
fix: "'@voidzero-dev/vite-plus'"
---
id: rewrite-vitest-config-import-double-quotes
language: TypeScript
rule:
  pattern: '"vitest/config"'
  inside:
    kind: import_statement
fix: '"@voidzero-dev/vite-plus"'
---
id: rewrite-vite-import
language: TypeScript
rule:
  pattern: "'vite'"
  inside:
    kind: import_statement
fix: "'@voidzero-dev/vite-plus'"
---
id: rewrite-vite-import-double-quotes
language: TypeScript
rule:
  pattern: '"vite"'
  inside:
    kind: import_statement
fix: '"@voidzero-dev/vite-plus"'
"#;

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

/// Result of rewriting imports in vite config
#[derive(Debug)]
pub struct RewriteResult {
    /// The updated vite config content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
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
/// * `config_key` - The key to use in the vite config (e.g., "lint", "format")
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
///
/// // Merge oxfmt config with "format" key
/// let result = merge_json_config(
///     Path::new("vite.config.ts"),
///     Path::new(".oxfmtrc.json"),
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

    // Read and parse the JSON config file
    let json_config_content = std::fs::read_to_string(json_config_path)?;
    let json_config: Value = serde_json::from_str(&json_config_content)?;

    // Convert JSON to TypeScript object literal
    let ts_config = json_to_js_object_literal(&json_config, 0);

    // Merge the config
    merge_json_config_content(&vite_config_content, &ts_config, config_key)
}

/// Rewrite imports in vite config file from 'vite' or 'vitest/config' to '@voidzero-dev/vite-plus'
///
/// This function reads a vite configuration file and rewrites the import statements
/// to use '@voidzero-dev/vite-plus' instead of 'vite' or 'vitest/config'.
///
/// # Arguments
///
/// * `vite_config_path` - Path to the vite.config.ts or vite.config.js file
///
/// # Returns
///
/// Returns a `RewriteResult` containing:
/// - `content`: The updated vite config content
/// - `updated`: Whether any changes were made
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vite_migration::rewrite_import;
///
/// let result = rewrite_import(Path::new("vite.config.ts"))?;
/// if result.updated {
///     std::fs::write("vite.config.ts", &result.content)?;
/// }
/// ```
pub fn rewrite_import(vite_config_path: &Path) -> Result<RewriteResult, Error> {
    // Read the vite config file
    let vite_config_content = std::fs::read_to_string(vite_config_path)?;

    // Rewrite the imports
    rewrite_import_content(&vite_config_content)
}

/// Rewrite imports in vite config content from 'vite' or 'vitest/config' to '@voidzero-dev/vite-plus'
///
/// This is the internal function that performs the actual rewrite using ast-grep.
fn rewrite_import_content(vite_config_content: &str) -> Result<RewriteResult, Error> {
    let (content, updated) = ast_grep::apply_rules(vite_config_content, REWRITE_IMPORT_RULES)?;
    Ok(RewriteResult { content, updated })
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
/// * `config_key` - The key to use in the vite config (e.g., "lint", "format")
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

/// Convert a JSON value to JavaScript object literal format
///
/// This function recursively converts JSON values to their JavaScript
/// object literal representation with proper formatting.
fn json_to_js_object_literal(value: &Value, indent: usize) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{}'", escape_single_quotes(s)),
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let items: Vec<String> =
                arr.iter().map(|item| json_to_js_object_literal(item, indent + 2)).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(obj) => {
            // Filter out $schema field (used for JSON schema validation, not needed in JS)
            let filtered: Vec<_> = obj.iter().filter(|(key, _)| *key != "$schema").collect();

            if filtered.is_empty() {
                return "{}".to_string();
            }

            let spaces = " ".repeat(indent);
            let inner_spaces = " ".repeat(indent + 2);

            let props: Vec<String> = filtered
                .iter()
                .map(|(key, val)| {
                    let formatted_key = format_object_key(key);
                    let formatted_value = json_to_js_object_literal(val, indent + 2);
                    format!("{inner_spaces}{formatted_key}: {formatted_value}")
                })
                .collect();

            format!("{{\n{},\n{spaces}}}", props.join(",\n"))
        }
    }
}

/// Format an object key for TypeScript
///
/// If the key is a valid identifier, return it as-is.
/// Otherwise, wrap it in single quotes.
fn format_object_key(key: &str) -> String {
    // Check if the key is a valid JavaScript identifier
    if is_valid_identifier(key) {
        key.to_string()
    } else {
        format!("'{}'", escape_single_quotes(key))
    }
}

/// Check if a string is a valid JavaScript identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();

    // First character must be a letter, underscore, or dollar sign
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }

    // Rest can also include digits
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '$' {
            return false;
        }
    }

    // Check against reserved words (basic set)
    !matches!(
        s,
        "break"
            | "case"
            | "catch"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "in"
            | "instanceof"
            | "new"
            | "return"
            | "switch"
            | "this"
            | "throw"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "class"
            | "const"
            | "enum"
            | "export"
            | "extends"
            | "import"
            | "super"
            | "implements"
            | "interface"
            | "let"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
}

/// Escape single quotes in a string for TypeScript string literals
fn escape_single_quotes(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_json_to_js_object_literal_primitives() {
        assert_eq!(json_to_js_object_literal(&Value::Null, 0), "null");
        assert_eq!(json_to_js_object_literal(&Value::Bool(true), 0), "true");
        assert_eq!(json_to_js_object_literal(&Value::Bool(false), 0), "false");
        assert_eq!(json_to_js_object_literal(&serde_json::json!(42), 0), "42");
        assert_eq!(json_to_js_object_literal(&serde_json::json!(3.14), 0), "3.14");
        assert_eq!(json_to_js_object_literal(&serde_json::json!("hello"), 0), "'hello'");
    }

    #[test]
    fn test_json_to_js_object_literal_string_escaping() {
        assert_eq!(json_to_js_object_literal(&serde_json::json!("it's"), 0), "'it\\'s'");
        assert_eq!(json_to_js_object_literal(&serde_json::json!("a\\b"), 0), "'a\\\\b'");
    }

    #[test]
    fn test_json_to_js_object_literal_array() {
        assert_eq!(json_to_js_object_literal(&serde_json::json!([]), 0), "[]");
        assert_eq!(json_to_js_object_literal(&serde_json::json!([1, 2, 3]), 0), "[1, 2, 3]");
        assert_eq!(json_to_js_object_literal(&serde_json::json!(["a", "b"]), 0), "['a', 'b']");
    }

    #[test]
    fn test_json_to_js_object_literal_object() {
        assert_eq!(json_to_js_object_literal(&serde_json::json!({}), 0), "{}");

        let obj = serde_json::json!({
            "key": "value"
        });
        let result = json_to_js_object_literal(&obj, 0);
        assert!(result.contains("key: 'value'"));
    }

    #[test]
    fn test_json_to_js_object_literal_ignores_schema() {
        // $schema field should be filtered out
        let obj = serde_json::json!({
            "$schema": "./node_modules/oxfmt/configuration_schema.json",
            "foo": "bar"
        });
        let result = json_to_js_object_literal(&obj, 0);
        assert!(!result.contains("$schema"));
        assert!(result.contains("foo: 'bar'"));

        // Object with only $schema should become empty
        let obj = serde_json::json!({
            "$schema": "./schema.json"
        });
        assert_eq!(json_to_js_object_literal(&obj, 0), "{}");
    }

    #[test]
    fn test_json_to_js_object_literal_complex() {
        let config = serde_json::json!({
            "rules": {
                "no-unused-vars": "error",
                "no-console": "warn"
            },
            "ignorePatterns": ["dist", "node_modules"]
        });

        let result = json_to_js_object_literal(&config, 2);
        assert!(result.contains("rules:"));
        assert!(result.contains("'no-unused-vars': 'error'"));
        assert!(result.contains("'no-console': 'warn'"));
        assert!(result.contains("ignorePatterns: ['dist', 'node_modules']"));
    }

    #[test]
    fn test_format_object_key() {
        assert_eq!(format_object_key("validKey"), "validKey");
        assert_eq!(format_object_key("_private"), "_private");
        assert_eq!(format_object_key("$special"), "$special");
        assert_eq!(format_object_key("key123"), "key123");
        assert_eq!(format_object_key("no-dashes"), "'no-dashes'");
        assert_eq!(format_object_key("has space"), "'has space'");
        assert_eq!(format_object_key("123start"), "'123start'");
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("validName"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("$jquery"));
        assert!(is_valid_identifier("camelCase"));
        assert!(is_valid_identifier("PascalCase"));
        assert!(is_valid_identifier("name123"));

        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123start"));
        assert!(!is_valid_identifier("has-dash"));
        assert!(!is_valid_identifier("has space"));
        assert!(!is_valid_identifier("class")); // reserved word
        assert!(!is_valid_identifier("const")); // reserved word
    }

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

        // Verify the result
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    rules: {
      'no-unused-vars': 'error',
      'no-console': 'warn',
    },
    ignorePatterns: ['dist', 'node_modules'],
  },
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_full_json_to_js_object_literal_conversion() {
        // Test a realistic .oxlintrc config
        let oxlint_json = serde_json::json!({
            "rules": {
                "no-unused-vars": "error",
                "no-console": "warn",
                "no-debugger": "error"
            },
            "ignorePatterns": ["dist", "node_modules", "*.config.js"],
            "plugins": ["react", "typescript"],
            "settings": {
                "react": {
                    "version": "detect"
                }
            }
        });

        let ts_literal = json_to_js_object_literal(&oxlint_json, 0);

        // Verify the conversion
        assert_eq!(
            ts_literal,
            r#"{
  rules: {
    'no-unused-vars': 'error',
    'no-console': 'warn',
    'no-debugger': 'error',
  },
  ignorePatterns: ['dist', 'node_modules', '*.config.js'],
  plugins: ['react', 'typescript'],
  settings: {
    react: {
      version: 'detect',
    },
  },
}"#
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
}
