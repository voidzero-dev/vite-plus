//! Static config extraction from vite.config.* files.
//!
//! Parses vite config files statically (without executing JavaScript) to extract
//! top-level fields whose values are pure JSON literals. This allows reading
//! config like `run` without needing a Node.js runtime.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind, Program, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use vite_path::AbsolutePath;

/// The result of statically analyzing a single config field's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// The field value was successfully extracted as a JSON literal.
    Json(serde_json::Value),
    /// The field exists but its value is not a pure JSON literal (e.g. contains
    /// function calls, variables, template literals with expressions, etc.)
    NonStatic,
}

/// Internal representation of extracted object fields.
///
/// Two variants model the closed-world vs open-world assumption:
///
/// - [`FieldMapInner::Closed`] — the object had no spreads or computed-key properties.
///   The map is exhaustive: every field is accounted for, absent keys do not exist.
///
/// - [`FieldMapInner::Open`] — the object had at least one spread or computed-key
///   property. The map contains only [`serde_json::Value`] entries for keys
///   explicitly declared **after** the last such entry. Absent keys may exist via
///   the spread and are treated as [`FieldValue::NonStatic`] by [`FieldMap::get`].
enum FieldMapInner {
    Closed(FxHashMap<Box<str>, FieldValue>),
    Open(FxHashMap<Box<str>, serde_json::Value>),
}

/// Extracted fields from a vite config object.
pub struct FieldMap(FieldMapInner);

impl FieldMap {
    /// Returns an open empty map — used when the config is not analyzable.
    /// `get()` returns `Some(NonStatic)` for any key, triggering NAPI fallback.
    fn unanalyzable() -> Self {
        Self(FieldMapInner::Open(FxHashMap::default()))
    }

    /// Returns a closed empty map — used when no config file exists.
    /// `get()` returns `None` for any key (field definitively absent).
    fn no_config() -> Self {
        Self(FieldMapInner::Closed(FxHashMap::default()))
    }

    /// Look up a field by name.
    ///
    /// - [`Closed`](FieldMapInner::Closed): returns the stored value, or `None`
    ///   if the field is definitively absent.
    /// - [`Open`](FieldMapInner::Open): returns the stored `Json` value if
    ///   explicitly declared after the last spread/computed key, or
    ///   `Some(NonStatic)` for any other key (it may exist in the spread).
    #[must_use]
    pub fn get(&self, key: &str) -> Option<FieldValue> {
        match &self.0 {
            FieldMapInner::Closed(map) => map.get(key).cloned(),
            FieldMapInner::Open(map) => {
                Some(map.get(key).map_or(FieldValue::NonStatic, |v| FieldValue::Json(v.clone())))
            }
        }
    }
}

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
/// Returns a [`FieldMap`]; use [`FieldMap::get`] to query individual fields.
#[must_use]
pub fn resolve_static_config(dir: &AbsolutePath) -> FieldMap {
    let Some(config_path) = resolve_config_path(dir) else {
        // No config file found — closed empty map; get() returns None for any key.
        return FieldMap::no_config();
    };
    let Ok(source) = std::fs::read_to_string(&config_path) else {
        return FieldMap::unanalyzable();
    };

    let extension = config_path.as_path().extension().and_then(|e| e.to_str()).unwrap_or("");

    if extension == "json" {
        return parse_json_config(&source);
    }

    parse_js_ts_config(&source, extension)
}

/// Parse a JSON config file into a map of field names to values.
/// All fields in a valid JSON object are fully static.
fn parse_json_config(source: &str) -> FieldMap {
    let Ok(serde_json::Value::Object(obj)) = serde_json::from_str(source) else {
        return FieldMap::unanalyzable();
    };
    let mut map = FxHashMap::with_capacity_and_hasher(obj.len(), Default::default());
    for (k, v) in &obj {
        map.insert(Box::from(k.as_str()), FieldValue::Json(v.clone()));
    }
    FieldMap(FieldMapInner::Closed(map))
}

