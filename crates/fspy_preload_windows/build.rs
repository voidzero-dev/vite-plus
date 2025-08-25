fn main() {
    if std::env::var_os("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        println!("cargo:rustc-cdylib-link-arg=/EXPORT:DetourFinishHelperProcess,@1,NONAME");
    }
}
