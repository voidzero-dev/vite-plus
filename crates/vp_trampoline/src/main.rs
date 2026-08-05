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

/// Locate the real `vp.exe` relative to the install base dir (the parent of
/// the bin dir the trampoline copy lives in).
///
/// Legacy layout first (`<base>/current/bin/vp.exe`, where the bin dir is
/// `<base>/bin`), then the split layout (`<base>/data/current/bin/vp.exe`,
/// where the bin dir is a separate `<base>/bin`). Returns `None` if neither
/// exists.
fn locate_vp_exe(base: &std::path::Path) -> Option<std::path::PathBuf> {
    let legacy = base.join("current").join("bin").join("vp.exe");
    if legacy.is_file() {
        return Some(legacy);
    }
    let split = base.join("data").join("current").join("bin").join("vp.exe");
    split.is_file().then_some(split)
}

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

fn main() {
    // 1. Determine tool name from our own executable filename
    let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
    let tool_name =
        exe_path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| process::exit(1));

    // 2. Locate vp.exe: legacy `<base>/current/bin/vp.exe` first, then the
    //    split layout's `<base>/data/current/bin/vp.exe`.
    let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
    let base = bin_dir.parent().unwrap_or_else(|| process::exit(1));
    let vp_exe = locate_vp_exe(base).unwrap_or_else(|| {
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = handle.write_all(b"vite-plus: could not locate vp.exe under ");
        let _ = handle.write_all(base.as_os_str().as_encoded_bytes());
        let _ = handle.write_all(b" (tried current\\bin and data\\current\\bin)\n");
        process::exit(1);
    });

    // 3. Install Ctrl+C handler that ignores signals (child will handle them).
    //    This prevents the "Terminate batch job (Y/N)?" prompt.
    #[cfg(windows)]
    install_ctrl_handler();

    // 4. Spawn vp.exe
    //    - No VP_HOME needed: vp.exe locates its install root from its own
    //      `<root>/current/bin/vp.exe` path.
    //    - If tool is "vp", run in normal CLI mode (no VP_SHIM_TOOL)
    //    - Otherwise, set VP_SHIM_TOOL so vp.exe enters shim dispatch
    let mut cmd = Command::new(&vp_exe);
    cmd.args(env::args_os().skip(1));

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
            let _ = handle.write_all(vp_exe.as_os_str().as_encoded_bytes());
            let _ = handle.write_all(b"\n");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn preserves_signal_exit_code() {
        let status = Command::new("/bin/sh").arg("-c").arg("kill -ILL $$").status().unwrap();
        assert_eq!(exit_code_from_status(status), 132);
    }

    fn test_base(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vp-trampoline-test-{name}-{}", process::id()))
    }

    #[test]
    fn locate_vp_exe_prefers_legacy_layout() {
        let base = test_base("legacy");
        let legacy_dir = base.join("current").join("bin");
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join("vp.exe"), b"MZ").unwrap();

        assert_eq!(locate_vp_exe(&base).unwrap(), legacy_dir.join("vp.exe"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn locate_vp_exe_falls_back_to_split_layout() {
        let base = test_base("split");
        let split_dir = base.join("data").join("current").join("bin");
        std::fs::create_dir_all(&split_dir).unwrap();
        std::fs::write(split_dir.join("vp.exe"), b"MZ").unwrap();

        assert_eq!(locate_vp_exe(&base).unwrap(), split_dir.join("vp.exe"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn locate_vp_exe_returns_none_when_absent() {
        let base = test_base("absent");
        std::fs::create_dir_all(&base).unwrap();

        assert!(locate_vp_exe(&base).is_none());

        let _ = std::fs::remove_dir_all(&base);
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
