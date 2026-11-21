use std::ops::Range;

use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};

/// A decoded shell word and its original UTF-8 byte range, including quotes.
#[derive(Debug, PartialEq, Eq)]
pub struct LiteralCommandToken {
    pub value: String,
    pub range: Range<usize>,
}

pub struct LiteralCommandPrefix {
    pub values: Vec<String>,
    pub complete: bool,
    pub substitutions: Vec<String>,
}

pub struct LiteralCommandPrefixes {
    pub commands: Vec<LiteralCommandPrefix>,
    pub success_chain: bool,
}

struct ParsedCommand {
    tokens: Vec<LiteralCommandToken>,
    complete: bool,
    substitutions: Vec<String>,
}

struct ParsedCommands {
    commands: Vec<ParsedCommand>,
    success_chain: bool,
}

type BashNode<'a> = Node<'a, StrDoc<SupportLang>>;

/// Read the restricted command syntax used by migrations, without evaluating
/// shell state. Assignments and package-manager wrappers remain in argv so each
/// caller can apply its own policy.
///
/// Use the same Bash AST as source-range script rewrites. Unlike brush-parser's
/// command formatter, this retains source spelling and byte ranges directly.
/// Only simple commands separated by `&&`, `||`, or `;` are supported when
/// `allow_lists` is true. More Bash syntax must not imply more rewrite authority.
pub fn parse_literal_commands(
    source: &str,
    allow_lists: bool,
) -> Option<Vec<Vec<LiteralCommandToken>>> {
    parse_literal_commands_with_env_values(source, allow_lists, &[])
}

/// Read the same command syntax for option analysis. Replace a double-quoted
/// value with simple environment expansions for a listed option with an empty value.
pub fn parse_literal_commands_with_env_values(
    source: &str,
    allow_lists: bool,
    options: &[String],
) -> Option<Vec<Vec<LiteralCommandToken>>> {
    Some(
        parse_commands(source, allow_lists, false, options)?
            .commands
            .into_iter()
            .map(|command| command.tokens)
            .collect(),
    )
}

/// Read command prefixes in lists, pipelines, redirections, conditionals, and loops.
/// Keep assignment names for unknown values and stop at an unknown argument.
pub fn parse_literal_command_prefixes(source: &str) -> Option<LiteralCommandPrefixes> {
    let parsed = parse_commands(source, true, true, &[])?;
    Some(LiteralCommandPrefixes {
        commands: parsed
            .commands
            .into_iter()
            .map(|command| LiteralCommandPrefix {
                values: command.tokens.into_iter().map(|token| token.value).collect(),
                complete: command.complete,
                substitutions: command.substitutions,
            })
            .collect(),
        success_chain: parsed.success_chain,
    })
}

