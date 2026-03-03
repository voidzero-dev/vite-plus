//! String similarity helpers shared by CLI crates.

/// Compute Levenshtein distance between two strings.
#[must_use]
pub fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();

    let mut prev: Vec<usize> = (0..=right_chars.len()).collect();
    let mut curr = vec![0; right_chars.len() + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            let deletion = prev[j + 1] + 1;
            let insertion = curr[j] + 1;
            let substitution = prev[j] + cost;
            curr[j + 1] = deletion.min(insertion).min(substitution);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[right_chars.len()]
}

/// Pick the best suggestion by Levenshtein distance and then shortest length.
#[must_use]
pub fn pick_best_suggestion(input: &str, candidates: &[String]) -> Option<String> {
    candidates
        .iter()
        .min_by_key(|candidate| (levenshtein_distance(input, candidate), candidate.len()))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{levenshtein_distance, pick_best_suggestion};

    #[test]
    fn distance_works_for_simple_inputs() {
        assert_eq!(levenshtein_distance("fnt", "fmt"), 1);
        assert_eq!(levenshtein_distance("fnt", "lint"), 2);
    }

    #[test]
    fn pick_best_prefers_closest_match() {
        let candidates = vec!["lint".to_string(), "fmt".to_string()];
        assert_eq!(pick_best_suggestion("fnt", &candidates), Some("fmt".to_string()));
    }
}
