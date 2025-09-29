fn main() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").unwrap() != "windows" {
        return;
    }
    println!("cargo:rerun-if-changed=detours/src");
    // https://github.com/Berrysoft/detours/blob/c9bc2ad6e9cd8f5f7b74cfa65365d61ecc45203f/detours-sys/build.rs
    cc::Build::new()
        .include("detours/src")
        .define("WIN32_LEAN_AND_MEAN", "1")
        .define("_WIN32_WINNT", "0x501")
        .file("detours/src/detours.cpp")
        .file("detours/src/modules.cpp")
        .file("detours/src/disasm.cpp")
        .file("detours/src/image.cpp")
        .file("detours/src/creatwth.cpp")
        .cpp(true)
        .compile("detours");
}
