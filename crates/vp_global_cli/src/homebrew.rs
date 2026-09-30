//! Identify Homebrew ownership without requiring `brew` on PATH or a fixed prefix.

use std::{path::Path, sync::OnceLock};

use vt_path::AbsolutePathBuf;

pub(crate) struct Installation {
    pub(crate) binary: AbsolutePathBuf,
    pub(crate) formula: String,
    pub(crate) source: Source,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Core,
    OfficialTap,
    Other,
}

impl Installation {
    pub(crate) fn source_label(&self) -> &'static str {
        match self.source {
            Source::Core => "Homebrew Core",
            Source::OfficialTap => "Vite+ Homebrew tap",
            Source::Other => "Homebrew",
        }
    }
}

pub(crate) fn current() -> Option<&'static Installation> {
    static HOMEBREW: OnceLock<Option<Installation>> = OnceLock::new();
    HOMEBREW
        .get_or_init(|| std::env::current_exe().ok().and_then(|binary| installation(&binary)))
        .as_ref()
}

pub(crate) fn owns_current_exe() -> bool {
    current().is_some()
}

fn installation(binary: &Path) -> Option<Installation> {
    // Resolve both Homebrew's public entrypoint and Vite+'s generated shims.
    let binary = std::fs::canonicalize(binary).ok()?;
    let bin = binary.parent().filter(|bin| bin.file_name().is_some_and(|name| name == "bin"))?;
    let prefix = bin.parent()?;
    let data = std::fs::read(prefix.join("INSTALL_RECEIPT.json")).ok()?;
    let receipt = serde_json::from_slice::<serde_json::Value>(&data).ok()?;
    if receipt.get("homebrew_version").and_then(serde_json::Value::as_str).is_none_or(str::is_empty)
    {
        return None;
    }
    let source = receipt.get("source");
    let name = source
        .and_then(|source| source.get("path"))
        .and_then(serde_json::Value::as_str)
        .and_then(|path| Path::new(path).file_stem())
        .and_then(|name| name.to_str())
        .filter(|name| valid_component(name))
        .or_else(|| {
            let rack = prefix.parent()?;
            (rack.parent()?.file_name()? == "Cellar")
                .then(|| rack.file_name()?.to_str().filter(|name| valid_component(name)))?
        })
        .unwrap_or("vite-plus");
    let tap = source
        .and_then(|source| source.get("tap"))
        .and_then(serde_json::Value::as_str)
        .filter(|tap| {
            let parts = tap.split('/').collect::<Vec<_>>();
            parts.len() == 2 && parts.iter().all(|part| valid_component(part))
        });
    let formula = match tap {
        Some("homebrew/core") | None => name.to_string(),
        Some(tap) => format!("{tap}/{name}"),
    };
    let source = match tap {
        Some("homebrew/core") => Source::Core,
        Some("voidzero-dev/vite-plus") if name == "vp" => Source::OfficialTap,
        _ => Source::Other,
    };
    Some(Installation { binary: AbsolutePathBuf::new(binary)?, formula, source })
}

fn valid_component(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_.@+".contains(&byte))
}

#[cfg(unix)]
impl Installation {
    /// Homebrew's standard Cellar layout exposes a stable link outside the keg.
    pub(crate) fn public_binary(&self) -> Option<AbsolutePathBuf> {
        let cellar = self.binary.parent()?.parent()?.parent()?.parent()?;
        if cellar.as_path().file_name()? != "Cellar" {
            return None;
        }
        let public = cellar.parent()?.join("bin/vp");
        (std::fs::canonicalize(&public).ok()? == self.binary.as_path()).then_some(public)
    }
}

/// A Homebrew package can ship the JS payload in its prefix (core), or let each
/// user install matching dependencies on first launch (the official tap).
pub(crate) fn user_package_dir() -> Result<Option<AbsolutePathBuf>, crate::error::Error> {
    let Some(homebrew) = current() else { return Ok(None) };
    if crate::self_setup::has_bundled_package(homebrew.binary.as_path()) {
        return Ok(None);
    }
    let platform = vp_setup::platform::detect_platform_suffix()?;
    Ok(Some(vp_shared::EnvConfig::get().dirs.cli_package(env!("CARGO_PKG_VERSION"), &platform)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owns_binary(binary: &Path) -> bool {
        installation(binary).is_some()
    }

    #[test]
    fn identifies_core_and_tap_formulae_from_receipts() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("Cellar/vp/0.3.4");
        let binary = prefix.join("bin/vp");
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"vp").unwrap();
        for (tap, path, expected, source) in [
            (
                "voidzero-dev/vite-plus",
                "/tap/HomebrewFormula/vp.rb",
                "voidzero-dev/vite-plus/vp",
                Source::OfficialTap,
            ),
            ("homebrew/core", "/tap/Formula/v/vite-plus.rb", "vite-plus", Source::Core),
            (
                "fengmk2/core",
                "/tap/Formula/v/vite-plus.rb",
                "fengmk2/core/vite-plus",
                Source::Other,
            ),
            ("bad/tap/extra", "/tap/HomebrewFormula/vp.rb", "vp", Source::Other),
            ("bad;command/tap", "/tap/HomebrewFormula/vp.rb", "vp", Source::Other),
        ] {
            std::fs::write(
                prefix.join("INSTALL_RECEIPT.json"),
                serde_json::to_vec(&serde_json::json!({
                    "homebrew_version": "7.0.2", "source": { "tap": tap, "path": path }
                }))
                .unwrap(),
            )
            .unwrap();
            let install = installation(&binary).unwrap();
            assert_eq!(install.formula, expected);
            assert_eq!(install.source, source);
            assert_eq!(install.binary.as_path(), std::fs::canonicalize(&binary).unwrap());
        }
    }

    #[cfg(unix)]
    #[test]
    fn public_entrypoint_survives_cellar_replacement_without_path() {
        let root = tempfile::tempdir().unwrap();
        for version in ["old", "new"] {
            let prefix = root.path().join("Cellar/vp").join(version);
            std::fs::create_dir_all(prefix.join("bin")).unwrap();
            std::fs::write(prefix.join("bin/vp"), version).unwrap();
            std::fs::write(prefix.join("INSTALL_RECEIPT.json"), br#"{"homebrew_version":"7.0.2"}"#)
                .unwrap();
        }
        let public = root.path().join("bin/vp");
        std::fs::create_dir_all(public.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink("../Cellar/vp/old/bin/vp", &public).unwrap();
        let old = installation(&public).unwrap();
        let stable = old.public_binary().unwrap();
        std::fs::remove_file(&public).unwrap();
        std::os::unix::fs::symlink("../Cellar/vp/new/bin/vp", &public).unwrap();
        std::fs::remove_dir_all(root.path().join("Cellar/vp/old")).unwrap();
        assert_eq!(std::fs::read_to_string(&stable).unwrap(), "new");
        assert_eq!(installation(&public).unwrap().public_binary().unwrap(), stable);
    }

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
        assert_eq!(installation(&binary).unwrap().source, Source::Other);

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
