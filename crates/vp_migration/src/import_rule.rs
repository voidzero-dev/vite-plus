use std::path::Path;

use serde::Deserialize;
use vp_error::Error;

/// The import rule's settings, with inherited configs flattened by the JS caller.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRuleConfig {
    enabled: bool,
    overrides: Vec<ImportRuleOverride>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportRuleOverride {
    files: Vec<String>,
    exclude_files: Vec<String>,
    enabled: bool,
}

impl Default for ImportRuleConfig {
    fn default() -> Self {
        Self { enabled: true, overrides: Vec::new() }
    }
}

impl ImportRuleConfig {
    pub fn from_json(json: &str) -> Result<Self, Error> {
        let mut config: Self = serde_json::from_str(json)?;
        for entry in &mut config.overrides {
            for pattern in entry.files.iter_mut().chain(&mut entry.exclude_files) {
                fast_glob::validate(&*pattern).map_err(|error| {
                    Error::InvalidArgument(format!("Invalid lint glob {pattern:?}: {error}").into())
                })?;
                // Match Oxlint's GlobSet: bare names match at every depth, while
                // ./ anchors a pattern to the config directory. These are glob
                // patterns, not gitignore rules.
                if let Some(relative) = pattern.strip_prefix("./") {
                    *pattern = relative.to_string();
                } else if !pattern.contains('/') {
                    *pattern = format!("**/{pattern}");
                }
            }
        }
        Ok(config)
    }

    pub(crate) fn is_enabled(&self, relative_path: &Path) -> bool {
        let path = relative_path.to_string_lossy().replace('\\', "/");
        self.overrides
            .iter()
            .filter(|entry| {
                entry.files.iter().any(|pattern| fast_glob::glob_match(pattern, &path))
                    && !entry
                        .exclude_files
                        .iter()
                        .any(|pattern| fast_glob::glob_match(pattern, &path))
            })
            .fold(self.enabled, |_, entry| entry.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_oxlint_globs_and_applies_overrides_in_order() {
        let config = ImportRuleConfig::from_json(r#"{
            "enabled": true,
            "overrides": [
                { "files": ["templates/**", "*.test.ts"], "excludeFiles": ["templates/keep/**"], "enabled": false },
                { "files": ["./root.test.ts", "templates/managed/**/*.{ts,tsx}"], "excludeFiles": [], "enabled": true }
            ]
        }"#).unwrap();
        for (path, enabled) in [
            ("app.ts", true),
            ("templates/starter/vite.config.ts", false),
            ("templates/keep/vite.config.ts", true),
            ("templates/managed/test.tsx", true),
            ("deep/unit.test.ts", false),
            ("root.test.ts", true),
            ("deep/root.test.ts", false),
            (r"templates\starter\vite.config.ts", false),
        ] {
            assert_eq!(config.is_enabled(Path::new(path)), enabled, "{path}");
        }
    }

    #[test]
    fn rejects_invalid_patterns() {
        let result = ImportRuleConfig::from_json(
            r#"{
            "enabled": true,
            "overrides": [{ "files": ["src/*.{ts"], "excludeFiles": [], "enabled": false }]
        }"#,
        );
        assert!(result.unwrap_err().to_string().contains("Invalid lint glob"));
    }
}
