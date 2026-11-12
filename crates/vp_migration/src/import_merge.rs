use oxc::{
    allocator::Allocator,
    ast::ast::{
        ImportDeclaration, ImportDeclarationSpecifier, ImportOrExportKind, Program, Statement,
    },
    parser::Parser,
    span::{GetSpan, SourceType},
};

/// Merge adjacent imports when migration changed at least one of their sources.
/// Keep unsupported declarations and comments intact rather than reconstructing them.
pub fn merge_duplicate_imports(
    original: &str,
    content: &str,
    filename: Option<&str>,
) -> Option<String> {
    let allocator = Allocator::default();
    let source_type = filename
        .and_then(|name| SourceType::from_path(name).ok())
        .unwrap_or_else(SourceType::ts)
        .with_module(true);
    let before = Parser::new(&allocator, original, source_type).parse();
    let after = Parser::new(&allocator, content, source_type).parse();
    if !before.diagnostics.is_empty() || !after.diagnostics.is_empty() {
        return None;
    }

    let original_imports = imports(&before.program);
    let rewritten_imports = imports(&after.program);
    // Source rewrites must not add or remove declarations. If they do, we
    // cannot reliably identify which imports changed.
    if original_imports.len() != rewritten_imports.len() {
        return None;
    }

    let mut groups: Vec<Vec<(&ImportDeclaration<'_>, bool)>> = Vec::new();
    for (original_import, import) in original_imports.iter().zip(rewritten_imports) {
        let changed = original_import.source.value != import.source.value;
        if let Some(group) = groups.last_mut() {
            let previous = group.last().expect("non-empty group").0;
            if previous.source.value == import.source.value
                && content[previous.span.end as usize..import.span.start as usize]
                    .chars()
                    .all(char::is_whitespace)
            {
                group.push((import, changed));
                continue;
            }
        }
        groups.push(vec![(import, changed)]);
    }

    let mut result = content.to_string();
    let mut updated = false;
    // Apply edits backwards so all spans still refer to the input text.
    for group in groups.into_iter().rev() {
        if group.len() < 2 || !group.iter().any(|(_, changed)| *changed) {
            continue;
        }
        let start = group[0].0.span.start;
        let end = group.last().expect("non-empty group").0.span.end;
        if after
            .program
            .comments
            .iter()
            .any(|comment| comment.span.start < end && comment.span.end > start)
        {
            continue;
        }
        if let Some(merged) = merge_group(&group, content) {
            result.replace_range(start as usize..end as usize, &merged);
            updated = true;
        }
    }
    updated.then_some(result)
}

fn imports<'a>(program: &'a Program<'a>) -> Vec<&'a ImportDeclaration<'a>> {
    program
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => Some(import.as_ref()),
            _ => None,
        })
        .collect()
}

