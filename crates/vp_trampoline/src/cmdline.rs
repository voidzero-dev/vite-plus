//! Portable helpers for UTF-16 code units and bytes.
//! The Windows implementation uses these helpers.
//! They stay outside win.rs so their unit tests run on all platforms.

const SPACE: u16 = b' ' as u16;
const TAB: u16 = b'\t' as u16;
const QUOTE: u16 = b'"' as u16;
const DOT: u16 = b'.' as u16;
const COLON: u16 = b':' as u16;
const QUESTION: u16 = b'?' as u16;
const BACKSLASH: u16 = b'\\' as u16;
const FORWARD_SLASH: u16 = b'/' as u16;
const U: u16 = b'U' as u16;
const N: u16 = b'N' as u16;
const C: u16 = b'C' as u16;

const VERBATIM_PREFIX: &[u16] = &[BACKSLASH, BACKSLASH, QUESTION, BACKSLASH];
const UNC_PREFIX: &[u16] = &[BACKSLASH, BACKSLASH, QUESTION, BACKSLASH, U, N, C, BACKSLASH];

/// Must match `vp_shared::SHIM_POINTER_HEADER`.
pub const SHIM_POINTER_HEADER: &str = "vite-plus-shim-v1";

#[derive(Debug, PartialEq, Eq)]
pub enum ShimLayout<'a> {
    SingleRoot,
    Split { cache: &'a str },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ShimPointer<'a> {
    pub data: &'a str,
    pub layout: ShimLayout<'a>,
}

/// Parse the UTF-8 `<name>.shim` sidecar written by `vp_shared::VpDirs`.
///
/// A sidecar records the directory layout, data root, and cache root.
/// The parser requires the versioned header.
/// The parser supports a UTF-8 BOM and CRLF line endings, as `vp_shared` does.
pub fn parse_shim_pointer(bytes: &[u8]) -> Option<ShimPointer<'_>> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = core::str::from_utf8(bytes).ok()?.trim();
    let mut lines = text.lines();
    if lines.next()? != SHIM_POINTER_HEADER {
        return None;
    }

    let mut layout = None;
    let mut data = None;
    let mut cache = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("layout=") {
            layout = Some(value);
        } else if let Some(value) = line.strip_prefix("data=") {
            data = (!value.is_empty()).then_some(value);
        } else if let Some(value) = line.strip_prefix("cache=") {
            cache = (!value.is_empty()).then_some(value);
        }
    }

    let data = data?;
    let layout = match layout? {
        "single-root" => ShimLayout::SingleRoot,
        "split" => ShimLayout::Split { cache: cache? },
        _ => return None,
    };
    Some(ShimPointer { data, layout })
}

/// Return the end index of the first program argument in a raw command line.
///
/// This function follows the MSVC rule for a program name.
/// A quote starts or stops quoted mode.
/// Backslashes do not escape characters.
/// Leading whitespace ends an empty program argument.
/// Forward `&cmdline[result..]` to the child without changes.
/// This remaining text includes its leading whitespace.
pub fn skip_program_argument(cmdline: &[u16]) -> usize {
    let mut i = 0;
    let mut quoted = false;
    while i < cmdline.len() {
        let c = cmdline[i];
        if c == QUOTE {
            quoted = !quoted;
        } else if (c == SPACE || c == TAB) && !quoted {
            break;
        }
        i += 1;
    }
    i
}

fn is_path_separator(unit: u16) -> bool {
    unit == BACKSLASH || unit == FORWARD_SLASH
}

