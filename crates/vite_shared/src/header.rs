//! Shared Vite+ header rendering.
//!
//! Header coloring behavior:
//! - Colorization and truecolor capability gates
//! - Foreground color OSC query (`ESC ] 10 ; ? ESC \\`) with timeout
//! - ANSI palette queries for blue/magenta with timeout
//! - Gradient/fade generation and RGB ANSI coloring

use std::{
    io::IsTerminal,
    sync::{LazyLock, OnceLock},
};
#[cfg(unix)]
use std::{
    io::Write,
    time::{Duration, Instant},
};

use supports_color::{Stream, on};

#[cfg(unix)]
const ESC: &str = "\x1b";
const CSI: &str = "\x1b[";
const RESET: &str = "\x1b[0m";

const HEADER_SUFFIX: &str = " - The Unified Toolchain for the Web";

const RESET_FG: &str = "\x1b[39m";
const DEFAULT_BLUE: Rgb = Rgb(88, 146, 255);
const DEFAULT_MAGENTA: Rgb = Rgb(187, 116, 247);
const ANSI_BLUE_INDEX: u8 = 4;
const ANSI_MAGENTA_INDEX: u8 = 5;
const HEADER_SUFFIX_FADE_GAMMA: f64 = 1.35;

static HEADER_COLORS: OnceLock<HeaderColors> = OnceLock::new();

