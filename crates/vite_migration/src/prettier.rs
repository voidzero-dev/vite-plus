use crate::script_rewrite::{FlagConversion, ScriptRewriteConfig, rewrite_script};

const PRETTIER_CONFIG: ScriptRewriteConfig = ScriptRewriteConfig {
    source_command: "prettier",
    target_subcommand: "fmt",
    boolean_flags: &[
        "--write",
        "-w",
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
        "--ignore-unknown",
        "-u",
        "--no-color",
        "--no-plugin-search",
    ],
    value_flags: &[
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
    ],
    flag_conversions: &[FlagConversion {
        source_flags: &["--list-different", "-l", "-c"],
        target_flag: "--check",
        dedup_flag: "--check",
    }],
};

/// Rewrite a single script: rename `prettier` → `vp fmt`, strip Prettier-only flags,
/// and convert `--list-different`/`-l` → `--check`.
pub(crate) fn rewrite_prettier_script(script: &str) -> String {
    rewrite_script(script, &PRETTIER_CONFIG)
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
    fn test_rewrite_prettier_short_flags() {
        // -w is short for --write (stripped)
        assert_eq!(rewrite_prettier_script("prettier -w ."), "vp fmt .");
        // -c is short for --check (converted)
        assert_eq!(rewrite_prettier_script("prettier -c ."), "vp fmt --check .");
        // Combined with other flags
        assert_eq!(rewrite_prettier_script("prettier -w --single-quote ."), "vp fmt .");
        assert_eq!(rewrite_prettier_script("prettier -c --single-quote ."), "vp fmt --check .");
    }

    #[test]
    fn test_rewrite_prettier_ignore_unknown_stripped() {
        // --ignore-unknown is a prettier-only flag, should be stripped
        assert_eq!(
            rewrite_prettier_script(
                "prettier . --cache --write --ignore-unknown --experimental-cli"
            ),
            "vp fmt ."
        );
        // -u is short for --ignore-unknown
        assert_eq!(rewrite_prettier_script("prettier --write -u ."), "vp fmt .");
        // --ignore-unknown with --check
        assert_eq!(
            rewrite_prettier_script("prettier --ignore-unknown --check ."),
            "vp fmt --check ."
        );
        // --no-color stripped
        assert_eq!(rewrite_prettier_script("prettier --no-color --write ."), "vp fmt .");
        // --no-plugin-search stripped
        assert_eq!(rewrite_prettier_script("prettier --no-plugin-search --write ."), "vp fmt .");
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
