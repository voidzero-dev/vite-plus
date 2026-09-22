//! Identify Homebrew ownership without requiring `brew` on PATH or a fixed prefix.

use std::{path::Path, sync::OnceLock};

pub(crate) fn owns_current_exe() -> bool {
    static HOMEBREW: OnceLock<bool> = OnceLock::new();
    *HOMEBREW.get_or_init(|| std::env::current_exe().is_ok_and(|binary| owns_binary(&binary)))
}

fn owns_binary(binary: &Path) -> bool {
    // Resolve both Homebrew's public entrypoint and Vite+'s generated shims.
    let Ok(binary) = std::fs::canonicalize(binary) else { return false };
    let Some(bin) = binary.parent().filter(|bin| bin.file_name().is_some_and(|name| name == "bin"))
    else {
        return false;
    };
    let Some(prefix) = bin.parent() else { return false };
    let Ok(data) = std::fs::read(prefix.join("INSTALL_RECEIPT.json")) else { return false };
    let Ok(receipt) = serde_json::from_slice::<serde_json::Value>(&data) else { return false };
    receipt
        .get("homebrew_version")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|version| !version.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_receipt_under_custom_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let prefix = temp.path().join("custom-cellar/vite-plus/0.3.2");
        let binary = prefix.join("bin/vp");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, "vp").unwrap();
        assert!(!owns_binary(&binary));

        let receipt = prefix.join("INSTALL_RECEIPT.json");
        for invalid in
            ["not json", "{}", r#"{"homebrew_version":null}"#, r#"{"homebrew_version":""}"#]
        {
            std::fs::write(&receipt, invalid).unwrap();
            assert!(!owns_binary(&binary));
        }
        std::fs::write(receipt, r#"{"homebrew_version":"7.0.2"}"#).unwrap();
        assert!(owns_binary(&binary));

        // A separate managed installation stays independent of Homebrew.
        let managed = temp.path().join("managed/0.3.2/bin/vp");
        std::fs::create_dir_all(managed.parent().unwrap()).unwrap();
        std::fs::copy(&binary, &managed).unwrap();
        assert!(!owns_binary(&managed));
        assert!(!owns_binary(&prefix.join("missing/bin/vp")));

        #[cfg(unix)]
        {
            let public = temp.path().join("public-vp");
            let shim = temp.path().join("shim-vp");
            std::os::unix::fs::symlink(&binary, &public).unwrap();
            std::os::unix::fs::symlink(&public, &shim).unwrap();
            assert!(owns_binary(&public));
            assert!(owns_binary(&shim));
        }
    }
}