fn parse_commands(
    source: &str,
    allow_lists: bool,
    detect_dependencies: bool,
    env_value_options: &[String],
) -> Option<ParsedCommands> {
    let has_escapes = source.contains('\\');
    if has_escapes && !detect_dependencies {
        return None;
    }
    let ast = SupportLang::Bash.ast_grep(source);
    let root = ast.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return None;
    }
    let success_chain = detect_dependencies && is_success_chain(&root);
    let mut nodes = Vec::new();
    collect_commands(root, &mut nodes, detect_dependencies)?;
    if nodes.is_empty() {
        return (!allow_lists && is_spacing(source)).then(|| ParsedCommands {
            commands: vec![ParsedCommand { tokens: vec![], complete: true, substitutions: vec![] }],
            success_chain,
        });
    }
    if !allow_lists && nodes.len() != 1 {
        return None;
    }
    let mut commands = Vec::new();
    let mut end = 0;
    for node in nodes {
        let range = node.range();
        if !detect_dependencies {
            let gap = &source[end..range.start];
            let separator = gap.trim_matches([' ', '\t']);
            let valid_gap = if commands.is_empty() {
                is_spacing(gap)
            } else {
                matches!(separator, "&&" | "||" | ";")
            };
            if !valid_gap {
                return None;
            }
        }
        let mut tokens = Vec::new();
        let mut word_end = range.start;
        let mut complete = true;
        let words = if node.kind() == "variable_assignment" {
            vec![node.clone()]
        } else {
            node.children().collect()
        };
        for word in words {
            let range = word.range();
            if !is_spacing(&source[word_end..range.start]) {
                return None;
            }
            word_end = range.end;
            if detect_dependencies
                && matches!(
                    word.kind().as_ref(),
                    "file_redirect" | "heredoc_redirect" | "herestring_redirect"
                )
            {
                continue;
            }
            let literal = if has_escapes && word.text().contains('\\') {
                None
            } else {
                literal_value(&word, allow_lists)
                    .or_else(|| expanded_option_value(&word, tokens.last(), env_value_options))
            };
            let value = match literal {
                Some(value) => value,
                None if detect_dependencies => {
                    complete = false;
                    let Some(assignment) = assignment_prefix(&word) else { break };
                    assignment
                }
                None => return None,
            };
            tokens.push(LiteralCommandToken { value, range });
        }
        if tokens.is_empty() && !detect_dependencies {
            return None;
        }
        let mut substitutions = Vec::new();
        if detect_dependencies {
            collect_command_substitutions(&node, &mut substitutions);
        }
        commands.push(ParsedCommand { tokens, complete, substitutions });
        end = range.end;
    }
    // Reject comments, newlines and trailing separators, including a single
    // command followed by `;`, even when the parser accepts them.
    (detect_dependencies || is_spacing(&source[end..]))
        .then_some(ParsedCommands { commands, success_chain })
}

fn is_spacing(source: &str) -> bool {
    source.chars().all(|c| matches!(c, ' ' | '\t'))
}

// Only simple commands on the same successful && path share changes to the directory.
fn is_success_chain(node: &BashNode<'_>) -> bool {
    match node.kind().as_ref() {
        "program" => {
            let mut children = node.children();
            children.next().is_some_and(|child| is_success_chain(&child))
                && children.next().is_none()
        }
        "list" => node.children().all(|child| child.kind() == "&&" || is_success_chain(&child)),
        "redirected_statement" => node.field("body").is_some_and(|body| is_success_chain(&body)),
        "command" | "variable_assignment" | "variable_assignments" => true,
        _ => false,
    }
}

fn collect_commands<'a>(
    node: BashNode<'a>,
    commands: &mut Vec<BashNode<'a>>,
    detect_dependencies: bool,
) -> Option<()> {
    let kind = node.kind();
    match kind.as_ref() {
        "program" | "list" | "pipeline" | "if_statement" | "elif_clause" | "else_clause"
        | "while_statement" | "do_group" | "subshell" | "compound_statement"
            if detect_dependencies || matches!(kind.as_ref(), "program" | "list") =>
        {
            for child in node.children() {
                collect_commands(child, commands, detect_dependencies)?;
            }
        }
        "redirected_statement" | "for_statement" | "c_style_for_statement"
            if detect_dependencies =>
        {
            if let Some(body) = node.field("body") {
                collect_commands(body, commands, detect_dependencies)?;
            }
        }
        // An assignment without an executable is still returned to the caller,
        // which decides whether it can reason about the resulting shell state.
        "command" | "variable_assignments" | "variable_assignment" => commands.push(node),
        "&&" | "||" | ";" => {}
        _ if detect_dependencies => {}
        _ => return None,
    }
    Some(())
}

// The nested commands run in their own scope. Let dependency detection
// inspect them separately so a nested cd cannot change the outer directory.
fn collect_command_substitutions(node: &BashNode<'_>, substitutions: &mut Vec<String>) {
    if node.kind() == "command_substitution" {
        let text = node.text();
        let source = text
            .strip_prefix("$(")
            .and_then(|value| value.strip_suffix(')'))
            .or_else(|| text.strip_prefix('`').and_then(|value| value.strip_suffix('`')));
        if let Some(source) = source {
            substitutions.push(source.to_owned());
        }
        return;
    }
    for child in node.children() {
        collect_command_substitutions(&child, substitutions);
    }
}

