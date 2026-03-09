use brush_parser::ast;

// Prettier-only flags that should be stripped when rewriting prettier → vp fmt.
// Value flags consume the next token (e.g., --config .prettierrc) or use = form (--config=.prettierrc).
const PRETTIER_ONLY_VALUE_FLAGS: &[&str] = &[
    "--config",
    "--plugin",
    "--parser",
    "--cache-location",
    "--cache-strategy",
    "--log-level",
    "--stdin-filepath",
    "--cursor-offset",
    "--range-start",
    "--range-end",
    "--config-precedence",
    "--tab-width",
    "--print-width",
    "--trailing-comma",
    "--arrow-parens",
    "--prose-wrap",
    "--end-of-line",
    "--html-whitespace-sensitivity",
    "--quote-props",
    "--embedded-language-formatting",
    "--experimental-ternaries",
];

// Shell keywords after which a newline is cosmetic (not a statement terminator).
const SHELL_CONTINUATION_KEYWORDS: &[&str] = &["then", "do", "else", "elif", "in"];

// Boolean flags are stripped on their own.
const PRETTIER_ONLY_BOOLEAN_FLAGS: &[&str] = &[
    "--write",
    "--cache",
    "--no-config",
    "--no-editorconfig",
    "--with-node-modules",
    "--require-pragma",
    "--insert-pragma",
    "--no-bracket-spacing",
    "--single-quote",
    "--no-semi",
    "--jsx-single-quote",
    "--bracket-same-line",
    "--use-tabs",
    "--debug-check",
    "--debug-print-doc",
    "--debug-benchmark",
    "--debug-repeat",
    "--experimental-cli",
];

// Flags that are converted to different flags.
const PRETTIER_LIST_DIFFERENT_FLAGS: &[&str] = &["--list-different", "-l"];

/// Rewrite a single script: rename `prettier` → `vp fmt`, strip Prettier-only flags,
/// and convert `--list-different`/`-l` → `--check`.
/// Uses brush-parser for proper shell AST parsing instead of manual tokenization.
pub(crate) fn rewrite_prettier_script(script: &str) -> String {
    let mut parser = brush_parser::Parser::new(
        script.as_bytes(),
        &brush_parser::ParserOptions::default(),
        &brush_parser::SourceInfo::default(),
    );
    let Ok(mut program) = parser.parse_program() else {
        return script.to_owned(); // fallback: return unchanged
    };

    if !rewrite_prettier_in_program(&mut program) {
        return script.to_owned(); // no prettier found — return original unchanged
    }
    let output = normalize_pipe_spacing(&program.to_string());
    // brush-parser reformats compound commands (if/while/brace groups) with newlines
    // and indentation, but package.json scripts must be single-line.
    collapse_newlines(&output)
}

fn rewrite_prettier_in_program(program: &mut ast::Program) -> bool {
    let mut changed = false;
    for cmd in &mut program.complete_commands {
        changed |= rewrite_prettier_in_compound_list(cmd);
    }
    changed
}

fn rewrite_prettier_in_compound_list(list: &mut ast::CompoundList) -> bool {
    let mut changed = false;
    for item in &mut list.0 {
        changed |= rewrite_prettier_in_and_or_list(&mut item.0);
    }
    changed
}

fn rewrite_prettier_in_and_or_list(list: &mut ast::AndOrList) -> bool {
    let mut changed = rewrite_prettier_in_pipeline(&mut list.first);
    for and_or in &mut list.additional {
        match and_or {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => {
                changed |= rewrite_prettier_in_pipeline(p);
            }
        }
    }
    changed
}

fn rewrite_prettier_in_pipeline(pipeline: &mut ast::Pipeline) -> bool {
    let mut changed = false;
    for cmd in &mut pipeline.seq {
        match cmd {
            ast::Command::Simple(simple) => {
                changed |= rewrite_prettier_in_simple_command(simple);
            }
            ast::Command::Compound(compound, _redirects) => {
                changed |= rewrite_prettier_in_compound_command(compound);
            }
            _ => {}
        }
    }
    changed
}

