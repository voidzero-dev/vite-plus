//! Raw Win32 trampoline implementation.
//!
//! The `#![no_main]` entry point calls this module directly.
//! Thus, the CRT startup and the Rust `std` runtime do not initialize.
//! This module uses KERNEL32 calls for all operating-system operations.
//! These operations include file I/O, environment setup, and process control.
//! `Vec` uses the Windows process heap and does not need runtime initialization.

use core::{ffi::c_void, ptr};

use crate::cmdline::{self, ShimLayout};

type Handle = *mut c_void;

const CP_UTF8: u32 = 65001;
const BACKSLASH: u16 = b'\\' as u16;
const QUESTION: u16 = b'?' as u16;
const GENERIC_READ: u32 = 0x8000_0000;
const FILE_SHARE_READ: u32 = 0x0000_0001;
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
const FILE_SHARE_DELETE: u32 = 0x0000_0004;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x0000_0080;
const INFINITE: u32 = 0xFFFF_FFFF;
const STD_ERROR_HANDLE: u32 = -12i32 as u32;
const STARTF_USESTDHANDLES: u32 = 0x0000_0100;
const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_FAILED: u32 = 0xFFFF_FFFF;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_PATH_NOT_FOUND: u32 = 3;
const ERROR_ENVVAR_NOT_FOUND: u32 = 203;
// Use the conservative threshold from Rust's Windows path handling.
// CreateDirectoryW reserves space below MAX_PATH.
// Thus, std normalizes paths at this length, even if another API accepts more units.
const LEGACY_MAX_PATH: usize = 248;
const MAX_SHIM_POINTER_BYTES: i64 = 1024 * 1024;
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

#[repr(C)]
struct StartupInfoW {
    cb: u32,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    x: u32,
    y: u32,
    x_size: u32,
    y_size: u32,
    x_count_chars: u32,
    y_count_chars: u32,
    fill_attribute: u32,
    flags: u32,
    show_window: u16,
    cb_reserved2: u16,
    reserved2: *mut u8,
    std_input: Handle,
    std_output: Handle,
    std_error: Handle,
}

#[repr(C)]
struct ProcessInformation {
    process: Handle,
    thread: Handle,
    process_id: u32,
    thread_id: u32,
}

