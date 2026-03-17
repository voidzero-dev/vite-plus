fn main() {
    println!("cargo:rerun-if-env-changed=VITE_PLUS_VERSION");

    let version = std::env::var("VITE_PLUS_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(version_from_git)
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap());

    println!("cargo:rustc-env=VITE_PLUS_VERSION={version}");
}

fn version_from_git() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["describe", "--tags", "--match", "v*", "--abbrev=0"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let tag = String::from_utf8(output.stdout).ok()?;
    let tag = tag.trim();
    tag.strip_prefix('v').map(|s| s.to_owned())
}
