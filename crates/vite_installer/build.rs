fn main() {
    // On Windows, set DEPENDENTLOADFLAG to only search system32 for DLLs at load time.
    // This prevents DLL hijacking when the installer is downloaded to a folder
    // containing malicious DLLs (e.g. Downloads). Matches rustup's approach.
    #[cfg(windows)]
    println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:0x800");
}
