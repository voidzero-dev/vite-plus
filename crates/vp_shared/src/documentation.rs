//! Documentation links are fixed at build time, not selected from the user's environment.

const ORIGIN: &str = match option_env!("VITE_PLUS_DOCS_ORIGIN") {
    Some(origin) if !origin.is_empty() => origin,
    _ => "https://viteplus.dev",
};

pub fn documentation_url(path: &str) -> String {
    format!("{}{path}", ORIGIN.trim_end_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::{ORIGIN, documentation_url};

    #[test]
    fn retains_documentation_paths_and_fragments() {
        assert_eq!(
            documentation_url("/guide/vitest-v5#preserve-existing-behavior"),
            format!("{}/guide/vitest-v5#preserve-existing-behavior", ORIGIN.trim_end_matches('/'))
        );
    }
}
