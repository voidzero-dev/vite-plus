use std::ops::Range;

use ast_grep_core::{Doc, Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};

use crate::vite_config::{
    is_direct_recognized_config_object, pair_key_matches, rewrite_pack_dts_generators,
};

type Edit = (Range<usize>, String);

/// Upgrade the configuration options removed in tsdown 0.23 without evaluating
/// user code. Only direct pack objects and standalone tsdown configs qualify.
pub(crate) fn rewrite_pack_config(content: &str, standalone: bool) -> String {
    let grep = SupportLang::TypeScript.ast_grep(content);
    let mut edits = Vec::new();
    for object in grep.root().dfs().filter(|node| node.kind() == "object") {
        if !is_pack_object(&object, standalone) {
            continue;
        }
        if external_skip_needs_manual_migration(&object) {
            continue;
        }
        let source = object.text();
        let rewritten = rewrite_options(&object);
        if rewritten != source {
            edits.push((object.range(), rewritten));
        }
    }
    // Select the declaration generator after the other option edits.
    rewrite_pack_dts_generators(&apply_edits(content, edits, 0), standalone)
}

pub(crate) fn is_pack_object<D: Doc>(object: &Node<'_, D>, standalone: bool) -> bool {
    let mut value = object.clone();
    loop {
        // The recognizer needs the object *inside* a concise arrow's parentheses.
        if standalone && is_top_config_value(&value) {
            return true;
        }
        let Some(parent) = value.parent() else { break };
        match parent.kind().as_ref() {
            "array" | "parenthesized_expression" | "satisfies_expression" | "as_expression" => {
                value = parent;
            }
            _ => break,
        }
    }
    value.parent().is_some_and(|pair| {
        pair.kind() == "pair"
            && pair.field("key").is_some_and(|key| pair_key_matches(&key, "pack"))
            && pair.parent().is_some_and(|object| is_top_config_value(&object))
    })
}

fn is_top_config_value<D: Doc>(node: &Node<'_, D>) -> bool {
    is_direct_recognized_config_object(node)
        && !node.ancestors().any(|ancestor| ancestor.kind() == "object")
}

pub(crate) fn property_comments<D: Doc>(node: &Node<'_, D>) -> String {
    let mut comments = String::new();
    for child in node.children().filter(|child| child.kind() == "comment") {
        comments.push_str(&child.text());
        comments.push('\n');
    }
    comments
}

fn apply_edits(content: &str, mut edits: Vec<Edit>, offset: usize) -> String {
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut result = content.to_owned();
    for (range, replacement) in edits {
        result.replace_range(range.start - offset..range.end - offset, &replacement);
    }
    result
}

fn find_property<'a, D: Doc>(node: &Node<'a, D>, name: &str) -> Option<Node<'a, D>> {
    node.children().find(|child| {
        child
            .field("key")
            .or_else(|| child.field("name"))
            .is_some_and(|key| pair_key_matches(&key, name))
            || child.kind() == "shorthand_property_identifier" && child.text() == name
    })
}

fn property_value<'a, D: Doc>(node: &Node<'a, D>, name: &str) -> Option<Node<'a, D>> {
    let property = find_property(node, name)?;
    property
        .field("value")
        .or_else(|| (property.kind() == "shorthand_property_identifier").then_some(property))
}

/// Edits only direct properties. Spreads, duplicate keys and computed keys make
/// property precedence unknown, so leave such objects for manual migration.
struct ObjectEditor<'a, D: Doc> {
    node: Node<'a, D>,
    edits: Vec<Edit>,
    additions: Vec<String>,
}

