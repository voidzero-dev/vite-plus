use brush_parser::ast;

/// Configuration for converting one flag (or set of aliases) into a different flag.
/// Example: Prettier's `--list-different`/`-l` → `--check`.
pub struct FlagConversion {
    /// Source flags that should be converted (e.g. `["--list-different", "-l"]`).
    pub(crate) source_flags: &'static [&'static str],
    /// The target flag to emit instead (e.g. `"--check"`).
    pub(crate) target_flag: &'static str,
    /// An existing flag that means the same thing — used for dedup
    /// (e.g. `"--check"` so we don't emit two `--check` flags).
    pub(crate) dedup_flag: &'static str,
}

/// Tool-specific configuration for script rewriting.
pub struct ScriptRewriteConfig {
    /// The source command name to match (e.g. `"prettier"`, `"eslint"`).
    pub(crate) source_command: &'static str,
    /// The `vp` subcommand to emit (e.g. `"fmt"`, `"lint"`).
    pub(crate) target_subcommand: &'static str,
    /// Boolean flags to strip (consumed alone, e.g. `"--cache"`).
    pub(crate) boolean_flags: &'static [&'static str],
    /// Value flags to strip (consume the next token, e.g. `"--config"`).
    pub(crate) value_flags: &'static [&'static str],
    /// Flags to convert to a different flag.
    pub(crate) flag_conversions: &'static [FlagConversion],
}

// Shell keywords after which a newline is cosmetic (not a statement terminator).
const SHELL_CONTINUATION_KEYWORDS: &[&str] = &["then", "do", "else", "elif", "in"];

/// Rewrite a shell script: find `source_command`, rename to `vp <subcommand>`,
/// strip tool-specific flags, and normalize the output.
pub fn rewrite_script(script: &str, config: &ScriptRewriteConfig) -> String {
    let mut parser = brush_parser::Parser::new(
        script.as_bytes(),
        &brush_parser::ParserOptions::default(),
        &brush_parser::SourceInfo::default(),
    );
    let Ok(mut program) = parser.parse_program() else {
        return script.to_owned();
    };

    if !rewrite_in_program(&mut program, config) {
        return script.to_owned();
    }
    let output = normalize_pipe_spacing(&program.to_string());
    collapse_newlines(&output)
}

fn rewrite_in_program(program: &mut ast::Program, config: &ScriptRewriteConfig) -> bool {
    let mut changed = false;
    for cmd in &mut program.complete_commands {
        changed |= rewrite_in_compound_list(cmd, config);
    }
    changed
}

fn rewrite_in_compound_list(list: &mut ast::CompoundList, config: &ScriptRewriteConfig) -> bool {
    let mut changed = false;
    for item in &mut list.0 {
        changed |= rewrite_in_and_or_list(&mut item.0, config);
    }
    changed
}

fn rewrite_in_and_or_list(list: &mut ast::AndOrList, config: &ScriptRewriteConfig) -> bool {
    let mut changed = rewrite_in_pipeline(&mut list.first, config);
    for and_or in &mut list.additional {
        match and_or {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => {
                changed |= rewrite_in_pipeline(p, config);
            }
        }
    }
    changed
}

fn rewrite_in_pipeline(pipeline: &mut ast::Pipeline, config: &ScriptRewriteConfig) -> bool {
    let mut changed = false;
    for cmd in &mut pipeline.seq {
        match cmd {
            ast::Command::Simple(simple) => {
                changed |= rewrite_in_simple_command(simple, config);
            }
            ast::Command::Compound(compound, _redirects) => {
                changed |= rewrite_in_compound_command(compound, config);
            }
            _ => {}
        }
    }
    changed
}

