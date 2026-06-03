use std::{borrow::Cow, path::Path, sync::LazyLock};

use ast_grep_config::{GlobalRules, RuleConfig, from_yaml_string};
use ast_grep_core::{Doc, Node};
use ast_grep_language::{LanguageExt, SupportLang};
use regex::Regex;
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

    // Strip "$schema" property — it's a JSON Schema annotation not valid in OxlintConfig
    let ts_config = strip_schema_property(ts_config);

    // Generate the ast-grep rules with the actual config
    let rule_yaml = generate_merge_rule(&ts_config, config_key);

    // Apply the transformation
    let (content, updated) = ast_grep::apply_rules(vite_config_content, &rule_yaml)?;

    Ok(MergeResult { content, updated, uses_function_callback })
}

/// Regex to match `"$schema": "..."` lines (with optional trailing comma).
static RE_SCHEMA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^\s*"\$schema"\s*:\s*"[^"]*"\s*,?\s*\n"#).unwrap());

/// Strip the `"$schema"` property from a JSON/JSONC config string.
///
/// JSON config files (`.oxlintrc.json`, `.oxfmtrc.json`) often contain a
/// `"$schema"` annotation that is meaningful only for editor validation.
/// When the JSON content is embedded into `vite.config.ts`, the `$schema`
/// property causes a TypeScript type error because it is not part of
/// `OxlintConfig` / `OxfmtConfig`.
fn strip_schema_property(config: &str) -> Cow<'_, str> {
    RE_SCHEMA.replace_all(config, "")
}