impl<'a, D: Doc> ObjectEditor<'a, D> {
    fn property(&self, name: &str) -> Option<Node<'a, D>> {
        find_property(&self.node, name)
    }

    fn value(&self, name: &str) -> Option<Node<'a, D>> {
        property_value(&self.node, name)
    }

    fn remove(&mut self, name: &str) {
        let Some(property) = self.property(name) else { return };
        self.edits.push((property.range(), property_comments(&property)));
        if let Some(next) = property.next_all().find(|node| node.kind() != "comment")
            && next.kind() == ","
        {
            self.edits.push((next.range(), String::new()));
        }
    }

    fn rename(&mut self, old: &str, new: &str) {
        let Some(property) = self.property(old) else { return };
        if self.property(new).is_some() {
            return;
        }
        if let Some(key) = property.field("key").or_else(|| property.field("name")) {
            self.edits.push((key.range(), new.to_owned()));
        } else if property.kind() == "shorthand_property_identifier" {
            self.edits.push((property.range(), format!("{new}: {old}")));
        }
    }

    fn set_default(&mut self, name: &str, value: &str) {
        if self.property(name).is_none() {
            self.additions.push(format!("{name}: {value}"));
        }
    }

    fn replace_value(&mut self, name: &str, replacement: String) {
        let Some(value) = self.value(name) else { return };
        self.edits.push((value.range(), replacement));
    }

    fn finish(mut self) -> String {
        if !self.additions.is_empty() {
            let start = self.node.range().start + 1;
            self.edits.push((start..start, format!(" {},", self.additions.join(", "))));
        }
        apply_edits(&self.node.text(), self.edits, self.node.range().start)
    }
}

