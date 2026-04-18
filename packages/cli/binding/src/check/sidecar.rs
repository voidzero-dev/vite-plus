//! Transient `.mjs` config that overrides `lint.options.typeCheck` to `false`.
//!
//! oxlint loads the `-c <path>` target with dynamic `import()` and reads
//! `.default.lint` when `VP_VERSION` is set (the vite-plus invocation path).
//! Writing a sidecar module with that shape lets a single invocation opt out
//! of type-check without mutating the project's `vite.config.ts`.

use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use vite_error::Error;
use vite_path::AbsolutePathBuf;

use crate::cli::ResolvedUniversalViteConfig;

/// Override returned by [`write_no_type_check_sidecar`]. The `_guard` field
/// deletes the sidecar file on drop; callers must keep the override alive
/// until oxlint has finished reading the config.
pub(super) struct SidecarOverride {
    pub(super) config: ResolvedUniversalViteConfig,
    _guard: SidecarCleanup,
}

struct SidecarCleanup {
    path: AbsolutePathBuf,
}

impl Drop for SidecarCleanup {
    fn drop(&mut self) {
        // Best-effort: ignore errors (file already gone, permission denied, etc.).
        let _ = fs::remove_file(self.path.as_path());
    }
}

static SIDECAR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write a sidecar `.mjs` that mirrors the project's lint config with
/// `options.typeCheck = false`, and return a clone of
/// `ResolvedUniversalViteConfig` pointing at the sidecar path.
///
/// Returns `Ok(None)` when the resolved config lacks either a `configFile` or
/// a `lint` entry — there is nothing for the override to replace and the
/// caller should run lint unchanged.
///
/// `typeAware` is intentionally untouched: the flag only disables type-check
/// (tsgolint), not type-aware lint rules.
pub(super) fn write_no_type_check_sidecar(
    resolved_vite_config: &ResolvedUniversalViteConfig,
) -> Result<Option<SidecarOverride>, Error> {
    if resolved_vite_config.config_file.is_none() {
        return Ok(None);
    }
    let Some(lint) = resolved_vite_config.lint.as_ref() else {
        return Ok(None);
    };

    let mut lint_clone: Value = lint.clone();
    let Some(options) = lint_clone.as_object_mut().and_then(|map| {
        map.entry("options")
            .or_insert_with(|| Value::Object(serde_json::Map::new()))
            .as_object_mut()
    }) else {
        // Lint value isn't a JSON object — unexpected shape. Skip the
        // override rather than guess at a structure.
        return Ok(None);
    };
    options.insert("typeCheck".to_string(), Value::Bool(false));

    // Contract check: `resolveUniversalViteConfig` returns the lint subtree at
    // the top level, so the clone must not be re-wrapped under another `lint`
    // key. Enforced in release too — a regression here would silently empty
    // the sidecar when oxlint reads `.default.lint`.
    if lint_clone.get("lint").is_some() {
        return Err(Error::Anyhow(anyhow::anyhow!(
            "resolved lint config unexpectedly wrapped under another `lint` key"
        )));
    }

    let pid = std::process::id();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let counter = SIDECAR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let filename = vite_str::format!("vite-plus-no-type-check-{pid}-{nanos}-{counter}.mjs");
    let filename_str: &str = filename.as_ref();
    let raw_temp_path = std::env::temp_dir().join(filename_str);
    let sidecar_path = AbsolutePathBuf::new(raw_temp_path).ok_or_else(|| {
        Error::Anyhow(anyhow::anyhow!("system temp dir resolved to a non-absolute path"))
    })?;

    // `serde_json::to_string` quotes keys and escapes string values, producing
    // output that is also a valid JS object literal on Node ≥ 10 (the minimum
    // already required by vite-plus).
    let lint_json = serde_json::to_string(&lint_clone).map_err(|e| {
        Error::Anyhow(anyhow::anyhow!("failed to serialize lint config for sidecar: {e}"))
    })?;
    let content = vite_str::format!("export default {{ lint: {lint_json} }};\n");
    let content_str: &str = content.as_ref();

    fs::write(sidecar_path.as_path(), content_str.as_bytes()).map_err(|e| {
        Error::Anyhow(anyhow::anyhow!(
            "failed to write sidecar at {}: {e}",
            sidecar_path.as_path().display()
        ))
    })?;

    // Keep the override internally consistent: any future reader of
    // `config.lint` (e.g., cache-key hashing, logging) will see the same
    // `typeCheck: false` value that oxlint reads from the sidecar file.
    let mut config_override = resolved_vite_config.clone();
    config_override.config_file = Some(sidecar_path.as_path().to_string_lossy().into_owned());
    config_override.lint = Some(lint_clone);

    Ok(Some(SidecarOverride {
        config: config_override,
        _guard: SidecarCleanup { path: sidecar_path },
    }))
}