/// Check whether `config_key` is already declared as a top-level property in
/// the vite config's `defineConfig({...})` (or equivalent) object literal.
///
/// Mirrors the six shapes the merger understands (see `generate_merge_rule`):
/// `defineConfig({...})`, `defineConfig((p) => ({...}))`, `return {...}`
/// inside a `defineConfig` callback, `export default {...}`, and the
/// `satisfies` export variant. The `return $VAR` variant cannot be inspected
/// statically — for that shape we conservatively report `false`, which is
/// safe because the merger uses object spread (`{ key: ..., ...$VAR }`) so
/// duplicate keys are resolved at runtime by JS spread semantics.
///
/// Returns `true` only when the key appears as a **direct** member of one of
/// those recognized object literals. Comments, string occurrences, nested
/// keys (e.g. `plugins: [{ fmt: ... }]`), and unrelated objects are all
/// ignored correctly.
pub fn has_config_key(vite_config_content: &str, config_key: &str) -> Result<bool, Error> {
    let grep = SupportLang::TypeScript.ast_grep(vite_config_content);
    let root = grep.root();

    for node in root.dfs() {
        if node.kind() != "pair" {
            continue;
        }
        let Some(key_node) = node.field("key") else { continue };
        if !pair_key_matches(&key_node, config_key) {
            continue;
        }
        let Some(parent_object) = node.parent() else { continue };
        if parent_object.kind() != "object" {
            continue;
        }
        if is_recognized_config_object(&parent_object) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Wrap safe inline Vite plugin arrays with `lazyPlugins(() => [...])`.
///
/// This transform is intentionally conservative: it only touches direct
/// `plugins: [...]` pairs inside recognized Vite config objects and skips
/// CommonJS configs rather than injecting ESM imports into them.
pub fn wrap_lazy_plugins(vite_config_path: &Path) -> Result<MergeResult, Error> {
    let vite_config_content = std::fs::read_to_string(vite_config_path)?;
    wrap_lazy_plugins_content(&vite_config_content, Some(vite_config_path))
}

fn wrap_lazy_plugins_content(
    vite_config_content: &str,
    vite_config_path: Option<&Path>,
) -> Result<MergeResult, Error> {
    let uses_function_callback = check_function_callback(vite_config_content)?;

    if is_commonjs_config(vite_config_content, vite_config_path)
        || has_conflicting_lazy_plugins_binding(vite_config_content)
    {
        return Ok(MergeResult {
            content: vite_config_content.to_owned(),
            updated: false,
            uses_function_callback,
        });
    }

    let grep = SupportLang::TypeScript.ast_grep(vite_config_content);
    let root = grep.root();
    let mut replacements = Vec::new();

    for node in root.dfs() {
        if node.kind() != "pair" {
            continue;
        }
        let Some(key_node) = node.field("key") else { continue };
        if !pair_key_matches(&key_node, "plugins") {
            continue;
        }
        let Some(parent_object) = node.parent() else { continue };
        if parent_object.kind() != "object" {
            continue;
        }
        if !is_recognized_config_object_for_lazy_plugins(&parent_object) {
            continue;
        }
        let Some(value_node) = node.field("value") else { continue };
        if value_node.kind() != "array" {
            continue;
        }

        let callback = if node_has_descendant_kind(&value_node, "await_expression") {
            "async () =>"
        } else {
            "() =>"
        };
        let range = value_node.range();
        replacements.push((
            range.start,
            range.end,
            format!("lazyPlugins({callback} {})", value_node.text()),
        ));
    }

    if replacements.is_empty() {
        return Ok(MergeResult {
            content: vite_config_content.to_owned(),
            updated: false,
            uses_function_callback,
        });
    }

    replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
    let mut content = vite_config_content.to_owned();
    for (start, end, replacement) in replacements {
        content.replace_range(start..end, &replacement);
    }
    content = ensure_lazy_plugins_import(&content);

    Ok(MergeResult { content, updated: true, uses_function_callback })
}

fn pair_key_matches<D: Doc>(key_node: &Node<'_, D>, config_key: &str) -> bool {
    let text = key_node.text();
    match key_node.kind().as_ref() {
        "property_identifier" => text == config_key,
        "string" => text.trim_matches(|c| c == '"' || c == '\'' || c == '`') == config_key,
        _ => false,
    }
}

fn is_commonjs_config(content: &str, path: Option<&Path>) -> bool {
    if path.is_some_and(|p| {
        p.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| matches!(ext, "cjs" | "cts"))
    }) {
        return true;
    }
    content.contains("module.exports") || RE_REQUIRE_CALL.is_match(content)
}

static RE_REQUIRE_CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\brequire\s*\("#).unwrap());

static RE_LOCAL_LAZY_PLUGINS_BINDING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?m)^\s*(?:export\s+(?:default\s+)?)?(?:(?:const|let|var|class)\s+lazyPlugins\b|(?:async\s+)?function\s+lazyPlugins\b)"#,
    )
    .unwrap()
});

static RE_DESTRUCTURED_LAZY_PLUGINS_BINDING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?ms)^\s*(?:export\s+)?(?:const|let|var)\s+[\{\[][^;]*\blazyPlugins\b[^;]*[\}\]]\s*="#,
    )
    .unwrap()
});

fn has_conflicting_lazy_plugins_binding(content: &str) -> bool {
    let grep = SupportLang::TypeScript.ast_grep(content);
    let root = grep.root();

    for node in root.dfs() {
        if node.kind() != "import_statement" {
            continue;
        }
        let text = node.text();
        if imports_from_vite_plus(&text) {
            continue;
        }
        if import_binds_lazy_plugins(&text) {
            return true;
        }
    }

    RE_LOCAL_LAZY_PLUGINS_BINDING.is_match(content)
        || RE_DESTRUCTURED_LAZY_PLUGINS_BINDING.is_match(content)
}

fn import_binds_lazy_plugins(import_statement: &str) -> bool {
    let statement = strip_import_comments(import_statement);
    if RE_DEFAULT_LAZY_PLUGINS_IMPORT.is_match(&statement)
        || RE_NAMESPACE_LAZY_PLUGINS_IMPORT.is_match(&statement)
    {
        return true;
    }

    let Some(open_brace) = statement.find('{') else { return false };
    let Some(close_brace) = statement.rfind('}') else { return false };
    statement[open_brace + 1..close_brace].split(',').any(|specifier| {
        let specifier = specifier.trim();
        specifier == "lazyPlugins" || specifier.ends_with(" as lazyPlugins")
    })
}