fn rewrite_prettier_in_compound_command(cmd: &mut ast::CompoundCommand) -> bool {
    match cmd {
        ast::CompoundCommand::BraceGroup(bg) => rewrite_prettier_in_compound_list(&mut bg.list),
        ast::CompoundCommand::Subshell(sub) => rewrite_prettier_in_compound_list(&mut sub.list),
        ast::CompoundCommand::IfClause(if_cmd) => {
            let mut changed = rewrite_prettier_in_compound_list(&mut if_cmd.condition);
            changed |= rewrite_prettier_in_compound_list(&mut if_cmd.then);
            if let Some(elses) = &mut if_cmd.elses {
                for else_clause in elses {
                    if let Some(cond) = &mut else_clause.condition {
                        changed |= rewrite_prettier_in_compound_list(cond);
                    }
                    changed |= rewrite_prettier_in_compound_list(&mut else_clause.body);
                }
            }
            changed
        }
        ast::CompoundCommand::WhileClause(wc) | ast::CompoundCommand::UntilClause(wc) => {
            let mut changed = rewrite_prettier_in_compound_list(&mut wc.0);
            changed |= rewrite_prettier_in_compound_list(&mut wc.1.list);
            changed
        }
        ast::CompoundCommand::ForClause(fc) => rewrite_prettier_in_compound_list(&mut fc.body.list),
        ast::CompoundCommand::ArithmeticForClause(afc) => {
            rewrite_prettier_in_compound_list(&mut afc.body.list)
        }
        ast::CompoundCommand::CaseClause(cc) => {
            let mut changed = false;
            for case_item in &mut cc.cases {
                if let Some(cmd_list) = &mut case_item.cmd {
                    changed |= rewrite_prettier_in_compound_list(cmd_list);
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

fn rewrite_prettier_in_simple_command(cmd: &mut ast::SimpleCommand) -> bool {
    let cmd_name = cmd.word_or_name.as_ref().map(|w| w.value.as_str());

    if cmd_name == Some("prettier") {
        // Direct prettier invocation: rename prettier → vp fmt
        if let Some(word) = &mut cmd.word_or_name {
            word.value = "vp".to_owned();
        }
        match &mut cmd.suffix {
            Some(suffix) => suffix.0.insert(0, make_suffix_word("fmt")),
            None => cmd.suffix = Some(ast::CommandSuffix(vec![make_suffix_word("fmt")])),
        }
        strip_prettier_flags_from_suffix(cmd, 1); // skip index 0 ("fmt")
        return true;
    }

    if cmd_name == Some("cross-env") || cmd_name == Some("cross-env-shell") {
        // cross-env wrapper: scan suffix for prettier word
        return rewrite_prettier_in_cross_env(cmd);
    }

    false
}

/// Rewrite `cross-env ... prettier [flags] [args]` → `cross-env ... vp fmt [args]`.
/// The prettier word in the suffix marks the boundary between env-var args and the command.
fn rewrite_prettier_in_cross_env(cmd: &mut ast::SimpleCommand) -> bool {
    let suffix = match &mut cmd.suffix {
        Some(s) => s,
        None => return false,
    };

    // Find the index of the "prettier" word in the suffix
    let prettier_idx = suffix.0.iter().position(
        |item| matches!(item, ast::CommandPrefixOrSuffixItem::Word(w) if w.value == "prettier"),
    );
    let Some(idx) = prettier_idx else {
        return false;
    };

    // Rename "prettier" → "vp" and insert "fmt" after it
    if let ast::CommandPrefixOrSuffixItem::Word(w) = &mut suffix.0[idx] {
        w.value = "vp".to_owned();
    }
    suffix.0.insert(idx + 1, make_suffix_word("fmt"));

    // Strip Prettier-only flags starting after "fmt"
    strip_prettier_flags_from_suffix(cmd, idx + 2);
    true
}

/// Strip Prettier-only flags from the suffix, starting at `start_idx`.
/// Items before `start_idx` are kept unconditionally.
/// Also converts `--list-different`/`-l` → `--check`.
fn strip_prettier_flags_from_suffix(cmd: &mut ast::SimpleCommand, start_idx: usize) {
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
    let mut has_check = false;
    for (_, item) in iter {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let ast::CommandPrefixOrSuffixItem::Word(ref w) = item {
            let val = w.value.as_str();

            // Boolean flags: strip just this token
            if PRETTIER_ONLY_BOOLEAN_FLAGS.contains(&val) {
                continue;
            }

            // Value flags: --flag=value form
            if let Some(eq_pos) = val.find('=')
                && PRETTIER_ONLY_VALUE_FLAGS.contains(&&val[..eq_pos])
            {
                continue;
            }

            // Value flags: --flag value form (strip flag + next token)
            if PRETTIER_ONLY_VALUE_FLAGS.contains(&val) {
                skip_next = true;
                continue;
            }

            // Convert --list-different / -l → --check
            if PRETTIER_LIST_DIFFERENT_FLAGS.contains(&val) {
                if !has_check {
                    suffix.0.push(make_suffix_word("--check"));
                    has_check = true;
                }
                continue;
            }

            // Track if --check already exists
            if val == "--check" {
                has_check = true;
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
    fn test_rewrite_prettier_script() {
        // Basic rename: prettier → vp fmt
        assert_eq!(rewrite_prettier_script("prettier ."), "vp fmt .");
        assert_eq!(rewrite_prettier_script("prettier --write ."), "vp fmt .");
        assert_eq!(rewrite_prettier_script("prettier --check ."), "vp fmt --check .");
        assert_eq!(rewrite_prettier_script("prettier --list-different ."), "vp fmt --check .");
        assert_eq!(rewrite_prettier_script("prettier -l ."), "vp fmt --check .");

        // Styling flags stripped
        assert_eq!(
            rewrite_prettier_script("prettier --write --single-quote --tab-width 4 ."),
            "vp fmt ."
        );
        assert_eq!(rewrite_prettier_script("prettier --cache --write ."), "vp fmt .");
        assert_eq!(rewrite_prettier_script("prettier --config .prettierrc --write ."), "vp fmt .");
        assert_eq!(
            rewrite_prettier_script("prettier --plugin prettier-plugin-tailwindcss --write ."),
            "vp fmt ."
        );
        assert_eq!(
            rewrite_prettier_script("prettier --ignore-path .gitignore --write ."),
            "vp fmt --ignore-path .gitignore ."
        );
        assert_eq!(
            rewrite_prettier_script("prettier --ignore-path=.gitignore --write ."),
            "vp fmt --ignore-path=.gitignore ."
        );

        // --experimental-cli stripped
        assert_eq!(rewrite_prettier_script("prettier --experimental-cli --write ."), "vp fmt .");

        // cross-env wrapper
        assert_eq!(
            rewrite_prettier_script("cross-env NODE_ENV=test prettier --write ."),
            "cross-env NODE_ENV=test vp fmt ."
        );

        // compound: only prettier segments rewritten, other commands untouched
        assert_eq!(
            rewrite_prettier_script("prettier --write . && eslint --fix ."),
            "vp fmt . && eslint --fix ."
        );

        // pipe: only prettier segment rewritten
        assert_eq!(
            rewrite_prettier_script("prettier --write . | tee report.txt"),
            "vp fmt . | tee report.txt"
        );

        // env var prefix
        assert_eq!(
            rewrite_prettier_script("NODE_ENV=test prettier --write ."),
            "NODE_ENV=test vp fmt ."
        );

        // if clause
        assert_eq!(
            rewrite_prettier_script("if [ -f .prettierrc ]; then prettier --write .; fi"),
            "if [ -f .prettierrc ]; then vp fmt .; fi"
        );

        // npx wrappers unchanged
        assert_eq!(rewrite_prettier_script("npx prettier --write ."), "npx prettier --write .");

        // already rewritten (no-op)
        assert_eq!(rewrite_prettier_script("vp fmt ."), "vp fmt .");

        // no-error-on-unmatched-pattern is kept
        assert_eq!(
            rewrite_prettier_script("prettier --write --no-error-on-unmatched-pattern ."),
            "vp fmt --no-error-on-unmatched-pattern ."
        );
    }

    #[test]
    fn test_rewrite_prettier_compound_commands() {
        // subshell (brush-parser adds spaces inside parentheses)
        assert_eq!(rewrite_prettier_script("(prettier --write .)"), "( vp fmt . )");

        // brace group
        assert_eq!(rewrite_prettier_script("{ prettier --write .; }"), "{ vp fmt .; }");

        // if clause
        assert_eq!(
            rewrite_prettier_script("if [ -f .prettierrc ]; then prettier --write .; fi"),
            "if [ -f .prettierrc ]; then vp fmt .; fi"
        );

        // while loop
        assert_eq!(
            rewrite_prettier_script("while true; do prettier --write .; done"),
            "while true; do vp fmt .; done"
        );
    }

    #[test]
    fn test_rewrite_prettier_cross_env() {
        // cross-env with prettier
        assert_eq!(
            rewrite_prettier_script("cross-env NODE_ENV=test prettier --write --cache ."),
            "cross-env NODE_ENV=test vp fmt ."
        );

        // cross-env with prettier and --check
        assert_eq!(
            rewrite_prettier_script("cross-env NODE_ENV=test prettier --check ."),
            "cross-env NODE_ENV=test vp fmt --check ."
        );

        // cross-env without prettier — passes through unchanged
        assert_eq!(
            rewrite_prettier_script("cross-env NODE_ENV=test jest"),
            "cross-env NODE_ENV=test jest"
        );

        // multiple env vars before prettier
        assert_eq!(
            rewrite_prettier_script("cross-env NODE_ENV=test CI=true prettier --write --cache ."),
            "cross-env NODE_ENV=test CI=true vp fmt ."
        );
    }

    #[test]
    fn test_rewrite_prettier_list_different_to_check() {
        // --list-different → --check
        assert_eq!(rewrite_prettier_script("prettier --list-different ."), "vp fmt --check .");
        // -l → --check
        assert_eq!(rewrite_prettier_script("prettier -l ."), "vp fmt --check .");

        // --list-different with other flags
        assert_eq!(
            rewrite_prettier_script("prettier --list-different --single-quote ."),
            "vp fmt --check ."
        );

        // --check + --list-different → single --check (no duplicate)
        assert_eq!(
            rewrite_prettier_script("prettier --check --list-different ."),
            "vp fmt --check ."
        );
    }

    #[test]
    fn test_rewrite_prettier_value_flags() {
        // --flag=value form
        assert_eq!(rewrite_prettier_script("prettier --tab-width=4 --write ."), "vp fmt .");
        assert_eq!(rewrite_prettier_script("prettier --print-width=120 --write ."), "vp fmt .");

        // Multiple value flags
        assert_eq!(
            rewrite_prettier_script(
                "prettier --config .prettierrc --plugin prettier-plugin-tailwindcss --write ."
            ),
            "vp fmt ."
        );

        // --parser flag
        assert_eq!(rewrite_prettier_script("prettier --parser typescript --write ."), "vp fmt .");
    }
}