fn edit_object(
    source: &str,
    edit: impl FnOnce(&mut ObjectEditor<'_, StrDoc<SupportLang>>),
) -> String {
    let wrapped = format!("({source})");
    let grep = SupportLang::TypeScript.ast_grep(&wrapped);
    let root = grep.root();
    let Some(node) = root.dfs().find(|node| node.kind() == "object") else {
        return source.to_owned();
    };
    if !can_edit_object(&node) {
        return source.to_owned();
    }
    let mut editor = ObjectEditor { node, edits: Vec::new(), additions: Vec::new() };
    edit(&mut editor);
    editor.finish()
}

pub(crate) fn can_edit_object<D: Doc>(node: &Node<'_, D>) -> bool {
    let mut names = std::collections::HashSet::new();
    for child in node.children() {
        if child.kind() == "spread_element" {
            return false;
        }
        if let Some(key) = child.field("key").or_else(|| child.field("name")) {
            if key.kind() == "computed_property_name"
                || !names.insert(key.text().trim_matches(['\'', '"']).to_owned())
            {
                return false;
            }
        } else if child.kind() == "shorthand_property_identifier"
            && !names.insert(child.text().into_owned())
        {
            return false;
        }
    }
    true
}

const EXTERNAL_SKIP_WARNING: &str = "Cannot safely combine external with skipNodeModulesBundle. Migrate this pack config manually; its options were left unchanged.";

pub(crate) fn pack_config_warnings(content: &str, standalone: bool) -> Vec<String> {
    let grep = SupportLang::TypeScript.ast_grep(content);
    if grep.root().dfs().any(|node| {
        node.kind() == "object"
            && is_pack_object(&node, standalone)
            && external_skip_needs_manual_migration(&node)
    }) {
        vec![EXTERNAL_SKIP_WARNING.to_owned()]
    } else {
        Vec::new()
    }
}

fn has_external_skip<D: Doc>(config: &Node<'_, D>) -> bool {
    if find_property(config, "external").is_none() {
        return false;
    }
    property_value(config, "skipNodeModulesBundle").is_some_and(|value| value.kind() == "true")
        || property_value(config, "deps").is_some_and(|deps| {
            deps.kind() == "object"
                && property_value(&deps, "skipNodeModulesBundle")
                    .is_some_and(|value| value.kind() == "true")
        })
}

pub(crate) fn external_skip_needs_manual_migration<D: Doc>(config: &Node<'_, D>) -> bool {
    if !has_external_skip(config) {
        return false;
    }
    if !can_edit_object(config)
        || property_value(config, "external").is_none_or(|value| !is_static_external(&value))
    {
        return true;
    }
    for (namespace, conflicts) in
        [("deps", &["neverBundle", "dts"][..]), ("inputOptions", &["external"][..])]
    {
        if find_property(config, namespace).is_some() {
            let Some(value) = property_value(config, namespace) else { return true };
            if value.kind() != "object"
                || !can_edit_object(&value)
                || conflicts.iter().any(|name| find_property(&value, name).is_some())
            {
                return true;
            }
        }
    }
    false
}

fn is_static_external<D: Doc>(value: &Node<'_, D>) -> bool {
    match value.kind().as_ref() {
        // tsdown interprets a top-level '/pattern/' string as a regular
        // expression. Leave that form, and dynamic matchers, for manual review.
        "string" => {
            let text = value.text();
            let regex_string = text.as_bytes().get(1) == Some(&b'/')
                && text.as_bytes().get(text.len() - 2) == Some(&b'/');
            !(text.contains('\\') || regex_string)
        }
        "regex" => true,
        // Array entries pass through tsdown without string-to-regexp conversion.
        "array" => value.children().all(|child| {
            matches!(child.kind().as_ref(), "[" | "]" | "," | "comment" | "string" | "regex")
        }),
        _ => false,
    }
}

fn rewrite_options<D: Doc>(object: &Node<'_, D>) -> String {
    // Rolldown can retain the original static matcher while tsdown's deps
    // plugin handles neverBundle: true. Avoid overriding DTS-specific matchers
    // or user inputOptions; those combinations are reported for manual review.
    let source = if has_external_skip(object) {
        move_option(&object.text(), "external", "inputOptions", "external", false)
    } else {
        object.text().into_owned()
    };
    // First update nested namespaces; subsequent moves see the new keys and
    // cannot create duplicate deps/css objects or overwrite explicit settings.
    let source = edit_object(&source, |config| {
        for name in ["deps", "dts", "attw"] {
            let Some(value) = config.value(name) else { continue };
            if value.kind() != "object" {
                continue;
            }
            let updated = edit_object(&value.text(), |options| match name {
                "deps" => {
                    options.rename("onlyAllowBundle", "onlyBundle");
                    if let Some(skip) = options.value("skipNodeModulesBundle") {
                        if skip.kind() == "false" {
                            options.remove("skipNodeModulesBundle");
                        } else if skip.kind() == "true" && options.property("neverBundle").is_none()
                        {
                            options.rename("skipNodeModulesBundle", "neverBundle");
                        }
                    }
                    options.set_default("resolveDepSubpath", "true");
                }
                "dts" => {
                    if options
                        .value("cjsReexport")
                        .is_some_and(|value| matches!(value.kind().as_ref(), "true" | "false"))
                    {
                        options.remove("cjsReexport");
                    }
                }
                "attw" => {
                    if options.value("enabled").is_none_or(|value| value.kind() != "false") {
                        options.set_default("profile", "'strict'");
                    }
                }
                _ => unreachable!(),
            });
            config.replace_value(name, updated);
        }
    });
    let source = edit_object(&source, |config| {
        config.rename("outExtension", "outExtensions");
        config.rename("publicDir", "copy");
        for (old, new, replacement) in
            [("bundle", "unbundle", "true"), ("removeNodeProtocol", "nodeProtocol", "'strip'")]
        {
            let Some(value) = config.value(old) else { continue };
            let active = if old == "bundle" { "false" } else { "true" };
            if value.kind() == active && config.property(new).is_none() {
                config.rename(old, new);
                config.replace_value(old, replacement.to_owned());
            } else if matches!(value.kind().as_ref(), "true" | "false")
                && (value.kind() != active || old == "bundle")
            {
                config.remove(old);
            }
        }
        if config.value("attw").is_some_and(|value| value.kind() == "true") {
            config.replace_value("attw", "{ profile: 'strict' }".to_owned());
        }
    });
    let source = move_option(&source, "injectStyle", "css", "inject", false);
    let source = move_option(&source, "inlineOnly", "deps", "onlyBundle", false);
    let source = move_option(&source, "skipNodeModulesBundle", "deps", "neverBundle", true);
    edit_object(&source, |config| {
        config.set_default("deps", "{ resolveDepSubpath: true }");
    })
}

fn move_option(source: &str, old: &str, group: &str, new: &str, boolean: bool) -> String {
    edit_object(source, |config| {
        let Some(value) = config.value(old) else { return };
        if boolean {
            match value.kind().as_ref() {
                "false" => {
                    config.remove(old);
                    return;
                }
                "true" => {}
                _ => return,
            }
        }
        if let Some(namespace) = config.value(group) {
            if namespace.kind() != "object" {
                return;
            }
            let mut moved = false;
            let updated = edit_object(&namespace.text(), |options| {
                if options.property(new).is_none() {
                    options.set_default(new, &value.text());
                    moved = true;
                }
            });
            if moved {
                config.replace_value(group, updated);
                config.remove(old);
            }
        } else if config.property(group).is_none() {
            let defaults = if group == "deps" { ", resolveDepSubpath: true" } else { "" };
            // Replace in place so comments on the old option stay attached.
            if let Some(property) = config.property(old) {
                config.edits.push((
                    property.range(),
                    format!("{group}: {{ {new}: {}{defaults} }}", value.text()),
                ));
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate(options: &str) -> String {
        let input = format!("export default defineConfig({{ pack: {options} }});");
        let actual = rewrite_pack_config(&input, false);
        assert_eq!(rewrite_pack_config(&actual, false), actual, "migration must be idempotent");
        let grep = SupportLang::TypeScript.ast_grep(&actual);
        assert!(!grep.root().dfs().any(|node| node.kind() == "ERROR"), "{actual}");
        actual
    }

    #[test]
    fn migrates_static_external_with_both_skip_forms() {
        for external in
            ["['foo']", "['foo', /^virtual:/, './local.js']", "'foo'", "/^virtual:/", "[]"]
        {
            for skip in ["skipNodeModulesBundle: true", "deps: { skipNodeModulesBundle: true }"] {
                let actual = migrate(&format!("{{ external: {external}, {skip} }}"));
                assert!(
                    actual.contains(&format!("inputOptions: {{ external: {external} }}")),
                    "{actual}"
                );
                assert!(actual.contains("neverBundle: true"), "{actual}");
                assert!(!actual.contains("skipNodeModulesBundle"), "{actual}");
                assert!(pack_config_warnings(&actual, false).is_empty());
            }
        }
        let actual = migrate(
            "{ external: ['foo'], skipNodeModulesBundle: true, inputOptions: { treeshake: false } }",
        );
        assert!(actual.contains("external: ['foo'], treeshake: false"), "{actual}");
    }

    #[test]
    fn unsafe_external_combinations_stay_unchanged_and_warn() {
        for options in [
            "external: dynamicExternal",
            "external",
            "external: (id) => id === 'foo'",
            "external: '/foo/'",
            "external: ['foo'], inputOptions: customOptions",
            "external: ['foo'], inputOptions: { ...customOptions }",
            "external: ['foo'], inputOptions: { external: ['bar'] }",
            "external: ['foo'], inputOptions: { external() {} }",
            "external: ['foo'], ...otherOptions",
        ] {
            for skip in ["skipNodeModulesBundle: true", "deps: { skipNodeModulesBundle: true }"] {
                let input = format!(
                    "export default {{ pack: {{ {options}, {skip}, bundle: false, dts: {{ tsgo: true }} }} }};"
                );
                assert_eq!(rewrite_pack_config(&input, false), input);
                assert_eq!(pack_config_warnings(&input, false), [EXTERNAL_SKIP_WARNING]);
            }
        }
        let input = "export default { pack: { external: ['foo'], deps: { skipNodeModulesBundle: true, dts: { neverBundle: ['types'] } } } };";
        assert_eq!(rewrite_pack_config(input, false), input);
        assert_eq!(pack_config_warnings(input, false), [EXTERNAL_SKIP_WARNING]);
        for input in [
            "export default { pack: { external: ['foo'], skipNodeModulesBundle: false } };",
            "export default { plugins: [plugin({ external, skipNodeModulesBundle: true })] };",
        ] {
            assert!(pack_config_warnings(input, false).is_empty());
        }
    }

    #[test]
    fn standalone_concise_arrows_migrate_unbundle_and_generator() {
        for input in [
            "export default defineConfig(() => ({ bundle: false, dts: { tsgo: true } }));",
            "export default defineConfig(async () => ({ bundle: false, dts: { tsgo: true } }));",
            "export default defineConfig(() => ([{ bundle: false, dts: { tsgo: true } }]));",
            "export default defineConfig(() => (({ bundle: false, dts: { tsgo: true } }) satisfies UserConfig));",
            "export default defineConfig(() => { return { bundle: false, dts: { tsgo: true } }; });",
        ] {
            let actual = rewrite_pack_config(input, true);
            assert!(actual.contains("unbundle: true"), "{actual}");
            assert!(actual.contains("generator: 'tsgo'"), "{actual}");
            assert!(!actual.contains("tsgo: true"), "{actual}");
            assert_eq!(rewrite_pack_config(&actual, true), actual);
        }
    }

    #[test]
    fn renames_method_options_without_changing_bodies() {
        let input = "export default defineConfig({ outExtension() { return { js: '.custom.js' }; }, async 'publicDir'() { /* assets */ return ['assets']; } });";
        let actual = rewrite_pack_config(input, true);
        assert!(actual.contains("outExtensions() { return { js: '.custom.js' }; }"), "{actual}");
        assert!(actual.contains("async copy() { /* assets */ return ['assets']; }"), "{actual}");
        assert_eq!(rewrite_pack_config(&actual, true), actual);
    }

    #[test]
    fn removed_options_and_previous_defaults() {
        let actual = migrate(
            r#"{
            bundle: false,
            outExtension: ({ format }) => ({ js: `.${format}.js` }),
            publicDir: ['public'],
            removeNodeProtocol: true,
            injectStyle: false,
            inlineOnly: [/^allowed/],
            skipNodeModulesBundle: true,
            dts: { tsgo: true, cjsReexport: false, sourcemap: true },
            attw: true,
        }"#,
        );
        for expected in [
            "unbundle: true",
            "outExtensions: ({ format })",
            "copy: ['public']",
            "nodeProtocol: 'strip'",
            "inject: false",
            "onlyBundle: [/^allowed/]",
            "neverBundle: true",
            "resolveDepSubpath: true",
            "generator: 'tsgo'",
            "sourcemap: true",
            "profile: 'strict'",
        ] {
            assert!(actual.contains(expected), "missing {expected}: {actual}");
        }
        for removed in [
            "bundle:",
            "outExtension:",
            "publicDir",
            "removeNodeProtocol",
            "injectStyle",
            "inlineOnly",
            "skipNodeModulesBundle",
            "cjsReexport",
            "tsgo:",
        ] {
            // unbundle contains bundle as a substring.
            assert!(!actual.contains(&format!(" {removed}")), "{actual}");
        }
    }

    #[test]
    fn comments_inside_removed_properties_remain_valid() {
        let actual =
            migrate("{ dts: { cjsReexport: // removed option\ntrue, tsgo: /* compiler */ true } }");
        assert!(actual.contains("// removed option\n"), "{actual}");
        assert!(actual.contains("/* compiler */"), "{actual}");
    }

    #[test]
    fn nested_define_config_calls_are_not_pack_configs() {
        let input = "export default defineConfig({ plugins: [defineConfig({ bundle: false, dts: { tsgo: true } })] });";
        let actual = rewrite_pack_config(input, true);
        assert!(actual.contains("plugins: [defineConfig({ bundle: false, dts: { tsgo: true } })]"));
        assert_eq!(actual.matches("resolveDepSubpath").count(), 1);
        assert_eq!(rewrite_pack_config(&actual, true), actual);
        assert_eq!(rewrite_pack_config(input, false), input);
    }

    #[test]
    fn nested_options_and_explicit_defaults() {
        let actual = migrate(
            r#"{
            bundle: true, removeNodeProtocol: false,
            deps: { onlyAllowBundle: false, skipNodeModulesBundle: true, resolveDepSubpath: false },
            css: { modules: true }, injectStyle: true,
            dts: { oxc: true, cjsReexport: true },
            attw: { profile: 'node16', enabled: false },
        }"#,
        );
        for expected in [
            "onlyBundle: false",
            "neverBundle: true",
            "resolveDepSubpath: false",
            "modules: true",
            "inject: true",
            "generator: 'oxc'",
            "profile: 'node16'",
            "enabled: false",
        ] {
            assert!(actual.contains(expected), "{actual}");
        }
        assert!(!actual.contains("skipNodeModulesBundle"));
        assert!(!actual.contains("cjsReexport"));
        assert!(!actual.contains("unbundle"));
        assert!(!actual.contains("nodeProtocol"));
    }

    #[test]
    fn preserves_method_conflicts() {
        let actual = migrate(
            "{ outExtension: extensions, outExtensions() { return {}; }, publicDir: 'public', copy() { return []; }, deps: { onlyBundle() { return false; } }, inlineOnly: false }",
        );
        for expected in [
            "outExtension: extensions",
            "outExtensions()",
            "publicDir: 'public'",
            "copy()",
            "onlyBundle()",
            "inlineOnly: false",
        ] {
            assert!(actual.contains(expected), "{actual}");
        }
    }

    #[test]
    fn shorthand_and_comments() {
        let actual = migrate(
            "{ publicDir, outExtension, inlineOnly, deps: { /* deps */ }, dts: { cjsReexport: true /* keep */ }, /* tail */ }",
        );
        for expected in [
            "copy: publicDir",
            "outExtensions: outExtension",
            "onlyBundle: inlineOnly",
            "/* deps */",
            "/* keep */",
            "/* tail */",
        ] {
            assert!(actual.contains(expected), "{actual}");
        }
    }

    #[test]
    fn skips_ambiguous_objects_and_conflicts() {
        for options in [
            "{ ...shared, bundle: false }",
            "{ [key]: value, bundle: false }",
            "{ bundle: false, 'bundle': true }",
            "{ ...shared, dts: { tsgo: true } }",
            "{ dts: { tsgo: true, tsgo: false }, deps: { resolveDepSubpath: true } }",
        ] {
            let input = format!("export default {{ pack: {options} }};");
            assert_eq!(rewrite_pack_config(&input, false), input);
        }
        let actual = migrate(
            "{ publicDir: 'old', copy: 'new', injectStyle: true, css: cssOptions, inlineOnly: ['x'], deps: { onlyBundle: ['y'], resolveDepSubpath: false }, dts: { ...dtsOptions, cjsReexport: true }, attw: attwOptions }",
        );
        for expected in [
            "publicDir: 'old'",
            "copy: 'new'",
            "injectStyle: true",
            "css: cssOptions",
            "inlineOnly: ['x']",
            "onlyBundle: ['y']",
            "cjsReexport: true",
            "attw: attwOptions",
        ] {
            assert!(actual.contains(expected), "{actual}");
        }
    }

    #[test]
    fn scope_arrays_callbacks_and_json() {
        for (input, standalone) in [
            ("export default defineConfig([{ bundle: false }, { publicDir: 'public' }]);", true),
            ("export default defineConfig(() => ({ pack: [{ bundle: false }] }));", false),
            (
                "export default defineConfig(async () => { return { pack: { bundle: false } }; });",
                false,
            ),
            ("export default { pack: ({ bundle: false } satisfies PackConfig) };", false),
            ("export default { pack: { \"bundle\": false, \"dts\": { \"tsgo\": true } } };", false),
        ] {
            let actual = rewrite_pack_config(input, standalone);
            assert!(!actual.contains("bundle: false"), "{actual}");
            assert!(actual.contains("resolveDepSubpath: true"), "{actual}");
            assert_eq!(rewrite_pack_config(&actual, standalone), actual);
        }
        for input in [
            "export default { publicDir: 'vite-public', plugins: [plugin({ bundle: false })] };",
            "export default { test: { pack: { bundle: false } } };",
            "export default defineConfig({ plugins: [{ config() { return { pack: { bundle: false } }; } }] });",
            "const config = { bundle: false }; export default config;",
        ] {
            assert_eq!(rewrite_pack_config(input, false), input);
        }
    }

    #[test]
    fn generator_objects_keep_their_options() {
        for (options, generator, expected) in [
            (
                "{ dts: { tsgo: { path: './tsgo' }, oxc: true } }",
                "tsgo",
                "tsgo: { path: './tsgo' }",
            ),
            (
                "{ dts: { oxc: { stripInternal: true }, tsgo: false } }",
                "oxc",
                "oxc: { stripInternal: true }",
            ),
            (
                "{ dts: { generator: 'tsc', tsgo: { path: './tsgo' }, oxc: true } }",
                "tsc",
                "tsgo: { path: './tsgo' }",
            ),
        ] {
            let actual = migrate(options);
            assert!(actual.contains(&format!("generator: '{generator}'")), "{actual}");
            assert!(actual.contains(expected), "{actual}");
            assert!(!actual.contains("oxc: true"), "{actual}");
            assert!(!actual.contains("tsgo: false"), "{actual}");
        }
    }

    #[test]
    fn preserve_old_defaults_without_removed_options() {
        let actual = migrate("{ entry: 'src/index.ts', attw: { enabled: true } }");
        assert!(actual.contains("resolveDepSubpath: true"));
        assert!(actual.contains("profile: 'strict'"));
        let actual = migrate("{ deps: { resolveDepSubpath: false }, attw: { enabled: false } }");
        assert!(!actual.contains("'strict'"));
        assert!(actual.contains("resolveDepSubpath: false"));
    }
}