static RE_DEFAULT_LAZY_PLUGINS_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*import\s+lazyPlugins\b"#).unwrap());

static RE_NAMESPACE_LAZY_PLUGINS_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*import\s+\*\s+as\s+lazyPlugins\b"#).unwrap());

fn is_recognized_config_object_for_lazy_plugins<D: Doc>(object_node: &Node<'_, D>) -> bool {
    let Some(parent) = object_node.parent() else { return false };
    match parent.kind().as_ref() {
        "export_statement" => true,
        "satisfies_expression" => parent.parent().is_some_and(|p| {
            p.kind() == "export_statement"
                || (p.kind() == "arguments"
                    && p.parent().is_some_and(|c| is_define_config_call(&c)))
        }),
        "arguments" => parent.parent().is_some_and(|c| is_define_config_call(&c)),
        "parenthesized_expression" => is_define_config_arrow_body(&parent),
        "return_statement" => is_direct_return_in_define_config_callback(&parent),
        _ => false,
    }
}

fn is_recognized_config_object<D: Doc>(object_node: &Node<'_, D>) -> bool {
    let Some(parent) = object_node.parent() else { return false };
    match parent.kind().as_ref() {
        "export_statement" => true,
        // `export default { ... } satisfies T` — hop past the satisfies wrapper.
        "satisfies_expression" => parent.parent().is_some_and(|p| p.kind() == "export_statement"),
        "arguments" => parent.parent().is_some_and(|c| is_define_config_call(&c)),
        "parenthesized_expression" => is_define_config_arrow_body(&parent),
        "return_statement" => is_inside_define_config_callback(&parent),
        _ => false,
    }
}

fn is_define_config_call<D: Doc>(call_node: &Node<'_, D>) -> bool {
    call_node.kind() == "call_expression"
        && call_node.field("function").is_some_and(|f| f.text() == "defineConfig")
}

fn is_define_config_arrow_body<D: Doc>(paren_node: &Node<'_, D>) -> bool {
    paren_node
        .parent()
        .filter(|n| n.kind() == "arrow_function")
        .and_then(|n| n.parent())
        .filter(|n| n.kind() == "arguments")
        .and_then(|n| n.parent())
        .is_some_and(|c| is_define_config_call(&c))
}

fn is_inside_define_config_callback<D: Doc>(node: &Node<'_, D>) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if is_define_config_call(&n) {
            return true;
        }
        current = n.parent();
    }
    false
}

fn is_direct_return_in_define_config_callback<D: Doc>(return_node: &Node<'_, D>) -> bool {
    let mut current = return_node.parent();
    while let Some(node) = current {
        match node.kind().as_ref() {
            "arrow_function" | "function_expression" => {
                return node
                    .parent()
                    .filter(|parent| parent.kind() == "arguments")
                    .and_then(|parent| parent.parent())
                    .is_some_and(|call| is_define_config_call(&call));
            }
            "function_declaration" | "method_definition" => return false,
            _ => current = node.parent(),
        }
    }
    false
}

fn node_has_descendant_kind<D: Doc>(node: &Node<'_, D>, kind: &str) -> bool {
    node.dfs().any(|child| child.kind() == kind)
}

