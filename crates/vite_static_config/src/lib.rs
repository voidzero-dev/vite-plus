//! Static config extraction from vite.config.* files.
//!
//! Parses vite config files statically (without executing JavaScript) to extract
//! top-level fields whose values are pure JSON literals. This allows reading
//! config like `run` without needing a Node.js runtime.

use oxc::{
    ast::ast::{Expression, ObjectPropertyKind, Program, Statement},
    parser::Parser,
    span::SourceType,
};
use oxc_allocator::Allocator;
use rustc_hash::FxHashMap;
use vite_path::AbsolutePath;

/// The result of statically analyzing a single config field's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticFieldValue {
    /// The field value was successfully extracted as a JSON literal.
    Json(serde_json::Value),
    /// The field exists but its value is not a pure JSON literal (e.g. contains
    /// function calls, variables, template literals with expressions, etc.)
    NonStatic,
}

/// The result of statically analyzing a vite config file.
///
/// - `None` — the config is not analyzable (no config file found, parse error,
///   no `export default`, or the default export is not an object literal).
///   The caller should fall back to a runtime evaluation (e.g. NAPI).
/// - `Some(map)` — the default export object was successfully located.
///   - Key maps to [`StaticFieldValue::Json`] — field value was extracted.
///   - Key maps to [`StaticFieldValue::NonStatic`] — field exists but its value
///     cannot be represented as pure JSON.
///   - Key absent — the field does not exist in the object.
pub type StaticConfig = Option<FxHashMap<Box<str>, StaticFieldValue>>;

/// Config file names to try, in priority order.
/// This matches Vite's `DEFAULT_CONFIG_FILES`:
/// <https://github.com/vitejs/vite/blob/25227bbdc7de0ed07cf7bdc9a1a733e3a9a132bc/packages/vite/src/node/constants.ts#L98-L105>
///
/// Vite resolves config files by iterating this list and checking `fs.existsSync` — no
/// module resolution involved, so `oxc_resolver` is not needed here:
/// <https://github.com/vitejs/vite/blob/25227bbdc7de0ed07cf7bdc9a1a733e3a9a132bc/packages/vite/src/node/config.ts#L2231-L2237>
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
/// See [`StaticConfig`] for the return type semantics.
#[must_use]
pub fn resolve_static_config(dir: &AbsolutePath) -> StaticConfig {
    let config_path = resolve_config_path(dir)?;
    let source = std::fs::read_to_string(&config_path).ok()?;

    let extension = config_path.as_path().extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "json" {
        return parse_json_config(&source);
    }

    parse_js_ts_config(&source, extension)
}

/// Parse a JSON config file into a map of field names to values.
/// All fields in a valid JSON object are fully static.
fn parse_json_config(source: &str) -> StaticConfig {
    let value: serde_json::Value = serde_json::from_str(source).ok()?;
    let obj = value.as_object()?;
    Some(
        obj.iter()
            .map(|(k, v)| (Box::from(k.as_str()), StaticFieldValue::Json(v.clone())))
            .collect(),
    )
}

/// Parse a JS/TS config file, extracting the default export object's fields.
fn parse_js_ts_config(source: &str, extension: &str) -> StaticConfig {
    let allocator = Allocator::default();
    let source_type = match extension {
        "ts" | "mts" | "cts" => SourceType::ts(),
        _ => SourceType::mjs(),
    };

    let parser = Parser::new(&allocator, source, source_type);
    let result = parser.parse();

    if result.panicked || !result.errors.is_empty() {
        return None;
    }

    extract_config_fields(&result.program)
}

/// Find the config object in a parsed program and extract its fields.
///
/// Searches for the config value in the following patterns (in order):
/// 1. `export default defineConfig({ ... })`
/// 2. `export default { ... }`
/// 3. `module.exports = defineConfig({ ... })`
/// 4. `module.exports = { ... }`
fn extract_config_fields(program: &Program<'_>) -> StaticConfig {
    for stmt in &program.body {
        // ESM: export default ...
        if let Statement::ExportDefaultDeclaration(decl) = stmt {
            if let Some(expr) = decl.declaration.as_expression() {
                return extract_config_from_expr(expr);
            }
            // export default class/function — not analyzable
            return None;
        }

        // CJS: module.exports = ...
        if let Statement::ExpressionStatement(expr_stmt) = stmt
            && let Expression::AssignmentExpression(assign) = &expr_stmt.expression
            && assign.left.as_member_expression().is_some_and(|m| {
                m.object().is_specific_id("module") && m.static_property_name() == Some("exports")
            })
        {
            return extract_config_from_expr(&assign.right);
        }
    }

    None
}

