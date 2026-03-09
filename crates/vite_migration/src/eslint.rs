use brush_parser::ast;

// ESLint-only flags that should be stripped when rewriting eslint → vp lint.
// Value flags consume the next token (e.g., --ext .ts,.tsx) or use = form (--ext=.ts,.tsx).
const ESLINT_ONLY_VALUE_FLAGS: &[&str] = &[
    "--ext",
    "--rulesdir",
    "--resolve-plugins-relative-to",
    "--parser",
    "--parser-options",
    "--plugin",
    "--output-file",
    "--env",
];

// Shell keywords after which a newline is cosmetic (not a statement terminator).
const SHELL_CONTINUATION_KEYWORDS: &[&str] = &["then", "do", "else", "elif", "in"];

// Boolean flags are stripped on their own.
const ESLINT_ONLY_BOOLEAN_FLAGS: &[&str] = &[
    "--cache",
    "--no-eslintrc",
    "--no-error-on-unmatched-pattern",
    "--debug",
    "--no-inline-config",
];

/// Rewrite a single script: rename `eslint` → `vp lint` and strip ESLint-only flags.
/// Uses brush-parser for proper shell AST parsing instead of manual tokenization.
pub(crate) fn rewrite_eslint_script(script: &str) -> String {
    let mut parser = brush_parser::Parser::new(
        script.as_bytes(),
        &brush_parser::ParserOptions::default(),
        &brush_parser::SourceInfo::default(),
    );
    let Ok(mut program) = parser.parse_program() else {
        return script.to_owned(); // fallback: return unchanged
    };

    if !rewrite_eslint_in_program(&mut program) {
        return script.to_owned(); // no eslint found — return original unchanged
    }
    let output = normalize_pipe_spacing(&program.to_string());
    // brush-parser reformats compound commands (if/while/brace groups) with newlines
    // and indentation, but package.json scripts must be single-line.
    collapse_newlines(&output)
}

fn rewrite_eslint_in_program(program: &mut ast::Program) -> bool {
    let mut changed = false;
    for cmd in &mut program.complete_commands {
        changed |= rewrite_eslint_in_compound_list(cmd);
    }
    changed
}

fn rewrite_eslint_in_compound_list(list: &mut ast::CompoundList) -> bool {
    let mut changed = false;
    for item in &mut list.0 {
        changed |= rewrite_eslint_in_and_or_list(&mut item.0);
    }
    changed
}

fn rewrite_eslint_in_and_or_list(list: &mut ast::AndOrList) -> bool {
    let mut changed = rewrite_eslint_in_pipeline(&mut list.first);
    for and_or in &mut list.additional {
        match and_or {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => {
                changed |= rewrite_eslint_in_pipeline(p);
            }
        }
    }
    changed
}

fn rewrite_eslint_in_pipeline(pipeline: &mut ast::Pipeline) -> bool {
    let mut changed = false;
    for cmd in &mut pipeline.seq {
        match cmd {
            ast::Command::Simple(simple) => {
                changed |= rewrite_eslint_in_simple_command(simple);
            }
            ast::Command::Compound(compound, _redirects) => {
                changed |= rewrite_eslint_in_compound_command(compound);
            }
            _ => {}
        }
    }
    changed
}