fn rewrite_in_compound_command(
    cmd: &mut ast::CompoundCommand,
    config: &ScriptRewriteConfig,
) -> bool {
    match cmd {
        ast::CompoundCommand::BraceGroup(bg) => rewrite_in_compound_list(&mut bg.list, config),
        ast::CompoundCommand::Subshell(sub) => rewrite_in_compound_list(&mut sub.list, config),
        ast::CompoundCommand::IfClause(if_cmd) => {
            let mut changed = rewrite_in_compound_list(&mut if_cmd.condition, config);
            changed |= rewrite_in_compound_list(&mut if_cmd.then, config);
            if let Some(elses) = &mut if_cmd.elses {
                for else_clause in elses {
                    if let Some(cond) = &mut else_clause.condition {
                        changed |= rewrite_in_compound_list(cond, config);
                    }
                    changed |= rewrite_in_compound_list(&mut else_clause.body, config);
                }
            }
            changed
        }
        ast::CompoundCommand::WhileClause(wc) | ast::CompoundCommand::UntilClause(wc) => {
            let mut changed = rewrite_in_compound_list(&mut wc.0, config);
            changed |= rewrite_in_compound_list(&mut wc.1.list, config);
            changed
        }
        ast::CompoundCommand::ForClause(fc) => rewrite_in_compound_list(&mut fc.body.list, config),
        ast::CompoundCommand::ArithmeticForClause(afc) => {
            rewrite_in_compound_list(&mut afc.body.list, config)
        }
        ast::CompoundCommand::CaseClause(cc) => {
            let mut changed = false;
            for case_item in &mut cc.cases {
                if let Some(cmd_list) = &mut case_item.cmd {
                    changed |= rewrite_in_compound_list(cmd_list, config);
                }
            }
            changed
        }
        ast::CompoundCommand::Arithmetic(_) => false,
    }
}

fn make_suffix_word(value: &str) -> ast::CommandPrefixOrSuffixItem {
    ast::CommandPrefixOrSuffixItem::Word(ast::Word { value: value.to_owned(), loc: None })
}

fn rewrite_in_simple_command(cmd: &mut ast::SimpleCommand, config: &ScriptRewriteConfig) -> bool {
    let cmd_name = cmd.word_or_name.as_ref().map(|w| w.value.as_str());

    if cmd_name == Some(config.source_command) {
        if let Some(word) = &mut cmd.word_or_name {
            word.value = "vp".to_owned();
        }
        match &mut cmd.suffix {
            Some(suffix) => suffix.0.insert(0, make_suffix_word(config.target_subcommand)),
            None => {
                cmd.suffix =
                    Some(ast::CommandSuffix(vec![make_suffix_word(config.target_subcommand)]));
            }
        }
        strip_flags_from_suffix(cmd, 1, config);
        return true;
    }

    if cmd_name == Some("cross-env") || cmd_name == Some("cross-env-shell") {
        return rewrite_in_cross_env(cmd, config);
    }

    false
}

fn rewrite_in_cross_env(cmd: &mut ast::SimpleCommand, config: &ScriptRewriteConfig) -> bool {
    let suffix = match &mut cmd.suffix {
        Some(s) => s,
        None => return false,
    };

    let source_idx = suffix.0.iter().position(|item| {
        matches!(item, ast::CommandPrefixOrSuffixItem::Word(w) if w.value == config.source_command)
    });
    let Some(idx) = source_idx else {
        return false;
    };

    if let ast::CommandPrefixOrSuffixItem::Word(w) = &mut suffix.0[idx] {
        w.value = "vp".to_owned();
    }
    suffix.0.insert(idx + 1, make_suffix_word(config.target_subcommand));

    strip_flags_from_suffix(cmd, idx + 2, config);
    true
}

