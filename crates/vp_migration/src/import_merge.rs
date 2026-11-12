use std::collections::HashSet;

use oxc::{
    allocator::Allocator,
    ast::ast::{
        BindingIdentifier, ImportDeclaration, ImportDeclarationSpecifier, ImportDefaultSpecifier,
        ImportOrExportKind, ImportSpecifier, ModuleDeclaration, ModuleExportName,
    },
    parser::{ParseOptions, Parser},
    span::{GetSpan, SourceType, Span},
};

/// Information extracted from one import declaration.
#[derive(Debug, Clone)]
struct ImportInfo {
    span: Span,
    source: String,
    import_kind: ImportOrExportKind,
    default_name: Option<String>,
    named: Vec<NamedImport>,
    has_namespace: bool,
    has_attributes: bool,
    phase: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct NamedImport {
    /// Exported name being imported.
    imported: String,
    /// Local binding name.
    local: String,
    /// Whether this specific specifier is type-only.
    import_kind: ImportOrExportKind,
}

/// Merge adjacent import declarations that import from the same module specifier.
///
/// This is a post-processing pass for the migration import rewriter. ast-grep
/// rules only rewrite one import source at a time, so a file that already
/// contains a `vite-plus` import and a `vite` import that becomes `vite-plus`
/// ends up with two import statements from the same module. This helper
/// collapses such groups into a single import declaration.
///
/// Only simple groups are merged:
/// - At most one default import per group.
/// - No namespace (`import * as ...`) imports.
/// - No import attributes (`with` clause) and no non-standard import phases.
/// - Skip if a type-only declaration contains a default import (cannot be
///   represented without `import { type default as ... }` syntax).
///
/// A group containing both value and type-only named imports is emitted as a
/// value import with inline `type` specifiers, matching TypeScript's standard
/// form and the expected output in the reported issue.
pub fn merge_duplicate_imports(content: &str, filename: Option<&str>) -> Option<String> {
    let allocator = Allocator::default();
    let source_type = filename
        .and_then(|name| SourceType::from_path(name).ok())
        .unwrap_or_else(SourceType::ts)
        .with_module(true);
    let parsed = Parser::new(&allocator, content, source_type)
        .with_options(ParseOptions { preserve_parens: false, ..ParseOptions::default() })
        .parse();

    if !parsed.diagnostics.is_empty() {
        return None;
    }

    let program = parsed.program;
    let mut imports: Vec<ImportInfo> = Vec::new();
    for stmt in &program.body {
        if let Some(ModuleDeclaration::ImportDeclaration(import_decl)) =
            stmt.as_module_declaration()
        {
            if let Some(info) = extract_import_info(import_decl.as_ref()) {
                imports.push(info);
            }
        }
    }

    if imports.len() < 2 {
        return None;
    }

    // Build groups of adjacent imports that share the same source specifier.
    // We preserve source order, which is also program.body order.
    let mut groups: Vec<Vec<ImportInfo>> = Vec::new();
    for info in imports {
        if let Some(last_group) = groups.last_mut() {
            if last_group.last().expect("non-empty group").source == info.source {
                last_group.push(info);
                continue;
            }
        }
        groups.push(vec![info]);
    }

    let mut replacements: Vec<(Span, String)> = Vec::new();
    for group in groups {
        if group.len() < 2 {
            continue;
        }
        if let Some(merged) = try_merge_import_group(&group) {
            let start = group.first().expect("group non-empty").span.start;
            let end = group.last().expect("group non-empty").span.end;
            replacements.push((Span::new(start, end), merged));
        }
    }

    if replacements.is_empty() {
        return None;
    }

    // Apply replacements from end to start so byte offsets stay valid.
    replacements.sort_by_key(|(span, _)| std::cmp::Reverse(span.end));

    let mut result = content.to_string();
    for (span, replacement) in replacements {
        let start = usize::try_from(span.start).expect("u32 span start fits in usize");
        let end = usize::try_from(span.end).expect("u32 span end fits in usize");
        if start <= result.len() && end <= result.len() && start <= end {
            result.replace_range(start..end, &replacement);
        }
    }

    Some(result)
}

fn extract_import_info(import_decl: &ImportDeclaration) -> Option<ImportInfo> {
    let source = import_decl.source.value.as_str()?.to_string();
    let mut default_name: Option<String> = None;
    let mut named: Vec<NamedImport> = Vec::new();
    let mut has_namespace = false;

    let specifiers = import_decl.specifiers.as_deref()?;
    for spec in specifiers {
        match spec {
            ImportDeclarationSpecifier::ImportSpecifier(spec) => {
                named.push(extract_named_import(spec.as_ref(), import_decl.import_kind));
            }
            ImportDeclarationSpecifier::ImportDefaultSpecifier(spec) => {
                default_name = Some(default_specifier_name(spec.as_ref()));
            }
            ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                has_namespace = true;
            }
        }
    }

    Some(ImportInfo {
        span: import_decl.span(),
        source,
        import_kind: import_decl.import_kind,
        default_name,
        named,
        has_namespace,
        has_attributes: import_decl.with_clause.is_some(),
        phase: import_decl.phase.is_some(),
    })
}

fn default_specifier_name(spec: &ImportDefaultSpecifier) -> String {
    binding_name(&spec.local).to_string()
}

