//! Error-formatting helpers.

use std::error::Error;

/// Maximum chain depth `format_error_chain` will walk.
///
/// Guards against pathological / cyclic `source()` chains (rare but possible
/// when an error type holds itself via `Box<dyn Error>` or `Arc`).
const MAX_CHAIN_DEPTH: usize = 16;

/// Format an error and its full `source()` chain as `top: cause: deeper-cause`.
///
/// Use this when stringifying an error into a field of a higher-level error
/// type — otherwise the Display impl of types like `reqwest::Error` only shows
/// the top-level message, hiding the actual cause (TLS handshake failure,
/// connection refused, etc.).
///
/// Behavior notes:
/// - Walks at most [`MAX_CHAIN_DEPTH`] levels; further sources are summarized
///   as `: ...`.
/// - Skips a source whose Display is already contained in the accumulated
///   message — avoids duplicates when a parent thiserror variant inlines its
///   `#[from]` source via `{0}`.
#[must_use]
pub fn format_error_chain(err: &(dyn Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut current = err.source();
    let mut depth = 0_usize;
    while let Some(source) = current {
        if depth >= MAX_CHAIN_DEPTH {
            out.push_str(": ...");
            break;
        }
        let part = source.to_string();
        if !part.is_empty() && !out.contains(&part) {
            out.push_str(": ");
            out.push_str(&part);
        }
        current = source.source();
        depth += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{error::Error as StdError, fmt};

    use super::*;

    #[derive(Debug)]
    struct Layer {
        msg: String,
        cause: Option<Box<Layer>>,
    }

    impl Layer {
        fn new(msg: &str) -> Self {
            Self { msg: msg.to_string(), cause: None }
        }

        fn with_cause(mut self, cause: Layer) -> Self {
            self.cause = Some(Box::new(cause));
            self
        }
    }

    impl fmt::Display for Layer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.msg)
        }
    }

    impl StdError for Layer {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.cause.as_deref().map(|c| c as &(dyn StdError + 'static))
        }
    }

    #[test]
    fn single_error_no_chain() {
        let e = Layer::new("top");
        assert_eq!(format_error_chain(&e), "top");
    }

    #[test]
    fn walks_full_chain() {
        let e = Layer::new("send request")
            .with_cause(Layer::new("tls handshake").with_cause(Layer::new("UnknownIssuer")));
        assert_eq!(format_error_chain(&e), "send request: tls handshake: UnknownIssuer");
    }

    #[test]
    fn dedupes_source_already_in_parent() {
        // thiserror's `#[error("Wrapped: {0}")]` style: parent already
        // contains the inner message — don't print it twice.
        let inner = Layer::new("TLS error: UnknownIssuer");
        let outer = Layer::new("Wrapped: TLS error: UnknownIssuer").with_cause(inner);
        assert_eq!(format_error_chain(&outer), "Wrapped: TLS error: UnknownIssuer");
    }

    #[test]
    fn dedupes_partial_overlap() {
        // The source message appears as a substring of the parent — skip it.
        let parent = Layer::new("top: foo bar").with_cause(Layer::new("foo bar"));
        assert_eq!(format_error_chain(&parent), "top: foo bar");
    }

    #[test]
    fn caps_at_max_depth() {
        let mut chain = Layer::new("leaf");
        for i in 0..(MAX_CHAIN_DEPTH + 5) {
            chain = Layer::new(&format!("level-{i}")).with_cause(chain);
        }
        let out = format_error_chain(&chain);
        assert!(out.ends_with(": ..."), "expected truncation marker, got {out}");
    }

    #[test]
    fn skips_empty_source_messages() {
        let parent = Layer::new("top").with_cause(Layer::new(""));
        assert_eq!(format_error_chain(&parent), "top");
    }
}