/// Strip tool-specific flags from the suffix, starting at `start_idx`.
/// Items before `start_idx` are kept unconditionally.
/// Also applies flag conversions defined in the config.
fn strip_flags_from_suffix(
    cmd: &mut ast::SimpleCommand,
    start_idx: usize,
    config: &ScriptRewriteConfig,
) {
    let suffix = cmd.suffix.as_mut().expect("suffix was just set");
    let items = std::mem::take(&mut suffix.0);
    let mut iter = items.into_iter().enumerate();

    // Keep items before start_idx unconditionally
    for (i, item) in iter.by_ref() {
        suffix.0.push(item);
        if i + 1 >= start_idx {
            break;
        }
    }

    let mut skip_next = false;
    // One dedup tracker per flag conversion rule (no allocation when empty)
    let mut conversion_emitted = vec![false; config.flag_conversions.len()];

    for (_, item) in iter {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let ast::CommandPrefixOrSuffixItem::Word(ref w) = item {
            let val = w.value.as_str();

            // Boolean flags: strip just this token
            if config.boolean_flags.contains(&val) {
                continue;
            }

            // Value flags: --flag=value form
            if let Some(eq_pos) = val.find('=')
                && config.value_flags.contains(&&val[..eq_pos])
            {
                continue;
            }

            // Value flags: --flag value form (strip flag + next token)
            if config.value_flags.contains(&val) {
                skip_next = true;
                continue;
            }

            // Flag conversions + dedup tracking in a single pass
            let mut converted = false;
            for (ci, conv) in config.flag_conversions.iter().enumerate() {
                if conv.source_flags.contains(&val) {
                    if !conversion_emitted[ci] {
                        suffix.0.push(make_suffix_word(conv.target_flag));
                        conversion_emitted[ci] = true;
                    }
                    converted = true;
                    break;
                }
                if val == conv.dedup_flag {
                    conversion_emitted[ci] = true;
                }
            }
            if converted {
                continue;
            }
        }
        suffix.0.push(item);
    }

    if suffix.0.is_empty() {
        cmd.suffix = None;
    }
}

/// Collapse newlines and surrounding whitespace into single-line form.
/// brush-parser reformats compound commands with newlines + indentation,
/// but package.json scripts must remain single-line.
fn collapse_newlines(s: &str) -> String {
    if !s.contains('\n') {
        return s.to_owned();
    }
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            while result.ends_with(' ') || result.ends_with('\t') {
                result.pop();
            }
            while chars.peek().is_some_and(|&ch| ch == ' ' || ch == '\t') {
                chars.next();
            }
            if needs_semicolon(&result) {
                result.push_str("; ");
            } else {
                result.push(' ');
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn needs_semicolon(before: &str) -> bool {
    let trimmed = before.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    let last_byte = trimmed.as_bytes()[trimmed.len() - 1];
    if matches!(last_byte, b'{' | b'(' | b';' | b'|' | b'&' | b'!') {
        return false;
    }
    for kw in SHELL_CONTINUATION_KEYWORDS {
        if trimmed.ends_with(kw) {
            let prefix_len = trimmed.len() - kw.len();
            if prefix_len == 0 || !trimmed.as_bytes()[prefix_len - 1].is_ascii_alphanumeric() {
                return false;
            }
        }
    }
    true
}

/// Fix pipe spacing in brush-parser Display output.
/// brush-parser renders pipes as `cmd1 |cmd2` instead of `cmd1 | cmd2`.
fn normalize_pipe_spacing(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len() + 16);
    for i in 0..bytes.len() {
        result.push(bytes[i]);
        if bytes[i] == b'|'
            && i > 0
            && bytes[i - 1] == b' '
            && i + 1 < bytes.len()
            && bytes[i + 1] != b'|'
            && bytes[i + 1] != b' '
        {
            result.push(b' ');
        }
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_pipe_spacing() {
        assert_eq!(normalize_pipe_spacing("cmd1 |cmd2"), "cmd1 | cmd2");
        assert_eq!(normalize_pipe_spacing("cmd1 | cmd2"), "cmd1 | cmd2");
        assert_eq!(normalize_pipe_spacing("cmd1 || cmd2"), "cmd1 || cmd2");
        assert_eq!(normalize_pipe_spacing("cmd1 && cmd2"), "cmd1 && cmd2");
    }
}