fn ensure_lazy_plugins_import(content: &str) -> String {
    let grep = SupportLang::TypeScript.ast_grep(content);
    let root = grep.root();
    let mut import_insert_at = None;
    let mut value_import_replacement = None;

    for node in root.dfs() {
        if node.kind() != "import_statement" {
            continue;
        }
        import_insert_at =
            Some(import_insert_at.map_or(node.range().end, |end: usize| end.max(node.range().end)));

        let text = node.text();
        if !imports_from_vite_plus(&text) || text.trim_start().starts_with("import type") {
            continue;
        }
        let Some(open_brace) = text.find('{') else { continue };
        let Some(close_brace) = text.rfind('}') else { continue };
        let specifiers = &text[open_brace + 1..close_brace];
        if has_lazy_plugins_specifier(specifiers) {
            return content.to_owned();
        }
        if value_import_replacement.is_some() || import_specifiers_contain_comments(specifiers) {
            continue;
        }

        let replacement = if specifiers.contains('\n') {
            let specifiers = specifiers.trim_end();
            let comma = if specifiers.ends_with(',') { "" } else { "," };
            format!(
                "{}{specifiers}{comma}\n  lazyPlugins,\n{}",
                &text[..=open_brace],
                &text[close_brace..]
            )
        } else if specifiers.trim().is_empty() {
            format!("{} lazyPlugins {}", &text[..=open_brace], &text[close_brace..])
        } else {
            let specifiers = specifiers.trim().trim_end_matches(',').trim_end();
            format!("{} {}, lazyPlugins {}", &text[..=open_brace], specifiers, &text[close_brace..])
        };
        let range = node.range();
        value_import_replacement = Some((range.start, range.end, replacement));
    }

    if let Some((start, end, replacement)) = value_import_replacement {
        let mut updated = content.to_owned();
        updated.replace_range(start..end, &replacement);
        return updated;
    }

    let import_stmt = "import { lazyPlugins } from 'vite-plus';";
    if let Some(insert_at) = import_insert_at {
        let mut updated = content.to_owned();
        updated.insert_str(insert_at, &format!("\n{import_stmt}"));
        updated
    } else {
        format!("{import_stmt}\n\n{content}")
    }
}

fn imports_from_vite_plus(import_statement: &str) -> bool {
    import_statement.contains("from 'vite-plus'") || import_statement.contains("from \"vite-plus\"")
}

fn has_lazy_plugins_specifier(specifiers: &str) -> bool {
    strip_import_comments(specifiers).split(',').any(|specifier| specifier.trim() == "lazyPlugins")
}

fn import_specifiers_contain_comments(specifiers: &str) -> bool {
    specifiers.contains("//") || specifiers.contains("/*")
}

fn strip_import_comments(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() {
                output.push('\n');
            }
        } else if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
        } else {
            output.push(chars[i]);
            i += 1;
        }
    }
    output
}