/// Extract the config object from an expression that is either:
/// - `defineConfig({ ... })` → extract the object argument
/// - `{ ... }` → extract directly
/// - anything else → not analyzable
fn extract_config_from_expr(expr: &Expression<'_>) -> StaticConfig {
    let expr = expr.without_parentheses();
    match expr {
        Expression::CallExpression(call) => {
            if !call.callee.is_specific_id("defineConfig") {
                return None;
            }
            if let Some(first_arg) = call.arguments.first()
                && let Some(Expression::ObjectExpression(obj)) = first_arg.as_expression()
            {
                return Some(extract_object_fields(obj));
            }
            None
        }
        Expression::ObjectExpression(obj) => Some(extract_object_fields(obj)),
        _ => None,
    }
}

/// Extract fields from an object expression, converting each value to JSON.
/// Fields whose values cannot be represented as pure JSON are recorded as
/// [`StaticFieldValue::NonStatic`]. Spread elements and computed properties
/// are not representable so they are silently skipped (their keys are unknown).
fn extract_object_fields(
    obj: &oxc::ast::ast::ObjectExpression<'_>,
) -> FxHashMap<Box<str>, StaticFieldValue> {
    let mut map = FxHashMap::default();

    for prop in &obj.properties {
        if prop.is_spread() {
            // Spread elements — keys are unknown at static analysis time
            continue;
        }
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let Some(key) = prop.key.static_name() else {
            // Computed properties — keys are unknown at static analysis time
            continue;
        };

        let value =
            expr_to_json(&prop.value).map_or(StaticFieldValue::NonStatic, StaticFieldValue::Json);
        map.insert(Box::from(key.as_ref()), value);
    }

    map
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
            let quasi = lit.single_quasi()?;
            Some(serde_json::Value::String(quasi.to_string()))
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
                if elem.is_elision() {
                    values.push(serde_json::Value::Null);
                } else if elem.is_spread() {
                    return None;
                } else {
                    let elem_expr = elem.as_expression()?;
                    values.push(expr_to_json(elem_expr)?);
                }
            }
            Some(serde_json::Value::Array(values))
        }

        Expression::ObjectExpression(obj) => {
            let mut map = serde_json::Map::new();
            for prop in &obj.properties {
                if prop.is_spread() {
                    return None;
                }
                let ObjectPropertyKind::ObjectProperty(prop) = prop else {
                    continue;
                };
                let key = prop.key.static_name()?;
                let value = expr_to_json(&prop.value)?;
                map.insert(key.into_owned(), value);
            }
            Some(serde_json::Value::Object(map))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Helper: parse JS/TS source, unwrap the `Some` (asserting it's analyzable),
    /// and return the field map.
    fn parse(source: &str) -> FxHashMap<Box<str>, StaticFieldValue> {
        parse_js_ts_config(source, "ts").expect("expected analyzable config")
    }

    /// Shorthand for asserting a field extracted as JSON.
    fn assert_json(
        map: &FxHashMap<Box<str>, StaticFieldValue>,
        key: &str,
        expected: serde_json::Value,
    ) {
        assert_eq!(map.get(key), Some(&StaticFieldValue::Json(expected)));
    }

    /// Shorthand for asserting a field is `NonStatic`.
    fn assert_non_static(map: &FxHashMap<Box<str>, StaticFieldValue>, key: &str) {
        assert_eq!(
            map.get(key),
            Some(&StaticFieldValue::NonStatic),
            "expected field {key:?} to be NonStatic"
        );
    }

    // ── Config file resolution ──────────────────────────────────────────

    #[test]
    fn resolves_ts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.ts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path).unwrap();
        assert!(result.contains_key("run"));
    }

    #[test]
    fn resolves_js_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.js"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path).unwrap();
        assert!(result.contains_key("run"));
    }

    #[test]
    fn resolves_mts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.mts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path).unwrap();
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
        let result = resolve_static_config(&dir_path).unwrap();
        assert!(result.contains_key("fromJs"));
        assert!(!result.contains_key("fromTs"));
    }

    #[test]
    fn returns_none_for_no_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        assert!(resolve_static_config(&dir_path).is_none());
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
        let result = resolve_static_config(&dir_path).unwrap();
        assert_json(
            &result,
            "run",
            serde_json::json!({ "tasks": { "build": { "command": "echo hello" } } }),
        );
    }

    // ── export default { ... } ──────────────────────────────────────────

    #[test]
    fn plain_export_default_object() {
        let result = parse("export default { foo: 'bar', num: 42 }");
        assert_json(&result, "foo", serde_json::json!("bar"));
        assert_json(&result, "num", serde_json::json!(42));
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
            r"
            import { defineConfig } from 'vite-plus';
            export default defineConfig({
                run: { cacheScripts: true },
                lint: { plugins: ['a'] },
            });
            ",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
        assert_json(&result, "lint", serde_json::json!({ "plugins": ["a"] }));
    }

    // ── module.exports = { ... } ───────────────────────────────────────

    #[test]
    fn module_exports_object() {
        let result = parse_js_ts_config("module.exports = { run: { cache: true } }", "cjs")
            .expect("expected analyzable config");
        assert_json(&result, "run", serde_json::json!({ "cache": true }));
    }

    #[test]
    fn module_exports_define_config() {
        let result = parse_js_ts_config(
            r"
            const { defineConfig } = require('vite-plus');
            module.exports = defineConfig({
                run: { cacheScripts: true },
            });
            ",
            "cjs",
        )
        .expect("expected analyzable config");
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
    }

    #[test]
    fn module_exports_non_object() {
        assert!(parse_js_ts_config("module.exports = 42;", "cjs").is_none());
    }

    #[test]
    fn module_exports_unknown_call() {
        assert!(parse_js_ts_config("module.exports = otherFn({ a: 1 });", "cjs").is_none());
    }

    // ── Primitive values ────────────────────────────────────────────────

    #[test]
    fn string_values() {
        let result = parse(r#"export default { a: "double", b: 'single' }"#);
        assert_json(&result, "a", serde_json::json!("double"));
        assert_json(&result, "b", serde_json::json!("single"));
    }

    #[test]
    fn numeric_values() {
        let result = parse("export default { a: 42, b: 1.5, c: 0, d: -1 }");
        assert_json(&result, "a", serde_json::json!(42));
        assert_json(&result, "b", serde_json::json!(1.5));
        assert_json(&result, "c", serde_json::json!(0));
        assert_json(&result, "d", serde_json::json!(-1));
    }

    #[test]
    fn boolean_values() {
        let result = parse("export default { a: true, b: false }");
        assert_json(&result, "a", serde_json::json!(true));
        assert_json(&result, "b", serde_json::json!(false));
    }

    #[test]
    fn null_value() {
        let result = parse("export default { a: null }");
        assert_json(&result, "a", serde_json::Value::Null);
    }

    // ── Arrays ──────────────────────────────────────────────────────────

    #[test]
    fn array_of_strings() {
        let result = parse("export default { items: ['a', 'b', 'c'] }");
        assert_json(&result, "items", serde_json::json!(["a", "b", "c"]));
    }

    #[test]
    fn nested_arrays() {
        let result = parse("export default { matrix: [[1, 2], [3, 4]] }");
        assert_json(&result, "matrix", serde_json::json!([[1, 2], [3, 4]]));
    }

    #[test]
    fn empty_array() {
        let result = parse("export default { items: [] }");
        assert_json(&result, "items", serde_json::json!([]));
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
        assert_json(
            &result,
            "run",
            serde_json::json!({
                "tasks": {
                    "build": {
                        "command": "echo build",
                        "dependsOn": ["lint"],
                        "cache": true,
                    }
                }
            }),
        );
    }

    // ── NonStatic fields ────────────────────────────────────────────────

    #[test]
    fn non_static_function_call_values() {
        let result = parse(
            r"export default {
                run: { cacheScripts: true },
                plugins: [myPlugin()],
            }",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
        assert_non_static(&result, "plugins");
    }

    #[test]
    fn non_static_identifier_values() {
        let result = parse(
            r"
            const myVar = 'hello';
            export default { a: myVar, b: 42 }
            ",
        );
        assert_non_static(&result, "a");
        assert_json(&result, "b", serde_json::json!(42));
    }

    #[test]
    fn non_static_template_literal_with_expressions() {
        let result = parse(
            r"
            const x = 'world';
            export default { a: `hello ${x}`, b: 'plain' }
            ",
        );
        assert_non_static(&result, "a");
        assert_json(&result, "b", serde_json::json!("plain"));
    }

    #[test]
    fn keeps_pure_template_literal() {
        let result = parse("export default { a: `hello` }");
        assert_json(&result, "a", serde_json::json!("hello"));
    }

    #[test]
    fn non_static_spread_in_object_value() {
        let result = parse(
            r"
            const base = { x: 1 };
            export default { a: { ...base, y: 2 }, b: 'ok' }
            ",
        );
        assert_non_static(&result, "a");
        assert_json(&result, "b", serde_json::json!("ok"));
    }

    #[test]
    fn spread_in_top_level_skipped() {
        let result = parse(
            r"
            const base = { x: 1 };
            export default { ...base, b: 'ok' }
            ",
        );
        // Spread at top level — keys unknown, so not in map at all
        assert!(!result.contains_key("x"));
        assert_json(&result, "b", serde_json::json!("ok"));
    }

    #[test]
    fn computed_properties_skipped() {
        let result = parse(
            r"
            const key = 'dynamic';
            export default { [key]: 'value', plain: 'ok' }
            ",
        );
        // Computed key — not in map at all (key is unknown)
        assert!(!result.contains_key("dynamic"));
        assert_json(&result, "plain", serde_json::json!("ok"));
    }

    #[test]
    fn non_static_array_with_spread() {
        let result = parse(
            r"
            const arr = [1, 2];
            export default { a: [...arr, 3], b: 'ok' }
            ",
        );
        assert_non_static(&result, "a");
        assert_json(&result, "b", serde_json::json!("ok"));
    }

    // ── Property key types ──────────────────────────────────────────────

    #[test]
    fn string_literal_keys() {
        let result = parse(r"export default { 'string-key': 42 }");
        assert_json(&result, "string-key", serde_json::json!(42));
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
        assert_json(
            &result,
            "run",
            serde_json::json!({
                "tasks": {
                    "build": {
                        "command": "echo 'build from vite.config.ts'",
                        "dependsOn": [],
                    }
                }
            }),
        );
    }

    #[test]
    fn real_world_with_non_json_fields() {
        let result = parse(
            r"
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
            ",
        );
        assert_json(
            &result,
            "lint",
            serde_json::json!({
                "plugins": ["unicorn", "typescript"],
                "rules": {
                    "no-console": ["error", { "allow": ["error"] }],
                },
            }),
        );
        assert_json(
            &result,
            "run",
            serde_json::json!({
                "tasks": {
                    "build:src": {
                        "command": "vp run rolldown#build-binding:release",
                    }
                }
            }),
        );
    }

    #[test]
    fn skips_non_default_exports() {
        let result = parse(
            r"
            export const config = { a: 1 };
            export default { b: 2 };
            ",
        );
        assert!(!result.contains_key("a"));
        assert_json(&result, "b", serde_json::json!(2));
    }

    // ── Not analyzable cases (return None) ──────────────────────────────

    #[test]
    fn returns_none_for_no_default_export() {
        assert!(parse_js_ts_config("export const config = { a: 1 };", "ts").is_none());
    }

    #[test]
    fn returns_none_for_non_object_default_export() {
        assert!(parse_js_ts_config("export default 42;", "ts").is_none());
    }

    #[test]
    fn returns_none_for_unknown_function_call() {
        assert!(parse_js_ts_config("export default someOtherFn({ a: 1 });", "ts").is_none());
    }

    #[test]
    fn handles_trailing_commas() {
        let result = parse(
            r"export default {
                a: [1, 2, 3,],
                b: { x: 1, y: 2, },
            }",
        );
        assert_json(&result, "a", serde_json::json!([1, 2, 3]));
        assert_json(&result, "b", serde_json::json!({ "x": 1, "y": 2 }));
    }

    #[test]
    fn task_with_cache_config() {
        let result = parse(
            r"export default {
                run: {
                    tasks: {
                        hello: {
                            command: 'node hello.mjs',
                            envs: ['FOO', 'BAR'],
                            cache: true,
                        },
                    },
                },
            }",
        );
        assert_json(
            &result,
            "run",
            serde_json::json!({
                "tasks": {
                    "hello": {
                        "command": "node hello.mjs",
                        "envs": ["FOO", "BAR"],
                        "cache": true,
                    }
                }
            }),
        );
    }

    #[test]
    fn non_static_method_call_in_nested_value() {
        let result = parse(
            r"export default {
                run: {
                    tasks: {
                        'build:src': {
                            command: ['cmd1', 'cmd2'].join(' && '),
                        },
                    },
                },
                lint: { plugins: ['a'] },
            }",
        );
        // `run` is NonStatic because its nested value contains a method call
        assert_non_static(&result, "run");
        assert_json(&result, "lint", serde_json::json!({ "plugins": ["a"] }));
    }

    #[test]
    fn cache_scripts_only() {
        let result = parse(
            r"export default {
                run: {
                    cacheScripts: true,
                },
            }",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
    }
}
