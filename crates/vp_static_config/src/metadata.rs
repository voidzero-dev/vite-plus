use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use vt_path::{AbsolutePath, AbsolutePathBuf};

use super::{expr_to_json, resolve_config_path};

/// Metadata from a config whose entire module can be read without execution.
pub struct StaticConfigMetadata {
    pub config_file: AbsolutePathBuf,
    pub metadata: serde_json::Value,
}

/// Read a plain metadata export without initializing Vite.
///
/// Unlike field extraction, this requires the whole module to be a single
/// literal export. Vite settings, imports, executable code, and unsupported
/// syntax return `None`, so the caller retains runtime validation and effects.
/// Missing or unreadable files also use the caller's existing resolution path.
pub fn resolve_static_metadata(dir: &AbsolutePath) -> Option<StaticConfigMetadata> {
    let config_file = resolve_config_path(dir)?;
    let source = std::fs::read_to_string(&config_file).ok()?;
    let source_type = SourceType::from_path(config_file.as_path()).ok()?;
    let metadata = parse_metadata(&source, source_type)?;
    Some(StaticConfigMetadata { config_file, metadata })
}

fn parse_metadata(source: &str, source_type: SourceType) -> Option<serde_json::Value> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() || !parsed.program.directives.is_empty()
    {
        return None;
    }
    let [Statement::ExportDefaultDeclaration(export)] = parsed.program.body.as_slice() else {
        return None;
    };
    let expression = export.declaration.as_expression()?;
    if !is_json_literal(expression) {
        return None;
    }
    let metadata = expr_to_json(expression)?;
    let fields = metadata.as_object()?;
    // Even literal Vite settings can trigger validation, environment changes,
    // or plugin integration. Only these unmodified metadata fields are safe.
    if fields.keys().any(|key| !matches!(key.as_str(), "lint" | "fmt" | "check" | "run" | "staged"))
    {
        return None;
    }
    Some(metadata)
}

fn is_json_literal(expression: &Expression<'_>) -> bool {
    match expression.get_inner_expression() {
        Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => true,
        Expression::NumericLiteral(value) => value.value.is_finite(),
        Expression::StringLiteral(value) => !value.lone_surrogates,
        Expression::UnaryExpression(value) => {
            value.operator == oxc_ast::ast::UnaryOperator::UnaryNegation
                && matches!(&value.argument, Expression::NumericLiteral(number) if number.value.is_finite())
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .all(|element| element.as_expression().is_some_and(is_json_literal)),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            let key = match &property.key {
                PropertyKey::StaticIdentifier(key) => key.name.as_str(),
                PropertyKey::StringLiteral(key) if !key.lone_surrogates => key.value.as_str(),
                _ => return false,
            };
            // __proto__ changes inheritance instead of creating a JSON field.
            key != "__proto__"
                && property.kind == PropertyKind::Init
                && !property.computed
                && !property.method
                && !property.shorthand
                && is_json_literal(&property.value)
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_metadata_preserves_values_and_last_duplicate_key() {
        let fields = parse_metadata(
            r#"// A plain, typed metadata export.
            export default ({
                lint: { rules: { 'no-debugger': 'error' }, options: { typeAware: true } },
                fmt: { singleQuote: true, printWidth: 90 },
                check: { lint: false }, check: { fmt: false },
                run: { tasks: { build: { command: 'echo ok', values: [null, -2, 1.5] } } },
                staged: { '*.ts': 'vp check --fix' }
            } as const);"#,
            SourceType::ts(),
        )
        .unwrap();
        assert_eq!(
            fields,
            serde_json::json!({
                "lint": { "rules": { "no-debugger": "error" }, "options": { "typeAware": true } },
                "fmt": { "singleQuote": true, "printWidth": 90 },
                "check": { "fmt": false },
                "run": { "tasks": { "build": { "command": "echo ok", "values": [null, -2, 1.5] } } },
                "staged": { "*.ts": "vp check --fix" }
            })
        );
    }

    #[test]
    fn runtime_features_and_vite_settings_require_fallback() {
        for source in [
            "export default {};\nthrow new Error('must run');",
            "throw new Error('must run'); export default {};",
            "import './side-effect.js'; export default {};",
            "import { defineConfig } from 'vite-plus'; export default defineConfig({ lint: {} });",
            "import type { UserConfig } from 'vite'; export default {} satisfies UserConfig;",
            "export default () => ({ lint: {} });",
            "export default async () => ({ lint: {} });",
            "const config = {}; export default config;",
            "export default { plugins: [] };",
            "export default { root: 'app', lint: {} };",
            "export default { envPrefix: '', lint: {} };",
            "export default { devtools: true };",
            "export default { configFile: false };",
            "export default { ...base, lint: {} };",
            "export default { ['lint']: {} };",
            "export default { lint: { get rules() { return {}; } } };",
            "export default { lint: { rules() {} } };",
            "export default { lint };",
            "export default { lint: JSON.parse('{}') };",
            "export default { lint: { __proto__: { options: { typeAware: true } } } };",
            "export default { __proto__: { check: { fmt: false } } };",
            "export default { lint: { ['__proto__']: {} } };",
            "export default { lint: { rules: { '\\ud800': 'error' } } };",
            "export default { fmt: { value: '\\ud800' } };",
            "export default { fmt: { value: 1e400 } };",
            "export default { fmt: { value: [, 1] } };",
            "export default { fmt: { value: [...items] } };",
            "export default { fmt: { value: /pattern/ } };",
            "export default { fmt: { value: undefined } };",
            "export default { fmt: { value: 1n } };",
            "module.exports = { lint: {} };",
            "export default null;",
            "export default [];",
            "export default { lint:",
            "export default {}; export default {};",
        ] {
            assert!(parse_metadata(source, SourceType::ts()).is_none(), "{source}");
        }
    }

    #[test]
    fn respects_extensions_precedence_and_file_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = AbsolutePathBuf::new(temp.path().to_path_buf()).unwrap();
        assert!(resolve_static_metadata(&root).is_none());
        for name in super::super::CONFIG_FILE_NAMES {
            let file = root.join(name);
            std::fs::write(&file, "export default { lint: {} };").unwrap();
            let metadata = resolve_static_metadata(&root).unwrap();
            assert_eq!(metadata.config_file, file);
            assert_eq!(metadata.metadata["lint"], serde_json::json!({}));
            std::fs::write(&file, "export default { check: { fmt: false } };").unwrap();
            assert!(resolve_static_metadata(&root).unwrap().metadata.get("lint").is_none());
            std::fs::remove_file(&file).unwrap();
            assert!(resolve_static_metadata(&root).is_none());
        }
        std::fs::write(root.join("vite.config.ts"), "export default {};").unwrap();
        std::fs::write(root.join("vite.config.js"), "throw new Error('higher priority');").unwrap();
        assert!(resolve_static_metadata(&root).is_none());
        std::fs::remove_file(root.join("vite.config.js")).unwrap();
        std::fs::create_dir(root.join("vite.config.js")).unwrap();
        assert!(resolve_static_metadata(&root).is_none());
    }
}
