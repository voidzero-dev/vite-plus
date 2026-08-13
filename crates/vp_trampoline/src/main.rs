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

/// Locate `vp.exe` from this trampoline's directory using on-disk layout only.
///
/// Directory env vars are owned by `EnvConfig` in the child `vp.exe` — this
/// binary must not read `VP_HOME` / `VP_*_DIR`.
///
/// 1. `<bin>/../current/bin/vp.exe` — monolithic (`<root>/bin` next to `<root>/current`)
/// 2. `<bin>/../data/current/bin/vp.exe` — Windows split default
///
/// `VP_HOME` is injected only when the monolithic `current` payload exists,
/// matching the old `%~dp0..` wrappers. Split layouts leave the env alone.
fn resolve_vp_exe(bin_dir: &std::path::Path) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
    let parent = bin_dir.parent().unwrap_or_else(|| process::exit(1));
    let monolithic = parent.join("current").join("bin").join("vp.exe");
    if monolithic.exists() {
        return (monolithic, Some(parent.to_path_buf()));
    }
    let split = parent.join("data").join("current").join("bin").join("vp.exe");
    if split.exists() || parent.join("data").is_dir() {
        return (split, None);
    }
    (monolithic, Some(parent.to_path_buf()))
}

fn main() {
    // 1. Determine tool name from our own executable filename
    let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
    let tool_name =
        exe_path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| process::exit(1));

    // 2. Locate vp.exe (monolithic `<root>/current` or split `<root>/data/current`)
    let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
    let (vp_exe, vp_home) = resolve_vp_exe(bin_dir);

    // 3. Install Ctrl+C handler that ignores signals (child will handle them).
    //    This prevents the "Terminate batch job (Y/N)?" prompt.
    #[cfg(windows)]
    install_ctrl_handler();

    // 4. Spawn vp.exe
    //    - Pin VP_HOME only for a monolithic root (matches the old .cmd
    //      wrappers' `%~dp0..`). Split installs leave the env alone so the
    //      child resolves XDG / platform category roots itself.
    //    - If tool is "vp", run in normal CLI mode (no VP_SHIM_TOOL)
    //    - Otherwise, set VP_SHIM_TOOL so vp.exe enters shim dispatch
    let mut cmd = Command::new(&vp_exe);
    cmd.args(env::args_os().skip(1));
    if let Some(home) = vp_home {
        cmd.env("VP_HOME", home);
    }

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
    fn prefers_monolithic_current_and_pins_home() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-mono-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        write_exe(&root.join("current").join("bin").join("vp.exe"));

        let (exe, home) = resolve_vp_exe(&root.join("bin"));
        assert_eq!(exe, root.join("current").join("bin").join("vp.exe"));
        assert_eq!(home, Some(root.clone()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn split_data_sibling_does_not_pin_home() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-split-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        write_exe(&root.join("data").join("current").join("bin").join("vp.exe"));

        let (exe, home) = resolve_vp_exe(&root.join("bin"));
        assert_eq!(exe, root.join("data").join("current").join("bin").join("vp.exe"));
        assert_eq!(home, None);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn split_data_dir_without_payload_does_not_pin_home() {
        let root = std::env::temp_dir().join(format!("vp-trampoline-data-{}", process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("data")).unwrap();

        let (exe, home) = resolve_vp_exe(&root.join("bin"));
        assert_eq!(exe, root.join("data").join("current").join("bin").join("vp.exe"));
        assert_eq!(home, None);
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
