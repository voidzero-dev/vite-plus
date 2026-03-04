//! Static config extraction from vite.config.* files.
//!
//! Parses vite config files statically (without executing JavaScript) to extract
//! top-level fields whose values are pure JSON literals. This allows reading
//! config like `run` without needing a Node.js runtime.

use oxc::{
    ast::ast::{
        ArrayExpressionElement, Expression, ObjectPropertyKind, Program, PropertyKey, Statement,
    },
    parser::Parser,
    span::SourceType,
};
use oxc_allocator::Allocator;
use rustc_hash::FxHashMap;
use vite_path::AbsolutePath;

/// Config file names to try, in priority order.
/// This matches Vite's `DEFAULT_CONFIG_FILES` order.
const CONFIG_FILE_NAMES: &[&str] = &[
    "vite.config.js",
    "vite.config.mjs",
    "vite.config.ts",
    "vite.config.cjs",
    "vite.config.mts",
    "vite.config.cts",
];

/// Resolve the vite config file path in the given directory.
///
/// Tries each config file name in priority order and returns the first one that exists.
fn resolve_config_path(dir: &AbsolutePath) -> Option<vite_path::AbsolutePathBuf> {
    for name in CONFIG_FILE_NAMES {
        let path = dir.join(name);
        if path.as_path().exists() {
            return Some(path);
        }
    }
    None
}

/// Resolve and parse a vite config file from the given directory.
///
/// Returns a map of top-level field names to their JSON values for fields
/// whose values are pure JSON literals. Fields with non-JSON values (function calls,
/// variables, template literals, etc.) are skipped.
///
/// # Arguments
/// * `dir` - The directory to search for a vite config file
///
/// # Returns
/// A map of field name to JSON value for all statically extractable fields.
/// Returns an empty map if no config file is found or if it cannot be parsed.
#[must_use]
pub fn resolve_static_config(dir: &AbsolutePath) -> FxHashMap<Box<str>, serde_json::Value> {
    let Some(config_path) = resolve_config_path(dir) else {
        return FxHashMap::default();
    };

    let Ok(source) = std::fs::read_to_string(&config_path) else {
        return FxHashMap::default();
    };

    let extension = config_path.as_path().extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "json" {
        return parse_json_config(&source);
    }

    parse_js_ts_config(&source, extension)
}

/// Parse a JSON config file into a map of field names to values.
fn parse_json_config(source: &str) -> FxHashMap<Box<str>, serde_json::Value> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(source) else {
        return FxHashMap::default();
    };
    let Some(obj) = value.as_object() else {
        return FxHashMap::default();
    };
    obj.iter().map(|(k, v)| (Box::from(k.as_str()), v.clone())).collect()
}

/// Parse a JS/TS config file, extracting the default export object's fields.
fn parse_js_ts_config(source: &str, extension: &str) -> FxHashMap<Box<str>, serde_json::Value> {
    let allocator = Allocator::default();
    let source_type = match extension {
        "ts" | "mts" | "cts" => SourceType::ts(),
        _ => SourceType::mjs(),
    };

    let parser = Parser::new(&allocator, source, source_type);
    let result = parser.parse();

    if result.panicked || !result.errors.is_empty() {
        return FxHashMap::default();
    }

    extract_default_export_fields(&result.program)
}

/// Find the default export in a parsed program and extract its object fields.
///
/// Supports two patterns:
/// 1. `export default defineConfig({ ... })`
/// 2. `export default { ... }`
fn extract_default_export_fields(program: &Program<'_>) -> FxHashMap<Box<str>, serde_json::Value> {
    for stmt in &program.body {
        let Statement::ExportDefaultDeclaration(decl) = stmt else {
            continue;
        };

        let Some(expr) = decl.declaration.as_expression() else {
            continue;
        };

        // Unwrap parenthesized expressions
        let expr = expr.without_parentheses();

        match expr {
            // Pattern: export default defineConfig({ ... })
            Expression::CallExpression(call) => {
                if !is_define_config_call(&call.callee) {
                    continue;
                }
                if let Some(first_arg) = call.arguments.first()
                    && let Some(Expression::ObjectExpression(obj)) = first_arg.as_expression()
                {
                    return extract_object_fields(obj);
                }
            }
            // Pattern: export default { ... }
            Expression::ObjectExpression(obj) => {
                return extract_object_fields(obj);
            }
            _ => {}
        }
    }

    FxHashMap::default()
}