type HandlerRoutine = unsafe extern "system" fn(ctrl_type: u32) -> i32;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleFileNameW(module: Handle, filename: *mut u16, size: u32) -> u32;
    fn GetFullPathNameW(
        file_name: *const u16,
        buffer_length: u32,
        buffer: *mut u16,
        file_part: *mut *mut u16,
    ) -> u32;
    fn GetCommandLineW() -> *const u16;
    fn GetLastError() -> u32;
    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *const c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: Handle,
    ) -> Handle;
    fn GetFileSizeEx(file: Handle, file_size: *mut i64) -> i32;
    fn ReadFile(
        file: Handle,
        buffer: *mut u8,
        bytes_to_read: u32,
        bytes_read: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn SetEnvironmentVariableW(name: *const u16, value: *const u16) -> i32;
    fn GetStartupInfoW(si: *mut StartupInfoW);
    fn SetHandleInformation(object: Handle, mask: u32, flags: u32) -> i32;
    fn CreateProcessW(
        application_name: *const u16,
        command_line: *mut u16,
        process_attributes: *const c_void,
        thread_attributes: *const c_void,
        inherit_handles: i32,
        creation_flags: u32,
        environment: *const c_void,
        current_directory: *const u16,
        startup_info: *const StartupInfoW,
        process_information: *mut ProcessInformation,
    ) -> i32;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn GetExitCodeProcess(process: Handle, exit_code: *mut u32) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
    fn GetStdHandle(std_handle: u32) -> Handle;
    fn WriteFile(
        handle: Handle,
        buffer: *const u8,
        bytes_to_write: u32,
        bytes_written: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn WideCharToMultiByte(
        codepage: u32,
        flags: u32,
        wide: *const u16,
        wide_len: i32,
        out: *mut u8,
        out_len: i32,
        default_char: *const u8,
        used_default: *mut i32,
    ) -> i32;
    fn ExitProcess(exit_code: u32) -> !;
}

// Current nightly toolchains register TLS cleanup through C `atexit`.
// The CRT implementation would add its startup code to this no_main binary.
// ExitProcess does not run TLS destructors.
// Thus, this process uses a successful no-op implementation.
#[unsafe(no_mangle)]
pub extern "C" fn atexit(_f: Option<unsafe extern "C" fn()>) -> i32 {
    0
}

/// NUL-terminated UTF-16 literal (compile-time, ASCII input only).
macro_rules! w {
    ($s:literal) => {{
        const S: &str = $s;
        const N: usize = S.len();
        const OUT: [u16; N + 1] = {
            let mut out = [0u16; N + 1];
            let bytes = S.as_bytes();
            let mut i = 0;
            while i < N {
                out[i] = bytes[i] as u16;
                i += 1;
            }
            out
        };
        &OUT
    }};
}

fn without_nul(wide: &[u16]) -> &[u16] {
    &wide[..wide.len() - 1]
}

fn nul_terminated(wide: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(wide.len() + 1);
    out.extend_from_slice(wide);
    out.push(0);
    out
}

fn is_separator(value: u16) -> bool {
    value == b'\\' as u16 || value == b'/' as u16
}

fn join_path(base: &[u16], suffix: &[u16]) -> Vec<u16> {
    let mut path = Vec::with_capacity(base.len() + suffix.len() + 1);
    path.extend_from_slice(base);
    if path.last().is_some_and(|&last| !is_separator(last)) {
        path.push(b'\\' as u16);
    }
    path.extend_from_slice(suffix);
    path
}

fn is_verbatim(path: &[u16]) -> bool {
    matches!(
        path,
        [BACKSLASH, BACKSLASH, QUESTION, BACKSLASH, ..]
            | [BACKSLASH, QUESTION, QUESTION, BACKSLASH, ..]
    )
}

/// Return a NUL-terminated path suitable for Win32 file and process APIs.
///
/// This function matches the behavior of the replaced standard-library code.
/// It makes long paths absolute.
/// It normalizes the paths.
/// It then puts the paths in the extended-length namespace.
fn win32_api_path(path: &[u16]) -> Vec<u16> {
    let path_nul = nul_terminated(path);
    if is_verbatim(path) || path_nul.len() < LEGACY_MAX_PATH {
        return path_nul;
    }

    let required =
        unsafe { GetFullPathNameW(path_nul.as_ptr(), 0, ptr::null_mut(), ptr::null_mut()) };
    if required == 0 {
        fail_path_call(b"GetFullPathNameW", path, unsafe { GetLastError() });
    }
    let mut absolute = Vec::with_capacity(required as usize);
    let len = unsafe {
        GetFullPathNameW(path_nul.as_ptr(), required, absolute.as_mut_ptr(), ptr::null_mut())
    };
    if len == 0 || len >= required {
        fail_path_call(b"GetFullPathNameW", path, unsafe { GetLastError() });
    }
    unsafe { absolute.set_len(len as usize) };

    let mut extended = cmdline::verbatim_path(&absolute);
    extended.push(0);
    extended
}

fn utf8_path(text: &str) -> Option<Vec<u16>> {
    let mut path = Vec::with_capacity(text.len());
    for unit in text.encode_utf16() {
        if unit == 0 {
            return None;
        }
        path.push(unit);
    }
    (!path.is_empty()).then_some(path)
}

// ---------------------------------------------------------------------------
// Diagnostics. Error paths are cold and avoid core::fmt.
// ---------------------------------------------------------------------------

fn stderr_write(bytes: &[u8]) {
    unsafe {
        let stderr = GetStdHandle(STD_ERROR_HANDLE);
        if !stderr.is_null() && stderr != INVALID_HANDLE_VALUE {
            let mut written = 0u32;
            WriteFile(
                stderr,
                bytes.as_ptr(),
                bytes.len() as u32,
                &raw mut written,
                ptr::null_mut(),
            );
        }
    }
}

/// Write a UTF-16 slice to stderr as UTF-8 (best effort).
fn stderr_write_wide(wide: &[u16]) {
    if wide.is_empty() {
        return;
    }
    let len = unsafe {
        WideCharToMultiByte(
            CP_UTF8,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            ptr::null_mut(),
            0,
            ptr::null(),
            ptr::null_mut(),
        )
    };
    if len <= 0 {
        stderr_write(b"<path with unconvertible characters>");
        return;
    }
    let mut utf8 = Vec::with_capacity(len as usize);
    let written = unsafe {
        WideCharToMultiByte(
            CP_UTF8,
            0,
            wide.as_ptr(),
            wide.len() as i32,
            utf8.as_mut_ptr(),
            len,
            ptr::null(),
            ptr::null_mut(),
        )
    };
    if written > 0 {
        unsafe { utf8.set_len(written as usize) };
        stderr_write(&utf8);
    }
}

fn stderr_write_num(value: u32) {
    let mut buf = [0u8; 10];
    stderr_write(cmdline::format_u32(value, &mut buf));
}

#[cold]
fn report_call_failure(what: &[u8], error: u32) {
    stderr_write(b"vite-plus shim: ");
    stderr_write(what);
    stderr_write(b" failed (Windows error ");
    stderr_write_num(error);
    stderr_write(b")\n");
}

#[cold]
fn fail_call(what: &[u8]) -> ! {
    report_call_failure(what, unsafe { GetLastError() });
    unsafe { ExitProcess(1) }
}

#[cold]
fn fail_path_call(what: &[u8], path: &[u16], error: u32) -> ! {
    stderr_write(b"vite-plus shim: ");
    stderr_write(what);
    stderr_write(b" failed for \"");
    stderr_write_wide(path);
    stderr_write(b"\" (Windows error ");
    stderr_write_num(error);
    stderr_write(b")\n");
    unsafe { ExitProcess(1) }
}

#[cold]
fn fail_invalid_pointer(path: &[u16]) -> ! {
    stderr_write(b"vite-plus shim: invalid or unsupported shim pointer \"");
    stderr_write_wide(path);
    stderr_write(b"\"; reinstall vite-plus or run `vp env setup`\n");
    unsafe { ExitProcess(1) }
}

// ---------------------------------------------------------------------------
// Sidecar and path handling.
// ---------------------------------------------------------------------------

fn module_path() -> Vec<u16> {
    let mut buf = Vec::with_capacity(512);
    loop {
        let cap = buf.capacity();
        let len = unsafe { GetModuleFileNameW(ptr::null_mut(), buf.as_mut_ptr(), cap as u32) };
        if len == 0 {
            fail_call(b"GetModuleFileNameW");
        }
        if (len as usize) < cap {
            unsafe { buf.set_len(len as usize) };
            return buf;
        }
        buf.reserve(cap * 2);
    }
}

fn pointer_path(exe: &[u16], last_separator: usize, file_name: &[u16]) -> Vec<u16> {
    let stem_len = cmdline::file_stem_len(file_name);
    let mut path = Vec::with_capacity(last_separator + stem_len + 6);
    path.extend_from_slice(&exe[..last_separator + 1]);
    path.extend_from_slice(&file_name[..stem_len]);
    path.extend_from_slice(without_nul(w!(".shim")));
    path
}

fn read_pointer_file(path: &[u16]) -> Vec<u8> {
    let path_nul = win32_api_path(path);
    let handle = unsafe {
        CreateFileW(
            path_nul.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        fail_path_call(b"CreateFileW", path, unsafe { GetLastError() });
    }

    let mut size = 0i64;
    if unsafe { GetFileSizeEx(handle, &raw mut size) } == 0 {
        let error = unsafe { GetLastError() };
        unsafe { CloseHandle(handle) };
        fail_path_call(b"GetFileSizeEx", path, error);
    }
    if !(1..=MAX_SHIM_POINTER_BYTES).contains(&size) {
        unsafe { CloseHandle(handle) };
        fail_invalid_pointer(path);
    }

    let mut bytes = Vec::with_capacity(size as usize);
    let mut read = 0u32;
    let ok = unsafe {
        ReadFile(handle, bytes.as_mut_ptr(), size as u32, &raw mut read, ptr::null_mut())
    };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        unsafe { CloseHandle(handle) };
        fail_path_call(b"ReadFile", path, error);
    }
    unsafe { CloseHandle(handle) };
    if i64::from(read) != size {
        fail_invalid_pointer(path);
    }
    unsafe { bytes.set_len(read as usize) };
    bytes
}

// ---------------------------------------------------------------------------
// Process environment and launch.
// ---------------------------------------------------------------------------

fn set_env(name: &[u16], name_ascii: &[u8], value: Option<&[u16]>) {
    let value_nul = value.map(nul_terminated);
    let value_ptr = value_nul.as_ref().map_or(ptr::null(), |value| value.as_ptr());
    let ok = unsafe { SetEnvironmentVariableW(name.as_ptr(), value_ptr) };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        if value_nul.is_none() && error == ERROR_ENVVAR_NOT_FOUND {
            return;
        }
        stderr_write(b"vite-plus shim: SetEnvironmentVariableW(");
        stderr_write(name_ascii);
        stderr_write(b") failed (Windows error ");
        stderr_write_num(error);
        stderr_write(b")\n");
        unsafe { ExitProcess(1) }
    }
}

