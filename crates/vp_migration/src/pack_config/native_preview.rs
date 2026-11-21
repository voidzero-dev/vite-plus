use ast_grep_core::{Doc, Node};
use ast_grep_language::{LanguageExt, SupportLang};

use super::{can_edit_object, find_property};
use crate::vite_config::{is_define_config_call, rewrite_pack_dts_generators};

#[derive(Clone, Copy)]
enum Config<'a> {
    Vite(Option<&'a str>, bool),
    Pack(Option<&'a str>),
    Dts,
}

#[derive(Clone, Copy)]
enum Compiler {
    Other,
    Classic,
    Tsgo,
    Standalone,
    Manual,
}

impl Compiler {
    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::Standalone, Self::Other | Self::Standalone)
            | (Self::Other, Self::Standalone) => Self::Standalone,
            (Self::Standalone, _)
            | (_, Self::Standalone)
            | (Self::Manual, _)
            | (_, Self::Manual)
            | (Self::Classic, Self::Tsgo)
            | (Self::Tsgo, Self::Classic) => Self::Manual,
            (Self::Tsgo, _) | (_, Self::Tsgo) => Self::Tsgo,
            (Self::Classic, _) | (_, Self::Classic) => Self::Classic,
            _ => Self::Other,
        }
    }
}

/// Check whether replacing a project's native-preview compiler with TypeScript 7
/// can preserve its pack config. An indirect or conflicting selection needs review.
pub fn classify_native_preview_pack_config(
    source: &str,
    standalone: bool,
    tsdown_config: Option<&str>,
) -> &'static str {
    // Apply the same legacy selector precedence as the actual source migration.
    let normalized = rewrite_pack_dts_generators(source, standalone);
    let parsed = SupportLang::TypeScript.ast_grep(&normalized);
    let root = parsed.root();
    let import = tsdown_config.and_then(|file| standalone_import(&root, file));
    let config = if standalone {
        Config::Pack(None)
    } else {
        Config::Vite(import.as_deref(), tsdown_config.is_some())
    };
    let selection =
        default_export(&root).map_or(Compiler::Manual, |value| classify(&value, config));
    match selection {
        Compiler::Tsgo => "tsgo",
        Compiler::Standalone => "standalone",
        Compiler::Manual => "manual",
        Compiler::Other | Compiler::Classic => "none",
    }
}

fn default_export<'a, D: Doc>(root: &Node<'a, D>) -> Option<Node<'a, D>> {
    if let Some(export) = root
        .children()
        .find(|n| n.kind() == "export_statement" && n.children().any(|c| c.kind() == "default"))
    {
        return export.field("value");
    }
    // Also accept a single direct CommonJS export. Additional statements such as
    // `module.exports.pack = getConfig()` could change the value after assignment.
    let statement = root.children().filter(|n| n.is_named() && n.kind() != "comment").last()?;
    if statement.kind() != "expression_statement" {
        return None;
    }
    let assignment = statement.children().find(|n| n.kind() == "assignment_expression")?;
    if !assignment.children().any(|n| n.kind() == "=")
        || assignment.field("left")?.text() != "module.exports"
    {
        return None;
    }
    assignment.field("right")
}

fn standalone_import<D: Doc>(root: &Node<'_, D>, filename: &str) -> Option<String> {
    let specifier = format!("./{filename}");
    let runtime_specifier = specifier.strip_suffix("ts").map(|prefix| format!("{prefix}js"));
    let import = root.children().find(|n| {
        n.kind() == "import_statement"
            && !n.children().any(|c| c.kind() == "type")
            && n.field("source").is_some_and(|value| {
                let source = value.text();
                let source = source.trim_matches(['\'', '"']);
                source == specifier || Some(source) == runtime_specifier.as_deref()
            })
    })?;
    let clause = import.children().find(|n| n.kind() == "import_clause")?;
    clause.children().find(|n| n.kind() == "identifier").map(|n| n.text().into_owned())
}