/// Check if a callee expression is `defineConfig`.
fn is_define_config_call(callee: &Expression<'_>) -> bool {
    matches!(callee, Expression::Identifier(ident) if ident.name == "defineConfig")
}

/// Extract fields from an object expression, converting each value to JSON.
/// Fields whose values cannot be represented as pure JSON are skipped.
fn extract_object_fields(
    obj: &oxc::ast::ast::ObjectExpression<'_>,
) -> FxHashMap<Box<str>, serde_json::Value> {
    let mut map = FxHashMap::default();

    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            // Skip spread elements
            continue;
        };

        // Skip computed properties
        if prop.computed {
            continue;
        }

        let Some(key) = property_key_to_string(&prop.key) else {
            continue;
        };

        if let Some(value) = expr_to_json(&prop.value) {
            map.insert(key, value);
        }
    }

    map
}

/// Convert a property key to a string.
fn property_key_to_string(key: &PropertyKey<'_>) -> Option<Box<str>> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(Box::from(ident.name.as_str())),
        PropertyKey::StringLiteral(lit) => Some(Box::from(lit.value.as_str())),
        PropertyKey::NumericLiteral(lit) => {
            let s = if lit.value.fract() == 0.0 && lit.value.is_finite() {
                #[expect(clippy::cast_possible_truncation)]
                {
                    (lit.value as i64).to_string()
                }
            } else {
                lit.value.to_string()
            };
            Some(Box::from(s.as_str()))
        }
        _ => None,
    }
}

/// Convert an f64 to a JSON value, preserving integers when possible.
#[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn f64_to_json_number(value: f64) -> serde_json::Value {
    // If the value is a whole number that fits in i64, use integer representation
    if value.fract() == 0.0
        && value.is_finite()
        && value >= i64::MIN as f64
        && value <= i64::MAX as f64
    {
        serde_json::Value::Number(serde_json::Number::from(value as i64))
    } else if let Some(n) = serde_json::Number::from_f64(value) {
        serde_json::Value::Number(n)
    } else {
        serde_json::Value::Null
    }
}

/// Try to convert an AST expression to a JSON value.
///
/// Returns `None` if the expression contains non-JSON-literal nodes
/// (function calls, identifiers, template literals, etc.)
fn expr_to_json(expr: &Expression<'_>) -> Option<serde_json::Value> {
    let expr = expr.without_parentheses();
    match expr {
        Expression::NullLiteral(_) => Some(serde_json::Value::Null),

        Expression::BooleanLiteral(lit) => Some(serde_json::Value::Bool(lit.value)),

        Expression::NumericLiteral(lit) => Some(f64_to_json_number(lit.value)),

        Expression::StringLiteral(lit) => Some(serde_json::Value::String(lit.value.to_string())),

        Expression::TemplateLiteral(lit) => {
            // Only convert template literals with no expressions (pure strings)
            if lit.expressions.is_empty() && lit.quasis.len() == 1 {
                let raw = &lit.quasis[0].value.cooked.as_ref()?;
                Some(serde_json::Value::String(raw.to_string()))
            } else {
                None
            }
        }

        Expression::UnaryExpression(unary) => {
            // Handle negative numbers: -42
            if unary.operator == oxc::ast::ast::UnaryOperator::UnaryNegation
                && let Expression::NumericLiteral(lit) = &unary.argument
            {
                return Some(f64_to_json_number(-lit.value));
            }
            None
        }

        Expression::ArrayExpression(arr) => {
            let mut values = Vec::with_capacity(arr.elements.len());
            for elem in &arr.elements {
                match elem {
                    ArrayExpressionElement::Elision(_) => {
                        values.push(serde_json::Value::Null);
                    }
                    ArrayExpressionElement::SpreadElement(_) => {
                        return None;
                    }
                    _ => {
                        let elem_expr = elem.as_expression()?;
                        values.push(expr_to_json(elem_expr)?);
                    }
                }
            }
            Some(serde_json::Value::Array(values))
        }

        Expression::ObjectExpression(obj) => {
            let mut map = serde_json::Map::new();
            for prop in &obj.properties {
                match prop {
                    ObjectPropertyKind::ObjectProperty(prop) => {
                        if prop.computed {
                            return None;
                        }
                        let key = property_key_to_json_key(&prop.key)?;
                        let value = expr_to_json(&prop.value)?;
                        map.insert(key, value);
                    }
                    ObjectPropertyKind::SpreadProperty(_) => {
                        return None;
                    }
                }
            }
            Some(serde_json::Value::Object(map))
        }

        _ => None,
    }
}

