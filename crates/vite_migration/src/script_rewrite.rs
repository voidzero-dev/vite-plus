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
    let rewritten_bunx =
        rewrite_bunx_commands(script, |inner| rewrite_direct_script(inner, config));
    rewrite_direct_script(&rewritten_bunx, config)
}

fn rewrite_direct_script(script: &str, config: &ScriptRewriteConfig) -> String {
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
    visit_simple_commands(program, &mut |cmd| rewrite_in_simple_command(cmd, config))
}

fn visit_simple_commands(
    program: &mut ast::Program,
    visitor: &mut impl FnMut(&mut ast::SimpleCommand) -> bool,
) -> bool {
    let mut changed = false;
    for command in &mut program.complete_commands {
        changed |= visit_compound_list(command, visitor);
    }
    changed
}

fn visit_compound_list(
    list: &mut ast::CompoundList,
    visitor: &mut impl FnMut(&mut ast::SimpleCommand) -> bool,
) -> bool {
    let mut changed = false;
    for item in &mut list.0 {
        changed |= visit_and_or_list(&mut item.0, visitor);
    }
    changed
}

fn visit_and_or_list(
    list: &mut ast::AndOrList,
    visitor: &mut impl FnMut(&mut ast::SimpleCommand) -> bool,
) -> bool {
    let mut changed = visit_pipeline(&mut list.first, visitor);
    for and_or in &mut list.additional {
        match and_or {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => {
                changed |= visit_pipeline(p, visitor);
            }
        }
    }
    changed
}

fn visit_pipeline(
    pipeline: &mut ast::Pipeline,
    visitor: &mut impl FnMut(&mut ast::SimpleCommand) -> bool,
) -> bool {
    let mut changed = false;
    for cmd in &mut pipeline.seq {
        match cmd {
            ast::Command::Simple(simple) => {
                changed |= visitor(simple);
            }
            ast::Command::Compound(compound, _redirects) => {
                changed |= visit_compound_command(compound, visitor);
            }
            _ => {}
        }
    }
    changed
}

fn visit_compound_command(
    cmd: &mut ast::CompoundCommand,
    visitor: &mut impl FnMut(&mut ast::SimpleCommand) -> bool,
) -> bool {
    match cmd {
        ast::CompoundCommand::BraceGroup(bg) => visit_compound_list(&mut bg.list, visitor),
        ast::CompoundCommand::Subshell(sub) => visit_compound_list(&mut sub.list, visitor),
        ast::CompoundCommand::IfClause(if_cmd) => {
            let mut changed = visit_compound_list(&mut if_cmd.condition, visitor);
            changed |= visit_compound_list(&mut if_cmd.then, visitor);
            if let Some(elses) = &mut if_cmd.elses {
                for else_clause in elses {
                    if let Some(cond) = &mut else_clause.condition {
                        changed |= visit_compound_list(cond, visitor);
                    }
                    changed |= visit_compound_list(&mut else_clause.body, visitor);
                }
            }
            changed
        }
        ast::CompoundCommand::WhileClause(wc) | ast::CompoundCommand::UntilClause(wc) => {
            let mut changed = visit_compound_list(&mut wc.0, visitor);
            changed |= visit_compound_list(&mut wc.1.list, visitor);
            changed
        }
        ast::CompoundCommand::ForClause(fc) => visit_compound_list(&mut fc.body.list, visitor),
        ast::CompoundCommand::ArithmeticForClause(afc) => {
            visit_compound_list(&mut afc.body.list, visitor)
        }
        ast::CompoundCommand::CaseClause(cc) => {
            let mut changed = false;
            for case_item in &mut cc.cases {
                if let Some(cmd_list) = &mut case_item.cmd {
                    changed |= visit_compound_list(cmd_list, visitor);
                }
            }
            changed
        }
        ast::CompoundCommand::Arithmetic(_) => false,
    }
}

#[derive(Clone, Copy)]
enum CommandWordPosition {
    Name,
    Suffix(usize),
}

struct CommandWord {
    position: CommandWordPosition,
    ordinal: usize,
    value: String,
}

struct BunxInvocation {
    target_suffix_index: usize,
}

fn collect_command_words(cmd: &ast::SimpleCommand) -> Vec<CommandWord> {
    let mut words = Vec::new();
    if let Some(name) = &cmd.word_or_name {
        words.push(CommandWord {
            position: CommandWordPosition::Name,
            ordinal: 0,
            value: name.value.clone(),
        });
    }
    if let Some(suffix) = &cmd.suffix {
        for (index, item) in suffix.0.iter().enumerate() {
            if let ast::CommandPrefixOrSuffixItem::Word(word) = item {
                words.push(CommandWord {
                    position: CommandWordPosition::Suffix(index),
                    ordinal: index + 1,
                    value: word.value.clone(),
                });
            }
        }
    }
    words
}