/// Add the Win32 extended-length prefix to a normalized absolute path.
///
/// Before the call, resolve `.` and `..` components.
/// Before the call, replace `/` separators.
/// The extended-length namespace uses the remaining path without changes.
pub fn verbatim_path(path: &[u16]) -> Vec<u16> {
    let (prefix, tail) = match path {
        // Keep an existing extended-length or NT namespace.
        [BACKSLASH, BACKSLASH, QUESTION, BACKSLASH, ..]
        | [BACKSLASH, QUESTION, QUESTION, BACKSLASH, ..] => return path.to_vec(),
        // C:\path => \\?\C:\path
        [_, COLON, BACKSLASH, ..] => (VERBATIM_PREFIX, path),
        // \\.\device => \\?\device
        [BACKSLASH, BACKSLASH, DOT, BACKSLASH, tail @ ..] => (VERBATIM_PREFIX, tail),
        // \\server\share => \\?\UNC\server\share
        [BACKSLASH, BACKSLASH, tail @ ..] => (UNC_PREFIX, tail),
        _ => return path.to_vec(),
    };

    let mut extended = Vec::with_capacity(prefix.len() + tail.len());
    extended.extend_from_slice(prefix);
    extended.extend_from_slice(tail);
    extended
}

/// Return the end index of a parent directory.
/// Keep the separator when it is part of a Windows root.
///
/// Without the separator, `C:\vp.exe` produces the drive-relative path `C:`.
/// Device roots such as `\\?\Volume{...}\vp.exe` have the same constraint.
/// Other parent paths omit the final separator, as `Path::parent` does.
pub fn parent_dir_len(path: &[u16], last_separator: usize) -> usize {
    let drive_root = last_separator >= 1 && path[last_separator - 1] == COLON;
    let device_root = last_separator >= 4
        && is_path_separator(path[0])
        && is_path_separator(path[1])
        && (path[2] == QUESTION || path[2] == DOT)
        && is_path_separator(path[3])
        && !path[4..last_separator].iter().any(|&unit| is_path_separator(unit));

    if last_separator == 0 || drive_root || device_root {
        last_separator + 1
    } else {
        last_separator
    }
}

/// Return the file-stem length, as `Path::file_stem` does.
/// The stem ends before the last `.`, but a leading `.` does not start an extension.
pub fn file_stem_len(name: &[u16]) -> usize {
    match name.iter().skip(1).rposition(|&c| c == DOT) {
        Some(pos) => pos + 1,
        None => name.len(),
    }
}

/// Case-sensitive comparison of a UTF-16 slice against an ASCII string.
pub fn eq_ascii(wide: &[u16], ascii: &[u8]) -> bool {
    wide.len() == ascii.len() && wide.iter().zip(ascii).all(|(&w, &a)| w == u16::from(a))
}

