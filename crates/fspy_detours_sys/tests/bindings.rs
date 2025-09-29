#![cfg(windows)]

use std::{env, fs};

#[test]
fn detours_bindings() {
    // https://github.com/0xC9C3/rust-ms-detours/blob/0abc7b11c038afbc2f976de88d44e5691f43bd9a/build.rs#L78
    let bindings = bindgen::Builder::default()
        .clang_args(["-Idetours/src", "-DWIN32_LEAN_AND_MEAN", "-target", "i686-pc-windows-msvc"])
        .header_contents("wrapper.h", "#include <windows.h>\n#include <detours.h>\n")
        .allowlist_function("Detour.*")
        .blocklist_type("LP.*")
        .blocklist_type("_GUID")
        .blocklist_type("GUID")
        .blocklist_type("ULONG")
        .blocklist_type("PVOID")
        .blocklist_type("DWORD")
        .blocklist_type("wchar_t")
        .blocklist_type("BOOL")
        .blocklist_type("BYTE")
        .blocklist_type("WORD")
        .blocklist_type("PBYTE")
        .blocklist_type("PDWORD")
        .blocklist_type("INT")
        .blocklist_type("CHAR")
        .blocklist_type("LONG")
        .blocklist_type("WCHAR")
        .blocklist_type("HANDLE")
        .blocklist_type("HMODULE")
        .blocklist_type("HINSTANCE.*")
        .blocklist_type("HWND.*")
        .blocklist_type("_SECURITY_ATTRIBUTES")
        .blocklist_type("_PROCESS_INFORMATION")
        .blocklist_type("_STARTUPINFOA")
        .blocklist_type("_STARTUPINFOW")
        .disable_header_comment()
        .raw_line("use winapi::shared::minwindef::*;")
        .raw_line("use winapi::um::winnt::*;")
        .raw_line("use winapi::um::winnt::{INT};")
        .raw_line("use winapi::um::minwinbase::*;")
        .raw_line("use winapi::um::processthreadsapi::*;")
        .raw_line("use winapi::shared::guiddef::*;")
        .raw_line("use winapi::shared::windef::*;")
        .layout_tests(false)
        // Detour functions are stdcall on 32-bit Windows
        .override_abi(bindgen::Abi::System, ".*")
        .generate()
        .expect("Unable to generate bindings");
    let bindings_content = bindings.to_string();
    let bindings_path = "src/generated_bindings.rs";

    if env::var("FSPY_DETOURS_WRITE_BINDINGS").as_deref() == Ok("1") {
        fs::write(bindings_path, bindings_content).unwrap();
    } else {
        let existing_bindings_content = fs::read_to_string(bindings_path).unwrap_or_default();
        assert_eq!(
            existing_bindings_content, bindings_content,
            "Bindings are out of date. Run this test with FSPY_DETOURS_WRITE_BINDINGS=1 to update them."
        );
    }
}
