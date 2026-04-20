//! Shared Vite+ header rendering.
//!
//! Header coloring behavior:
//! - Color capability detection via `supports-color`
//! - Default Vite+ blue-to-magenta truecolor gradient when supported
//! - Plain text fallback when stdout is not a color-capable TTY

use std::{io::IsTerminal, sync::LazyLock};

use supports_color::{Stream, on};

const CSI: &str = "\x1b[";
const RESET: &str = "\x1b[0m";

const HEADER_SUFFIX: &str = " - The Unified Toolchain for the Web";

const RESET_FG: &str = "\x1b[39m";
const DEFAULT_BLUE: Rgb = Rgb(88, 146, 255);
const DEFAULT_MAGENTA: Rgb = Rgb(187, 116, 247);
const HEADER_SUFFIX_FADE_GAMMA: f64 = 1.35;

/// Whether the terminal is Warp, whose block-mode renderer needs small
/// command-picker layout adjustments.
#[must_use]
pub fn is_warp_terminal() -> bool {
    static IS_WARP: LazyLock<bool> =
        LazyLock::new(|| std::env::var("TERM_PROGRAM").as_deref() == Ok("WarpTerminal"));
    *IS_WARP
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

fn bold(text: &str, enabled: bool) -> String {
    if enabled { format!("\x1b[1m{text}\x1b[22m") } else { text.to_string() }
}

fn fg_rgb(color: Rgb) -> String {
    format!("{CSI}38;2;{};{};{}m", color.0, color.1, color.2)
}

fn supports_true_color() -> bool {
    std::io::stdout().is_terminal() && on(Stream::Stdout).is_some_and(|color| color.has_16m)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn gradient_eased(count: usize, start: Rgb, end: Rgb, gamma: f64) -> Vec<Rgb> {
    let n = count.max(1);
    let denom = (n - 1).max(1) as f64;

    (0..n)
        .map(|i| {
            let t = (i as f64 / denom).powf(gamma);
            Rgb(
                lerp(start.0 as f64, end.0 as f64, t).round() as u8,
                lerp(start.1 as f64, end.1 as f64, t).round() as u8,
                lerp(start.2 as f64, end.2 as f64, t).round() as u8,
            )
        })
        .collect()
}

fn colorize(text: &str, colors: &[Rgb]) -> String {
    if text.is_empty() {
        return String::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let denom = (chars.len() - 1).max(1) as f64;
    let max_idx = colors.len().saturating_sub(1) as f64;

    let mut out = String::new();
    for (i, ch) in chars.into_iter().enumerate() {
        let idx = ((i as f64 / denom) * max_idx).round() as usize;
        out.push_str(&fg_rgb(colors[idx]));
        out.push(ch);
    }
    out.push_str(RESET);
    out
}

fn render_header_variant(
    primary: Rgb,
    suffix_colors: &[Rgb],
    prefix_bold: bool,
    suffix_bold: bool,
) -> String {
    let vite_plus = format!("{}VITE+{RESET_FG}", fg_rgb(primary));
    let suffix = colorize(HEADER_SUFFIX, suffix_colors);
    format!("{}{}", bold(&vite_plus, prefix_bold), bold(&suffix, suffix_bold))
}

fn default_colored_header() -> String {
    let suffix_gradient = gradient_eased(
        HEADER_SUFFIX.chars().count(),
        DEFAULT_BLUE,
        DEFAULT_MAGENTA,
        HEADER_SUFFIX_FADE_GAMMA,
    );
    render_header_variant(DEFAULT_BLUE, &suffix_gradient, true, true)
}

/// Render the Vite+ CLI header string.
#[must_use]
pub fn vite_plus_header() -> String {
    if !supports_true_color() {
        return format!("VITE+{HEADER_SUFFIX}");
    }

    default_colored_header()
}

/// Whether the Vite+ banner should be emitted in the current environment.
///
/// The banner is cosmetic and assumes an interactive terminal; it's
/// suppressed when:
/// - stdout is piped or redirected (lefthook/husky, `execSync`, CI, pagers).
/// - a git commit-flow hook is running. Direct shell hooks inherit the
///   terminal for stdout, so the TTY check alone doesn't catch them; git
///   sets `GIT_INDEX_FILE` for pre-commit / commit-msg / prepare-commit-msg,
///   which is where `vp check --fix` typically runs.
#[must_use]
pub fn should_print_header() -> bool {
    if !std::io::stdout().is_terminal() {
        return false;
    }
    if std::env::var_os("GIT_INDEX_FILE").is_some() {
        return false;
    }
    true
}

/// Emit the Vite+ banner (header line + trailing blank line) to stdout, but
/// only when the environment is interactive. No-op otherwise.
pub fn print_header() {
    if !should_print_header() {
        return;
    }
    println!("{}", vite_plus_header());
    println!();
}

#[cfg(test)]
mod tests {
    use super::{Rgb, gradient_eased};

    #[test]
    fn gradient_counts_match() {
        assert_eq!(gradient_eased(0, Rgb(0, 0, 0), Rgb(255, 255, 255), 1.0).len(), 1);
        assert_eq!(gradient_eased(5, Rgb(10, 20, 30), Rgb(40, 50, 60), 1.0).len(), 5);
    }

    #[test]
    fn gradient_interpolates_endpoints() {
        let gradient = gradient_eased(3, Rgb(0, 0, 0), Rgb(10, 20, 30), 1.0);
        assert_eq!(gradient, vec![Rgb(0, 0, 0), Rgb(5, 10, 15), Rgb(10, 20, 30)]);
    }
}