/// Format `value` as decimal ASCII in `buf`.
/// Return the used suffix.
pub fn format_u32(mut value: u32, buf: &mut [u8; 10]) -> &[u8] {
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    &buf[i..]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn skips_unquoted_program() {
        let cl = wide(r"C:\bin\node.exe --version");
        assert_eq!(&cl[skip_program_argument(&cl)..], &wide(" --version")[..]);
    }

    #[test]
    fn skips_quoted_program_with_spaces() {
        let cl = wide(r#""C:\Program Files\node.exe" -e "1 + 1""#);
        assert_eq!(&cl[skip_program_argument(&cl)..], &wide(r#" -e "1 + 1""#)[..]);
    }

    #[test]
    fn treats_leading_whitespace_as_an_empty_program() {
        for cl in [wide("  script.js --flag"), wide("\tscript.js --flag")] {
            assert_eq!(skip_program_argument(&cl), 0);
        }
    }

    #[test]
    fn skips_bare_program() {
        let cl = wide("node");
        assert_eq!(skip_program_argument(&cl), cl.len());
        assert_eq!(skip_program_argument(&[]), 0);
    }

    #[test]
    fn keeps_argument_tail_verbatim() {
        let cl = wide(r#"npx "a  b\" literal" --flag"#);
        assert_eq!(&cl[skip_program_argument(&cl)..], &wide(r#" "a  b\" literal" --flag"#)[..]);
    }

    #[test]
    fn parent_directory_preserves_windows_roots() {
        fn parent(path: &str) -> Vec<u16> {
            let path = wide(path);
            let last_separator = path.iter().rposition(|&unit| is_path_separator(unit)).unwrap();
            path[..parent_dir_len(&path, last_separator)].to_vec()
        }

        assert_eq!(parent(r"C:\vp.exe"), wide("C:\\"));
        assert_eq!(parent(r"C:\bin\vp.exe"), wide(r"C:\bin"));
        assert_eq!(parent(r"\\?\C:\vp.exe"), wide("\\\\?\\C:\\"));
        assert_eq!(
            parent(r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\vp.exe"),
            wide("\\\\?\\Volume{01234567-89ab-cdef-0123-456789abcdef}\\")
        );
        assert_eq!(parent(r"\\?\UNC\server\share\vp.exe"), wide(r"\\?\UNC\server\share"));
        assert_eq!(parent(r"\\server\share\vp.exe"), wide(r"\\server\share"));
        assert_eq!(parent(r"\vp.exe"), wide("\\"));
    }

    #[test]
    fn prefixes_normalized_absolute_paths_for_win32() {
        assert_eq!(verbatim_path(&wide(r"C:\data\vp.exe")), wide(r"\\?\C:\data\vp.exe"));
        assert_eq!(
            verbatim_path(&wide(r"\\server\share\vp.exe")),
            wide(r"\\?\UNC\server\share\vp.exe")
        );
        assert_eq!(
            verbatim_path(&wide(r"\\.\Volume{123}\vp.exe")),
            wide(r"\\?\Volume{123}\vp.exe")
        );
    }

    #[test]
    fn keeps_existing_namespaces_and_relative_paths() {
        for path in [r"\\?\C:\data\vp.exe", r"\??\C:\data\vp.exe", r"data\vp.exe"] {
            assert_eq!(verbatim_path(&wide(path)), wide(path));
        }
    }

    #[test]
    fn file_stem_matches_path_file_stem() {
        assert_eq!(file_stem_len(&wide("node.exe")), 4);
        assert_eq!(file_stem_len(&wide("node")), 4);
        assert_eq!(file_stem_len(&wide("NODE.EXE")), 4);
        assert_eq!(file_stem_len(&wide("a.b.exe")), 3);
        assert_eq!(file_stem_len(&wide(".hidden")), 7);
        assert_eq!(file_stem_len(&wide("node.")), 4);
    }

    #[test]
    fn eq_ascii_is_exact() {
        assert!(eq_ascii(&wide("vp"), b"vp"));
        assert!(!eq_ascii(&wide("VP"), b"vp"));
        assert!(!eq_ascii(&wide("vpx"), b"vp"));
    }

    #[test]
    fn formats_decimal() {
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(0, &mut buf), b"0");
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(203, &mut buf), b"203");
        let mut buf = [0u8; 10];
        assert_eq!(format_u32(u32::MAX, &mut buf), b"4294967295");
    }

    #[test]
    fn parses_versioned_shim_pointers() {
        assert_eq!(
            parse_shim_pointer(
                b"vite-plus-shim-v1\nlayout=single-root\ndata=C:\\vp\ncache=C:\\cache\n"
            ),
            Some(ShimPointer { data: r"C:\vp", layout: ShimLayout::SingleRoot })
        );
        assert_eq!(
            parse_shim_pointer(
                b"\xEF\xBB\xBFvite-plus-shim-v1\r\nlayout=split\r\ndata=D:\\data\r\ncache=C:\\cache\r\n",
            ),
            Some(ShimPointer {
                data: r"D:\data",
                layout: ShimLayout::Split { cache: r"C:\cache" },
            })
        );
    }

    #[test]
    fn rejects_invalid_shim_pointers() {
        assert_eq!(parse_shim_pointer(b""), None);
        assert_eq!(parse_shim_pointer(b"\xff"), None);
        assert_eq!(parse_shim_pointer(b" C:\\vite-plus\\data\r\n"), None);
        assert_eq!(parse_shim_pointer(b"vite-plus-shim-v1\nlayout=split\ndata=C:\\data\n"), None);
        assert_eq!(
            parse_shim_pointer(
                b"vite-plus-shim-v1\nlayout=unknown\ndata=C:\\data\ncache=C:\\cache\n",
            ),
            None
        );
    }
}