/// Whether the terminal is Warp, which does not respond to OSC color queries
/// and renders alternate screen content flush against block edges.
#[must_use]
pub fn is_warp_terminal() -> bool {
    static IS_WARP: LazyLock<bool> =
        LazyLock::new(|| std::env::var("TERM_PROGRAM").as_deref() == Ok("WarpTerminal"));
    *IS_WARP
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

struct HeaderColors {
    blue: Rgb,
    suffix_gradient: Vec<Rgb>,
}

fn bold(text: &str, enabled: bool) -> String {
    if enabled { format!("\x1b[1m{text}\x1b[22m") } else { text.to_string() }
}

fn fg_rgb(color: Rgb) -> String {
    format!("{CSI}38;2;{};{};{}m", color.0, color.1, color.2)
}

fn should_colorize() -> bool {
    let stdout = std::io::stdout();
    stdout.is_terminal() && on(Stream::Stdout).is_some()
}

fn supports_true_color() -> bool {
    let stdout = std::io::stdout();
    stdout.is_terminal() && on(Stream::Stdout).is_some_and(|color| color.has_16m)
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

fn gradient_three_stop(count: usize, start: Rgb, middle: Rgb, end: Rgb, gamma: f64) -> Vec<Rgb> {
    let n = count.max(1);
    let denom = (n - 1).max(1) as f64;

    (0..n)
        .map(|i| {
            let t = i as f64 / denom;
            if t <= 0.5 {
                let local_t = (t / 0.5).powf(gamma);
                Rgb(
                    lerp(start.0 as f64, middle.0 as f64, local_t).round() as u8,
                    lerp(start.1 as f64, middle.1 as f64, local_t).round() as u8,
                    lerp(start.2 as f64, middle.2 as f64, local_t).round() as u8,
                )
            } else {
                let local_t = ((t - 0.5) / 0.5).powf(gamma);
                Rgb(
                    lerp(middle.0 as f64, end.0 as f64, local_t).round() as u8,
                    lerp(middle.1 as f64, end.1 as f64, local_t).round() as u8,
                    lerp(middle.2 as f64, end.2 as f64, local_t).round() as u8,
                )
            }
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

#[cfg(unix)]
fn to_8bit(hex: &str) -> Option<u8> {
    match hex.len() {
        2 => u8::from_str_radix(hex, 16).ok(),
        4 => {
            let value = u16::from_str_radix(hex, 16).ok()?;
            Some((f64::from(value) / f64::from(u16::MAX) * 255.0).round() as u8)
        }
        len if len > 0 => {
            let value = u128::from_str_radix(hex, 16).ok()?;
            let max = (16_u128).pow(len as u32) - 1;
            Some(((value as f64 / max as f64) * 255.0).round() as u8)
        }
        _ => None,
    }
}

#[cfg(unix)]
fn parse_rgb_triplet(input: &str) -> Option<Rgb> {
    let mut parts = input.split('/');
    let r_hex = parts.next()?;
    let g_hex = parts.next()?;
    let b_raw = parts.next()?;
    let b_hex = b_raw.chars().take_while(|c| c.is_ascii_hexdigit()).collect::<String>();

    Some(Rgb(to_8bit(r_hex)?, to_8bit(g_hex)?, to_8bit(&b_hex)?))
}

#[cfg(unix)]
fn parse_osc10_rgb(buffer: &str) -> Option<Rgb> {
    let start = buffer.find("\x1b]10;")?;
    let tail = &buffer[start..];
    let rgb_start = tail.find("rgb:")?;
    parse_rgb_triplet(&tail[rgb_start + 4..])
}

#[cfg(unix)]
fn parse_osc4_rgb(buffer: &str, index: u8) -> Option<Rgb> {
    let prefix = format!("\x1b]4;{index};");
    let start = buffer.find(&prefix)?;
    let tail = &buffer[start + prefix.len()..];
    let rgb_start = tail.find("rgb:")?;
    parse_rgb_triplet(&tail[rgb_start + 4..])
}

#[cfg(unix)]
fn query_terminal_colors(palette_indices: &[u8]) -> (Option<Rgb>, Vec<(u8, Rgb)>) {
    use std::{
        fs::OpenOptions,
        os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd},
    };

    use nix::{
        poll::{PollFd, PollFlags, PollTimeout, poll},
        sys::termios::{SetArg, Termios, cfmakeraw, tcgetattr, tcsetattr},
        unistd::read,
    };

    if std::env::var_os("CI").is_some() {
        return (None, vec![]);
    }

    // Warp terminal does not respond to OSC color queries in its block-mode
    // renderer. Sending the queries causes the process to appear stuck until
    // the user presses a key (which is consumed as a fake "response").
    if is_warp_terminal() {
        return (None, vec![]);
    }

    // tmux does not reliably forward OSC color query responses back to the
    // child process, causing the same hang-until-keypress behavior as Warp.
    if std::env::var_os("TMUX").is_some() {
        return (None, vec![]);
    }

    let mut tty = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
        Ok(file) => file,
        Err(_) => return (None, vec![]),
    };

    if !std::io::stdout().is_terminal() {
        return (None, vec![]);
    }

    struct RawGuard {
        fd: RawFd,
        original: Termios,
    }

    impl Drop for RawGuard {
        fn drop(&mut self) {
            // SAFETY: `fd` comes from an open `/dev/tty` and the guard does not outlive that file.
            let borrowed = unsafe { BorrowedFd::borrow_raw(self.fd) };
            let _ = tcsetattr(borrowed, SetArg::TCSANOW, &self.original);
        }
    }

    let original = match tcgetattr(tty.as_fd()) {
        Ok(value) => value,
        Err(_) => return (None, vec![]),
    };
    let mut raw = original.clone();
    cfmakeraw(&mut raw);
    if tcsetattr(tty.as_fd(), SetArg::TCSANOW, &raw).is_err() {
        return (None, vec![]);
    }
    let _guard = RawGuard { fd: tty.as_raw_fd(), original };

    let mut query = format!("{ESC}]10;?{ESC}\\");
    for index in palette_indices {
        query.push_str(&format!("{ESC}]4;{index};?{ESC}\\"));
    }
    if tty.write_all(query.as_bytes()).is_err() {
        return (None, vec![]);
    }
    if tty.flush().is_err() {
        return (None, vec![]);
    }

    let deadline = Instant::now() + Duration::from_millis(100);
    let mut last_data = Instant::now();
    let mut buffer = String::new();
    let mut foreground = None;
    let mut palette_colors: Vec<(u8, Option<Rgb>)> =
        palette_indices.iter().copied().map(|index| (index, None)).collect();

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait = remaining.min(Duration::from_millis(10));

        let mut fds = [PollFd::new(tty.as_fd(), PollFlags::POLLIN)];
        let timeout = match PollTimeout::try_from(wait) {
            Ok(value) => value,
            Err(_) => break,
        };
        let ready = match poll(&mut fds, timeout) {
            Ok(value) => value,
            Err(_) => break,
        };
        if ready == 0 {
            if Instant::now().saturating_duration_since(last_data) >= Duration::from_millis(50) {
                buffer.clear();
            }
            continue;
        }

        let mut chunk = [0_u8; 256];
        let read_size = match read(tty.as_fd(), &mut chunk) {
            Ok(value) => value,
            Err(_) => break,
        };
        if read_size == 0 {
            continue;
        }

        last_data = Instant::now();
        buffer.push_str(&String::from_utf8_lossy(&chunk[..read_size]));
        if buffer.len() > 1024 {
            let keep_from = buffer.len() - 1024;
            buffer = buffer[keep_from..].to_string();
        }

        if foreground.is_none() {
            foreground = parse_osc10_rgb(&buffer);
        }
        for (index, color) in &mut palette_colors {
            if color.is_none() {
                *color = parse_osc4_rgb(&buffer, *index);
            }
        }

        if foreground.is_some() && palette_colors.iter().all(|(_, color)| color.is_some()) {
            break;
        }
    }

    let resolved = palette_colors
        .into_iter()
        .filter_map(|(index, color)| color.map(|rgb| (index, rgb)))
        .collect();
    (foreground, resolved)
}

#[cfg(not(unix))]
fn query_terminal_colors(_palette_indices: &[u8]) -> (Option<Rgb>, Vec<(u8, Rgb)>) {
    (None, vec![])
}

fn palette_color(palette: &[(u8, Rgb)], index: u8) -> Option<Rgb> {
    palette.iter().find_map(|(palette_index, color)| (*palette_index == index).then_some(*color))
}

fn get_header_colors() -> &'static HeaderColors {
    HEADER_COLORS.get_or_init(|| {
        let (foreground, palette) = query_terminal_colors(&[ANSI_BLUE_INDEX, ANSI_MAGENTA_INDEX]);
        let blue = palette_color(&palette, ANSI_BLUE_INDEX).unwrap_or(DEFAULT_BLUE);
        let magenta = palette_color(&palette, ANSI_MAGENTA_INDEX).unwrap_or(DEFAULT_MAGENTA);

        let suffix_gradient = match foreground {
            Some(color) => gradient_three_stop(
                HEADER_SUFFIX.chars().count(),
                blue,
                magenta,
                color,
                HEADER_SUFFIX_FADE_GAMMA,
            ),
            None => gradient_eased(
                HEADER_SUFFIX.chars().count(),
                blue,
                magenta,
                HEADER_SUFFIX_FADE_GAMMA,
            ),
        };

        HeaderColors { blue, suffix_gradient }
    })
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

/// Render the Vite+ CLI header string with JS-parity coloring behavior.
#[must_use]
pub fn vite_plus_header() -> String {
    if !should_colorize() || !supports_true_color() {
        return format!("VITE+{HEADER_SUFFIX}");
    }

    let header_colors = get_header_colors();
    render_header_variant(header_colors.blue, &header_colors.suffix_gradient, true, true)
}

#[cfg(all(test, unix))]
mod tests {
    use super::{Rgb, gradient_eased, parse_osc4_rgb, parse_osc10_rgb, to_8bit};

    #[test]
    fn to_8bit_matches_js_rules() {
        assert_eq!(to_8bit("ff"), Some(255));
        assert_eq!(to_8bit("7f"), Some(127));
        assert_eq!(to_8bit("ffff"), Some(255));
        assert_eq!(to_8bit("0000"), Some(0));
        assert_eq!(to_8bit("fff"), Some(255));
    }

    #[test]
    fn parse_osc10_response_extracts_rgb() {
        let response = "\x1b]10;rgb:aaaa/bbbb/cccc\x1b\\";
        assert_eq!(parse_osc10_rgb(response), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_osc4_response_extracts_rgb() {
        let response = "\x1b]4;5;rgb:aaaa/bbbb/cccc\x1b\\";
        assert_eq!(parse_osc4_rgb(response, 5), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn gradient_counts_match() {
        assert_eq!(gradient_eased(0, Rgb(0, 0, 0), Rgb(255, 255, 255), 1.0).len(), 1);
        assert_eq!(gradient_eased(5, Rgb(10, 20, 30), Rgb(40, 50, 60), 1.0).len(), 5);
    }
}
