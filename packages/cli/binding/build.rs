fn main() {
    napi_build::setup();
    if std::env::var_os("CARGO_FEATURE_OXC_NODE").is_some() {
        script_cache_build_id();
    }
}

// Include the compiler, helper ABI, and host adapter in the cache namespace. A
// package version alone is not sufficient for development builds and previews.
#[expect(
    clippy::disallowed_macros,
    clippy::print_stdout,
    clippy::disallowed_types,
    reason = "build scripts use Cargo's stdout protocol and do not depend on vt_path"
)]
fn script_cache_build_id() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut hash = blake3::Hasher::new();
    for relative in [
        "Cargo.lock",
        "Cargo.toml",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "oxc-node/Cargo.toml",
        "oxc-node/src/lib.rs",
        "oxc-node/src/transform_cache.rs",
        "oxc-node/src/windows_file_url.rs",
        "packages/tools/.upstream-versions.json",
        "packages/cli/package.json",
        "packages/cli/src/script-hooks.ts",
        "packages/cli/binding/build.rs",
        "packages/cli/binding/src/oxc_node.rs",
        "packages/cli/binding/src/oxc_node/cache.rs",
        "packages/cli/binding/src/oxc_node/cache/packed.rs",
    ] {
        let path = root.join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = std::fs::read(&path).expect("script cache build input must exist");
        hash.update(relative.as_bytes());
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
    }
    hash.update(std::env::var("TARGET").expect("Cargo sets TARGET").as_bytes());
    println!("cargo:rustc-env=VPX_CACHE_BUILD_ID={}", hash.finalize().to_hex());
}
