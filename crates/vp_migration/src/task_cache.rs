use std::{ops::Range, path::Path};

use ast_grep_core::{Doc, Node};
use ast_grep_language::{LanguageExt, SupportLang};
use vp_error::Error;

use crate::{
    pack_config::can_edit_object,
    vite_config::{is_direct_recognized_config_object, pair_key_matches},
};

/// Task settings that Vite Task only accepts inside a task's `cache` object.
const CACHE_FIELDS: [&str; 4] = ["env", "untrackedEnv", "input", "output"];

type Edit = (Range<usize>, String);

/// A moved property with its leading comments, and the line comment after it.
struct Item {
    text: String,
    comment: Option<String>,
}

/// Result of moving task cache settings under `cache`.
#[derive(Debug)]
pub struct TaskCacheMigrationResult {
    /// The updated vite config content
    pub content: String,
    /// Whether any changes were made
    pub updated: bool,
    /// Tasks that set cache settings outside `cache` but could not be updated
    pub manual_tasks: Vec<String>,
}

/// Move `env`, `untrackedEnv`, `input`, and `output` from the top level of
/// each `run.tasks` entry into its `cache` object without evaluating user code.
///
/// Only static task objects in a direct config object are updated. Tasks with
/// spreads, computed, escaped, or duplicate keys, comments the edit cannot
/// place, or a `cache` value other than `true` or an object literal are
/// reported in `manual_tasks` and left unchanged. Moved settings keep their
/// text and indentation; the formatter nests them inside `cache`.
pub fn migrate_task_cache_config(
    vite_config_path: &Path,
) -> Result<TaskCacheMigrationResult, Error> {
    let content = std::fs::read_to_string(vite_config_path)?;
    Ok(migrate_task_cache_config_content(&content))
}

fn migrate_task_cache_config_content(content: &str) -> TaskCacheMigrationResult {
    let grep = SupportLang::TypeScript.ast_grep(content);
    let mut edits = Vec::new();
    let mut manual_tasks = Vec::new();
    for task in grep.root().dfs().filter(|node| node.kind() == "object") {
        let Some(name) = task_name(&task) else { continue };
        let fields: Vec<_> = task.children().filter(is_cache_field).collect();
        if fields.is_empty() {
            continue;
        }
        match move_fields_under_cache(content, &task, &fields) {
            Some(task_edits) => edits.extend(task_edits),
            None => manual_tasks.push(name),
        }
    }

    if edits.is_empty() {
        return TaskCacheMigrationResult {
            content: content.to_owned(),
            updated: false,
            manual_tasks,
        };
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut updated = content.to_owned();
    for (range, replacement) in edits {
        updated.replace_range(range, &replacement);
    }
    TaskCacheMigrationResult { content: updated, updated: true, manual_tasks }
}

/// Returns the task name when `object` is a task in `run.tasks` of a direct
/// config object.
fn task_name<D: Doc>(object: &Node<'_, D>) -> Option<String> {
    let task = value_pair(object)?;
    let tasks = task.parent().filter(|node| node.kind() == "object")?;
    let tasks_pair = value_pair(&tasks).filter(|pair| has_key(pair, "tasks"))?;
    let run = tasks_pair.parent().filter(|node| node.kind() == "object")?;
    let run_pair = value_pair(&run).filter(|pair| has_key(pair, "run"))?;
    let config = run_pair.parent().filter(|node| node.kind() == "object")?;
    // `is_direct_recognized_config_object` looks through `satisfies` but not `as`.
    let mut asserted = config.clone();
    while let Some(parent) = asserted.parent().filter(|node| node.kind() == "as_expression") {
        asserted = parent;
    }
    if !is_direct_recognized_config_object(&asserted)
        || config.ancestors().any(|ancestor| ancestor.kind() == "object")
    {
        return None;
    }
    let key = task.field("key")?;
    Some(key.text().trim_matches(['\'', '"']).to_owned())
}

/// Returns the pair whose value is `node`, looking through parentheses and
/// type assertions.
fn value_pair<'a, D: Doc>(node: &Node<'a, D>) -> Option<Node<'a, D>> {
    let mut value = node.clone();
    loop {
        let parent = value.parent()?;
        match parent.kind().as_ref() {
            "parenthesized_expression" | "satisfies_expression" | "as_expression" => {
                value = parent;
            }
            "pair" => {
                return parent
                    .field("value")
                    .is_some_and(|pair_value| pair_value.range() == value.range())
                    .then_some(parent);
            }
            _ => return None,
        }
    }
}

fn has_key<D: Doc>(pair: &Node<'_, D>, name: &str) -> bool {
    pair.field("key").is_some_and(|key| pair_key_matches(&key, name))
}