fn merge_group(group: &[(&ImportDeclaration<'_>, bool)], content: &str) -> Option<String> {
    let type_only = group.iter().all(|(import, _)| import.import_kind == ImportOrExportKind::Type);
    let mut default = None;
    let mut locals = Vec::new();
    let mut named = Vec::new();
    for &(import, _) in group {
        if import.with_clause.is_some() || import.phase.is_some() {
            return None;
        }
        let specifiers = import.specifiers.as_deref()?;
        // Empty and side-effect imports can force module evaluation.
        if specifiers.is_empty() {
            return None;
        }
        for specifier in specifiers {
            let local = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    let is_type = import.import_kind == ImportOrExportKind::Type
                        || specifier.import_kind == ImportOrExportKind::Type;
                    let text = specifier.span.source_text(content);
                    // Reuse the original specifier, including quoted export
                    // names, aliases, escapes, and existing inline `type`.
                    let text = if !type_only && import.import_kind == ImportOrExportKind::Type {
                        format!("type {text}")
                    } else {
                        text.to_string()
                    };
                    named.push((is_type, text));
                    &specifier.local
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                    if default.is_some() || import.import_kind == ImportOrExportKind::Type {
                        return None;
                    }
                    default = Some(specifier.local.span.source_text(content));
                    &specifier.local
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => return None,
            };
            // Different aliases for the same export are valid. Repeated
            // locals, including a default/named collision, are not.
            if locals.contains(&local.name) {
                return None;
            }
            locals.push(local.name);
        }
    }

    let mut parts = Vec::new();
    if let Some(default) = default {
        parts.push(default.to_string());
    }
    if !named.is_empty() {
        named.sort_by_key(|(is_type, _)| *is_type);
        let named = named.into_iter().map(|(_, text)| text).collect::<Vec<_>>().join(", ");
        parts.push(format!("{{ {named} }}"));
    }
    let kind = if type_only { " type" } else { "" };
    let source = group[0].0.source.span().source_text(content);
    Some(format!("import{kind} {} from {source};", parts.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge(content: &str) -> Option<String> {
        let original = content.replace("vite-plus", "vite");
        merge_duplicate_imports(&original, content, Some("file.ts"))
    }

    #[test]
    fn merges_value_and_type_only_vite_plus_imports() {
        // After the migration rewriter has rewritten the `vite` specifier to
        // `vite-plus`, this helper collapses the resulting duplicate imports.
        let input = r#"import type { Plugin } from 'vite-plus'
import { defineConfig } from 'vite-plus'
"#;
        let expected = "import { defineConfig, type Plugin } from 'vite-plus';\n";
        assert_eq!(merge(input).as_deref(), Some(expected));
    }

    #[test]
    fn merges_two_value_imports_from_same_source() {
        let input = r#"import { defineConfig } from 'vite-plus'
import { Plugin } from 'vite-plus'
"#;
        let expected = "import { defineConfig, Plugin } from 'vite-plus';\n";
        assert_eq!(merge(input).as_deref(), Some(expected));
    }

    #[test]
    fn merges_two_type_only_imports() {
        let input = r#"import type { Plugin } from 'vite-plus'
import type { UserConfig } from 'vite-plus'
"#;
        let expected = "import type { Plugin, UserConfig } from 'vite-plus';\n";
        assert_eq!(merge(input).as_deref(), Some(expected));
    }

    #[test]
    fn does_not_merge_namespace_imports() {
        let input = r#"import * as Vite from 'vite-plus'
import { defineConfig } from 'vite-plus'
"#;
        assert_eq!(merge(input), None);
    }

    #[test]
    fn does_not_merge_default_with_type_only_decl() {
        let input = r#"import type Default from 'vite-plus'
import { a } from 'vite-plus'
"#;
        assert_eq!(merge(input), None);
    }

    #[test]
    fn does_not_merge_different_sources() {
        let input = r#"import { a } from 'x'
import { b } from 'y'
"#;
        assert_eq!(merge(input), None);
    }

    #[test]
    fn does_not_merge_conflicting_local_bindings() {
        let input = r#"import { a } from 'vite-plus'
import { b as a } from 'vite-plus'
"#;
        assert_eq!(merge(input), None);
    }

    #[test]
    fn preserves_statements_and_comments_between_imports() {
        for separator in [
            "const keep = 1;",
            "run();",
            "export const keep = 1;",
            "import './setup';",
            "import 'vite-plus';",
            "import {} from 'vite-plus';",
            "// Keep this comment",
            "/* Keep this comment */",
            "// @ts-expect-error: upstream type mismatch",
        ] {
            let input = format!(
                "import {{ a }} from 'vite-plus';\n{separator}\nimport {{ b }} from 'vite-plus';"
            );
            assert_eq!(merge(&input), None, "separator: {separator}");
        }
    }

    #[test]
    fn preserves_unsupported_imports_and_binding_conflicts() {
        for input in [
            "import { /* keep */ a } from 'vite-plus';\nimport { b } from 'vite-plus';",
            "import { a } from 'vite-plus' with { type: 'json' };\nimport { b } from 'vite-plus';",
            "import a from 'vite-plus';\nimport b from 'vite-plus';",
            "import a from 'vite-plus';\nimport { b as a } from 'vite-plus';",
            "import { a } from 'vite-plus';\nimport { a } from 'vite-plus';",
            "import type { a } from 'vite-plus';\nimport { a } from 'vite-plus';",
            "import type { a } from 'vite-plus';\nimport {} from 'vite-plus';",
            "import { a } from 'vite-plus';\nimport { b } from 'vite-plus';\nconst = ;",
        ] {
            assert_eq!(merge(input), None, "input: {input}");
        }
    }

    #[test]
    fn preserves_quoted_names_escapes_and_aliases() {
        let input = r#"import { "a-b" as a, "say\"hi" as hi } from "vite-plus";
import { "a-b" as b, \u0063 as c } from 'vite-plus';"#;
        assert_eq!(
            merge(input).as_deref(),
            Some(
                r#"import { "a-b" as a, "say\"hi" as hi, "a-b" as b, \u0063 as c } from "vite-plus";"#
            )
        );
    }

    #[test]
    fn merges_default_and_inline_type_imports() {
        let input = "import vite, { type Plugin } from 'vite-plus';\nimport { defineConfig } from 'vite-plus';";
        assert_eq!(
            merge(input).as_deref(),
            Some("import vite, { defineConfig, type Plugin } from 'vite-plus';")
        );
    }

    #[test]
    fn preserves_inline_type_import_evaluation() {
        let input = "import { type Plugin } from 'vite-plus';\nimport type { UserConfig } from 'vite-plus';";
        assert_eq!(
            merge(input).as_deref(),
            Some("import { type Plugin, type UserConfig } from 'vite-plus';")
        );
    }

    #[test]
    fn preserves_unchanged_import_groups() {
        let input = "import { a } from 'vite-plus';\nimport { b } from 'vite-plus';\nconst x = import('vite');";
        let rewritten = input.replace("import('vite')", "import('vite-plus')");
        assert_eq!(merge_duplicate_imports(input, &rewritten, Some("file.ts")), None);
    }

    #[test]
    fn merges_multiple_groups_without_changing_surrounding_code() {
        let input = "// leading\nimport { a } from 'vite-plus';\nimport { b } from 'vite-plus'; // trailing\nconst keep = '你好';\nimport type { C } from 'vite-plus';\nimport type { D } from 'vite-plus';\n";
        assert_eq!(
            merge(input).as_deref(),
            Some(
                "// leading\nimport { a, b } from 'vite-plus'; // trailing\nconst keep = '你好';\nimport type { C, D } from 'vite-plus';\n"
            )
        );
    }

    #[test]
    fn supports_jsx_and_typescript_extensions() {
        for filename in
            ["config.tsx", "config.jsx", "config.mts", "config.cts", "config.mjs", "config.cjs"]
        {
            let tail = if filename.ends_with("x") { "\nconst view = <div />;" } else { "" };
            let input =
                format!("import {{ a }} from 'vite';\nimport {{ b }} from 'vite-plus';{tail}");
            let rewritten = input.replace("'vite'", "'vite-plus'");
            let expected = format!("import {{ a, b }} from 'vite-plus';{tail}");
            assert_eq!(
                merge_duplicate_imports(&input, &rewritten, Some(filename)),
                Some(expected),
                "filename: {filename}"
            );
        }
    }
}