/// Parse a JS/TS config file, extracting the default export object's fields.
fn parse_js_ts_config(source: &str, extension: &str) -> FieldMap {
    let allocator = Allocator::default();
    let source_type = match extension {
        "ts" | "mts" | "cts" => SourceType::ts(),
        _ => SourceType::mjs(),
    };

    let parser = Parser::new(&allocator, source, source_type);
    let result = parser.parse();

    if result.panicked || !result.errors.is_empty() {
        return FieldMap::unanalyzable();
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
fn extract_config_fields(program: &Program<'_>) -> FieldMap {
    for stmt in &program.body {
        // ESM: export default ...
        if let Statement::ExportDefaultDeclaration(decl) = stmt {
            if let Some(expr) = decl.declaration.as_expression() {
                return extract_config_from_expr(expr);
            }
            // export default class/function — not analyzable
            return FieldMap::unanalyzable();
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

    FieldMap::unanalyzable()
}

/// Extract the config object from an expression that is either:
/// - `defineConfig({ ... })` → extract the object argument
/// - `defineConfig(() => ({ ... }))` → extract from arrow function expression body
/// - `defineConfig(() => { return { ... }; })` → extract from return statement
/// - `defineConfig(function() { return { ... }; })` → extract from return statement
/// - `{ ... }` → extract directly
/// - anything else → not analyzable
fn extract_config_from_expr(expr: &Expression<'_>) -> FieldMap {
    let expr = expr.without_parentheses();
    match expr {
        Expression::CallExpression(call) => {
            if !call.callee.is_specific_id("defineConfig") {
                return FieldMap::unanalyzable();
            }
            let Some(first_arg) = call.arguments.first() else {
                return FieldMap::unanalyzable();
            };
            let Some(first_arg_expr) = first_arg.as_expression() else {
                return FieldMap::unanalyzable();
            };
            match first_arg_expr {
                Expression::ObjectExpression(obj) => extract_object_fields(obj),
                Expression::ArrowFunctionExpression(arrow) => {
                    extract_config_from_function_body(&arrow.body)
                }
                Expression::FunctionExpression(func) => {
                    let Some(body) = func.body.as_ref() else {
                        return FieldMap::unanalyzable();
                    };
                    extract_config_from_function_body(body)
                }
                _ => FieldMap::unanalyzable(),
            }
        }
        Expression::ObjectExpression(obj) => extract_object_fields(obj),
        _ => FieldMap::unanalyzable(),
    }
}

/// Extract the config object from the body of a function passed to `defineConfig`.
///
/// Handles two patterns:
/// - Concise arrow body: `() => ({ ... })` — body has a single `ExpressionStatement`
/// - Block body with exactly one return: `() => { ... return { ... }; }`
///
/// Returns `FieldMap::unanalyzable()` if the body contains multiple `return` statements
/// (at any nesting depth), since the returned config would depend on runtime control flow.
fn extract_config_from_function_body(body: &oxc_ast::ast::FunctionBody<'_>) -> FieldMap {
    // Reject functions with multiple returns — the config depends on control flow.
    if count_returns_in_stmts(&body.statements) > 1 {
        return FieldMap::unanalyzable();
    }

    for stmt in &body.statements {
        match stmt {
            Statement::ReturnStatement(ret) => {
                let Some(arg) = ret.argument.as_ref() else {
                    return FieldMap::unanalyzable();
                };
                if let Expression::ObjectExpression(obj) = arg.without_parentheses() {
                    return extract_object_fields(obj);
                }
                return FieldMap::unanalyzable();
            }
            Statement::ExpressionStatement(expr_stmt) => {
                // Concise arrow: `() => ({ ... })` is represented as ExpressionStatement
                if let Expression::ObjectExpression(obj) =
                    expr_stmt.expression.without_parentheses()
                {
                    return extract_object_fields(obj);
                }
            }
            _ => {}
        }
    }
    FieldMap::unanalyzable()
}

/// Count `return` statements recursively in a slice of statements.
/// Does not descend into nested function/arrow expressions (they have their own returns).
fn count_returns_in_stmts(stmts: &[Statement<'_>]) -> usize {
    let mut count = 0;
    for stmt in stmts {
        count += count_returns_in_stmt(stmt);
    }
    count
}

fn count_returns_in_stmt(stmt: &Statement<'_>) -> usize {
    match stmt {
        Statement::ReturnStatement(_) => 1,
        Statement::BlockStatement(block) => count_returns_in_stmts(&block.body),
        Statement::IfStatement(if_stmt) => {
            let mut n = count_returns_in_stmt(&if_stmt.consequent);
            if let Some(alt) = &if_stmt.alternate {
                n += count_returns_in_stmt(alt);
            }
            n
        }
        Statement::SwitchStatement(switch) => {
            let mut n = 0;
            for case in &switch.cases {
                n += count_returns_in_stmts(&case.consequent);
            }
            n
        }
        Statement::TryStatement(try_stmt) => {
            let mut n = count_returns_in_stmts(&try_stmt.block.body);
            if let Some(handler) = &try_stmt.handler {
                n += count_returns_in_stmts(&handler.body.body);
            }
            if let Some(finalizer) = &try_stmt.finalizer {
                n += count_returns_in_stmts(&finalizer.body);
            }
            n
        }
        Statement::ForStatement(s) => count_returns_in_stmt(&s.body),
        Statement::ForInStatement(s) => count_returns_in_stmt(&s.body),
        Statement::ForOfStatement(s) => count_returns_in_stmt(&s.body),
        Statement::WhileStatement(s) => count_returns_in_stmt(&s.body),
        Statement::DoWhileStatement(s) => count_returns_in_stmt(&s.body),
        Statement::LabeledStatement(s) => count_returns_in_stmt(&s.body),
        Statement::WithStatement(s) => count_returns_in_stmt(&s.body),
        _ => 0,
    }
}

/// Extract fields from an object expression into a [`FieldMap`].
///
/// Objects with no spreads or computed-key properties produce a [`FieldMapInner::Closed`]
/// map — absent keys are definitively absent. Objects with at least one such property
/// produce a [`FieldMapInner::Open`] map — absent keys may exist via the spread and
/// [`FieldMap::get`] returns [`FieldValue::NonStatic`] for them.
///
/// When a spread or computed key is encountered the map transitions to
/// [`FieldMapInner::Open`] (discarding pre-spread entries): they would all be
/// [`FieldValue::NonStatic`] anyway, and `Open` already returns `NonStatic` for every absent key.
///
/// Fields declared after the last spread/computed key are still extractable:
///
/// ```js
/// { a: 1, ...x, b: 2 }  // Open{ b: Json(2) };  get("a") = NonStatic, get("b") = Json(2)
/// { a: 1, [k]: 2, b: 3 } // Open{ b: Json(3) };  get("a") = NonStatic, get("b") = Json(3)
/// { a: 1, b: 2 }         // Closed{ a: Json(1), b: Json(2) }; get("c") = None
/// ```
fn extract_object_fields(obj: &oxc_ast::ast::ObjectExpression<'_>) -> FieldMap {
    let mut inner = FieldMapInner::Closed(FxHashMap::default());

    for prop in &obj.properties {
        if prop.is_spread() {
            inner = FieldMapInner::Open(FxHashMap::default());
            continue;
        }
        let ObjectPropertyKind::ObjectProperty(prop) = prop else { continue };
        let Some(key) = prop.key.static_name() else {
            inner = FieldMapInner::Open(FxHashMap::default());
            continue;
        };

        match &mut inner {
            FieldMapInner::Closed(map) => {
                let value =
                    expr_to_json(&prop.value).map_or(FieldValue::NonStatic, FieldValue::Json);
                map.insert(Box::from(key.as_ref()), value);
            }
            FieldMapInner::Open(map) => {
                // Only Json values are meaningful in Open — NonStatic is already implied
                // for any absent key, so there's no need to record it explicitly.
                if let Some(json) = expr_to_json(&prop.value) {
                    map.insert(Box::from(key.as_ref()), json);
                }
            }
        }
    }

    FieldMap(inner)
}

/// Convert an f64 to a JSON value following `JSON.stringify` semantics.
/// `NaN`, `Infinity`, `-Infinity` become `null`; `-0` becomes `0`.
fn f64_to_json_number(value: f64) -> serde_json::Value {
    // fract() == 0.0 ensures the value is a whole number, so the cast is lossless.
    #[expect(clippy::cast_possible_truncation)]
    if value.fract() == 0.0
        && let Ok(i) = i64::try_from(value as i128)
    {
        serde_json::Value::from(i)
    } else {
        // From<f64> for Value: finite → Number, NaN/Infinity → Null
        serde_json::Value::from(value)
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
            if unary.operator == oxc_ast::ast::UnaryOperator::UnaryNegation
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

    /// Helper: parse JS/TS source and return the field map.
    fn parse(source: &str) -> FieldMap {
        parse_js_ts_config(source, "ts")
    }

    /// Shorthand for asserting a field extracted as JSON.
    fn assert_json(map: &FieldMap, key: &str, expected: serde_json::Value) {
        assert_eq!(map.get(key), Some(FieldValue::Json(expected)));
    }

    /// Shorthand for asserting a field is `NonStatic`.
    fn assert_non_static(map: &FieldMap, key: &str) {
        assert_eq!(
            map.get(key),
            Some(FieldValue::NonStatic),
            "expected field {key:?} to be NonStatic"
        );
    }

    // ── Config file resolution ──────────────────────────────────────────

    #[test]
    fn resolves_ts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.ts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.get("run").is_some());
    }

    #[test]
    fn resolves_js_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.js"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.get("run").is_some());
    }

    #[test]
    fn resolves_mts_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        std::fs::write(dir.path().join("vite.config.mts"), "export default { run: {} }").unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.get("run").is_some());
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
        assert!(result.get("fromJs").is_some());
        assert!(result.get("fromTs").is_none());
    }

    #[test]
    fn returns_empty_map_for_no_config() {
        let dir = TempDir::new().unwrap();
        let dir_path = vite_path::AbsolutePathBuf::new(dir.path().to_path_buf()).unwrap();
        let result = resolve_static_config(&dir_path);
        assert!(result.get("run").is_none());
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
        assert!(result.get("run").is_none());
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
        let result = parse_js_ts_config("module.exports = { run: { cache: true } }", "cjs");
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
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
    }

    #[test]
    fn module_exports_non_object() {
        assert_non_static(&parse_js_ts_config("module.exports = 42;", "cjs"), "run");
    }

    #[test]
    fn module_exports_unknown_call() {
        assert_non_static(&parse_js_ts_config("module.exports = otherFn({ a: 1 });", "cjs"), "run");
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
    fn numeric_overflow_to_infinity_is_null() {
        // 1e999 overflows f64 to Infinity; JSON.stringify(Infinity) === "null"
        let result = parse("export default { a: 1e999, b: -1e999 }");
        assert_json(&result, "a", serde_json::Value::Null);
        assert_json(&result, "b", serde_json::Value::Null);
    }

    #[test]
    fn negative_zero_is_zero() {
        // JSON.stringify(-0) === "0"
        let result = parse("export default { a: -0 }");
        assert_json(&result, "a", serde_json::json!(0));
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
    fn spread_unknown_keys_not_in_map() {
        // The spread produces an Open map. Keys from inside the spread (like "x") are
        // not explicitly declared, so get("x") returns NonStatic (may exist via spread).
        // Fields declared after the spread are still extracted as Json.
        let result = parse(
            r"
            const base = { x: 1 };
            export default { ...base, b: 'ok' }
            ",
        );
        assert_non_static(&result, "x");
        assert_json(&result, "b", serde_json::json!("ok"));
    }

    #[test]
    fn spread_invalidates_previous_fields() {
        // Pre-spread fields are discarded from the Open map (they're all NonStatic via
        // the open-world fallback). Fields after the spread are still extracted.
        let result = parse(
            r"
            const base = { x: 1 };
            export default { a: 1, run: { cacheScripts: true }, ...base, b: 'ok' }
            ",
        );
        assert_non_static(&result, "a");
        assert_non_static(&result, "run");
        assert_non_static(&result, "x");
        assert_json(&result, "b", serde_json::json!("ok"));
    }

    #[test]
    fn spread_only() {
        // A bare spread with no explicit keys after it: every key is NonStatic.
        let result = parse(
            r"
            const base = { run: { cacheScripts: true } };
            export default { ...base }
            ",
        );
        assert_non_static(&result, "run");
    }

    #[test]
    fn spread_then_explicit_run() {
        // run is explicitly declared after the spread and is still extractable as Json.
        let result = parse(
            r"
            const base = { plugins: [] };
            export default { ...base, run: { cacheScripts: true } }
            ",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
    }

    #[test]
    fn no_spread_absent_is_none() {
        // No spreads: Closed map — absent keys are definitively absent.
        let result = parse(r"export default { plugins: [] }");
        assert!(result.get("run").is_none());
    }

    #[test]
    fn computed_key_unknown_not_in_map() {
        // The computed key produces an Open map. Keys not declared after it return NonStatic.
        let result = parse(
            r"
            const key = 'dynamic';
            export default { [key]: 'value', plain: 'ok' }
            ",
        );
        assert_non_static(&result, "dynamic");
        assert_json(&result, "plain", serde_json::json!("ok"));
    }

    #[test]
    fn computed_key_invalidates_previous_fields() {
        // A computed key produces Open — pre-computed fields become NonStatic via the
        // open-world fallback. Fields declared after are extracted normally.
        let result = parse(
            r"
            const key = 'run';
            export default { a: 1, run: { cacheScripts: true }, [key]: 'override', b: 2 }
            ",
        );
        assert_non_static(&result, "a");
        assert_non_static(&result, "run");
        assert_json(&result, "b", serde_json::json!(2));
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
        assert!(result.get("a").is_none());
        assert_json(&result, "b", serde_json::json!(2));
    }

    // ── defineConfig with function argument ────────────────────────────

    #[test]
    fn define_config_arrow_block_body() {
        let result = parse(
            r"
            export default defineConfig(({ mode }) => {
                const env = loadEnv(mode, process.cwd(), '');
                return {
                    run: { cacheScripts: true },
                    plugins: [vue()],
                };
            });
            ",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
        assert_non_static(&result, "plugins");
    }

    #[test]
    fn define_config_arrow_expression_body() {
        let result = parse(
            r"
            export default defineConfig(() => ({
                run: { cacheScripts: true },
                build: { outDir: 'dist' },
            }));
            ",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
        assert_json(&result, "build", serde_json::json!({ "outDir": "dist" }));
    }

    #[test]
    fn define_config_function_expression() {
        let result = parse(
            r"
            export default defineConfig(function() {
                return {
                    run: { cacheScripts: true },
                    plugins: [react()],
                };
            });
            ",
        );
        assert_json(&result, "run", serde_json::json!({ "cacheScripts": true }));
        assert_non_static(&result, "plugins");
    }

    #[test]
    fn define_config_arrow_no_return_object() {
        // Arrow function that doesn't return an object literal
        assert_non_static(
            &parse_js_ts_config(
                r"
            export default defineConfig(({ mode }) => {
                return someFunction();
            });
            ",
                "ts",
            ),
            "run",
        );
    }

    #[test]
    fn define_config_arrow_multiple_returns() {
        // Multiple top-level returns → not analyzable
        assert_non_static(
            &parse_js_ts_config(
                r"
            export default defineConfig(({ mode }) => {
                if (mode === 'production') {
                    return { run: { cacheScripts: true } };
                }
                return { run: { cacheScripts: false } };
            });
            ",
                "ts",
            ),
            "run",
        );
    }

    #[test]
    fn define_config_arrow_empty_body() {
        assert_non_static(
            &parse_js_ts_config("export default defineConfig(() => {});", "ts"),
            "run",
        );
    }

    // ── Not analyzable cases ─────────────────────────────────────────────

    #[test]
    fn returns_none_for_no_default_export() {
        assert_non_static(&parse_js_ts_config("export const config = { a: 1 };", "ts"), "run");
    }

    #[test]
    fn returns_none_for_non_object_default_export() {
        assert_non_static(&parse_js_ts_config("export default 42;", "ts"), "run");
    }

    #[test]
    fn returns_none_for_unknown_function_call() {
        assert_non_static(
            &parse_js_ts_config("export default someOtherFn({ a: 1 });", "ts"),
            "run",
        );
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
