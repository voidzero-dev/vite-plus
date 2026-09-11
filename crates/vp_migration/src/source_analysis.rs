use std::collections::HashMap;

use oxc::{
    allocator::Allocator,
    ast::AstKind,
    ast_visit::utf8_to_utf16::Utf8ToUtf16,
    parser::{ParseOptions, Parser},
    semantic::SemanticBuilder,
    span::{GetSpan, SourceType},
};
use serde_json::{Value, json};

/// Parse migration input and resolve bindings with the Oxc already shipped by Rolldown.
/// Keep transformation rules in TypeScript; export only the semantic facts they need,
/// rather than duplicating JavaScript scope analysis or bundling another parser.
pub fn analyze_migration_source(filename: &str, source: &str) -> Result<String, String> {
    let allocator = Allocator::default();
    let source_type =
        SourceType::from_path(filename).map_err(|error| error.to_string())?.with_unambiguous(true);
    // JavaScript migration input can contain JSX. For TypeScript, retain the
    // extension's TS/TSX mode so `<T>(value: T) => value` is not parsed as JSX.
    let source_type = source_type.with_jsx(source_type.is_javascript() || source_type.is_jsx());
    let parsed = Parser::new(&allocator, source, source_type)
        .with_options(ParseOptions { preserve_parens: false, ..ParseOptions::default() })
        .parse();
    if let Some(error) = parsed.diagnostics.first() {
        return Err(error.to_string());
    }
    let mut program = parsed.program;
    let analyzed = SemanticBuilder::new_compiler().with_build_nodes(true).build(&program);
    if let Some(error) = analyzed.diagnostics.first() {
        return Err(error.to_string());
    }

    // JavaScript slices and diagnostic columns use UTF-16, whereas Rust spans use
    // UTF-8 bytes. Convert both semantic offsets and AST/comment spans together.
    let spans = Utf8ToUtf16::new(source);
    let mut converter = spans.converter();
    let mut offset = |mut value| {
        if let Some(converter) = converter.as_mut() {
            converter.convert_offset(&mut value);
        }
        value
    };
    let semantic = analyzed.semantic;
    let scoping = semantic.scoping();
    // Oxc keeps type and value references separate. An invalid runtime use of
    // a type-only import is unresolved, but must not be mistaken for a Vitest
    // global by migration rules. Preserve its lexical import binding too.
    let mut type_import_uses = HashMap::<_, Vec<u32>>::new();
    for node in semantic.nodes().iter() {
        if let AstKind::IdentifierReference(identifier) = node.kind()
            && !scoping.has_binding(identifier.reference_id.get().unwrap())
            && let Some(symbol) = scoping.find_binding(node.scope_id(), identifier.name)
            && scoping.symbol_flags(symbol).is_type_import()
        {
            type_import_uses.entry(symbol).or_default().push(offset(identifier.span.start));
        }
    }
    let bindings: Vec<Value> = scoping
        .symbol_ids()
        .map(|symbol| {
            let mut references: Vec<u32> = scoping
                .get_resolved_references(symbol)
                .map(|reference| offset(semantic.nodes().kind(reference.node_id()).span().start))
                .collect();
            references.extend(type_import_uses.remove(&symbol).unwrap_or_default());
            // Do not follow reassigned or redeclared aliases, including invalid
            // writes to const bindings. `symbol_is_mutated` alone ignores those.
            let constant = scoping.symbol_redeclarations(symbol).is_empty()
                && !scoping.get_resolved_references(symbol).any(|reference| reference.is_write());
            json!({
                "start": offset(scoping.symbol_span(symbol).start),
                "references": references,
                "constant": constant,
            })
        })
        .collect();
    drop(semantic);
    spans.convert_program_and_comments(&mut program);
    let comments: Vec<Value> = program
        .comments
        .iter()
        .map(|comment| json!({ "start": comment.span.start, "end": comment.span.end }))
        .collect();

    // Reuse the serializer already instantiated by oxc_parser_napi. Migration
    // reads literal spelling from source, so it does not need the JS-only fixes
    // that reconstruct RegExp/BigInt values from this ESTree JSON representation.
    let mut result: Value = serde_json::from_str(&program.to_estree_json_with_fixes(true, false))
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&json!({
        "program": result["node"].take(),
        "bindings": bindings,
        "comments": comments,
    }))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::analyze_migration_source;

    #[test]
    fn rejects_flow_and_invalid_syntax() {
        for source in ["// @flow\nconst x: string = 'x';", "const = ;"] {
            assert!(analyze_migration_source("test.js", source).is_err());
        }
    }

    #[test]
    fn preserves_type_imports_for_unresolved_value_uses() {
        for source in [
            "import type { expect } from 'vitest'; expect();",
            "import { type expect } from 'vitest'; expect();",
            "import type * as expect from 'vitest'; expect();",
        ] {
            let result: Value =
                serde_json::from_str(&analyze_migration_source("test.ts", source).unwrap())
                    .unwrap();
            assert_eq!(
                result["bindings"][0]["references"],
                serde_json::json!([source.rfind("expect").unwrap()])
            );
        }
    }

    #[test]
    fn resolves_shadowing_and_mutated_aliases_with_utf16_offsets() {
        let source = "// 😀\nimport { vi } from 'vitest'; vi.fn(); function f(vi) { vi.fn(); } let alias = vi; alias = other; alias.fn();";
        let result: Value =
            serde_json::from_str(&analyze_migration_source("test.ts", source).unwrap()).unwrap();
        let bindings = result["bindings"].as_array().unwrap();
        let utf16 = |text: &str| source[..source.find(text).unwrap()].encode_utf16().count();
        let imported = bindings.iter().find(|binding| binding["start"] == utf16("vi }")).unwrap();
        assert_eq!(
            imported["references"],
            serde_json::json!([utf16("vi.fn()"), utf16("vi; alias")])
        );
        assert_eq!(imported["constant"], true);
        let alias =
            bindings.iter().find(|binding| binding["start"] == utf16("alias = vi")).unwrap();
        assert_eq!(alias["constant"], false);
        assert_eq!(result["program"]["body"][0]["start"], utf16("import"));
        assert_eq!(result["comments"][0]["end"], "// 😀".encode_utf16().count());
    }
}