unsafe extern "system" fn ignore_ctrl(_ctrl_type: u32) -> i32 {
    1
}

pub fn run() -> ! {
    // 1. Resolve the tool, bin directory, and per-tool sidecar from our path.
    let exe = module_path();
    let Some(last_separator) = exe.iter().rposition(|&unit| is_separator(unit)) else {
        stderr_write(b"vite-plus shim: cannot resolve the shim directory from \"");
        stderr_write_wide(&exe);
        stderr_write(b"\"\n");
        unsafe { ExitProcess(1) }
    };
    let bin_dir = &exe[..cmdline::parent_dir_len(&exe, last_separator)];
    let file_name = &exe[last_separator + 1..];
    let tool = &file_name[..cmdline::file_stem_len(file_name)];
    let pointer_path = pointer_path(&exe, last_separator, file_name);
    let pointer_bytes = read_pointer_file(&pointer_path);
    let Some(parsed) = cmdline::parse_shim_pointer(&pointer_bytes) else {
        fail_invalid_pointer(&pointer_path);
    };
    let Some(data) = utf8_path(parsed.data) else {
        fail_invalid_pointer(&pointer_path);
    };

    // 2. Pin the directory layout selected by the sidecar.
    match parsed.layout {
        ShimLayout::SingleRoot => {
            set_env(w!("VP_HOME"), b"VP_HOME", Some(&data));
        }
        ShimLayout::Split { cache } => {
            let Some(cache) = utf8_path(cache) else {
                fail_invalid_pointer(&pointer_path);
            };
            set_env(w!("VP_HOME"), b"VP_HOME", None);
            set_env(w!("VP_DATA_DIR"), b"VP_DATA_DIR", Some(&data));
            set_env(w!("VP_BIN_DIR"), b"VP_BIN_DIR", Some(bin_dir));
            set_env(w!("VP_CACHE_DIR"), b"VP_CACHE_DIR", Some(&cache));
        }
    }

    if !cmdline::eq_ascii(tool, b"vp") {
        set_env(w!("VP_SHIM_TOOL"), b"VP_SHIM_TOOL", Some(tool));
        set_env(w!("VP_TOOL_RECURSION"), b"VP_TOOL_RECURSION", None);
    }

    // 3. Build the child command line from the active payload.
    // Append the caller's raw argument tail without changes.
    // This preserves the caller's quotation marks.
    let vp_exe = join_path(&data, without_nul(w!("current\\bin\\vp.exe")));
    let vp_exe_nul = win32_api_path(&vp_exe);
    let tail = unsafe {
        let command_line = GetCommandLineW();
        if command_line.is_null() {
            fail_call(b"GetCommandLineW");
        }
        let mut len = 0usize;
        while *command_line.add(len) != 0 {
            len += 1;
        }
        let all = core::slice::from_raw_parts(command_line, len);
        &all[cmdline::skip_program_argument(all)..]
    };
    let mut child_cmdline = Vec::with_capacity(vp_exe.len() + tail.len() + 3);
    child_cmdline.push(b'"' as u16);
    child_cmdline.extend_from_slice(&vp_exe);
    child_cmdline.push(b'"' as u16);
    child_cmdline.extend_from_slice(tail);
    child_cmdline.push(0);

    // 4. Ignore console control events in the trampoline.
    // The child receives the same event.
    // The child handles the event.
    if unsafe { SetConsoleCtrlHandler(Some(ignore_ctrl), 1) } == 0 {
        report_call_failure(b"warning: SetConsoleCtrlHandler", unsafe { GetLastError() });
    }

    // 5. Reuse the trampoline startup information.
    // If the parent redirected standard I/O, make those handles inheritable.
    // Do this before the CreateProcessW call.
    let mut si = unsafe { core::mem::zeroed::<StartupInfoW>() };
    si.cb = size_of::<StartupInfoW>() as u32;
    unsafe { GetStartupInfoW(&raw mut si) };
    if si.flags & STARTF_USESTDHANDLES != 0 {
        for handle in [si.std_input, si.std_output, si.std_error] {
            if !handle.is_null()
                && handle != INVALID_HANDLE_VALUE
                && unsafe { SetHandleInformation(handle, HANDLE_FLAG_INHERIT, HANDLE_FLAG_INHERIT) }
                    == 0
            {
                report_call_failure(b"warning: SetHandleInformation", unsafe { GetLastError() });
            }
        }
    }

    let mut pi = ProcessInformation {
        process: ptr::null_mut(),
        thread: ptr::null_mut(),
        process_id: 0,
        thread_id: 0,
    };
    let ok = unsafe {
        CreateProcessW(
            vp_exe_nul.as_ptr(),
            child_cmdline.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            1,
            0,
            ptr::null(),
            ptr::null(),
            &raw const si,
            &raw mut pi,
        )
    };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        stderr_write(b"vite-plus: could not execute \"");
        stderr_write_wide(&vp_exe);
        stderr_write(b"\" (Windows error ");
        stderr_write_num(error);
        stderr_write(b")");
        if error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND {
            stderr_write(b": vp.exe is missing; reinstall vite-plus or run `vp env setup`");
        }
        stderr_write(b"\n");
        unsafe { ExitProcess(1) }
    }

    // 6. Wait for the child.
    // Propagate its exact exit code.
    unsafe {
        CloseHandle(pi.thread);
        let wait = WaitForSingleObject(pi.process, INFINITE);
        match wait {
            WAIT_OBJECT_0 => {}
            WAIT_FAILED => fail_call(b"WaitForSingleObject"),
            _ => {
                report_call_failure(b"WaitForSingleObject returned an unexpected status", wait);
                ExitProcess(1);
            }
        }
        let mut code = 1u32;
        if GetExitCodeProcess(pi.process, &raw mut code) == 0 {
            fail_call(b"GetExitCodeProcess");
        }
        ExitProcess(code)
    }
}
