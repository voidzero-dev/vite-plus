//! Minimal Windows trampoline for vite-plus shims.
//!
//! Vite+ copies and renames this binary for each shim tool, such as `node.exe`
//! and `npm.exe`. The trampoline gets the tool name from its filename. It then
//! starts `vp.exe` with the `VP_SHIM_TOOL` environment variable. This variable
//! puts `vp.exe` in shim dispatch mode.
//!
//! The trampoline ignores Ctrl+C because the child process handles it. This
//! prevents the termination prompt that `.cmd` wrappers produce.
//!
//! **Size optimization:** `core::fmt` adds approximately 100 KB. This binary
//! does not use `format!`, `eprintln!`, `println!`, or `.unwrap()`. Each error
//! path calls `process::exit(1)` directly.
//!
//! See: <https://github.com/voidzero-dev/vite-plus/issues/835>

use std::{
    env,
    process::{self, Command, ExitStatus},
};

/// Preserve Unix signal termination using the shell's `128 + signal` convention.
fn exit_code_from_status(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    status.code().unwrap_or(1)
}

/// Must match [`vp_shared::SHIM_POINTER_EXTENSION`]. Keep a local copy so this
/// binary has no dependency on `vp_shared`. Each trampoline reads
/// `<name>.shim` next to itself. For example, `node.exe` reads `node.shim`.
const SHIM_POINTER_EXTENSION: &str = "shim";
/// Must match [`vp_shared::SHIM_POINTER_HEADER`].
const SHIM_POINTER_HEADER: &str = "vite-plus-shim-v1";

enum ShimLayout {
    SingleRoot,
    Split {
        cache: std::path::PathBuf,
    },
    /// One-line sidecars from earlier PR previews did not record provenance.
    Legacy,
}

struct ShimPointer {
    data: std::path::PathBuf,
    layout: ShimLayout,
}

struct VpLocation {
    exe: std::path::PathBuf,
    pointer: ShimPointer,
}

/// How the child `vp.exe` should resolve category roots.
enum ChildDirPins<'a> {
    /// `VP_HOME` or a grandfathered install explicitly selected one root.
    SingleRoot,
    /// The versioned sidecar explicitly selected split roots.
    Split { cache: &'a std::path::Path },
    /// A legacy sidecar and independent roots. Old sidecars did not record
    /// cache, so use the Windows platform fallback.
    LegacySplit { cache: std::path::PathBuf },
}

fn legacy_split_cache(data: &std::path::Path) -> std::path::PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("vite-plus").join("cache"))
        .unwrap_or_else(|| data.join("cache"))
}

fn child_dir_pins<'a>(bin_dir: &std::path::Path, pointer: &'a ShimPointer) -> ChildDirPins<'a> {
    match &pointer.layout {
        ShimLayout::SingleRoot => ChildDirPins::SingleRoot,
        ShimLayout::Split { cache } => ChildDirPins::Split { cache },
        ShimLayout::Legacy if bin_dir == pointer.data.join("bin").as_path() => {
            // Compatibility with one-line sidecars from earlier PR previews.
            // Their path shape is ambiguous, so preserve their old behavior.
            ChildDirPins::SingleRoot
        }
        ShimLayout::Legacy => {
            ChildDirPins::LegacySplit { cache: legacy_split_cache(&pointer.data) }
        }
    }
}

/// Locate `vp.exe` from `<BIN>/<name>.shim`.
///
/// `EnvConfig` in the child `vp.exe` owns the directory variables. This binary
/// must not read `VP_HOME` or `VP_*_DIR`. Installation and `vp env setup` write
/// a sidecar for each trampoline copy. Thus, this function does not check
/// sibling layout paths.
fn resolve_vp_exe(exe_path: &std::path::Path) -> Option<VpLocation> {
    let pointer = read_shim_pointer(exe_path)?;
    let exe = pointer.data.join("current").join("bin").join("vp.exe");
    exe.exists().then_some(VpLocation { exe, pointer })
}

