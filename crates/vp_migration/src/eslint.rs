use crate::script_rewrite::{FlagConversion, ScriptRewriteConfig, rewrite_script};

const ESLINT_CONFIG: ScriptRewriteConfig = ScriptRewriteConfig {
    source_command: "eslint",
    target_subcommand: "lint",
    boolean_flags: &[
        "--cache",
        "--no-eslintrc",
        "--debug",
        "--no-inline-config",
        "--color",
        "--no-color",
        "--exit-on-fatal-error",
        "--stats",
        "--pass-on-unpruned-suppressions",
    ],
    value_flags: &[
        "--ext",
        "--rulesdir",
        "--resolve-plugins-relative-to",
        "--parser",
        "--parser-options",
        "--plugin",
        "-o",
        "--output-file",
        "--env",
        "--cache-location",
        "--cache-file",
        "--cache-strategy",
        "--flag",
        "--concurrency",
    ],
    // ESLint only warns about ignored files, but Oxlint fails when they leave nothing to lint.
    flag_conversions: &[FlagConversion {
        source_flags: &["--no-warn-ignored"],
        target_flag: "--no-error-on-unmatched-pattern",
        dedup_flag: "--no-error-on-unmatched-pattern",
    }],
};

/// Rewrite a single script: rename `eslint` → `vp lint` and strip ESLint-only flags.
pub fn rewrite_eslint_script(script: &str) -> String {
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
        // Oxlint supports this flag with the same meaning, so it is kept
        assert_eq!(
            rewrite_eslint_script("eslint --no-error-on-unmatched-pattern --fix ."),
            "vp lint --no-error-on-unmatched-pattern --fix ."
        );
        // ignored-only file lists keep succeeding, as they did with ESLint
        assert_eq!(
            rewrite_eslint_script("eslint --max-warnings 0 --no-warn-ignored --fix"),
            "vp lint --max-warnings 0 --no-error-on-unmatched-pattern --fix"
        );
        assert_eq!(
            rewrite_eslint_script(
                "eslint --no-warn-ignored --no-error-on-unmatched-pattern --fix ."
            ),
            "vp lint --no-error-on-unmatched-pattern --fix ."
        );
        assert_eq!(rewrite_eslint_script("eslint --no-inline-config ."), "vp lint .");
        assert_eq!(rewrite_eslint_script("eslint --no-color --stats --fix ."), "vp lint --fix .");

        // Oxlint has no cache, so cache options go with --cache
        assert_eq!(
            rewrite_eslint_script(
                "eslint --cache --cache-location node_modules/.cache/eslint --cache-strategy content --fix ."
            ),
            "vp lint --fix ."
        );
        assert_eq!(rewrite_eslint_script("eslint -o report.txt ."), "vp lint .");

        // flags Oxlint accepts stay
        assert_eq!(
            rewrite_eslint_script("eslint --quiet --max-warnings 0 --ignore-pattern dist ."),
            "vp lint --quiet --max-warnings 0 --ignore-pattern dist ."
        );

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
    fn test_rewrite_eslint_strips_flags_oxlint_rejects() {
        // Each value flag must also drop its value; a boolean flag must keep the next token.
        for flags in [
            "-o report.txt",
            "--cache-location .eslintcache",
            "--cache-file .eslintcache",
            "--cache-strategy content",
            "--flag unstable_config_lookup_from_file",
            "--concurrency 2",
            "--color",
            "--no-color",
            "--stats",
            "--exit-on-fatal-error",
            "--pass-on-unpruned-suppressions",
        ] {
            assert_eq!(
                rewrite_eslint_script(&format!("eslint {flags} --fix .")),
                "vp lint --fix .",
                "{flags}"
            );
        }

        // Stripping these would change what the script checks or fixes, so they stay and fail loudly.
        assert_eq!(
            rewrite_eslint_script("eslint -c eslint.ci.config.mjs src"),
            "vp lint -c eslint.ci.config.mjs src"
        );
        assert_eq!(
            rewrite_eslint_script("eslint --rule 'no-console: error' src"),
            "vp lint --rule 'no-console: error' src"
        );
        for flags in [
            "--no-config-lookup",
            "--global appRuntime",
            "--report-unused-inline-configs error",
            "--suppress-rule no-console",
            "--suppressions-location suppressions.json",
        ] {
            assert_eq!(
                rewrite_eslint_script(&format!("eslint {flags} --fix src")),
                format!("vp lint {flags} --fix src"),
            );
        }
        assert_eq!(
            rewrite_eslint_script("eslint --fix --pass-on-no-patterns $FILES"),
            "vp lint --fix --pass-on-no-patterns $FILES"
        );
        assert_eq!(
            rewrite_eslint_script("eslint --fix --fix-type layout src"),
            "vp lint --fix --fix-type layout src"
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
    fn test_rewrite_eslint_bunx() {
        assert_eq!(
            rewrite_eslint_script("bunx --bun eslint --cache --fix ."),
            "bunx --bun vp lint --fix ."
        );
        assert_eq!(
            rewrite_eslint_script("dotenv -e .env -- bunx --bun eslint --ext .ts ."),
            "dotenv -e .env -- bunx --bun vp lint ."
        );
        assert_eq!(
            rewrite_eslint_script("bunx --bun eslint-plugin-foo"),
            "bunx --bun eslint-plugin-foo"
        );
    }
}
