use crate::script_rewrite::{ScriptRewriteConfig, rewrite_script};

const ESLINT_CONFIG: ScriptRewriteConfig = ScriptRewriteConfig {
    source_command: "eslint",
    target_subcommand: "lint",
    boolean_flags: &[
        "--cache",
        "--no-eslintrc",
        "--no-error-on-unmatched-pattern",
        "--debug",
        "--no-inline-config",
    ],
    value_flags: &[
        "--ext",
        "--rulesdir",
        "--resolve-plugins-relative-to",
        "--parser",
        "--parser-options",
        "--plugin",
        "--output-file",
        "--env",
    ],
    flag_conversions: &[],
};

/// Rewrite a single script: rename `eslint` → `vp lint` and strip ESLint-only flags.
pub(crate) fn rewrite_eslint_script(script: &str) -> String {
    rewrite_script(script, &ESLINT_CONFIG)
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
}