/// Check if the vite config uses a function callback pattern
fn check_function_callback(vite_config_content: &str) -> Result<bool, Error> {
    // Match both sync and async arrow functions
    let check_rule = r"
---
id: check-function-callback
language: TypeScript
rule:
  any:
    - pattern: defineConfig(($PARAMS) => $BODY)
    - pattern: defineConfig(async ($PARAMS) => $BODY)
";

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

    let template = r"---
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
";

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
/// and adds `pack: tsdownConfig` to the defineConfig.
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
/// 2. The pack config in defineConfig: `pack: tsdownConfig`
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

    // Step 2: Add pack: tsdownConfig to defineConfig
    let pack_rule = generate_merge_rule("tsdownConfig", "pack");
    let (final_content, _) = ast_grep::apply_rules(&content_with_import, &pack_rule)?;

    Ok(MergeResult { content: final_content, updated: true, uses_function_callback })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;

    use super::*;

    // ── has_config_key ────────────────────────────────────────────────────

    #[test]
    fn test_has_config_key_top_level_in_defineconfig() {
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: { singleQuote: true },
  lint: { rules: {} },
});
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(has_config_key(cfg, "lint").unwrap());
        assert!(!has_config_key(cfg, "pack").unwrap());
        assert!(!has_config_key(cfg, "staged").unwrap());
    }

    #[test]
    fn test_has_config_key_quoted_key() {
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  'fmt': { singleQuote: true },
  "lint": {},
});
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_ignores_comment_mentions() {
        // The regex check was a false positive on these — AST check ignores them.
        let cfg = r#"import { defineConfig } from 'vite-plus';

// fmt: configure formatter here
/* lint: TODO wire this up */
export default defineConfig({
  plugins: [],
});
"#;
        assert!(!has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_ignores_string_literal_mentions() {
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
  description: 'has fmt: foo and lint: bar inside',
});
"#;
        assert!(!has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_ignores_nested_keys() {
        // `fmt:` is a nested property inside `plugins[0].options`, not top-level.
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [
    somePlugin({
      fmt: 'auto',
      lint: { enabled: true },
    }),
  ],
});
"#;
        assert!(!has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_arrow_callback() {
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig((env) => ({
  fmt: { singleQuote: env.mode === 'production' },
}));
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_return_block_callback() {
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig(({ mode }) => {
  return {
    fmt: { singleQuote: true },
  };
});
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_async_return_block_callback() {
        let cfg = r#"
export default defineConfig(async ({ command, mode }) => {
  const data = await asyncFunction();
  return {
    lint: { rules: {} },
  };
});
"#;
        assert!(has_config_key(cfg, "lint").unwrap());
        assert!(!has_config_key(cfg, "fmt").unwrap());
    }

    #[test]
    fn test_has_config_key_plain_export() {
        let cfg = r#"export default {
  fmt: { singleQuote: true },
};
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(!has_config_key(cfg, "lint").unwrap());
    }

    #[test]
    fn test_has_config_key_satisfies_export() {
        let cfg = r#"import type { UserConfig } from 'vite-plus';

export default {
  lint: { rules: {} },
} satisfies UserConfig;
"#;
        assert!(has_config_key(cfg, "lint").unwrap());
        assert!(!has_config_key(cfg, "fmt").unwrap());
    }

    #[test]
    fn test_has_config_key_return_variable_is_unknown() {
        // The merger handles this via object spread, so duplication is benign.
        // We conservatively report `false`.
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig(({ mode }) => {
  const configObject = { fmt: { singleQuote: true } };
  return configObject;
});
"#;
        assert!(!has_config_key(cfg, "fmt").unwrap());
    }

    #[test]
    fn test_has_config_key_arrow_wrapper_around_defineconfig() {
        // export default () => defineConfig({ ... }) — the wrapper is irrelevant;
        // detection follows the defineConfig argument object.
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default () =>
  defineConfig({
    fmt: { singleQuote: true },
  });
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
    }

    #[test]
    fn test_has_config_key_fate_template_shape() {
        // Mirrors create-fate's drizzle template — the bug that motivated this fix.
        let cfg = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    experimentalSortImports: { newlinesBetween: false },
    ignorePatterns: ['coverage/', 'dist/'],
    singleQuote: true,
  },
  lint: {
    extends: [nkzw],
    options: { typeAware: true, typeCheck: true },
    rules: { '@typescript-eslint/no-explicit-any': 'off' },
  },
  staged: { '*': 'vp check --fix' },
});
"#;
        assert!(has_config_key(cfg, "fmt").unwrap());
        assert!(has_config_key(cfg, "lint").unwrap());
        assert!(has_config_key(cfg, "staged").unwrap());
        assert!(!has_config_key(cfg, "pack").unwrap());
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
    fn test_strip_schema_property() {
        // With trailing comma
        let input = r#"{
  "$schema": "https://raw.githubusercontent.com/nicolo-ribaudo/tc39-proposal-json-schema/refs/heads/main/schema.json",
  "rules": {
    "no-console": "warn"
  }
}"#;
        let result = strip_schema_property(input);
        assert!(!result.contains("$schema"));
        assert!(result.contains(r#""no-console": "warn""#));

        // Without trailing comma
        let input = r#"{
  "$schema": "https://example.com/schema.json"
}"#;
        let result = strip_schema_property(input);
        assert!(!result.contains("$schema"));

        // No $schema - unchanged
        let input = r#"{
  "rules": {}
}"#;
        assert_eq!(strip_schema_property(input), input);
    }

    #[test]
    fn test_merge_json_config_content_strips_schema() {
        let vite_config = r#"import { defineConfig } from 'vite';

export default defineConfig({});"#;

        let oxlint_config = r#"{
  "$schema": "https://raw.githubusercontent.com/nicolo-ribaudo/tc39-proposal-json-schema/refs/heads/main/schema.json",
  "rules": {
    "no-console": "warn"
  }
}"#;

        let result = merge_json_config_content(vite_config, oxlint_config, "lint").unwrap();
        assert!(result.updated);
        assert!(!result.content.contains("$schema"));
        assert!(result.content.contains(r#""no-console": "warn""#));
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
    fn test_merge_json_config_content_empty_object() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: { options: { typeAware: true, typeCheck: true } },
});"#;

        let result = merge_json_config_content(vite_config, "{}", "fmt").unwrap();
        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: { options: { typeAware: true, typeCheck: true } },
});"#
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_simple_adds_import() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react(), nitro()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig, lazyPlugins } from 'vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: lazyPlugins(() => [react(), nitro()]),
});"#
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_handles_single_line_trailing_import_comma() {
        let vite_config = r#"import { defineConfig, } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(() => [react()]),
});"#
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_local_binding() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