fn bunx_target(words: &[CommandWord], start: usize) -> Option<usize> {
    let contiguous = |left: usize, right: usize| {
        words.get(left).zip(words.get(right)).is_some_and(|(a, b)| b.ordinal == a.ordinal + 1)
    };
    let next = |index: usize| contiguous(index, index + 1).then_some(index + 1);

    if words.get(start)?.value != "bunx" {
        return None;
    }
    let mut target = next(start)?;

    // Skip over `--bun` flags to locate the inner command target. The flags are
    // preserved (not removed) so the user's runtime choice survives the rewrite.
    while words.get(target)?.value == "--bun" {
        target = next(target)?;
    }
    Some(target)
}

/// Commands that launch a following command, either env-injection proxies
/// (`dotenv`, `cross-env`, `portless`, ...) or package runners (`npm`, `pnpm`,
/// `yarn`, ...). Only when the command name is one of these does a trailing
/// `--`/`run`/`exec` separator turn a subsequent `bunx` into a launcher whose
/// inner command should be rewritten.
///
/// This deliberately excludes commands like `echo`/`printf` that treat
/// `-- bunx --bun vite build` as literal text rather than a command to run, so
/// their arguments are never corrupted by the rewrite.
const KNOWN_RUNNER_WRAPPERS: &[&str] = &[
    // Package runners.
    "npm",
    "pnpm",
    "yarn",
    "yarnpkg",
    "bun",
    "npx",
    "pnpx",
    // Env-injection / proxy wrappers.
    "cross-env",
    "cross-env-shell",
    "dotenv",
    "dotenvx",
    "portless",
];

/// Whether `name` is a command that launches a following command (see
/// [`KNOWN_RUNNER_WRAPPERS`]).
fn is_known_runner_wrapper(name: &str) -> bool {
    KNOWN_RUNNER_WRAPPERS.contains(&name)
}

fn find_bunx_invocations(cmd: &ast::SimpleCommand) -> Vec<BunxInvocation> {
    let words = collect_command_words(cmd);
    let mut invocations = Vec::new();

    for start in 0..words.len() {
        let Some(target) = bunx_target(&words, start) else {
            continue;
        };
        let CommandWordPosition::Suffix(target_suffix_index) = words[target].position else {
            continue;
        };

        let allowed_position = match words[start].position {
            // `bunx` is itself the command being run.
            CommandWordPosition::Name => true,
            // `bunx` appears as an argument: only treat it as a launcher when a
            // recognized wrapper command (`words[0]`) hands off to it via a
            // `--`/`run`/`exec` separator. A bare `--`/`run`/`exec` on a
            // non-wrapper command (e.g. `echo -- bunx ...`) is literal text.
            CommandWordPosition::Suffix(runner_index) => {
                let preceded_by_separator = cmd
                    .suffix
                    .as_ref()
                    .and_then(|suffix| runner_index.checked_sub(1).and_then(|i| suffix.0.get(i)))
                    .is_some_and(|item| {
                        matches!(
                            item,
                            ast::CommandPrefixOrSuffixItem::Word(word)
                                if matches!(word.value.as_str(), "--" | "run" | "exec")
                        )
                    });
                let command_is_wrapper = cmd
                    .word_or_name
                    .as_ref()
                    .is_some_and(|name| is_known_runner_wrapper(&name.value));
                preceded_by_separator && command_is_wrapper
            }
        };
        if allowed_position {
            invocations.push(BunxInvocation { target_suffix_index });
        }
    }

    invocations
}

fn parse_single_simple_command(script: &str) -> Option<ast::SimpleCommand> {
    let mut parser = brush_parser::Parser::new(
        script.as_bytes(),
        &brush_parser::ParserOptions::default(),
        &brush_parser::SourceInfo::default(),
    );
    let mut program = parser.parse_program().ok()?;
    if program.complete_commands.len() != 1 {
        return None;
    }
    let mut compound_list = program.complete_commands.pop()?;
    if compound_list.0.len() != 1 {
        return None;
    }
    let and_or = compound_list.0.pop()?.0;
    if !and_or.additional.is_empty() || and_or.first.seq.len() != 1 {
        return None;
    }
    match and_or.first.seq.into_iter().next()? {
        ast::Command::Simple(command) if command.prefix.is_none() => Some(command),
        _ => None,
    }
}

