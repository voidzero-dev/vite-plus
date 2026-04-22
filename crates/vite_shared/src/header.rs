//! Shared Vite+ header rendering.
//!
//! Header coloring behavior:
//! - Colorization and truecolor capability gates
//! - Foreground color OSC query (`ESC ] 10 ; ? BEL`) with timeout
//! - ANSI palette queries for blue/magenta with timeout
//! - DA1 sandwich technique to detect unsupported terminals
//! - Stream-based response parsing (modelled after `terminal-colorsaurus`)
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

/// Returns `true` if the terminal is known to not support OSC color queries
/// or if the environment is unreliable for escape-sequence round-trips.
///
/// Modelled after `terminal-colorsaurus`'s quirks detection, extended with
/// additional checks for Docker, CI, devcontainers, and other environments.
#[cfg(unix)]
fn is_osc_query_unsupported() -> bool {
    static UNSUPPORTED: OnceLock<bool> = OnceLock::new();
    *UNSUPPORTED.get_or_init(|| {
        if !std::io::stdout().is_terminal() || !std::io::stdin().is_terminal() {
            return true;
        }

        // CI environments have no real terminal emulator behind the PTY.
        if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
            return true;
        }

        // Warp terminal does not respond to OSC color queries in its
        // block-mode renderer, causing a hang until the user presses a key.
        if is_warp_terminal() {
            return true;
        }

        // Emacs terminal emulators (ansi-term, vterm, eshell) don't support
        // OSC queries.
        if std::env::var_os("INSIDE_EMACS").is_some() {
            return true;
        }

        // Docker containers and devcontainers may have a PTY with no real
        // terminal emulator, causing OSC responses to leak as visible text.
        if std::path::Path::new("/.dockerenv").exists()
            || std::env::var_os("REMOTE_CONTAINERS").is_some()
            || std::env::var_os("CODESPACES").is_some()
            || std::env::var_os("KUBERNETES_SERVICE_HOST").is_some()
        {
            return true;
        }

        match std::env::var("TERM") {
            // Missing or non-unicode TERM is highly suspect.
            Err(_) => return true,
            // `TERM=dumb` indicates a minimal terminal with no escape support.
            Ok(term) if term == "dumb" => return true,
            // GNU Screen responds to OSC queries in the wrong order, breaking
            // the DA1 sandwich technique. It also only supports OSC 11, not
            // OSC 10 or OSC 4.
            Ok(term) if term == "screen" || term.starts_with("screen.") => return true,
            // Eterm doesn't support DA1, so we skip to avoid the timeout.
            Ok(term) if term == "Eterm" => return true,
            _ => {}
        }

        // tmux and GNU Screen (via STY) do not reliably forward OSC color
        // query responses back to the child process.
        if std::env::var_os("TMUX").is_some() || std::env::var_os("STY").is_some() {
            return true;
        }

        false
    })
}

/// DA1 (Primary Device Attributes) query — supported by virtually all
/// terminals. Used as a sentinel in the "DA1 sandwich" technique:
/// we send our OSC queries followed by DA1, then read responses. If the
/// DA1 response (`ESC [ ? ...`) arrives first, the terminal doesn't
/// support OSC queries and we bail out immediately instead of waiting
/// for a timeout.
#[cfg(unix)]
const DA1: &str = "\x1b[c";

