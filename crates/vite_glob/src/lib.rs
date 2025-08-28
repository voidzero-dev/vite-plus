use std::path::Path;

use compact_str::CompactString;
use wax::{Glob, Pattern};

use vite_error::Error;

/// A glob pattern set, support the last match wins semantics.
///
/// If there no any negated pattern, it will follow the first match wins semantics.
/// Otherwise, it will follow the last match wins semantics.
#[derive(Debug)]
pub struct GlobPatternSet<'a> {
    /// (glob_pattern, match_or_not)
    patterns: Vec<(Glob<'a>, bool)>,
    has_negated: bool,
}

impl<'a> GlobPatternSet<'a> {
    pub fn new(match_patterns: &'a [CompactString]) -> Result<Self, Error> {
        let mut patterns = Vec::new();
        let mut has_negated = false;
        for pattern in match_patterns {
            if let Some(negated) = pattern.strip_prefix('!') {
                // negated pattern, ignore the path
                patterns.push((Glob::new(negated)?, false));
                // set to true to follow last match wins semantics
                has_negated = true;
            } else {
                // positive pattern, match the path
                patterns.push((Glob::new(pattern)?, true));
            }
        }
        Ok(Self { patterns, has_negated })
    }

    pub fn is_match(&self, path: impl AsRef<Path>) -> bool {
        let mut should_match = false; // Default: don't match
        for (glob, match_or_not) in &self.patterns {
            if glob.is_match(path.as_ref()) {
                should_match = *match_or_not;
                if !self.has_negated {
                    // first match wins semantics
                    break;
                }
            }
        }
        should_match
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_ignores_node_modules() -> Result<(), Error> {
        let patterns = vec![
            // ignore all paths
            "**/*".into(),
            // keep node_modules directories themselves
            "!**/node_modules".into(),
            "!node_modules".into(),
            // keep lock files and package.json
            "!**/package.json".into(),
            "!**/package-lock.json".into(),
            "!**/yarn.lock".into(),
            "!**/pnpm-lock.yaml".into(),
        ];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore paths inside node_modules
        assert!(ignores.is_match("node_modules/react/index.js"));
        assert!(ignores.is_match("apps/web/node_modules/react/index.js"));
        assert!(ignores.is_match("packages/cli/node_modules/@types/node/index.d.ts"));

        // Should ignore paths outside node_modules
        assert!(ignores.is_match("src/index.js"));
        assert!(ignores.is_match("tsbuildinfo.json"));

        // Should NOT ignore node_modules directories themselves (due to negation)
        assert!(!ignores.is_match("node_modules"));
        assert!(!ignores.is_match("apps/web/node_modules"));
        assert!(!ignores.is_match("packages/cli/node_modules"));

        // Should NOT ignore lock files and package.json
        assert!(!ignores.is_match("package.json"));
        assert!(!ignores.is_match("apps/web/package.json"));
        assert!(!ignores.is_match("package-lock.json"));
        assert!(!ignores.is_match("apps/web/yarn.lock"));
        assert!(!ignores.is_match("pnpm-lock.yaml"));
        assert!(!ignores.is_match("node_modules/react/package.json"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_with_file_patterns() -> Result<(), Error> {
        let mut patterns = Vec::<CompactString>::new();
        patterns.push("*.log".into());
        patterns.push("**/*.tmp".into());
        patterns.push("!important.log".into());
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore matching files
        assert!(ignores.is_match("debug.log"));
        assert!(ignores.is_match("error.log"));
        assert!(ignores.is_match("temp/file.tmp"));
        assert!(ignores.is_match("deep/nested/path/cache.tmp"));

        // Should NOT ignore negated patterns
        assert!(!ignores.is_match("important.log"));

        // Should NOT ignore non-matching files
        assert!(!ignores.is_match("file.txt"));
        assert!(!ignores.is_match("logs/file.txt"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_directory_patterns() -> Result<(), Error> {
        let patterns = vec!["dist/**".into(), "build/**".into(), "!dist/public/**".into()];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore paths in dist and build
        assert!(ignores.is_match("dist/bundle.js"));
        assert!(ignores.is_match("dist/assets/style.css"));
        assert!(ignores.is_match("build/output.js"));
        assert!(ignores.is_match("build/assets/image.png"));

        // Should NOT ignore negated paths
        assert!(!ignores.is_match("dist/public/index.html"));
        assert!(!ignores.is_match("dist/public/assets/logo.png"));

        // Should NOT ignore paths outside target directories
        assert!(!ignores.is_match("src/index.js"));
        assert!(!ignores.is_match("public/index.html"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_complex_patterns() -> Result<(), Error> {
        let patterns = vec![
            "**/*.test.js".into(),
            "**/*.spec.ts".into(),
            "**/test/**".into(),
            "**/tests/**".into(),
            "!**/integration/tests/**".into(),
        ];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore test files
        assert!(ignores.is_match("src/utils.test.js"));
        assert!(ignores.is_match("components/Button.spec.ts"));
        assert!(ignores.is_match("lib/test/helper.js"));
        assert!(ignores.is_match("src/tests/unit/math.js"));

        // Should NOT ignore negated patterns
        assert!(!ignores.is_match("integration/tests/e2e.js"));
        assert!(!ignores.is_match("integration/tests/api/user.js"));

        // Should NOT ignore non-test files
        assert!(!ignores.is_match("src/index.js"));
        assert!(!ignores.is_match("lib/utils.js"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_empty_patterns() -> Result<(), Error> {
        let patterns = vec![];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should not ignore anything with empty patterns
        assert!(!ignores.is_match("node_modules/package.json"));
        assert!(!ignores.is_match("src/index.js"));
        assert!(!ignores.is_match("dist/bundle.js"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_with_wildcards() -> Result<(), Error> {
        let patterns = vec!["*.{js,ts,jsx,tsx}".into(), "!index.js".into(), "!main.ts".into()];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore matching extensions
        assert!(ignores.is_match("utils.js"));
        assert!(ignores.is_match("component.tsx"));
        assert!(ignores.is_match("service.ts"));
        assert!(ignores.is_match("App.jsx"));

        // Should NOT ignore negated files
        assert!(!ignores.is_match("index.js"));
        assert!(!ignores.is_match("main.ts"));

        // Should NOT ignore other extensions
        assert!(!ignores.is_match("styles.css"));
        assert!(!ignores.is_match("data.json"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_dotfiles() -> Result<(), Error> {
        let patterns = vec![".*".into(), "!.gitignore".into(), "!.env.example".into()];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Should ignore dotfiles
        assert!(ignores.is_match(".env"));
        assert!(ignores.is_match(".DS_Store"));
        assert!(ignores.is_match(".vscode"));

        // Should NOT ignore negated dotfiles
        assert!(!ignores.is_match(".gitignore"));
        assert!(!ignores.is_match(".env.example"));

        // Should NOT ignore regular files
        assert!(!ignores.is_match("README.md"));
        assert!(!ignores.is_match("src/index.js"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_root_patterns() -> Result<(), Error> {
        // Note: wax doesn't support leading / for root patterns like gitignore
        // Using glob patterns that work with wax
        let patterns = vec![
            "**/dist".into(), // Match dist at any level
            "!dist/public".into(),
            "**/node_modules".into(),
        ];
        let ignores = GlobPatternSet::new(&patterns)?;
        // Patterns match at any level
        assert!(ignores.is_match("dist"));
        assert!(ignores.is_match("src/dist")); // Also matches nested

        // Negation works
        assert!(!ignores.is_match("dist/public"));

        // Node_modules patterns
        assert!(ignores.is_match("node_modules"));
        assert!(ignores.is_match("src/node_modules"));
        assert!(ignores.is_match("packages/app/node_modules"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_directory_only_patterns() -> Result<(), Error> {
        let patterns = vec![
            "build/**".into(),       // Match everything under build
            "!build/keep/**".into(), // But not under build/keep
        ];
        let ignores = GlobPatternSet::new(&patterns)?;
        // Directory patterns
        assert!(ignores.is_match("build/output.js"));
        assert!(ignores.is_match("build/assets/style.css"));

        // Negated directory
        assert!(!ignores.is_match("build/keep/important.txt"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_mixed_patterns() -> Result<(), Error> {
        let patterns = vec![
            "**/*.log".into(), // Match .log files at any depth
            "**/temp/**".into(),
            "node_modules/**".into(),
            "!**/temp/keep/**".into(),
            "!debug.log".into(),
        ];
        let ignores = GlobPatternSet::new(&patterns)?;

        // Test various patterns together
        assert!(ignores.is_match("error.log"));
        assert!(ignores.is_match("src/app.log"));
        assert!(!ignores.is_match("debug.log")); // Negated

        assert!(ignores.is_match("temp/file.txt"));
        assert!(ignores.is_match("src/temp/cache.dat"));
        assert!(!ignores.is_match("temp/keep/important.txt")); // Negated

        assert!(ignores.is_match("node_modules/react/index.js"));
        assert!(ignores.is_match("node_modules/@types/node/index.d.ts"));

        assert!(!ignores.is_match("src/index.js"));
        assert!(!ignores.is_match("package.json"));

        Ok(())
    }

    #[test]
    fn test_match_ignores_last_matching_pattern() -> Result<(), Error> {
        // Test that the last matching pattern wins (gitignore semantics)
        let patterns = vec![
            "logs/**".into(),             // First: ignore everything in logs/
            "!logs/important.log".into(), // Second: don't ignore important.log
            "logs/important.log".into(),  // Third: ignore important.log again (this wins)
        ];
        let ignores = GlobPatternSet::new(&patterns)?;

        assert!(ignores.is_match("logs/error.log"));
        assert!(ignores.is_match("logs/src/app.log"));
        assert!(ignores.is_match("logs/debug.log"));
        // The last pattern "logs/important.log" (positive) wins over "!logs/important.log" (negative)
        assert!(ignores.is_match("logs/important.log")); // Should be ignored!

        Ok(())
    }
}