fn classify<D: Doc>(node: &Node<'_, D>, config: Config<'_>) -> Compiler {
    let mut children = node.children().filter(|n| n.is_named() && n.kind() != "comment");
    match node.kind().as_ref() {
        "parenthesized_expression"
        | "as_expression"
        | "satisfies_expression"
        | "non_null_expression" => {
            children.next().map_or(Compiler::Manual, |value| classify(&value, config))
        }
        "type_assertion" => {
            children.last().map_or(Compiler::Manual, |value| classify(&value, config))
        }
        "ternary_expression" => match (node.field("consequence"), node.field("alternative")) {
            (Some(a), Some(b)) => classify(&a, config).combine(classify(&b, config)),
            _ => Compiler::Manual,
        },
        "array" if !matches!(config, Config::Dts) => {
            children.fold(Compiler::Other, |result, item| result.combine(classify(&item, config)))
        }
        "call_expression" if !matches!(config, Config::Dts) && is_define_config_call(node) => {
            let Some(args) = node.field("arguments") else { return Compiler::Manual };
            let mut args = args.children().filter(|n| n.is_named() && n.kind() != "comment");
            match (args.next(), args.next()) {
                (Some(arg), None) => classify(&arg, config),
                _ => Compiler::Manual,
            }
        }
        "arrow_function" | "function_expression" if !matches!(config, Config::Dts) => {
            let Some(body) = node.field("body") else { return Compiler::Manual };
            if body.kind() != "statement_block" {
                return classify(&body, config);
            }
            // Follow a simple return, without executing a function with other control flow.
            let mut statements = body.children().filter(|n| n.is_named() && n.kind() != "comment");
            match (statements.next(), statements.next()) {
                (Some(ret), None) if ret.kind() == "return_statement" => ret
                    .children()
                    .find(|n| n.is_named() && n.kind() != "comment")
                    .map_or(Compiler::Manual, |value| classify(&value, config)),
                _ => Compiler::Manual,
            }
        }
        "object" if can_edit_object(node) => match config {
            Config::Vite(import, has_standalone) => property(node, "pack")
                .map_or(if has_standalone { Compiler::Standalone } else { Compiler::Other }, |v| {
                    classify(&v, Config::Pack(import))
                }),
            Config::Pack(_) => {
                property(node, "dts").map_or(Compiler::Other, |v| classify(&v, Config::Dts))
            }
            Config::Dts => classify_dts(node),
        },
        "identifier" | "shorthand_property_identifier" if matches!(config, Config::Pack(Some(name)) if node.text() == name) =>
        {
            // A callback parameter may shadow a module import.
            if node
                .ancestors()
                .any(|n| matches!(n.kind().as_ref(), "arrow_function" | "function_expression"))
            {
                Compiler::Manual
            } else {
                Compiler::Standalone
            }
        }
        "true" | "false" if matches!(config, Config::Dts) => Compiler::Other,
        _ => Compiler::Manual,
    }
}

// A shorthand, getter or method is a present, dynamic value.
fn property<'a, D: Doc>(object: &Node<'a, D>, key: &str) -> Option<Node<'a, D>> {
    let prop = find_property(object, key)?;
    Some(prop.field("value").unwrap_or(prop))
}

fn classify_dts<D: Doc>(options: &Node<'_, D>) -> Compiler {
    if property(options, "enabled").is_some_and(|n| n.kind() == "false") {
        return Compiler::Other;
    }
    // Unconverted selectors occur in dynamic configs the source rewriter cannot edit.
    if ["tsgo", "oxc"]
        .into_iter()
        .filter_map(|key| property(options, key))
        .any(|n| n.kind() != "object")
    {
        return Compiler::Manual;
    }
    let selection = match property(options, "generator").as_ref().map(Node::text).as_deref() {
        Some("'tsgo'" | "\"tsgo\"") => Compiler::Tsgo,
        Some("'tsc'" | "\"tsc\"") => Compiler::Classic,
        Some("'oxc'" | "\"oxc\"" | "'tsrs'" | "\"tsrs\"") | None => Compiler::Other,
        Some(_) => Compiler::Manual,
    };
    if matches!(selection, Compiler::Tsgo)
        && property(options, "tsgo").is_some_and(|n| {
            !can_edit_object(&n)
                || property(&n, "path").is_some()
                || property(&n, "moduleUrl").is_some()
        })
    {
        return Compiler::Manual;
    }
    // Vue/custom language processors may need the classic TypeScript API.
    let classic = property(options, "vue").is_some_and(|n| n.kind() != "false")
        || property(options, "customLanguages").is_some()
        || property(options, "tsc").is_some_and(|n| {
            n.kind() != "object"
                || !can_edit_object(&n)
                || property(&n, "vue").is_some_and(|vue| vue.kind() != "false")
        });
    if classic { selection.combine(Compiler::Classic) } else { selection }
}

