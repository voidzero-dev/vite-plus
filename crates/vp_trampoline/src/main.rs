//! Minimal Windows trampoline for vite-plus shims.
//!
//! This binary is copied and renamed for each shim tool (node.exe, npm.exe, etc.).
//! It detects the tool name from its own filename, then spawns `vp.exe` with the
//! `VP_SHIM_TOOL` environment variable set, allowing `vp.exe` to enter
//! shim dispatch mode.
//!
//! On Ctrl+C, the trampoline ignores the signal (the child process handles it),
//! avoiding the "Terminate batch job (Y/N)?" prompt that `.cmd` wrappers produce.
//!
//! **Size optimization**: This binary avoids `core::fmt` (which adds ~100KB) by
//! never using `format!`, `eprintln!`, `println!`, or `.unwrap()`. All error
//! paths use `process::exit(1)` directly.
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

/// Must match [`vp_shared::SHIM_POINTER_EXTENSION`]. Duplicated here so this
/// binary stays dependency-free. Each trampoline reads `<name>.shim` next to
/// itself (`node.exe` → `node.shim`).
const SHIM_POINTER_EXTENSION: &str = "shim";

struct VpLocation {
    exe: std::path::PathBuf,
    /// Data root from `<name>.shim`; pinned as `VP_DATA_DIR` on the child.
    vp_data_dir: std::path::PathBuf,
}

/// Locate `vp.exe` from `<BIN>/<name>.shim`.
///
/// Directory env vars are owned by `EnvConfig` in the child `vp.exe` — this
/// binary must not read `VP_HOME` / `VP_*_DIR`. Every trampoline copy has a
/// sidecar written at install / `vp env setup`, so sibling-layout probing
/// is not needed.
fn resolve_vp_exe(exe_path: &std::path::Path) -> Option<VpLocation> {
    let data = read_shim_pointer(exe_path)?;
    let exe = data.join("current").join("bin").join("vp.exe");
    exe.exists().then_some(VpLocation { exe, vp_data_dir: data })
}

fn read_shim_pointer(exe_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let bytes = std::fs::read(exe_path.with_extension(SHIM_POINTER_EXTENSION)).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes.as_slice());
    let text = std::str::from_utf8(bytes).ok()?.trim();
    if text.is_empty() {
        return None;
    }
    Some(std::path::PathBuf::from(text))
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
        let _ = handle.write_all(b"vite-plus: failed to locate vp.exe via .shim\n");
        process::exit(1);
    };

    // 3. Install Ctrl+C handler that ignores signals (child will handle them).
    //    This prevents the "Terminate batch job (Y/N)?" prompt.
    #[cfg(windows)]
    install_ctrl_handler();

    // 4. Spawn vp.exe
    //    - Pin VP_DATA_DIR / VP_BIN_DIR from the sidecar so the child
    //      EnvConfig matches this install. Do not set VP_HOME.
    //    - If tool is "vp", run in normal CLI mode (no VP_SHIM_TOOL)
    //    - Otherwise, set VP_SHIM_TOOL so vp.exe enters shim dispatch
    let mut cmd = Command::new(&location.exe);
    cmd.args(env::args_os().skip(1));
    cmd.env("VP_DATA_DIR", &location.vp_data_dir);
    cmd.env("VP_BIN_DIR", bin_dir);

    if tool_name != "vp" {
        cmd.env("VP_SHIM_TOOL", tool_name);
        // Clear the recursion marker so nested shim invocations (e.g., npm
        // spawning node) get fresh version resolution instead of falling
        // through to passthrough mode. The old .cmd wrappers went through
        // `vp env exec` which cleared this in exec.rs; the trampoline
        // bypasses that path.
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
            let _ = handle.write_all(b"vite-plus: failed to execute ");
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
        assert_eq!(location.vp_data_dir, data);
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
        assert_eq!(location.vp_data_dir, node_data);
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
        contents.extend_from_slice(data.to_string_lossy().as_bytes());
        contents.extend_from_slice(b"\r\n");
        fs::write(bin.join("vp.shim"), contents).unwrap();

        let location = resolve_vp_exe(&bin.join("vp.exe")).unwrap();
        assert_eq!(location.exe, data.join("current").join("bin").join("vp.exe"));
        assert_eq!(location.vp_data_dir, data);
        let _ = fs::remove_dir_all(&root);
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
