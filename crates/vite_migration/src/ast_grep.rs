use ast_grep_config::{GlobalRules, RuleConfig, from_yaml_string};
use ast_grep_core::replacer::Replacer;
use ast_grep_language::{LanguageExt, SupportLang};
use vite_error::Error;

/// Apply ast-grep rules to content and return the transformed content
///
/// This is the core transformation function that:
/// 1. Parses the rule YAML
/// 2. Applies each rule to find matches
/// 3. Replaces matches from back to front to maintain correct positions
///
/// # Arguments
///
/// * `content` - The source content to transform
/// * `rule_yaml` - The ast-grep rules in YAML format
///
/// # Returns
///
/// A tuple of (transformed_content, was_updated)
pub(crate) fn apply_rules(content: &str, rule_yaml: &str) -> Result<(String, bool), Error> {
    let rules = load_rules(rule_yaml)?;
    let result = apply_loaded_rules(content, &rules);
    let updated = result != content;
    Ok((result, updated))
}

/// Load ast-grep rules from YAML string
pub(crate) fn load_rules(yaml: &str) -> Result<Vec<RuleConfig<SupportLang>>, Error> {
    let globals = GlobalRules::default();
    let rules: Vec<RuleConfig<SupportLang>> = from_yaml_string::<SupportLang>(yaml, &globals)?;
    Ok(rules)
}

/// Apply pre-loaded ast-grep rules to content
///
/// This is useful when you need to apply the same rules multiple times
/// (e.g., processing multiple scripts in a loop).
///
/// # Arguments
///
/// * `content` - The source content to transform
/// * `rules` - Pre-loaded ast-grep rules
///
/// # Returns
///
/// The transformed content (always returns a new string, even if unchanged)
pub(crate) fn apply_loaded_rules(content: &str, rules: &[RuleConfig<SupportLang>]) -> String {
    let mut current = content.to_string();

    for rule in rules {
        // Parse current content with the rule's language
        let grep = rule.language.ast_grep(&current);
        let root = grep.root();

        let matcher = &rule.matcher;

        // Get the fixer if available (rules without fix are pure lint, skip them)
        let fixers = match rule.get_fixer() {
            Ok(f) if !f.is_empty() => f,
            _ => continue,
        };

        // Collect all matches and their replacements
        let mut replacements = Vec::new();
        for node in root.find_all(matcher) {
            let range = node.range();
            let replacement_bytes = fixers[0].generate_replacement(&node);
            let replacement_str = String::from_utf8_lossy(&replacement_bytes).to_string();
            replacements.push((range.start, range.end, replacement_str));
        }

        // Replace from back to front to maintain correct positions
        replacements.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));

        for (start, end, replacement) in replacements {
            current.replace_range(start..end, &replacement);
        }
    }

    current
}