fn read_shim_pointer(exe_path: &std::path::Path) -> Option<ShimPointer> {
    let bytes = std::fs::read(exe_path.with_extension(SHIM_POINTER_EXTENSION)).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes.as_slice());
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.is_empty() {
        return None;
    }
    let mut lines = text.lines();
    if lines.next()? != SHIM_POINTER_HEADER {
        return Some(ShimPointer {
            data: std::path::PathBuf::from(text),
            layout: ShimLayout::Legacy,
        });
    }

    let mut layout = None;
    let mut data = None;
    let mut cache = None;
    for line in lines {
        if let Some(value) = line.strip_prefix("layout=") {
            layout = Some(value);
        } else if let Some(value) = line.strip_prefix("data=") {
            data = (!value.is_empty()).then(|| std::path::PathBuf::from(value));
        } else if let Some(value) = line.strip_prefix("cache=") {
            cache = (!value.is_empty()).then(|| std::path::PathBuf::from(value));
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

fn main() {
    // 1. Determine tool name from our own executable filename
    let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
    let tool_name =
        exe_path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| process::exit(1));

    // 2. Locate vp.exe via `<name>.shim` (written next to every trampoline).
    let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
    let Some(location) = resolve_vp_exe(&exe_path) else {
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = handle.write_all(b"vite-plus: could not locate vp.exe through .shim\n");
        process::exit(1);
    };

    // 3. Install a Ctrl+C handler that ignores the signal. The child handles
    //    the signal. This prevents the termination prompt from cmd.exe.
    #[cfg(windows)]
    install_ctrl_handler();

    // 4. Spawn vp.exe
    //    - Single root: set VP_HOME.
    //    - Split: clear VP_HOME and pin VP_DATA_DIR / VP_BIN_DIR / VP_CACHE_DIR.
    //    - If tool is "vp", run in normal CLI mode (no VP_SHIM_TOOL)
    //    - Otherwise, set VP_SHIM_TOOL so vp.exe enters shim dispatch
    let mut cmd = Command::new(&location.exe);
    cmd.args(env::args_os().skip(1));
    match child_dir_pins(bin_dir, &location.pointer) {
        ChildDirPins::SingleRoot => {
            cmd.env("VP_HOME", &location.pointer.data);
        }
        ChildDirPins::Split { cache } => {
            cmd.env_remove("VP_HOME");
            cmd.env("VP_DATA_DIR", &location.pointer.data);
            cmd.env("VP_BIN_DIR", bin_dir);
            cmd.env("VP_CACHE_DIR", cache);
        }
        ChildDirPins::LegacySplit { cache } => {
            cmd.env_remove("VP_HOME");
            cmd.env("VP_DATA_DIR", &location.pointer.data);
            cmd.env("VP_BIN_DIR", bin_dir);
            cmd.env("VP_CACHE_DIR", cache);
        }
    }

    if tool_name != "vp" {
        cmd.env("VP_SHIM_TOOL", tool_name);
        // Clear the recursion marker before a nested shim call, such as npm
        // starting node. The nested shim must resolve the version again instead
        // of using passthrough mode. Old .cmd wrappers used `vp env exec`, which
        // cleared this marker in exec.rs. The trampoline does not use that path.
        // Must match vp_shared::env_vars::VP_TOOL_RECURSION
        cmd.env_remove("VP_TOOL_RECURSION");
    }

    // 5. Execute and propagate exit code.
    //    Use write_all instead of eprintln!/format! to avoid pulling in core::fmt (~100KB).
    match cmd.status() {
        Ok(status) => process::exit(exit_code_from_status(status)),
        Err(_) => {
            use std::io::Write;
            let stderr = std::io::stderr();
            let mut handle = stderr.lock();
            let _ = handle.write_all(b"vite-plus: could not execute ");
            let _ = handle.write_all(location.exe.as_os_str().as_encoded_bytes());
            let _ = handle.write_all(b"\n");
            process::exit(1);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn preserves_signal_exit_code() {
        let status = Command::new("/bin/sh").arg("-c").arg("kill -ILL $$").status().unwrap();
        assert_eq!(exit_code_from_status(status), 132);
    }
}

#[cfg(test)]
mod resolve_tests {
    use std::{fs, path::Path};

    use super::*;

    fn write_exe(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    fn versioned_pointer(layout: &str, data: &Path, cache: &Path) -> String {
        format!(
            "{SHIM_POINTER_HEADER}\nlayout={layout}\ndata={}\ncache={}\n",
            data.display(),
            cache.display()
        )
    }

    #[test]
    fn missing_pointer_does_not_probe_sibling_layout() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-no-ptr-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        write_exe(&root.join("current").join("bin").join("vp.exe"));
        write_exe(&root.join("data").join("current").join("bin").join("vp.exe"));

        assert!(resolve_vp_exe(&root.join("bin").join("vp.exe")).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pointer_without_payload_is_none() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-empty-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("bin");
        let data = root.join("data-root");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(bin.join("vp.shim"), format!("{}\n", data.display())).unwrap();

        assert!(resolve_vp_exe(&bin.join("vp.exe")).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pointer_file_locates_data_root() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-ptr-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("custom-bin");
        let data = root.join("custom-data");
        fs::create_dir_all(&bin).unwrap();
        write_exe(&data.join("current").join("bin").join("vp.exe"));
        write_exe(&root.join("data").join("current").join("bin").join("vp.exe"));
        fs::write(bin.join("vp.shim"), format!("{}\n", data.display())).unwrap();

        let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
        assert_eq!(location.exe, data.join("current").join("bin").join("vp.exe"));
        assert_eq!(location.pointer.data, data);
        assert!(matches!(location.pointer.layout, ShimLayout::Legacy));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pointer_file_is_per_exe_name() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-per-exe-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("bin");
        let node_data = root.join("node-data");
        let decoy_data = root.join("decoy-data");
        fs::create_dir_all(&bin).unwrap();
        write_exe(&node_data.join("current").join("bin").join("vp.exe"));
        write_exe(&decoy_data.join("current").join("bin").join("vp.exe"));
        fs::write(bin.join("vp.shim"), format!("{}\n", decoy_data.display())).unwrap();
        fs::write(bin.join("node.shim"), format!("{}\n", node_data.display())).unwrap();

        let location = resolve_vp_exe(&bin.join("node.exe")).unwrap();
        assert_eq!(location.exe, node_data.join("current").join("bin").join("vp.exe"));
        assert_eq!(location.pointer.data, node_data);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn pointer_file_ignores_utf8_bom_and_crlf() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-bom-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("bin");
        let data = root.join("data-root");
        fs::create_dir_all(&bin).unwrap();
        write_exe(&data.join("current").join("bin").join("vp.exe"));
        let mut contents = vec![0xEF, 0xBB, 0xBF];
        contents.extend_from_slice(
            versioned_pointer("split", &data, &root.join("cache")).replace('\n', "\r\n").as_bytes(),
        );
        fs::write(bin.join("vp.shim"), contents).unwrap();

        let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
        assert_eq!(location.exe, data.join("current").join("bin").join("vp.exe"));
        assert_eq!(location.pointer.data, data);
        assert!(matches!(location.pointer.layout, ShimLayout::Split { .. }));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_split_does_not_become_single_root_when_bin_is_under_data() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-split-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let data = root.join("data");
        let bin = data.join("bin");
        let cache = root.join("platform-cache");
        write_exe(&data.join("current").join("bin").join("vp.exe"));
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("vp.shim"), versioned_pointer("split", &data, &cache)).unwrap();

        let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
        assert!(matches!(
            child_dir_pins(&bin, &location.pointer),
            ChildDirPins::Split { cache: value } if value == cache
        ));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn explicit_single_root_sets_vp_home() {
        let root =
            std::env::temp_dir().join(format!("vp-trampoline-single-root-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("bin");
        write_exe(&root.join("current").join("bin").join("vp.exe"));
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            bin.join("vp.shim"),
            versioned_pointer("single-root", &root, &root.join("cache")),
        )
        .unwrap();

        let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
        assert!(matches!(child_dir_pins(&bin, &location.pointer), ChildDirPins::SingleRoot));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_pointer_preserves_path_based_behavior() {
        let data = std::path::PathBuf::from("/data/root");
        let bin = std::path::PathBuf::from("/other/bin");
        let pointer = ShimPointer { data: data.clone(), layout: ShimLayout::Legacy };
        assert!(matches!(child_dir_pins(&data.join("bin"), &pointer), ChildDirPins::SingleRoot));
        assert!(matches!(child_dir_pins(&bin, &pointer), ChildDirPins::LegacySplit { .. }));
    }
}

/// Install a console control handler that ignores Ctrl+C, Ctrl+Break, etc.
///
/// When Ctrl+C is pressed, Windows sends the event to all processes in the
/// console group. By returning TRUE (1), we tell Windows we handled the event
/// (by ignoring it). The child process also receives the event and can
/// decide how to respond (typically by exiting gracefully).
///
/// This is the same pattern used by uv-trampoline and Python's distlib launcher.
#[cfg(windows)]
fn install_ctrl_handler() {
    // Raw FFI declaration to avoid pulling in the heavy `windows`/`windows-core` crates.
    // Signature: https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler
    type HandlerRoutine = unsafe extern "system" fn(ctrl_type: u32) -> i32;
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
    }

    unsafe extern "system" fn handler(_ctrl_type: u32) -> i32 {
        1 // TRUE - signal handled (ignored)
    }

    unsafe {
        SetConsoleCtrlHandler(Some(handler), 1);
    }
}