fn assignment_prefix(word: &BashNode<'_>) -> Option<String> {
    if word.kind() != "variable_assignment" {
        return None;
    }
    let name = word.field("name")?;
    (name.kind() == "variable_name").then(|| format!("{}=", name.text()))
}

fn expanded_option_value(
    word: &BashNode<'_>,
    previous: Option<&LiteralCommandToken>,
    options: &[String],
) -> Option<String> {
    if options.is_empty() {
        return None;
    }
    let text = word.text();
    for option in options {
        if previous.is_some_and(|token| token.value == *option) && is_quoted_env_value(word) {
            return Some(String::new());
        }
        if text.strip_prefix(option.as_str()).and_then(|value| value.strip_prefix('=')).is_some_and(
            |value| {
                word.children()
                    .last()
                    .is_some_and(|part| part.text() == value && is_quoted_env_value(&part))
            },
        ) {
            return Some(format!("{option}="));
        }
    }
    None
}

// Double quotes keep a directory value in one argument. Special parameters such
// as $@ and command substitutions do not provide the same guarantee.
fn is_quoted_env_value(word: &BashNode<'_>) -> bool {
    word.kind() == "string"
        && word.children().all(|part| match part.kind().as_ref() {
            "\"" | "string_content" => true,
            "simple_expansion" | "expansion" => is_named_variable(&part.text()),
            _ => false,
        })
}