/// Reads from a `BufRead` until one of two delimiter bytes is found.
/// Modelled after `terminal-colorsaurus`'s `read_until2`.
#[cfg(unix)]
fn read_until_either(
    r: &mut impl std::io::BufRead,
    d1: u8,
    d2: u8,
    buf: &mut Vec<u8>,
) -> std::io::Result<usize> {
    let mut total = 0;
    loop {
        let available = match r.fill_buf() {
            Ok(b) => b,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if available.is_empty() {
            return Ok(total);
        }
        if let Some(i) = available.iter().position(|&b| b == d1 || b == d2) {
            buf.extend_from_slice(&available[..=i]);
            let used = i + 1;
            r.consume(used);
            total += used;
            return Ok(total);
        }
        let len = available.len();
        buf.extend_from_slice(available);
        r.consume(len);
        total += len;
    }
}

/// Queries terminal colors using the DA1 sandwich technique with
/// stream-based response parsing (modelled after `terminal-colorsaurus`).
///
/// Responses are read sequentially using `BufReader` + `read_until`,
/// which provides exact response boundaries and eliminates the
/// ordering/completeness ambiguities of flat-buffer pattern matching.
#[cfg(unix)]
fn query_terminal_colors(palette_indices: &[u8]) -> (Option<Rgb>, Vec<(u8, Rgb)>) {
    use std::{
        fs::OpenOptions,
        io::{self, BufRead, BufReader},
        os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd},
    };

    use nix::{
        poll::{PollFd, PollFlags, PollTimeout, poll},
        sys::termios::{SetArg, Termios, cfmakeraw, tcgetattr, tcsetattr},
    };

    if is_osc_query_unsupported() {
        return (None, vec![]);
    }

    let mut tty = match OpenOptions::new().read(true).write(true).open("/dev/tty") {
        Ok(file) => file,
        Err(_) => return (None, vec![]),
    };

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
    // `_guard` is declared after `tty` so it drops first (reverse declaration
    // order), restoring terminal mode while the fd is still open.
    let _guard = RawGuard { fd: tty.as_raw_fd(), original };

    // Build the query: OSC 10 (foreground) + OSC 4 (palette) + DA1 (sentinel).
    // BEL (\x07) is used as string terminator instead of ST (\x1b\\) because
    // urxvt has a bug where it terminates responses with bare ESC instead of
    // ST, causing a parse hang. BEL-terminated queries produce BEL-terminated
    // responses, avoiding this issue.
    let mut query = format!("{ESC}]10;?\x07");
    for index in palette_indices {
        query.push_str(&format!("{ESC}]4;{index};?\x07"));
    }
    query.push_str(DA1);

    if tty.write_all(query.as_bytes()).is_err() {
        return (None, vec![]);
    }
    if tty.flush().is_err() {
        return (None, vec![]);
    }

    // Use a longer timeout for SSH to account for round-trip latency.
    let timeout_ms =
        if std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some() {
            1000
        } else {
            200
        };
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    // Timeout-aware reader: polls for readability before each read,
    // returning `TimedOut` when the deadline expires. Wrapping in
    // `BufReader` gives us `read_until` and `fill_buf`/`buffer` for
    // delimiter-based parsing with peek-ahead.
    struct TtyReader<'a> {
        tty: &'a std::fs::File,
        deadline: Instant,
    }

    impl io::Read for TtyReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "tty read timed out"));
            }
            let mut fds = [PollFd::new(self.tty.as_fd(), PollFlags::POLLIN)];
            let timeout = PollTimeout::try_from(remaining)
                .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "tty read timed out"))?;
            let ready = poll(&mut fds, timeout).map_err(io::Error::from)?;
            if ready == 0 {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "tty read timed out"));
            }
            io::Read::read(&mut &*self.tty, buf)
        }
    }

    let tty_reader = TtyReader { tty: &tty, deadline };
    let mut reader = BufReader::with_capacity(64, tty_reader);

    const ESC_BYTE: u8 = 0x1b;
    const BEL_BYTE: u8 = 0x07;

    // Read a single OSC response from the stream. Returns:
    //   Ok(bytes)  — an OSC response (ESC ] ... BEL/ST)
    //   Err(true)  — DA1 response arrived (terminal doesn't support this query)
    //   Err(false) — timeout or I/O error
    //
    // This mirrors `terminal-colorsaurus`'s `read_color_response`: read until
    // ESC, peek at the next byte to distinguish OSC (']') from DA1 ('['),
    // then read until the response terminator.
    let read_osc_response = |r: &mut BufReader<TtyReader>| -> Result<Vec<u8>, bool> {
        let mut buf = Vec::new();

        // Read until ESC — start of next response.
        r.read_until(ESC_BYTE, &mut buf).map_err(|_| false)?;

        // Peek at the next byte in BufReader's internal buffer.
        // ']' = OSC response, '[' = DA1/CSI response.
        let next = match r.fill_buf() {
            Ok(b) if !b.is_empty() => b[0],
            _ => return Err(false),
        };

        if next != b']' {
            // DA1 response (ESC [ ? ... c). Consume it so it doesn't leak.
            let mut discard = Vec::new();
            let _ = r.read_until(b'[', &mut discard);
            let _ = r.read_until(b'c', &mut discard);
            return Err(true);
        }

        // OSC response — read until BEL or ESC (for ST termination).
        read_until_either(r, BEL_BYTE, ESC_BYTE, &mut buf).map_err(|_| false)?;
        if buf.last() == Some(&ESC_BYTE) {
            // ST-terminated: ESC followed by '\'.
            r.read_until(b'\\', &mut buf).map_err(|_| false)?;
        }

        Ok(buf)
    };

    // Read foreground color (OSC 10 response).
    let foreground = match read_osc_response(&mut reader) {
        Ok(data) => {
            let s = String::from_utf8_lossy(&data);
            parse_osc10_rgb(&s)
        }
        Err(true) => return (None, vec![]), // DA1 first → unsupported
        Err(false) => return (None, vec![]), // timeout/error
    };

    // Read palette colors (OSC 4 responses).
    let mut palette_results = Vec::new();
    let mut da1_consumed = false;
    for &index in palette_indices {
        match read_osc_response(&mut reader) {
            Ok(data) => {
                let s = String::from_utf8_lossy(&data);
                if let Some(rgb) = parse_osc4_rgb(&s, index) {
                    palette_results.push((index, rgb));
                }
            }
            Err(is_da1) => {
                da1_consumed = is_da1;
                break;
            }
        }
    }

    // Drain the trailing DA1 response (ESC [ ? ... c) so it doesn't leak.
    // Skip if the DA1 was already consumed inside read_osc_response.
    if !da1_consumed {
        let mut discard = Vec::new();
        let _ = reader.read_until(ESC_BYTE, &mut discard);
        let _ = reader.read_until(b'[', &mut discard);
        let _ = reader.read_until(b'c', &mut discard);
    }

    (foreground, palette_results)
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

