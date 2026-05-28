//! Error-formatting helpers.

use std::error::Error;

/// Format an error and its full `source()` chain as `top: cause: deeper-cause`.
///
/// Use this when stringifying an error into a field of a higher-level error
/// type — otherwise the Display impl of types like `reqwest::Error` only shows
/// the top-level message, hiding the actual cause (TLS handshake failure,
/// connection refused, etc.).
#[must_use]
pub fn format_error_chain(err: &(dyn Error + 'static)) -> String {
    let mut out = err.to_string();
    let mut current = err.source();
    while let Some(source) = current {
        out.push_str(": ");
        out.push_str(&source.to_string());
        current = source.source();
    }
    out
}

#[cfg(test)]
mod tests {
    use std::{error::Error as StdError, fmt};

    use super::*;

    #[derive(Debug)]
    struct Layer {
        msg: &'static str,
        cause: Option<Box<Layer>>,
    }

    impl fmt::Display for Layer {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.msg)
        }
    }

    impl StdError for Layer {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.cause.as_deref().map(|c| c as &(dyn StdError + 'static))
        }
    }

    #[test]
    fn single_error_no_chain() {
        let e = Layer { msg: "top", cause: None };
        assert_eq!(format_error_chain(&e), "top");
    }

    #[test]
    fn walks_full_chain() {
        let e = Layer {
            msg: "send request",
            cause: Some(Box::new(Layer {
                msg: "tls handshake",
                cause: Some(Box::new(Layer { msg: "UnknownIssuer", cause: None })),
            })),
        };
        assert_eq!(format_error_chain(&e), "send request: tls handshake: UnknownIssuer");
    }
}