fn is_named_variable(value: &str) -> bool {
    let Some(name) = value.strip_prefix('$') else {
        return false;
    };
    let name = name.strip_prefix('{').and_then(|name| name.strip_suffix('}')).unwrap_or(name);
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn literal_value(node: &BashNode<'_>, reject_hash: bool) -> Option<String> {
    let text = node.text();
    match node.kind().as_ref() {
        "word" | "number" | "variable_name" | "=" => {
            if text.contains(['$', '`', '\r', '\n']) || (reject_hash && text.contains('#')) {
                return None;
            }
            Some(text.into_owned())
        }
        "raw_string" => Some(text.strip_prefix('\'')?.strip_suffix('\'')?.to_owned()),
        "string" => {
            // Read the original interior: AST string_content nodes can omit
            // literal newlines. Expansions inside double quotes remain unsafe.
            if node
                .children()
                .any(|child| !matches!(child.kind().as_ref(), "\"" | "string_content"))
                || text.contains(['$', '`'])
            {
                return None;
            }
            Some(text.strip_prefix('"')?.strip_suffix('"')?.to_owned())
        }
        "command_name" | "concatenation" | "variable_assignment" => {
            let mut value = String::new();
            for child in node.children() {
                value.push_str(&literal_value(&child, reject_hash)?);
            }
            Some(value)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_literal_command_prefixes, parse_literal_commands};

    fn prefix_values(source: &str) -> Vec<Vec<String>> {
        parse_literal_command_prefixes(source)
            .unwrap()
            .commands
            .into_iter()
            .map(|command| command.values)
            .collect()
    }

    #[test]
    fn reads_only_the_known_prefix_before_expansion() {
        let source =
            r#"echo "$LABEL" && pnpm exec vpx ./generate.ts "$TARGET" --force; "$RUNNER" vpx"#;
        let prefixes = prefix_values(source);
        assert_eq!(prefixes, [vec!["echo"], vec!["pnpm", "exec", "vpx", "./generate.ts"], vec![]]);
        assert_eq!(parse_literal_commands(source, true), None);
    }

    #[test]
    fn reads_executables_in_pipelines_and_redirections() {
        for (source, expected) in [
            (
                r#"vpx ./generate.ts "$TARGET" > vpx.json | tee copy.json"#,
                vec![vec!["vpx", "./generate.ts"], vec!["tee", "copy.json"]],
            ),
            (
                r#"<input pnpm exec vpx ./generate.ts | echo "vpx""#,
                vec![vec!["pnpm", "exec", "vpx", "./generate.ts"], vec!["echo", "vpx"]],
            ),
            (r#"vpx .\generate.ts > build.json"#, vec![vec!["vpx"]]),
        ] {
            assert_eq!(prefix_values(source), expected, "{source}");
            assert_eq!(parse_literal_commands(source, true), None, "{source}");
        }
    }

    #[test]
    fn reads_commands_with_dynamic_assignments_and_conditionals() {
        for (source, expected) in [
            (
                r#"BUILD_ENV=$NODE_ENV vpx ./generate.ts"#,
                vec![vec!["BUILD_ENV=", "vpx", "./generate.ts"]],
            ),
            (
                r#"if [ -n "$CI" ]; then vpx ./generate.ts; elif false; then pnpm exec vp fmt; else vpr prepare; fi"#,
                vec![
                    vec!["vpx", "./generate.ts"],
                    vec!["false"],
                    vec!["pnpm", "exec", "vp", "fmt"],
                    vec!["vpr", "prepare"],
                ],
            ),
            (
                r#"if true; then echo vpx; fi && BUILD_ENV=$NODE_ENV "$RUNNER" vpx"#,
                vec![vec!["true"], vec!["echo", "vpx"], vec!["BUILD_ENV="]],
            ),
        ] {
            assert_eq!(prefix_values(source), expected, "{source}");
            assert_eq!(parse_literal_commands(source, true), None, "{source}");
        }
    }

    #[test]
    fn reads_loop_commands_without_treating_iteration_values_as_commands() {
        for (source, expected) in [
            (
                r#"for target in client worker; do vpx ./generate.ts "$target"; done"#,
                vec![vec!["vpx", "./generate.ts"]],
            ),
            (r#"for target in vp vpx; do echo "$target"; done"#, vec![vec!["echo"]]),
            (r#"while true; do vp fmt; done"#, vec![vec!["true"], vec!["vp", "fmt"]]),
        ] {
            assert_eq!(prefix_values(source), expected, "{source}");
            assert_eq!(parse_literal_commands(source, true), None, "{source}");
        }
    }

    #[test]
    fn reads_command_prefixes_in_shell_groups() {
        for source in
            ["(cd scripts && vpx ./generate.ts) && pwd", "{ cd scripts && vpx ./generate.ts; }"]
        {
            let commands = parse_literal_command_prefixes(source).unwrap();
            assert!(!commands.success_chain, "{source}");
            assert!(
                commands.commands.iter().any(|command| command.values == ["vpx", "./generate.ts"])
            );
            assert!(parse_literal_commands(source, true).is_none());
        }
    }

    #[test]
    fn provides_command_substitutions_separately_from_literal_prefixes() {
        let source = r#"cd scripts && VERSION=$(vpx ./version.ts) node ./build.mjs && echo '$(vpx never.ts)'"#;
        let parsed = parse_literal_command_prefixes(source).unwrap();
        assert!(parsed.success_chain);
        assert_eq!(parsed.commands.len(), 3);
        assert_eq!(parsed.commands[1].values, ["VERSION=", "node", "./build.mjs"]);
        assert_eq!(parsed.commands[1].substitutions, ["vpx ./version.ts"]);
        assert!(parsed.commands[2].substitutions.is_empty());
        assert!(parse_literal_commands(source, true).is_none());

        let nested = r#"echo "$(echo "\$(literal)" "$(vpx ./script.ts)")" `vp --version`"#;
        let parsed = parse_literal_command_prefixes(nested).unwrap();
        assert_eq!(
            parsed.commands[0].substitutions,
            [r#"echo "\$(literal)" "$(vpx ./script.ts)""#, "vp --version"],
        );
        assert!(parse_literal_commands(nested, true).is_none());
    }

    #[test]
    fn identifies_success_chains_for_directory_tracking() {
        for source in [
            "cd scripts && ./generate.ts",
            "cd scripts && ./generate.ts > generated.txt",
            "cd scripts && ./generate.ts \"$TARGET\" > generated.txt && cat generated.txt",
        ] {
            let parsed = parse_literal_command_prefixes(source).unwrap();
            assert!(parsed.success_chain, "{source}");
            assert!(parsed.commands[0].complete, "{source}");
            assert_eq!(parsed.commands[0].values, ["cd", "scripts"], "{source}");
            assert_eq!(parsed.commands[1].values, ["./generate.ts"], "{source}");
        }
        let unknown =
            parse_literal_command_prefixes("cd scripts \"$EXTRA\" && ./generate.ts").unwrap();
        assert!(unknown.success_chain);
        assert!(!unknown.commands[0].complete);
        for source in [
            "cd scripts || ./generate.ts",
            "cd scripts; ./generate.ts",
            "cd scripts\n./generate.ts",
            "cd scripts | ./generate.ts",
            "cd scripts && ./generate.ts || ./fallback.ts",
        ] {
            assert!(!parse_literal_command_prefixes(source).unwrap().success_chain, "{source}");
        }
    }

    #[test]
    fn reads_literals_assignments_and_lists_with_original_ranges() {
        let source = "  echo '你好🦀; done' && MODE=\"unit && extra\" pnpm exec v\"itest\" --project='unit*'; vp test";
        let commands = parse_literal_commands(source, true).unwrap();
        let values: Vec<Vec<&str>> = commands
            .iter()
            .map(|command| command.iter().map(|token| token.value.as_str()).collect())
            .collect();
        assert_eq!(
            values,
            [
                vec!["echo", "你好🦀; done"],
                vec!["MODE=unit && extra", "pnpm", "exec", "vitest", "--project=unit*"],
                vec!["vp", "test"],
            ]
        );
        assert_eq!(&source[commands[0][1].range.clone()], "'你好🦀; done'");
        assert_eq!(&source[commands[1][3].range.clone()], "v\"itest\"");
        assert_eq!(&source[commands[1][4].range.clone()], "--project='unit*'");
        assert_eq!(&source[commands[2][0].range.clone()], "vp");
    }

    #[test]
    fn preserves_literal_quoted_text_and_empty_arguments() {
        let source = "vp\t'config' \"\" '$HOME `cmd` && # hi' \"one\ntwo\"";
        let commands = parse_literal_commands(source, false).unwrap();
        let values: Vec<&str> = commands[0].iter().map(|token| token.value.as_str()).collect();
        assert_eq!(values, ["vp", "config", "", "$HOME `cmd` && # hi", "one\ntwo"]);
    }

    #[test]
    fn rejects_unsupported_shell_syntax() {
        for source in [
            "vp config | tee log",
            "vp config & echo done",
            "vp config > log",
            "vp config < input",
            "vp config <<EOF\ntext\nEOF",
            "vp config $(echo --no-hooks)",
            "vp config `echo --no-hooks`",
            "vp config $FLAGS",
            "vp config \"$FLAGS\"",
            "vp config ${FLAGS}",
            "vp config $'--hooks'",
            "vp config $\"--hooks\"",
            "vp config --hooks\\-dir dir",
            "vp config 'a\\b'",
            "vp config\nvp config",
            "vp config\r",
            "vp config &&\nvp config",
            "vp config # comment",
            "vp config;",
            "vp config &&",
            "&& vp config",
            "vp config 'unterminated",
            "(vp config)",
            "{ vp config; }",
            "if true; then vp config; fi",
            "for x in a; do vp config; done",
            "setup() { vp config; }",
            "! vp config",
            "A=(one two) vp config",
        ] {
            assert_eq!(parse_literal_commands(source, true), None, "{source:?}");
        }
    }

    #[test]
    fn distinguishes_single_commands_and_lists() {
        assert_eq!(parse_literal_commands(" \t", false), Some(vec![vec![]]));
        assert_eq!(parse_literal_commands(" \t", true), None);
        for source in ["vp config && echo done", "vp config || echo failed", "vp config; echo done"]
        {
            assert!(parse_literal_commands(source, true).is_some(), "{source}");
            assert_eq!(parse_literal_commands(source, false), None, "{source}");
        }
    }
}