#[cfg(test)]
mod tests {
    use super::classify_native_preview_pack_config;

    #[test]
    fn recognizes_compilers_without_running_user_code() {
        for (dts, expected) in [
            ("true", "none"),
            ("{ typecheck: true }", "none"),
            ("{ generator: 'tsgo' }", "tsgo"),
            ("{ tsgo: true, oxc: true }", "tsgo"),
            ("{ tsgo: {} }", "tsgo"),
            ("{ tsgo: false, oxc: true }", "none"),
            ("{ tsgo: true, generator: 'tsc' }", "none"),
            ("enabled ? { generator: 'tsgo' } : true", "tsgo"),
            ("{ enabled: false, generator: 'tsgo' }", "none"),
            ("{ generator: choice }", "manual"),
            ("{ ...spread, tsgo: true }", "manual"),
            ("{ [compiler]: true }", "manual"),
            ("{ tsgo: useGo }", "manual"),
            ("{ tsgo: true, oxc: useOxc }", "manual"),
            ("{ tsgo: { path: './native' } }", "manual"),
            ("{ generator: 'tsgo', tsgo: { moduleUrl: url } }", "manual"),
            ("{ tsgo: { ...options } }", "manual"),
            ("{ generator: 'tsgo', tsc: { vue: true } }", "manual"),
        ] {
            let pack = format!("{{ dts: {dts} }}");
            for (source, standalone) in [
                (format!("export default defineConfig({pack});"), true),
                (format!("export default defineConfig(() => ({{ pack: [{pack}] }}));"), false),
            ] {
                assert_eq!(
                    classify_native_preview_pack_config(&source, standalone, None),
                    expected,
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn honors_only_the_selected_standalone_config() {
        for (source, expected) in [
            ("export default {}", "standalone"),
            ("export default { pack: {} }", "none"),
            (
                "import options from './tsdown.config.js'; export default { pack: options }",
                "standalone",
            ),
            (
                "import pack from './tsdown.config.ts'; export default defineConfig({ pack })",
                "standalone",
            ),
            ("import pack from './other.ts'; export default { pack }", "manual"),
            (
                "import pack from './tsdown.config.ts'; export default defineConfig(pack => ({ pack }))",
                "manual",
            ),
            ("'use strict'; module.exports = { pack: { dts: { generator: 'tsgo' } } }", "tsgo"),
            (
                "module.exports = {pack:{dts:{generator:'tsgo'}}}; if (cond) module.exports = config;",
                "manual",
            ),
        ] {
            assert_eq!(
                classify_native_preview_pack_config(source, false, Some("tsdown.config.ts")),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    fn accounts_for_other_declaration_configs_before_upgrading_typescript() {
        for (source, standalone, expected) in [
            (
                "export default { pack: [{ dts: { generator: 'tsgo' } }, { dts: { generator: 'tsc' } }] }",
                false,
                "manual",
            ),
            ("export default [{ dts: { tsgo: true } }, { dts: { vue: true } }]", true, "manual"),
            (
                "export default defineConfig(() => { return { dts: { tsgo: true } }; })",
                true,
                "tsgo",
            ),
            ("export default { plugins: [plugin({ dts: { tsgo: true } })] }", false, "none"),
            ("export default { pack: config }", false, "manual"),
            ("export default defineConfig(() => sharedConfig)", false, "manual"),
            ("export default [{ dts: { tsgo: true } }, ...others]", true, "manual"),
            ("export default { ...otherConfig, dts: { tsgo: true } }", true, "manual"),
        ] {
            assert_eq!(
                classify_native_preview_pack_config(source, standalone, None),
                expected,
                "{source}"
            );
        }
    }
}