fn extract_named_import(spec: &ImportSpecifier, decl_kind: ImportOrExportKind) -> NamedImport {
    let imported = export_name(&spec.imported).to_string();
    let local = binding_name(&spec.local).to_string();
    let import_kind = if decl_kind == ImportOrExportKind::Type {
        ImportOrExportKind::Type
    } else {
        spec.import_kind
    };
    NamedImport { imported, local, import_kind }
}

fn binding_name<'a>(ident: &'a BindingIdentifier<'a>) -> &'a str {
    ident.name.as_str()
}

fn export_name<'a>(name: &'a ModuleExportName<'a>) -> &'a str {
    match name {
        ModuleExportName::IdentifierName(ident) => ident.name.as_str(),
        ModuleExportName::StringLiteral(lit) => lit.value.as_str().unwrap_or(""),
        // The local side of an ImportSpecifier is never an IdentifierReference.
        ModuleExportName::IdentifierReference(_) => "",
    }
}

fn try_merge_import_group(group: &[ImportInfo]) -> Option<String> {
    let source = &group[0].source;

    // Validation: all imports must be mergeable.
    let mut total_default_count = 0usize;
    let mut has_type_decl_default = false;
    for info in group {
        if &info.source != source {
            return None;
        }
        if info.has_namespace || info.has_attributes || info.phase {
            return None;
        }
        if info.default_name.is_some() {
            total_default_count += 1;
            if info.import_kind == ImportOrExportKind::Type {
                has_type_decl_default = true;
            }
        }
        if total_default_count > 1 || has_type_decl_default {
            return None;
        }
    }

    // Collect default import name. If a default import appeared in a value
    // declaration, its local name is stored there; otherwise there is none.
    let default_name = group.iter().find_map(|info| info.default_name.clone());

    // Merge named imports, detecting irreconcilable conflicts (same local
    // bound to different imported names, or same imported name bound to
    // different locals).
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut named: Vec<NamedImport> = Vec::new();
    for info in group {
        for item in &info.named {
            // Detect conflicts. A duplicate (same imported + same local + same
            // kind) is harmless and can be de-duplicated.
            for existing in &named {
                if existing.local == item.local && existing.imported != item.imported {
                    return None;
                }
                if existing.imported == item.imported && existing.local != item.local {
                    return None;
                }
            }
            let key = (item.imported.clone(), item.local.clone());
            if seen.insert(key) {
                named.push(item.clone());
            }
        }
    }

    if default_name.is_none() && named.is_empty() {
        return None;
    }

    let specifier_parts = build_specifier_parts(default_name.as_deref(), &named);
    Some(format!("import {} from '{}';", specifier_parts, source))
}

fn build_specifier_parts(default_name: Option<&str>, named: &[NamedImport]) -> String {
    let mut parts = Vec::new();
    if let Some(name) = default_name {
        parts.push(name.to_string());
    }
    if !named.is_empty() {
        // Emit value imports before type-only imports while preserving the
        // original relative order within each group. This matches the common
        // TypeScript style expected by the reported issue.
        let value_named: Vec<_> =
            named.iter().filter(|item| item.import_kind == ImportOrExportKind::Value).collect();
        let type_named: Vec<_> =
            named.iter().filter(|item| item.import_kind == ImportOrExportKind::Type).collect();
        let render = |item: &&NamedImport| {
            let import = if item.imported == item.local {
                item.imported.clone()
            } else {
                format!("{} as {}", item.imported, item.local)
            };
            if item.import_kind == ImportOrExportKind::Type {
                format!("type {import}")
            } else {
                import
            }
        };
        let named_str = value_named
            .iter()
            .map(render)
            .chain(type_named.iter().map(render))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("{{ {named_str} }}"));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_value_and_type_only_vite_plus_imports() {
        // After the migration rewriter has rewritten the `vite` specifier to
        // `vite-plus`, this helper collapses the resulting duplicate imports.
        let input = r#"import type { Plugin } from 'vite-plus'
import { defineConfig } from 'vite-plus'
"#;
        let expected = "import { defineConfig, type Plugin } from 'vite-plus';\n";
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")).as_deref(), Some(expected));
    }

    #[test]
    fn merges_two_value_imports_from_same_source() {
        let input = r#"import { defineConfig } from 'vite-plus'
import { Plugin } from 'vite-plus'
"#;
        let expected = "import { defineConfig, Plugin } from 'vite-plus';\n";
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")).as_deref(), Some(expected));
    }

    #[test]
    fn merges_two_type_only_imports() {
        let input = r#"import type { Plugin } from 'vite-plus'
import type { UserConfig } from 'vite-plus'
"#;
        let expected = "import { type Plugin, type UserConfig } from 'vite-plus';\n";
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")).as_deref(), Some(expected));
    }

    #[test]
    fn does_not_merge_namespace_imports() {
        let input = r#"import * as Vite from 'vite-plus'
import { defineConfig } from 'vite-plus'
"#;
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")), None);
    }

    #[test]
    fn does_not_merge_default_with_type_only_decl() {
        let input = r#"import type Default from 'vite-plus'
import { a } from 'vite-plus'
"#;
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")), None);
    }

    #[test]
    fn does_not_merge_different_sources() {
        let input = r#"import { a } from 'x'
import { b } from 'y'
"#;
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")), None);
    }

    #[test]
    fn does_not_merge_conflicting_local_bindings() {
        let input = r#"import { a } from 'vite-plus'
import { b as a } from 'vite-plus'
"#;
        assert_eq!(merge_duplicate_imports(input, Some("file.ts")), None);
    }
}