#[cfg(all(test, unix))]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::{
        Rgb, gradient_eased, parse_osc4_rgb, parse_osc10_rgb, parse_rgb_triplet,
        query_terminal_colors, read_until_either, to_8bit,
    };

    #[test]
    fn to_8bit_matches_js_rules() {
        assert_eq!(to_8bit("ff"), Some(255));
        assert_eq!(to_8bit("7f"), Some(127));
        assert_eq!(to_8bit("ffff"), Some(255));
        assert_eq!(to_8bit("0000"), Some(0));
        assert_eq!(to_8bit("fff"), Some(255));
    }

    #[test]
    fn to_8bit_single_digit() {
        assert_eq!(to_8bit("f"), Some(255));
        assert_eq!(to_8bit("0"), Some(0));
        assert_eq!(to_8bit("a"), Some(170));
    }

    #[test]
    fn to_8bit_three_digit() {
        assert_eq!(to_8bit("fff"), Some(255));
        assert_eq!(to_8bit("000"), Some(0));
        assert_eq!(to_8bit("800"), Some(128));
    }

    #[test]
    fn to_8bit_empty_returns_none() {
        assert_eq!(to_8bit(""), None);
    }

    #[test]
    fn to_8bit_invalid_hex_returns_none() {
        assert_eq!(to_8bit("zz"), None);
        assert_eq!(to_8bit("gg"), None);
    }

    #[test]
    fn parse_rgb_triplet_standard() {
        assert_eq!(parse_rgb_triplet("ff/ff/ff"), Some(Rgb(255, 255, 255)));
        assert_eq!(parse_rgb_triplet("00/00/00"), Some(Rgb(0, 0, 0)));
    }

    #[test]
    fn parse_rgb_triplet_four_digit_channels() {
        assert_eq!(parse_rgb_triplet("ffff/ffff/ffff"), Some(Rgb(255, 255, 255)));
        assert_eq!(parse_rgb_triplet("0000/0000/0000"), Some(Rgb(0, 0, 0)));
        assert_eq!(parse_rgb_triplet("aaaa/bbbb/cccc"), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_rgb_triplet_mixed_digit_channels() {
        // Single digit channels
        assert_eq!(parse_rgb_triplet("f/e/d"), Some(Rgb(255, 238, 221)));
    }

    #[test]
    fn parse_rgb_triplet_trailing_junk_ignored() {
        // The parser stops at non-hex chars for the blue channel
        assert_eq!(parse_rgb_triplet("ff/ff/ff\x1b\\"), Some(Rgb(255, 255, 255)));
    }

    #[test]
    fn parse_rgb_triplet_missing_channel_returns_none() {
        assert_eq!(parse_rgb_triplet("ff/ff"), None);
        assert_eq!(parse_rgb_triplet("ff"), None);
    }

    #[test]
    fn parse_osc10_response_extracts_rgb() {
        let response = "\x1b]10;rgb:aaaa/bbbb/cccc\x1b\\";
        assert_eq!(parse_osc10_rgb(response), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_osc10_bel_terminated() {
        let response = "\x1b]10;rgb:aaaa/bbbb/cccc\x07";
        assert_eq!(parse_osc10_rgb(response), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_osc10_no_match_returns_none() {
        assert_eq!(parse_osc10_rgb("garbage"), None);
        assert_eq!(parse_osc10_rgb(""), None);
    }

    #[test]
    fn parse_osc4_response_extracts_rgb() {
        let response = "\x1b]4;5;rgb:aaaa/bbbb/cccc\x1b\\";
        assert_eq!(parse_osc4_rgb(response, 5), Some(Rgb(170, 187, 204)));
    }

    #[test]
    fn parse_osc4_bel_terminated() {
        let response = "\x1b]4;4;rgb:5858/9292/ffff\x07";
        assert_eq!(parse_osc4_rgb(response, 4), Some(Rgb(88, 146, 255)));
    }

    #[test]
    fn parse_osc4_wrong_index_returns_none() {
        let response = "\x1b]4;5;rgb:aaaa/bbbb/cccc\x1b\\";
        assert_eq!(parse_osc4_rgb(response, 4), None);
    }

    #[test]
    fn parse_osc4_no_match_returns_none() {
        assert_eq!(parse_osc4_rgb("garbage", 5), None);
        assert_eq!(parse_osc4_rgb("", 0), None);
    }

    #[test]
    fn parse_osc_multiple_responses_in_buffer() {
        // Simulates a buffer containing OSC 10 + OSC 4;4 + OSC 4;5 responses
        let buffer = "\x1b]10;rgb:d0d0/d0d0/d0d0\x07\
                       \x1b]4;4;rgb:5858/9292/ffff\x07\
                       \x1b]4;5;rgb:bbbb/7474/f7f7\x07";
        assert_eq!(parse_osc10_rgb(buffer), Some(Rgb(208, 208, 208)));
        assert_eq!(parse_osc4_rgb(buffer, 4), Some(Rgb(88, 146, 255)));
        assert_eq!(parse_osc4_rgb(buffer, 5), Some(Rgb(187, 116, 247)));
    }

    #[test]
    fn parse_osc_buffer_with_da1_response() {
        // DA1 response mixed in — OSC parsers should still find their data
        let buffer = "\x1b]10;rgb:d0d0/d0d0/d0d0\x07\x1b[?64;1;2;4c";
        assert_eq!(parse_osc10_rgb(buffer), Some(Rgb(208, 208, 208)));
    }

    #[test]
    fn gradient_counts_match() {
        assert_eq!(gradient_eased(0, Rgb(0, 0, 0), Rgb(255, 255, 255), 1.0).len(), 1);
        assert_eq!(gradient_eased(5, Rgb(10, 20, 30), Rgb(40, 50, 60), 1.0).len(), 5);
    }

    /// Regression test ported from terminal-colorsaurus (issue #38).
    /// In CI there is no real terminal, so `query_terminal_colors` must
    /// return `(None, vec![])` without hanging.
    #[test]
    fn query_terminal_colors_does_not_hang() {
        let (fg, palette) = query_terminal_colors(&[4, 5]);
        // In CI, the environment pre-screening or DA1 sandwich will cause an
        // early return. We don't assert specific values — just that it
        // completes promptly and doesn't panic.
        let _ = (fg, palette);
    }

    #[test]
    fn read_until_either_stops_at_first_delimiter() {
        let data = b"hello\x07world";
        let mut reader = BufReader::new(Cursor::new(data.as_slice()));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 6); // "hello" + BEL
        assert_eq!(&buf, b"hello\x07");
    }

    #[test]
    fn read_until_either_stops_at_second_delimiter() {
        let data = b"hello\x1bworld";
        let mut reader = BufReader::new(Cursor::new(data.as_slice()));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 6); // "hello" + ESC
        assert_eq!(&buf, b"hello\x1b");
    }

    #[test]
    fn read_until_either_no_delimiter_reads_all() {
        let data = b"hello world";
        let mut reader = BufReader::new(Cursor::new(data.as_slice()));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(&buf, b"hello world");
    }

    #[test]
    fn read_until_either_empty_input() {
        let data: &[u8] = b"";
        let mut reader = BufReader::new(Cursor::new(data));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn read_until_either_delimiter_at_start() {
        let data = b"\x07rest";
        let mut reader = BufReader::new(Cursor::new(data.as_slice()));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(&buf, b"\x07");
    }

    #[test]
    fn read_until_either_multi_chunk() {
        // Use a tiny BufReader capacity to force multiple fill_buf calls.
        let data = b"abcdefgh\x07rest";
        let mut reader = BufReader::with_capacity(3, Cursor::new(data.as_slice()));
        let mut buf = Vec::new();
        let n = read_until_either(&mut reader, 0x07, 0x1b, &mut buf).unwrap();
        assert_eq!(n, 9); // "abcdefgh" + BEL
        assert_eq!(&buf, b"abcdefgh\x07");
    }
}