fn rewrite_bunx_in_simple_command(
    cmd: &mut ast::SimpleCommand,
    rewrite_inner: &mut impl FnMut(&str) -> String,
) -> bool {
    for invocation in find_bunx_invocations(cmd) {
        let Some(suffix) = &cmd.suffix else {
            continue;
        };
        let Some(ast::CommandPrefixOrSuffixItem::Word(target)) =
            suffix.0.get(invocation.target_suffix_index)
        else {
            continue;
        };

        let inner_command = ast::SimpleCommand {
            prefix: None,
            word_or_name: Some(target.clone()),
            suffix: (invocation.target_suffix_index + 1 < suffix.0.len()).then(|| {
                ast::CommandSuffix(suffix.0[invocation.target_suffix_index + 1..].to_vec())
            }),
        };
        let original_inner = inner_command.to_string();
        let rewritten_inner = rewrite_inner(&original_inner);
        if rewritten_inner == original_inner {
            continue;
        }
        let Some(mut replacement) = parse_single_simple_command(&rewritten_inner) else {
            continue;
        };

        let suffix = cmd.suffix.as_mut().expect("executor target is in the suffix");
        let mut replacement_items = Vec::new();
        if let Some(word) = replacement.word_or_name.take() {
            replacement_items.push(ast::CommandPrefixOrSuffixItem::Word(word));
        }
        if let Some(inner_suffix) = replacement.suffix.take() {
            replacement_items.extend(inner_suffix.0);
        }
        suffix.0.splice(invocation.target_suffix_index.., replacement_items);
        return true;
    }
    false
}

/// Rewrite commands launched through `bunx`. The runner and its `--bun` flag are
/// preserved when the inner command becomes `vp`, so the user's runtime choice
/// survives the rewrite (e.g. `bunx --bun vite build` → `bunx --bun vp build`).
pub(crate) fn rewrite_bunx_commands(
    script: &str,
    mut rewrite_inner: impl FnMut(&str) -> String,
) -> String {
    let mut parser = brush_parser::Parser::new(
        script.as_bytes(),
        &brush_parser::ParserOptions::default(),
        &brush_parser::SourceInfo::default(),
    );
    let Ok(mut program) = parser.parse_program() else {
        return script.to_owned();
    };
    if !visit_simple_commands(&mut program, &mut |cmd| {
        rewrite_bunx_in_simple_command(cmd, &mut rewrite_inner)
    }) {
        return script.to_owned();
    }

    collapse_newlines(&normalize_pipe_spacing(&program.to_string()))
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

    /// Minimal inner rewriter for bunx tests: `vite <args>` -> `vp <args>`.
    fn rewrite_vite_to_vp(inner: &str) -> String {
        match inner.split_once(' ') {
            Some(("vite", rest)) => format!("vp {rest}"),
            _ if inner == "vite" => "vp dev".to_owned(),
            _ => inner.to_owned(),
        }
    }

    #[test]
    fn bunx_as_command_name_is_a_launcher() {
        assert_eq!(
            rewrite_bunx_commands("bunx --bun vite build", rewrite_vite_to_vp),
            "bunx --bun vp build"
        );
    }

    #[test]
    fn suffix_bunx_launches_only_for_known_wrapper_commands() {
        // Env-injection wrapper using `--` to separate the launched command.
        assert_eq!(
            rewrite_bunx_commands("dotenv -e .env -- bunx --bun vite build", rewrite_vite_to_vp),
            "dotenv -e .env -- bunx --bun vp build"
        );
        // Proxy wrapper using a `run` subcommand to launch the command.
        assert_eq!(
            rewrite_bunx_commands("portless run bunx --bun vite build", rewrite_vite_to_vp),
            "portless run bunx --bun vp build"
        );
        // Package runner using `exec` to launch the command.
        assert_eq!(
            rewrite_bunx_commands("pnpm exec bunx --bun vite build", rewrite_vite_to_vp),
            "pnpm exec bunx --bun vp build"
        );
    }

    #[test]
    fn suffix_bunx_after_non_wrapper_command_stays_literal() {
        // `echo` prints its arguments verbatim: `-- bunx --bun vite build` is
        // literal text, not a command to launch, so it must stay unchanged.
        assert_eq!(
            rewrite_bunx_commands("echo -- bunx --bun vite build", rewrite_vite_to_vp),
            "echo -- bunx --bun vite build"
        );
        assert_eq!(
            rewrite_bunx_commands("printf -- bunx --bun vite build", rewrite_vite_to_vp),
            "printf -- bunx --bun vite build"
        );
    }
}