/// The static name of a property. Escaped string keys have no name here, so
/// they cannot hide a cache setting or a conflict.
fn property_name<D: Doc>(property: &Node<'_, D>) -> Option<String> {
    match property.kind().as_ref() {
        "pair" | "method_definition" => {
            let key = property.field("key").or_else(|| property.field("name"))?;
            match key.kind().as_ref() {
                "property_identifier" => Some(key.text().into_owned()),
                "string" if !key.text().contains('\\') => {
                    Some(key.text().trim_matches(['\'', '"']).to_owned())
                }
                _ => None,
            }
        }
        "shorthand_property_identifier" => Some(property.text().into_owned()),
        _ => None,
    }
}

fn is_cache_field<D: Doc>(property: &Node<'_, D>) -> bool {
    property_name(property).is_some_and(|name| CACHE_FIELDS.contains(&name.as_str()))
}

fn find_property<'a, D: Doc>(object: &Node<'a, D>, name: &str) -> Option<Node<'a, D>> {
    object.children().find(|child| property_name(child).is_some_and(|key| key == name))
}

/// Whether every property of `object` has a unique static name.
fn can_migrate_object<D: Doc>(object: &Node<'_, D>) -> bool {
    can_edit_object(object)
        && object.children().all(|child| {
            !matches!(child.kind().as_ref(), "pair" | "method_definition")
                || property_name(&child).is_some()
        })
}

fn move_fields_under_cache<'a, D: Doc>(
    content: &str,
    task: &Node<'a, D>,
    fields: &[Node<'a, D>],
) -> Option<Vec<Edit>> {
    if !can_migrate_object(task)
        || fields
            .iter()
            .any(|field| field.kind() == "method_definition" || has_comment_before_comma(field))
    {
        return None;
    }
    let multiline = task.text().contains('\n');
    let cache = find_property(task, "cache");
    let indent = line_indent(content, cache.as_ref().unwrap_or(&fields[0]).range().start);
    let items: Vec<Item> = fields.iter().map(|field| moved_item(content, field)).collect();

    let mut edits = Vec::new();
    let removed = if let Some(cache) = cache {
        let value = cache.field("value")?;
        match value.kind().as_ref() {
            "true" => edits.push((value.range(), object_text(&items, &indent, multiline))),
            "object" => {
                edits.push(merge_into_object(content, &value, fields, &items, &indent, multiline)?)
            }
            _ => return None,
        }
        fields
    } else {
        let first = &fields[0];
        let mut replacement = format!("cache: {}", object_text(&items, &indent, multiline));
        let end = match trailing_line_comment(first) {
            // The comment moves into the object, so keep the comma it followed.
            Some(comment) => {
                if trailing_comma(first).is_some() {
                    replacement.push(',');
                }
                comment.range().end
            }
            None => first.range().end,
        };
        edits.push((leading_comment_start(first)..end, replacement));
        &fields[1..]
    };
    // Adjacent removals can share a comma, so merge overlapping ranges.
    let mut removals: Vec<Range<usize>> = removed.iter().map(removal_range).collect();
    removals.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in removals {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    // Removing the last property leaves the comma before it trailing.
    if let (Some(field), Some(range)) = (removed.last(), merged.last_mut())
        && trailing_comma(field).is_none()
        && let Some(comma) = task.children().filter(|node| node.range().end <= range.start).last()
        && comma.kind() == ","
    {
        range.start = comma.range().start;
    }
    edits.extend(merged.into_iter().map(|range| (range, String::new())));
    Some(edits)
}

/// Appends `items` to an existing `cache` object. Returns `None` when the
/// object's properties conflict with the moved fields or a comment follows
/// its last property.
fn merge_into_object<D: Doc>(
    content: &str,
    cache: &Node<'_, D>,
    fields: &[Node<'_, D>],
    items: &[Item],
    indent: &str,
    multiline: bool,
) -> Option<Edit> {
    if !can_migrate_object(cache)
        || fields.iter().any(|field| {
            property_name(field).is_some_and(|name| find_property(cache, &name).is_some())
        })
    {
        return None;
    }
    let inner: Vec<_> =
        cache.children().filter(|child| !matches!(child.kind().as_ref(), "{" | "}")).collect();
    let Some(last) = inner.last() else {
        return Some((cache.range(), object_text(items, indent, multiline)));
    };
    let has_comma = last.kind() == ",";
    let property = if has_comma { inner.iter().rev().nth(1)? } else { last };
    if property.kind() == "comment" {
        return None;
    }
    let insert_at = last.range().end;
    let mut text = if has_comma { String::new() } else { ",".to_owned() };
    if multiline {
        let indent = line_indent(content, property.range().start);
        for item in items {
            text.push_str(&format!("\n{indent}{}", item_line(item)));
        }
        // A moved line comment must not swallow the closing brace.
        if !content[insert_at..cache.range().end].contains('\n') {
            text.push('\n');
            text.push_str(&indent);
        }
    } else {
        text.push(' ');
        text.push_str(&inline_items(items));
        if has_comma {
            text.push(',');
        }
    }
    Some((insert_at..insert_at, text))
}

fn object_text(items: &[Item], indent: &str, multiline: bool) -> String {
    if !multiline {
        return format!("{{ {} }}", inline_items(items));
    }
    let mut text = String::from("{");
    for item in items {
        text.push_str(&format!("\n{indent}{}", item_line(item)));
    }
    text.push('\n');
    text.push_str(indent);
    text.push('}');
    text
}

fn item_line(item: &Item) -> String {
    match &item.comment {
        Some(comment) => format!("{}, {comment}", item.text),
        None => format!("{},", item.text),
    }
}

/// Items of a single-line task, which has no line comments to carry.
fn inline_items(items: &[Item]) -> String {
    items.iter().map(|item| item.text.as_str()).collect::<Vec<_>>().join(", ")
}

/// The field as written, with its leading comments and trailing line comment.
fn moved_item<D: Doc>(content: &str, field: &Node<'_, D>) -> Item {
    Item {
        text: content[leading_comment_start(field)..field.range().end].to_owned(),
        comment: trailing_line_comment(field).map(|comment| comment.text().into_owned()),
    }
}

/// Whether a comment sits between `property` and its comma.
fn has_comment_before_comma<D: Doc>(property: &Node<'_, D>) -> bool {
    property.next().is_some_and(|next| next.kind() == "comment")
        && property
            .next_all()
            .find(|node| node.kind() != "comment")
            .is_some_and(|node| node.kind() == ",")
}

/// A `//` comment on the line where `property` or its comma ends.
fn trailing_line_comment<'a, D: Doc>(property: &Node<'a, D>) -> Option<Node<'a, D>> {
    let mut before = property.clone();
    let mut next = property.next()?;
    if next.kind() == "," {
        before = next;
        next = before.next()?;
    }
    (next.kind() == "comment"
        && next.text().starts_with("//")
        && next.start_pos().line() == before.end_pos().line())
    .then_some(next)
}

