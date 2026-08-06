#![allow(clippy::allow_attributes, clippy::disallowed_macros)]

fn main() {
    // On Windows, set DEPENDENTLOADFLAG to only search system32 for DLLs at load time.
    // This prevents DLL hijacking when the installer is downloaded to a folder
    // containing malicious DLLs (e.g. Downloads). Matches rustup's approach.
    // Use CARGO_CFG_TARGET_OS (not #[cfg(windows)]) so the flag is emitted
    // correctly when cross-compiling from a non-Windows host.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:0x800");
    }
}