/// Convert a property key to a JSON-compatible string key.
#[expect(clippy::disallowed_types)]
fn property_key_to_json_key(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(ident) => Some(ident.name.to_string()),
        PropertyKey::StringLiteral(lit) => Some(lit.value.to_string()),
        PropertyKey::NumericLiteral(lit) => {
            if lit.value.fract() == 0.0 && lit.value.is_finite() {
                #[expect(clippy::cast_possible_truncation)]
                {
                    Some((lit.value as i64).to_string())
                }
            } else {
                Some(lit.value.to_string())
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn parse(source: &str) -> FxHashMap<Box<str>, serde_json::Value> {
        parse_js_ts_config(source, "ts")
    }

    // ── Config file resolution ──────────────────────────────────────────

    #[test]
    fn resolves_ts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.ts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.contains_key("run"));
    }

    #[test]
    fn resolves_js_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.js"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.contains_key("run"));
    }

    #[test]
    fn resolves_mts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.mts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.contains_key("run"));
    }

    #[test]
    fn js_takes_priority_over_ts() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.ts"), "export default { fromTs: true }")
            .unwrap();
        std::fs::write(dir.path().join("vite.config.js"), "export default { fromJs: true }")
            .unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.contains_key("fromJs"));
        assert!(!result.contains_key("fromTs"));
    }

    #[test]
    fn returns_empty_for_no_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.is_empty());
    }

    // ── JSON config parsing ─────────────────────────────────────────────

    #[test]
    fn parses_json_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(
            dir.path().join("vite.config.ts"),
            r#"export default { run: { tasks: { build: { command: "echo hello" } } } }"#,
        )
        .unwrap();
        let result = resolve_static_config(&dir_path);
        let run = result.get("run").unwrap();
        assert_eq!(run, &serde_json::json!({ "tasks": { "build": { "command": "echo hello" } } }));
    }

    // ── export default { ... } ──────────────────────────────────────────

    #[test]
    fn plain_export_default_object() {
        let result = parse("export default { foo: 'bar', num: 42 }");
        assert_eq!(result.get("foo").unwrap(), &serde_json::json!("bar"));
        assert_eq!(result.get("num").unwrap(), &serde_json::json!(42));
    }

    #[test]
    fn export_default_empty_object() {
        let result = parse("export default {}");
        assert!(result.is_empty());
    }

    // ── export default defineConfig({ ... }) ────────────────────────────

    #[test]
    fn define_config_call() {
        let result = parse(
            r#"
            import { defineConfig } from 'vite-plus';
            export default defineConfig({
                run: { cacheScripts: true },
                lint: { plugins: ['a'] },
            });
            "#,
        );
        assert_eq!(result.get("run").unwrap(), &serde_json::json!({ "cacheScripts": true }));
        assert_eq!(result.get("lint").unwrap(), &serde_json::json!({ "plugins": ["a"] }));
    }

    // ── Primitive values ────────────────────────────────────────────────

    #[test]
    fn string_values() {
        let result = parse(r#"export default { a: "double", b: 'single' }"#);
        assert_eq!(result.get("a").unwrap(), &serde_json::json!("double"));
        assert_eq!(result.get("b").unwrap(), &serde_json::json!("single"));
    }

    #[test]
    fn numeric_values() {
        let result = parse("export default { a: 42, b: 3.14, c: 0, d: -1 }");
        assert_eq!(result.get("a").unwrap(), &serde_json::json!(42));
        assert_eq!(result.get("b").unwrap(), &serde_json::json!(3.14));
        assert_eq!(result.get("c").unwrap(), &serde_json::json!(0));
        assert_eq!(result.get("d").unwrap(), &serde_json::json!(-1));
    }

    #[test]
    fn boolean_values() {
        let result = parse("export default { a: true, b: false }");
        assert_eq!(result.get("a").unwrap(), &serde_json::json!(true));
        assert_eq!(result.get("b").unwrap(), &serde_json::json!(false));
    }

    #[test]
    fn null_value() {
        let result = parse("export default { a: null }");
        assert_eq!(result.get("a").unwrap(), &serde_json::Value::Null);
    }

    // ── Arrays ──────────────────────────────────────────────────────────

    #[test]
    fn array_of_strings() {
        let result = parse("export default { items: ['a', 'b', 'c'] }");
        assert_eq!(result.get("items").unwrap(), &serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn nested_arrays() {
        let result = parse("export default { matrix: [[1, 2], [3, 4]] }");
        assert_eq!(result.get("matrix").unwrap(), &serde_json::json!([[1, 2], [3, 4]]));
    }

    #[test]
    fn empty_array() {
        let result = parse("export default { items: [] }");
        assert_eq!(result.get("items").unwrap(), &serde_json::json!([]));
    }

    // ── Nested objects ──────────────────────────────────────────────────

    #[test]
    fn nested_object() {
        let result = parse(
            r#"export default {
                run: {
                    tasks: {
                        build: {
                            command: "echo build",
                            dependsOn: ["lint"],
                            cache: true,
                        }
                    }
                }
            }"#,
        );
        assert_eq!(
            result.get("run").unwrap(),
            &serde_json::json!({
                "tasks": {
                    "build": {
                        "command": "echo build",
                        "dependsOn": ["lint"],
                        "cache": true,
                    }
                }
            })
        );
    }

    // ── Skipping non-JSON fields ────────────────────────────────────────

    #[test]
    fn skips_function_call_values() {
        let result = parse(
            r#"export default {
                run: { cacheScripts: true },
                plugins: [myPlugin()],
            }"#,
        );
        assert!(result.contains_key("run"));
        assert!(!result.contains_key("plugins"));
    }

    #[test]
    fn skips_identifier_values() {
        let result = parse(
            r#"
            const myVar = 'hello';
            export default { a: myVar, b: 42 }
            "#,
        );
        assert!(!result.contains_key("a"));
        assert!(result.contains_key("b"));
    }

    #[test]
    fn skips_template_literal_with_expressions() {
        let result = parse(
            r#"
            const x = 'world';
            export default { a: `hello ${x}`, b: 'plain' }
            "#,
        );
        assert!(!result.contains_key("a"));
        assert!(result.contains_key("b"));
    }

    #[test]
    fn keeps_pure_template_literal() {
        let result = parse("export default { a: `hello` }");
        assert_eq!(result.get("a").unwrap(), &serde_json::json!("hello"));
    }

    #[test]
    fn skips_spread_in_object_value() {
        let result = parse(
            r#"
            const base = { x: 1 };
            export default { a: { ...base, y: 2 }, b: 'ok' }
            "#,
        );
        assert!(!result.contains_key("a"));
        assert!(result.contains_key("b"));
    }

    #[test]
    fn skips_spread_in_top_level() {
        let result = parse(
            r#"
            const base = { x: 1 };
            export default { ...base, b: 'ok' }
            "#,
        );
        // Spread at top level is skipped; plain fields are kept
        assert!(!result.contains_key("x"));
        assert!(result.contains_key("b"));
    }

    #[test]
    fn skips_computed_properties() {
        let result = parse(
            r#"
            const key = 'dynamic';
            export default { [key]: 'value', plain: 'ok' }
            "#,
        );
        assert!(!result.contains_key("dynamic"));
        assert!(result.contains_key("plain"));
    }

    #[test]
    fn skips_array_with_spread() {
        let result = parse(
            r#"
            const arr = [1, 2];
            export default { a: [...arr, 3], b: 'ok' }
            "#,
        );
        assert!(!result.contains_key("a"));
        assert!(result.contains_key("b"));
    }

    // ── Property key types ──────────────────────────────────────────────

    #[test]
    fn string_literal_keys() {
        let result = parse(r#"export default { 'string-key': 42 }"#);
        assert_eq!(result.get("string-key").unwrap(), &serde_json::json!(42));
    }

    // ── Real-world patterns ─────────────────────────────────────────────

    #[test]
    fn real_world_run_config() {
        let result = parse(
            r#"
            export default {
                run: {
                    tasks: {
                        build: {
                            command: "echo 'build from vite.config.ts'",
                            dependsOn: [],
                        },
                    },
                },
            };
            "#,
        );
        assert_eq!(
            result.get("run").unwrap(),
            &serde_json::json!({
                "tasks": {
                    "build": {
                        "command": "echo 'build from vite.config.ts'",
                        "dependsOn": [],
                    }
                }
            })
        );
    }

    #[test]
    fn real_world_with_non_json_fields() {
        let result = parse(
            r#"
            import { defineConfig } from 'vite-plus';

            export default defineConfig({
                lint: {
                    plugins: ['unicorn', 'typescript'],
                    rules: {
                        'no-console': ['error', { allow: ['error'] }],
                    },
                },
                run: {
                    tasks: {
                        'build:src': {
                            command: 'vp run rolldown#build-binding:release',
                        },
                    },
                },
            });
            "#,
        );
        assert!(result.contains_key("lint"));
        assert!(result.contains_key("run"));
        assert_eq!(
            result.get("run").unwrap(),
            &serde_json::json!({
                "tasks": {
                    "build:src": {
                        "command": "vp run rolldown#build-binding:release",
                    }
                }
            })
        );
    }

    #[test]
    fn skips_non_default_exports() {
        let result = parse(
            r#"
            export const config = { a: 1 };
            export default { b: 2 };
            "#,
        );
        assert!(!result.contains_key("a"));
        assert!(result.contains_key("b"));
    }

    #[test]
    fn returns_empty_for_no_default_export() {
        let result = parse("export const config = { a: 1 };");
        assert!(result.is_empty());
    }

    #[test]
    fn returns_empty_for_non_object_default_export() {
        let result = parse("export default 42;");
        assert!(result.is_empty());
    }

    #[test]
    fn returns_empty_for_unknown_function_call() {
        let result = parse("export default someOtherFn({ a: 1 });");
        assert!(result.is_empty());
    }

    #[test]
    fn handles_trailing_commas() {
        let result = parse(
            r#"export default {
                a: [1, 2, 3,],
                b: { x: 1, y: 2, },
            }"#,
        );
        assert_eq!(result.get("a").unwrap(), &serde_json::json!([1, 2, 3]));
        assert_eq!(result.get("b").unwrap(), &serde_json::json!({ "x": 1, "y": 2 }));
    }

    #[test]
    fn task_with_cache_config() {
        let result = parse(
            r#"export default {
                run: {
                    tasks: {
                        hello: {
                            command: 'node hello.mjs',
                            envs: ['FOO', 'BAR'],
                            cache: true,
                        },
                    },
                },
            }"#,
        );
        assert_eq!(
            result.get("run").unwrap(),
            &serde_json::json!({
                "tasks": {
                    "hello": {
                        "command": "node hello.mjs",
                        "envs": ["FOO", "BAR"],
                        "cache": true,
                    }
                }
            })
        );
    }

    #[test]
    fn skips_method_call_in_nested_value() {
        let result = parse(
            r#"export default {
                run: {
                    tasks: {
                        'build:src': {
                            command: ['cmd1', 'cmd2'].join(' && '),
                        },
                    },
                },
                lint: { plugins: ['a'] },
            }"#,
        );
        // `run` should be skipped because its nested value contains a method call
        assert!(!result.contains_key("run"));
        // `lint` is pure JSON and should be kept
        assert!(result.contains_key("lint"));
    }

    #[test]
    fn cache_scripts_only() {
        let result = parse(
            r#"export default {
                run: {
                    cacheScripts: true,
                },
            }"#,
        );
        assert_eq!(result.get("run").unwrap(), &serde_json::json!({ "cacheScripts": true }));
    }
}