/// Start of the comments on their own lines directly above `property`.
fn leading_comment_start<D: Doc>(property: &Node<'_, D>) -> usize {
    let mut start = property.range().start;
    let mut current = property.clone();
    while let Some(previous) = current.prev() {
        if previous.kind() != "comment" {
            break;
        }
        // A comment on the same line as the token before it belongs to that token.
        if previous
            .prev()
            .is_some_and(|before| before.end_pos().line() == previous.start_pos().line())
        {
            break;
        }
        start = previous.range().start;
        current = previous;
    }
    start
}

/// Range that removes a property with its comments and trailing comma.
fn removal_range<D: Doc>(property: &Node<'_, D>) -> Range<usize> {
    let start = leading_comment_start(property);
    let from = property
        .prev_all()
        .find(|node| node.range().end <= start)
        .map_or(start, |node| node.range().end);
    let to = trailing_line_comment(property)
        .or_else(|| trailing_comma(property))
        .map_or(property.range().end, |node| node.range().end);
    from..to
}

fn trailing_comma<'a, D: Doc>(property: &Node<'a, D>) -> Option<Node<'a, D>> {
    property.next().filter(|node| node.kind() == ",")
}

fn line_indent(content: &str, offset: usize) -> String {
    let line_start = content[..offset].rfind('\n').map_or(0, |index| index + 1);
    content[line_start..].chars().take_while(|c| matches!(c, ' ' | '\t')).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate(input: &str) -> TaskCacheMigrationResult {
        let result = migrate_task_cache_config_content(input);
        let again = migrate_task_cache_config_content(&result.content);
        assert!(!again.updated, "migration must be idempotent:\n{}", result.content);
        let grep = SupportLang::TypeScript.ast_grep(&result.content);
        assert!(
            !grep.root().dfs().any(|node| node.is_error() || node.is_missing()),
            "invalid output:\n{}",
            result.content
        );
        result
    }

    fn migrate_tasks(tasks: &str) -> String {
        let input = format!("export default defineConfig({{ run: {{ tasks: {tasks} }} }});");
        let result = migrate(&input);
        assert!(result.manual_tasks.is_empty(), "{:?}", result.manual_tasks);
        result.content
    }

    #[test]
    fn moves_fields_into_a_new_cache_object() {
        assert_eq!(
            migrate_tasks(
                "{ build: { command: 'tsc', env: ['NODE_ENV'], untrackedEnv: ['CI'], input: ['src/**'], output: ['dist/**'] } }"
            ),
            "export default defineConfig({ run: { tasks: { build: { command: 'tsc', cache: { env: ['NODE_ENV'], untrackedEnv: ['CI'], input: ['src/**'], output: ['dist/**'] } } } } });"
        );
    }

    #[test]
    fn replaces_cache_true() {
        assert_eq!(
            migrate_tasks("{ build: { env: ['A'], command: 'tsc', cache: true, input: [] } }"),
            "export default defineConfig({ run: { tasks: { build: { command: 'tsc', cache: { env: ['A'], input: [] } } } } });"
        );
    }

    #[test]
    fn merges_into_an_existing_cache_object() {
        assert_eq!(
            migrate_tasks("{ build: { command: 'tsc', cache: { env: ['A'] }, input: [] } }"),
            "export default defineConfig({ run: { tasks: { build: { command: 'tsc', cache: { env: ['A'], input: [] } } } } });"
        );
        assert_eq!(
            migrate_tasks("{ build: { command: 'tsc', cache: {}, output: [] } }"),
            "export default defineConfig({ run: { tasks: { build: { command: 'tsc', cache: { output: [] } } } } });"
        );
    }

    #[test]
    fn keeps_shorthand_properties_and_quoted_keys() {
        assert_eq!(
            migrate_tasks("{ 'build:site': { command: 'tsc', untrackedEnv, 'input': inputs } }"),
            "export default defineConfig({ run: { tasks: { 'build:site': { command: 'tsc', cache: { untrackedEnv, 'input': inputs } } } } });"
        );
    }

    #[test]
    fn keeps_multiline_fields_and_comments_as_written() {
        let input = r"export default {
  run: {
    tasks: {
      'build:site': {
        command: 'vitepress build',
        // The docs URLs depend on the deploy target.
        env: ['DOCS_SITE_ORIGIN'],
        dependsOn: ['lint'], // runs first
        input: [
          { auto: true },
          '!.vitepress/dist/**',
        ],
        output: ['.vitepress/dist/**']
      },
    },
  },
};
";
        let expected = r"export default {
  run: {
    tasks: {
      'build:site': {
        command: 'vitepress build',
        cache: {
        // The docs URLs depend on the deploy target.
        env: ['DOCS_SITE_ORIGIN'],
        input: [
          { auto: true },
          '!.vitepress/dist/**',
        ],
        output: ['.vitepress/dist/**'],
        },
        dependsOn: ['lint'], // runs first
      },
    },
  },
};
";
        assert_eq!(migrate(input).content, expected);
    }

    #[test]
    fn replaces_multiline_cache_true_and_extends_existing_objects() {
        let input = r"export default defineConfig({
	run: {
		tasks: {
			hello: {
				command: 'node hello.mjs',
				untrackedEnv: ['FOO'],
				cache: true,
			},
			lint: {
				command: 'vp lint',
				cache: {
					env: ['A'],
				},
				input: ['src/**'],
			},
		},
	},
});
";
        let expected = r"export default defineConfig({
	run: {
		tasks: {
			hello: {
				command: 'node hello.mjs',
				cache: {
				untrackedEnv: ['FOO'],
				},
			},
			lint: {
				command: 'vp lint',
				cache: {
					env: ['A'],
					input: ['src/**'],
				},
			},
		},
	},
});
";
        assert_eq!(migrate(input).content, expected);
    }

    #[test]
    fn moves_leading_and_trailing_comments() {
        let input = r"export default defineConfig({
  run: {
    tasks: {
      test: {
        // Fingerprint the test mode.
        env: ['MODE'], // same-line note
        command: 'vp test',
        cache: {},
        /* restore coverage */
        output: ['coverage/**'],
      },
      last: {
        command: 'x',
        env: ['A'] // without a comma
      },
    },
  },
});
";
        let expected = r"export default defineConfig({
  run: {
    tasks: {
      test: {
        command: 'vp test',
        cache: {
        // Fingerprint the test mode.
        env: ['MODE'], // same-line note
        /* restore coverage */
        output: ['coverage/**'],
        },
      },
      last: {
        command: 'x',
        cache: {
        env: ['A'], // without a comma
        }
      },
    },
  },
});
";
        assert_eq!(migrate(input).content, expected);
    }

    #[test]
    fn keeps_the_closing_brace_after_a_moved_line_comment() {
        let input = "export default { run: { tasks: { build: {\n  command: 'x',\n  cache: { env: ['A'] },\n  input: ['src/**'], // note\n} } } };";
        let expected = "export default { run: { tasks: { build: {\n  command: 'x',\n  cache: { env: ['A'],\n  input: ['src/**'], // note\n   },\n} } } };";
        assert_eq!(migrate(input).content, expected);
    }

    #[test]
    fn keeps_multiline_template_literals_unchanged() {
        let input = "export default { run: { tasks: { build: {\n  command: 'x',\n  env: [`A\nB`],\n} } } };";
        let actual = migrate(input).content;
        assert!(actual.contains("env: [`A\nB`],"), "{actual}");
    }

    #[test]
    fn supports_callbacks_and_type_assertions() {
        for input in [
            "export default defineConfig(() => ({ run: { tasks: { build: { command: 'x', env: ['A'] } } } }));",
            "export default defineConfig(({ mode }) => { return { run: { tasks: { build: { command: 'x', env: ['A'] } } } }; });",
            "export default { run: { tasks: { build: { command: 'x', env: ['A'] } satisfies Task } } } satisfies UserConfig;",
            "export default defineConfig({ run: { tasks: ({ build: { command: 'x', env: ['A'] } }) } });",
            "export default defineConfig({ run: { tasks: { build: { command: 'x', env: ['A'] } } } } as UserConfig);",
            "export default { run: { tasks: { build: { command: 'x', env: ['A'] } } } } as UserConfig;",
        ] {
            let actual = migrate(input).content;
            assert!(actual.contains("cache: { env: ['A'] }"), "{actual}");
        }
    }

    #[test]
    fn ignores_objects_outside_run_tasks() {
        for input in [
            "export default defineConfig({ build: { rollupOptions: { input: 'src/index.ts', output: { dir: 'dist' } } } });",
            "export default defineConfig({ run: { cache: { tasks: true }, env: ['A'] } });",
            "export default defineConfig({ plugins: [{ config() { return { run: { tasks: { build: { env: ['A'] } } } }; } }] });",
            "export default defineConfig({ test: { run: { tasks: { build: { env: ['A'] } } } } });",
            "const config = { run: { tasks: { build: { command: 'x', env: ['A'] } } } }; export default config;",
            "export default defineConfig({ run: { tasks: { build: 'tsc', check: ['vp lint', 'vp build'] } } });",
            "export default defineConfig({ run: { tasks: { build: { command: 'x', cache: { env: ['A'] } } } } });",
        ] {
            let result = migrate(input);
            assert_eq!(result.content, input);
            assert!(result.manual_tasks.is_empty(), "{input}");
        }
    }

    #[test]
    fn reports_tasks_that_need_manual_migration() {
        let input = r"export default defineConfig({
  run: {
    tasks: {
      spread: { ...shared, command: 'x', env: ['A'] },
      disabled: { command: 'x', cache: false, input: [] },
      dynamic: { command: 'x', cache: isCI, input: [] },
      conflict: { command: 'x', cache: { env: ['B'] }, env: ['A'] },
      escapedConflict: { command: 'x', cache: { '\u0065nv': ['B'] }, env: ['A'] },
      escapedKey: { command: 'x', 'unt\u0072ackedEnv': [], env: ['A'] },
      computed: { [key]: 'x', env: ['A'] },
      duplicate: { command: 'x', env: ['A'], env: ['B'] },
      shorthand: { command: 'x', cache, env: ['A'] },
      spreadCache: { command: 'x', cache: { ...base }, env: ['A'] },
      method: { command: 'x', input() { return []; } },
      commentBeforeComma: { command: 'x', env: ['A'] /* why */, input: [] },
      lineCommentBeforeComma: {
        command: 'x',
        env: ['A'] // note
        , input: [],
      },
      commentInCache: { command: 'x', cache: { env: ['B'] /* why */ }, input: [] },
      ok: { command: 'x', env: ['A'] },
    },
  },
});
";
        let result = migrate(input);
        assert_eq!(
            result.manual_tasks,
            [
                "spread",
                "disabled",
                "dynamic",
                "conflict",
                "escapedConflict",
                "escapedKey",
                "computed",
                "duplicate",
                "shorthand",
                "spreadCache",
                "method",
                "commentBeforeComma",
                "lineCommentBeforeComma",
                "commentInCache",
            ]
        );
        assert_eq!(
            result.content.replace(
                "ok: { command: 'x', cache: { env: ['A'] } }",
                "ok: { command: 'x', env: ['A'] }"
            ),
            input
        );
    }
}