const lazyPlugins = [react()];

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_destructured_binding() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

const { lazyPlugins } = helpers;

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_destructured_alias_binding() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

const { pluginFactory: lazyPlugins } = helpers;

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_exported_local_binding() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export const lazyPlugins = [react()];

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_exported_function_binding() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export async function lazyPlugins() {
  return [react()];
}

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_conflicting_import_binding() {
        let vite_config = r#"import { lazyPlugins } from './helpers';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_handles_satisfies_inside_define_config() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
} satisfies UserConfig);"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(() => [react()]),
} satisfies UserConfig);"#
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_adds_separate_value_import_for_type_import() {
        let vite_config = r#"import type { UserConfig } from 'vite-plus';

export default {
  plugins: [react()],
} satisfies UserConfig;"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(
            result.content,
            r#"import type { UserConfig } from 'vite-plus';
import { lazyPlugins } from 'vite-plus';

export default {
  plugins: lazyPlugins(() => [react()]),
} satisfies UserConfig;"#
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_handles_multiline_imports() {
        let vite_config = r#"import {
  defineConfig
} from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert!(
            result
                .content
                .contains("import {\n  defineConfig,\n  lazyPlugins,\n} from 'vite-plus';")
        );
    }

    #[test]
    fn test_wrap_lazy_plugins_adds_separate_import_for_commented_imports() {
        let vite_config = r#"import {
  defineConfig // keep this comment
} from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert!(result.content.contains("import {\n  defineConfig // keep this comment\n} from 'vite-plus';\nimport { lazyPlugins } from 'vite-plus';"));
        assert!(result.content.contains("plugins: lazyPlugins(() => [react()])"));
    }

    #[test]
    fn test_wrap_lazy_plugins_detects_commented_existing_lazy_plugins_import() {
        let vite_config = r#"import {
  lazyPlugins, // keep this comment
} from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(result.content.matches("from 'vite-plus'").count(), 1);
        assert!(result.content.contains("plugins: lazyPlugins(() => [react()])"));
    }

    #[test]
    fn test_wrap_lazy_plugins_reuses_later_lazy_plugins_import() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';
import { lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(result.content.matches("lazyPlugins } from 'vite-plus'").count(), 1);
        assert_eq!(result.content.matches("lazyPlugins").count(), 2);
    }

    #[test]
    fn test_wrap_lazy_plugins_callback_returns() {
        let vite_config = r#"import { defineConfig, loadEnv } from 'vite-plus';

export default defineConfig(({ mode }) => {
  function helper() {
    return { plugins: [helperPlugin()] };
  }
  return {
    plugins: [react()],
    nested: { plugins: [nestedPlugin()] },
  };
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert_eq!(result.content.matches("lazyPlugins(() =>").count(), 1);
        assert!(result.content.contains("import { defineConfig, loadEnv, lazyPlugins }"));
        assert!(result.content.contains("plugins: lazyPlugins(() => [react()])"));
        assert!(result.content.contains("return { plugins: [helperPlugin()] };"));
        assert!(result.content.contains("nested: { plugins: [nestedPlugin()] }"));
    }

    #[test]
    fn test_wrap_lazy_plugins_uses_async_callback_for_await() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig(async () => ({
  plugins: [await makePlugin()],
}));"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();

        assert!(result.updated);
        assert!(result.content.contains("plugins: lazyPlugins(async () => [await makePlugin()])"));
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_unsupported_and_is_idempotent() {
        let vite_config = r#"import { defineConfig, lazyPlugins } from 'vite-plus';

const plugins = [react()];

export default defineConfig({
  plugins,
  a: { plugins: [nested()] },
});"#;

        let result = wrap_lazy_plugins_content(vite_config, None).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, vite_config);

        let already_wrapped = r#"import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(() => [react()]),
});"#;
        let result = wrap_lazy_plugins_content(already_wrapped, None).unwrap();
        assert!(!result.updated);
        assert_eq!(result.content, already_wrapped);
    }

    #[test]
    fn test_wrap_lazy_plugins_skips_commonjs() {
        let vite_config = r#"const { defineConfig } = require('vite');

module.exports = defineConfig({
  plugins: [react()],
});"#;

        let result =
            wrap_lazy_plugins_content(vite_config, Some(Path::new("vite.config.cjs"))).unwrap();

        assert!(!result.updated);
        assert_eq!(result.content, vite_config);
    }

    #[test]
    fn test_wrap_lazy_plugins_idempotent_after_transform() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});"#;

        let first = wrap_lazy_plugins_content(vite_config, None).unwrap();
        assert!(first.updated);
        let second = wrap_lazy_plugins_content(&first.content, None).unwrap();
        assert!(!second.updated);
        assert_eq!(second.content, first.content);
    }

    #[test]
    fn test_merge_tsdown_config_content_simple() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

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

import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: tsdownConfig,
  plugins: [],
});"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_with_existing_imports() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';
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

import { defineConfig } from 'vite-plus';
import react from '@vitejs/plugin-react';

export default defineConfig({
  pack: tsdownConfig,
  plugins: [react()],
});"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_function_callback() {
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig((env) => ({
  plugins: [],
}));"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(result.uses_function_callback);
        assert_eq!(
            result.content,
            r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig((env) => ({
  pack: tsdownConfig,
  plugins: [],
}));"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_idempotent() {
        // Already migrated config - import at the beginning
        let already_migrated = r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: tsdownConfig,
  plugins: [],
});"#;

        let result = merge_tsdown_config_content(already_migrated, "./tsdown.config.ts").unwrap();
        assert!(!result.updated, "Should not update already migrated config");
        assert_eq!(result.content, already_migrated);

        // Run migration twice and verify no duplicates
        let fresh_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [],
});"#;

        let expected_migrated = r#"import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: tsdownConfig,
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
  pack: tsdownConfig,
  server: { port: 3000 }
}"#
        );
    }

    #[test]
    fn test_merge_tsdown_config_content_no_false_positive_stdlib() {
        // "stdlib:" should not be detected as "pack:" key
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({
  stdlib: 'some-value',
});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.ts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.js'"));
        assert!(result.content.contains("pack: tsdownConfig"));
        assert!(result.content.contains("stdlib: 'some-value'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_mts_extension() {
        // .mts files should use .mjs extension in imports
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.mts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.mjs'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_cts_extension() {
        // .cts files should use .cjs extension in imports
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.cts").unwrap();
        assert!(result.updated);
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.cjs'"));
    }

    #[test]
    fn test_merge_tsdown_config_content_js_extension_unchanged() {
        // .js, .mjs, .cjs files should keep their extensions unchanged
        let vite_config = r#"import { defineConfig } from 'vite-plus';

export default defineConfig({});"#;

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.js").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.js'"));

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.mjs").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.mjs'"));

        let result = merge_tsdown_config_content(vite_config, "./tsdown.config.cjs").unwrap();
        assert!(result.content.contains("import tsdownConfig from './tsdown.config.cjs'"));
    }
}
