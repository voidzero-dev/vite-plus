use std::ops::Range;

use ast_grep_core::{Node, tree_sitter::StrDoc};
use ast_grep_language::{LanguageExt, SupportLang};

/// A decoded shell word and its original UTF-8 byte range, including quotes.
#[derive(Debug, PartialEq, Eq)]
pub struct LiteralCommandToken {
    pub value: String,
    pub range: Range<usize>,
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
    // Preserve the existing restriction on escapes, even in single quotes.
    if source.contains('\\') {
        return None;
    }
    let ast = SupportLang::Bash.ast_grep(source);
    let root = ast.root();
    if root.dfs().any(|node| node.is_error() || node.is_missing()) {
        return None;
    }
    let mut nodes = Vec::new();
    collect_commands(root, &mut nodes)?;
    if nodes.is_empty() {
        return (!allow_lists && is_spacing(source)).then(|| vec![vec![]]);
    }
    if !allow_lists && nodes.len() != 1 {
        return None;
    }
    let mut commands = Vec::new();
    let mut end = 0;
    for node in nodes {
        let range = node.range();
        let gap = &source[end..range.start];
        if commands.is_empty() {
            if !is_spacing(gap) {
                return None;
            }
        } else if !matches!(gap.trim_matches([' ', '\t']), "&&" | "||" | ";") {
            return None;
        }
        let mut tokens = Vec::new();
        let mut word_end = range.start;
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
            let value = literal_value(&word, allow_lists)?;
            word_end = range.end;
            tokens.push(LiteralCommandToken { value, range });
        }
        if tokens.is_empty() {
            return None;
        }
        commands.push(tokens);
        end = range.end;
    }
    // Reject comments, newlines and trailing separators, including a single
    // command followed by `;`, even when the parser accepts them.
    is_spacing(&source[end..]).then_some(commands)
}

fn is_spacing(source: &str) -> bool {
    source.chars().all(|c| matches!(c, ' ' | '\t'))
}

fn collect_commands<'a>(node: BashNode<'a>, commands: &mut Vec<BashNode<'a>>) -> Option<()> {
    match node.kind().as_ref() {
        "program" | "list" => {
            for child in node.children() {
                collect_commands(child, commands)?;
            }
        }
        // An assignment without an executable is still returned to the caller,
        // which decides whether it can reason about the resulting shell state.
        "command" | "variable_assignments" | "variable_assignment" => commands.push(node),
        "&&" | "||" | ";" => {}
        _ => return None,
    }
    Some(())
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
    use super::parse_literal_commands;

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