fn rewrite_eslint_in_compound_command(cmd: &mut ast::CompoundCommand) -> bool {
    match cmd {
        ast::CompoundCommand::BraceGroup(bg) => rewrite_eslint_in_compound_list(&mut bg.list),
        ast::CompoundCommand::Subshell(sub) => rewrite_eslint_in_compound_list(&mut sub.list),
        ast::CompoundCommand::IfClause(if_cmd) => {
            let mut changed = rewrite_eslint_in_compound_list(&mut if_cmd.condition);
            changed |= rewrite_eslint_in_compound_list(&mut if_cmd.then);
            if let Some(elses) = &mut if_cmd.elses {
                for else_clause in elses {
                    if let Some(cond) = &mut else_clause.condition {
                        changed |= rewrite_eslint_in_compound_list(cond);
                    }
                    changed |= rewrite_eslint_in_compound_list(&mut else_clause.body);
                }
            }
            changed
        }
        ast::CompoundCommand::WhileClause(wc) | ast::CompoundCommand::UntilClause(wc) => {
            let mut changed = rewrite_eslint_in_compound_list(&mut wc.0);
            changed |= rewrite_eslint_in_compound_list(&mut wc.1.list);
            changed
        }
        ast::CompoundCommand::ForClause(fc) => rewrite_eslint_in_compound_list(&mut fc.body.list),
        ast::CompoundCommand::ArithmeticForClause(afc) => {
            rewrite_eslint_in_compound_list(&mut afc.body.list)
        }
        ast::CompoundCommand::CaseClause(cc) => {
            let mut changed = false;
            for case_item in &mut cc.cases {
                if let Some(cmd_list) = &mut case_item.cmd {
                    changed |= rewrite_eslint_in_compound_list(cmd_list);
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

fn rewrite_eslint_in_simple_command(cmd: &mut ast::SimpleCommand) -> bool {
    let cmd_name = cmd.word_or_name.as_ref().map(|w| w.value.as_str());

    if cmd_name == Some("eslint") {
        // Direct eslint invocation: rename eslint → vp lint
        if let Some(word) = &mut cmd.word_or_name {
            word.value = "vp".to_owned();
        }
        match &mut cmd.suffix {
            Some(suffix) => suffix.0.insert(0, make_suffix_word("lint")),
            None => cmd.suffix = Some(ast::CommandSuffix(vec![make_suffix_word("lint")])),
        }
        strip_eslint_flags_from_suffix(cmd, 1); // skip index 0 ("lint")
        return true;
    }

    if cmd_name == Some("cross-env") || cmd_name == Some("cross-env-shell") {
        // cross-env wrapper: scan suffix for eslint word
        return rewrite_eslint_in_cross_env(cmd);
    }

    false
}

/// Rewrite `cross-env ... eslint [flags] [args]` → `cross-env ... vp lint [args]`.
/// The eslint word in the suffix marks the boundary between env-var args and the command.
fn rewrite_eslint_in_cross_env(cmd: &mut ast::SimpleCommand) -> bool {
    let suffix = match &mut cmd.suffix {
        Some(s) => s,
        None => return false,
    };

    // Find the index of the "eslint" word in the suffix
    let eslint_idx = suffix.0.iter().position(
        |item| matches!(item, ast::CommandPrefixOrSuffixItem::Word(w) if w.value == "eslint"),
    );
    let Some(idx) = eslint_idx else {
        return false;
    };

    // Rename "eslint" → "vp" and insert "lint" after it
    if let ast::CommandPrefixOrSuffixItem::Word(w) = &mut suffix.0[idx] {
        w.value = "vp".to_owned();
    }
    suffix.0.insert(idx + 1, make_suffix_word("lint"));

    // Strip ESLint-only flags starting after "lint"
    strip_eslint_flags_from_suffix(cmd, idx + 2);
    true
}

/// Strip ESLint-only flags from the suffix, starting at `start_idx`.
/// Items before `start_idx` are kept unconditionally.
fn strip_eslint_flags_from_suffix(cmd: &mut ast::SimpleCommand, start_idx: usize) {
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
    for (_, item) in iter {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let ast::CommandPrefixOrSuffixItem::Word(ref w) = item {
            let val = w.value.as_str();

            // Boolean flags: strip just this token
            if ESLINT_ONLY_BOOLEAN_FLAGS.contains(&val) {
                continue;
            }

            // Value flags: --flag=value form
            if let Some(eq_pos) = val.find('=')
                && ESLINT_ONLY_VALUE_FLAGS.contains(&&val[..eq_pos])
            {
                continue;
            }

            // Value flags: --flag value form (strip flag + next token)
            if ESLINT_ONLY_VALUE_FLAGS.contains(&val) {
                skip_next = true;
                continue;
            }
        }
        suffix.0.push(item);
    }

    // If suffix is empty, clear it
    if suffix.0.is_empty() {
        cmd.suffix = None;
    }
}

/// Collapse newlines and surrounding whitespace into single-line form.
/// brush-parser reformats compound commands with newlines + indentation,
/// but package.json scripts must remain single-line.
///
/// In shell syntax, newlines serve as statement terminators (like `;`).
/// After keywords like `then`, `do`, `else`, `{`, the newline is cosmetic
/// and can be replaced with a space. But before `fi`, `done`, `}`, `esac`,
/// the newline terminates the preceding command and must become `; `.
fn collapse_newlines(s: &str) -> String {
    if !s.contains('\n') {
        return s.to_owned();
    }
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            // Strip trailing whitespace before the newline
            while result.ends_with(' ') || result.ends_with('\t') {
                result.pop();
            }
            // Skip leading whitespace on the next line
            while chars.peek().is_some_and(|&ch| ch == ' ' || ch == '\t') {
                chars.next();
            }
            // Decide: space or semicolon?
            // If the line ended with a keyword/separator, newline is cosmetic → space
            // Otherwise the newline terminates a command → semicolon + space
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

/// Check if the content before a newline needs a semicolon to terminate the command.
/// Returns false when the content ends with a shell keyword or separator where
/// the newline is just cosmetic whitespace.
fn needs_semicolon(before: &str) -> bool {
    let trimmed = before.trim_end();
    if trimmed.is_empty() {
        return false;
    }
    // Check single-char separators
    let last_byte = trimmed.as_bytes()[trimmed.len() - 1];
    if matches!(last_byte, b'{' | b'(' | b';' | b'|' | b'&' | b'!') {
        return false;
    }
    // Check keyword endings
    for kw in SHELL_CONTINUATION_KEYWORDS {
        if trimmed.ends_with(kw) {
            // Make sure it's a whole word (preceded by whitespace or start of string)
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
        // Insert space after | when it's a pipe operator (not ||) and not already spaced
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
    // Safety: only ASCII space bytes inserted into valid UTF-8
    String::from_utf8(result).unwrap_or_else(|_| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_eslint_script() {
        // Basic rename: eslint → vp lint
        assert_eq!(rewrite_eslint_script("eslint ."), "vp lint .");
        assert_eq!(rewrite_eslint_script("eslint --fix ."), "vp lint --fix .");
        assert_eq!(rewrite_eslint_script("eslint"), "vp lint");

        // Flag stripping + rename combined
        assert_eq!(rewrite_eslint_script("eslint --fix --ext .ts,.tsx ."), "vp lint --fix .");
        assert_eq!(rewrite_eslint_script("eslint --ext .ts ."), "vp lint .");
        assert_eq!(rewrite_eslint_script("eslint --rulesdir ./rules --fix ."), "vp lint --fix .");
        assert_eq!(
            rewrite_eslint_script("eslint --parser @typescript-eslint/parser --fix ."),
            "vp lint --fix ."
        );
        assert_eq!(rewrite_eslint_script("eslint --output-file report.txt ."), "vp lint .");
        assert_eq!(rewrite_eslint_script("eslint --env browser --fix ."), "vp lint --fix .");

        // value flags: --flag=value form
        assert_eq!(rewrite_eslint_script("eslint --ext=.ts,.tsx ."), "vp lint .");

        // boolean flags
        assert_eq!(rewrite_eslint_script("eslint --cache --fix ."), "vp lint --fix .");
        assert_eq!(rewrite_eslint_script("eslint --no-eslintrc --fix ."), "vp lint --fix .");
        assert_eq!(rewrite_eslint_script("eslint --debug ."), "vp lint .");
        assert_eq!(
            rewrite_eslint_script("eslint --no-error-on-unmatched-pattern --fix ."),
            "vp lint --fix ."
        );
        assert_eq!(rewrite_eslint_script("eslint --no-inline-config ."), "vp lint .");

        // multiple flags stripped at once
        assert_eq!(
            rewrite_eslint_script("eslint --fix --ext .ts,.tsx --cache ."),
            "vp lint --fix ."
        );

        // edge case: value flag at end with no value
        assert_eq!(rewrite_eslint_script("eslint --ext"), "vp lint");

        // compound: only eslint segments rewritten, other commands untouched
        assert_eq!(
            rewrite_eslint_script("eslint --ext .ts . && vite build --debug"),
            "vp lint . && vite build --debug"
        );
        assert_eq!(
            rewrite_eslint_script("eslint --cache --fix . && other-tool --env production"),
            "vp lint --fix . && other-tool --env production"
        );
        assert_eq!(
            rewrite_eslint_script("some-tool --cache && eslint --ext .ts ."),
            "some-tool --cache && vp lint ."
        );
        assert_eq!(
            rewrite_eslint_script("eslint --ext .ts . && eslint --cache --fix src/"),
            "vp lint . && vp lint --fix src/"
        );
        assert_eq!(rewrite_eslint_script("eslint . && vite build"), "vp lint . && vite build");

        // non-eslint commands pass through unchanged (no-op returns original exactly)
        assert_eq!(rewrite_eslint_script("vp build"), "vp build");
        assert_eq!(rewrite_eslint_script("vp lint --cache --fix ."), "vp lint --cache --fix .");
        assert_eq!(rewrite_eslint_script("echo 'a |b'"), "echo 'a |b'");

        // pipe: only eslint segment rewritten, piped command untouched
        assert_eq!(
            rewrite_eslint_script("eslint --cache . | tee report.txt"),
            "vp lint . | tee report.txt"
        );

        // eslint with env var prefix
        assert_eq!(
            rewrite_eslint_script("NODE_ENV=test eslint --cache --ext .ts ."),
            "NODE_ENV=test vp lint ."
        );
    }

    #[test]
    fn test_rewrite_eslint_compound_commands() {
        // subshell (brush-parser adds spaces inside parentheses)
        assert_eq!(rewrite_eslint_script("(eslint --cache .)"), "( vp lint . )");

        // brace group: must have ; before }
        assert_eq!(rewrite_eslint_script("{ eslint --cache .; }"), "{ vp lint .; }");

        // if clause: must have ; before fi
        assert_eq!(
            rewrite_eslint_script("if [ -f .eslintrc ]; then eslint --cache .; fi"),
            "if [ -f .eslintrc ]; then vp lint .; fi"
        );

        // while loop
        assert_eq!(
            rewrite_eslint_script("while true; do eslint .; done"),
            "while true; do vp lint .; done"
        );
    }

    #[test]
    fn test_rewrite_eslint_cross_env() {
        // cross-env with eslint
        assert_eq!(
            rewrite_eslint_script("cross-env NODE_ENV=test eslint --cache --ext .ts ."),
            "cross-env NODE_ENV=test vp lint ."
        );

        // cross-env with eslint and --fix
        assert_eq!(
            rewrite_eslint_script("cross-env NODE_ENV=test eslint --cache --fix ."),
            "cross-env NODE_ENV=test vp lint --fix ."
        );

        // cross-env without eslint — passes through unchanged
        assert_eq!(
            rewrite_eslint_script("cross-env NODE_ENV=test jest"),
            "cross-env NODE_ENV=test jest"
        );

        // multiple env vars before eslint
        assert_eq!(
            rewrite_eslint_script("cross-env NODE_ENV=test CI=true eslint --cache ."),
            "cross-env NODE_ENV=test CI=true vp lint ."
        );
    }

    #[test]
    fn test_normalize_pipe_spacing() {
        // Single pipe gets space added
        assert_eq!(normalize_pipe_spacing("cmd1 |cmd2"), "cmd1 | cmd2");
        // Already spaced pipe is unchanged
        assert_eq!(normalize_pipe_spacing("cmd1 | cmd2"), "cmd1 | cmd2");
        // Double pipe (||) is unchanged
        assert_eq!(normalize_pipe_spacing("cmd1 || cmd2"), "cmd1 || cmd2");
        // No pipe
        assert_eq!(normalize_pipe_spacing("cmd1 && cmd2"), "cmd1 && cmd2");
    }
}
