//! Pin command for per-directory Node.js version management.
//!
//! Handles `vp env pin [VERSION]` to pin a Node.js version in the current directory.
//! The write target follows the compatibility-first rule from rfcs/dev-engines.md:
//! an existing `.node-version` keeps being updated; otherwise the pin is written to
//! `package.json#devEngines.runtime`; `.node-version` is only created when the
//! directory has no package.json. An explicit `--target` flag overrides the selection.
//! An existing `engines.node` is never deleted or modified.

use std::{io::Write, process::ExitStatus};

use vp_js_runtime::NodeProvider;
use vp_pm_cli::{
    PackageManagerType, download_package_manager, resolve_package_manager_from_package_json,
    resolve_package_manager_version,
};
use vp_shared::output;
use vt_path::AbsolutePathBuf;

use super::{
    config::{get_config_path, load_config},
    package_manager,
    spec::{EnvScope, EnvSpecs},
};
use crate::{cli::PinTarget, error::Error};

/// Node version file name
const NODE_VERSION_FILE: &str = ".node-version";

/// Package manifest file name
const PACKAGE_JSON_FILE: &str = "package.json";

/// Execute the pin command.
pub async fn execute(
    cwd: AbsolutePathBuf,
    specs: Vec<String>,
    unpin: bool,
    no_install: bool,
    force: bool,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    // Handle --unpin flag
    if unpin {
        let scope = match specs.as_slice() {
            [] => EnvScope::All,
            [scope] => EnvScope::parse(Some(scope))?,
            _ => return Err(Error::Other("pin --unpin accepts at most one scope".into())),
        };
        return do_unpin_scope(&cwd, scope, target).await;
    }

    if specs.is_empty() {
        show_pinned(&cwd).await?;
        println!();
        return show_package_manager_pin(&cwd).await;
    }

    let specs = EnvSpecs::parse(&specs)?;
    let package_manager_root = if specs.package_manager.is_some() {
        workspace_root(&cwd)?.ok_or_else(|| {
            Error::Other("cannot pin a package manager without package.json".into())
        })?
    } else {
        cwd.clone()
    };
    if specs.node.is_some()
        && specs.package_manager.is_some()
        && matches!(target, Some(PinTarget::NodeVersion | PinTarget::PackageManager))
    {
        return Err(Error::Other(
            "mixed Node.js and package-manager pins require the default targets or --target dev-engines"
                .into(),
        ));
    }

    if let Some(version) = specs.node {
        do_pin(&cwd, &version, no_install, force, target).await?;
    }
    if let Some((package_manager, version, hash)) = specs.package_manager {
        pin_package_manager(
            &package_manager_root,
            package_manager,
            &version,
            hash.as_deref(),
            no_install,
            force,
            target,
        )
        .await?;
    }
    Ok(ExitStatus::default())
}

async fn show_package_manager_pin(cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    match resolve_package_manager_from_package_json(cwd)? {
        Some(resolution) => {
            println!(
                "Pinned package manager: {}@{}",
                resolution.package_manager_type, resolution.version
            );
            println!(
                "  Source: {} ({})",
                resolution.source_path.as_path().display(),
                resolution.source
            );
        }
        None => println!("No package manager pinned."),
    }
    Ok(ExitStatus::default())
}

/// Show the current pinned version.
async fn show_pinned(cwd: &AbsolutePathBuf) -> Result<ExitStatus, Error> {
    let node_version_path = cwd.join(NODE_VERSION_FILE);

    // Check if .node-version exists in current directory
    if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
        let content = tokio::fs::read_to_string(&node_version_path).await?;
        let version = content.trim();
        println!("Pinned version: {version}");
        println!("  Source: {}", node_version_path.as_path().display());
        return Ok(ExitStatus::default());
    }

    // Check devEngines.runtime in the current directory's package.json
    if let Some(version) = read_dev_engines_node_version(cwd).await {
        println!("Pinned version: {version}");
        println!(
            "  Source: {} (devEngines.runtime)",
            cwd.join(PACKAGE_JSON_FILE).as_path().display()
        );
        return Ok(ExitStatus::default());
    }

    // Check for inherited version from parent directories
    if let Some((version, source)) = find_inherited_version(cwd).await? {
        println!("No version pinned in current directory.");
        println!("  Inherited: {version} from {source}");
        return Ok(ExitStatus::default());
    }

    // No .node-version anywhere - show default
    let config = load_config().await?;
    match config.default_node_version {
        Some(version) => {
            let config_path = get_config_path()?;
            println!("No version pinned.");
            println!("  Using default: {version} (from {})", config_path.as_path().display());
        }
        None => {
            println!("No version pinned.");
            println!("  Run 'vp env pin <version>' to pin a version.");
        }
    }

    Ok(ExitStatus::default())
}

/// Find an inherited pin (`.node-version` or `package.json#devEngines.runtime`)
/// in parent directories.
///
/// Mirrors the resolution order within each directory: `.node-version` first,
/// then the devEngines.runtime node entry. Returns the version and a display
/// string describing the source.
async fn find_inherited_version(cwd: &AbsolutePathBuf) -> Result<Option<(String, String)>, Error> {
    let mut current: Option<AbsolutePathBuf> = cwd.parent().map(|p| p.to_absolute_path_buf());

    while let Some(dir) = current {
        let node_version_path = dir.join(NODE_VERSION_FILE);
        if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
            let content = tokio::fs::read_to_string(&node_version_path).await?;
            return Ok(Some((
                content.trim().to_string(),
                node_version_path.as_path().display().to_string(),
            )));
        }
        if let Some(version) = read_dev_engines_node_version(&dir).await {
            return Ok(Some((
                version,
                format!("{} (devEngines.runtime)", dir.join(PACKAGE_JSON_FILE).as_path().display()),
            )));
        }
        current = dir.parent().map(|p| p.to_absolute_path_buf());
    }

    Ok(None)
}

/// Pin a version to the current directory.
async fn do_pin(
    cwd: &AbsolutePathBuf,
    version: &str,
    no_install: bool,
    force: bool,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    let provider = NodeProvider::new();

    // Resolve the version (aliases like lts/latest are resolved to exact versions)
    let (resolved_version, was_alias) = resolve_version_for_pin(version, &provider).await?;

    let node_version_exists =
        tokio::fs::try_exists(cwd.join(NODE_VERSION_FILE)).await.unwrap_or(false);
    let package_json_exists =
        tokio::fs::try_exists(cwd.join(PACKAGE_JSON_FILE)).await.unwrap_or(false);

    // Compatibility-first target selection (rfcs/dev-engines.md): an existing
    // .node-version keeps winning; otherwise pin into package.json#devEngines.runtime;
    // .node-version is only created when the directory has no package.json.
    let target = target.unwrap_or(if node_version_exists || !package_json_exists {
        PinTarget::NodeVersion
    } else {
        PinTarget::DevEngines
    });

    let pinned = match target {
        PinTarget::NodeVersion => {
            pin_node_version_file(cwd, version, &resolved_version, was_alias, force).await?
        }
        PinTarget::DevEngines => {
            if !package_json_exists {
                return Err(Error::Other(
                    format!(
                        "cannot pin to devEngines: no {} in {}",
                        PACKAGE_JSON_FILE,
                        cwd.as_path().display()
                    )
                    .into(),
                ));
            }
            let pinned = pin_dev_engines(cwd, version, &resolved_version, was_alias, force).await?;
            // .node-version still wins resolution, so warn that the devEngines pin
            // is shadowed until it is removed
            if pinned && node_version_exists {
                output::warn(&format!(
                    "{NODE_VERSION_FILE} still takes precedence over devEngines.runtime. Remove \
                     it with 'vp env unpin --target node-version' if devEngines should win."
                ));
            }
            pinned
        }
        PinTarget::PackageManager => {
            return Err(Error::Other(
                "--target package-manager requires a package-manager spec".into(),
            ));
        }
    };

    if !pinned {
        return Ok(ExitStatus::default());
    }

    // Invalidate resolve cache so the pinned version takes effect immediately
    crate::shim::invalidate_cache();

    // Pre-download the version unless --no-install is specified
    if no_install {
        output::note("Version will be downloaded on first use.");
    } else {
        // Download the runtime
        match vp_js_runtime::download_runtime(vp_js_runtime::JsRuntimeType::Node, &resolved_version)
            .await
        {
            Ok(_) => {
                output::success(&format!("Node.js {resolved_version} installed"));
            }
            Err(e) => {
                output::warn(&format!("Failed to download Node.js {resolved_version}: {e}"));
                output::note("Version will be downloaded on first use.");
            }
        }
    }

    Ok(ExitStatus::default())
}

/// Confirm overwriting an existing pin with a different version.
///
/// Returns `false` when the pin is already at `resolved_version` or when the
/// user declines the overwrite prompt (`force` skips the prompt only).
fn confirm_overwrite_pin(
    source_label: &str,
    existing_version: &str,
    resolved_version: &str,
    force: bool,
) -> Result<bool, Error> {
    if existing_version == resolved_version {
        println!("Already pinned to {resolved_version}");
        return Ok(false);
    }
    if force {
        return Ok(true);
    }

    // Prompt for confirmation, defaulting to yes (the user explicitly asked to
    // pin a new version, so only an explicit "no" cancels)
    print!("{source_label} {existing_version}");
    println!();
    print!("Overwrite with {resolved_version}? (Y/n): ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let answer = input.trim();
    if answer.eq_ignore_ascii_case("n") || answer.eq_ignore_ascii_case("no") {
        println!("Cancelled.");
        return Ok(false);
    }
    Ok(true)
}

/// Print the pin success message.
fn print_pin_success(input_version: &str, resolved_version: &str, was_alias: bool) {
    if was_alias {
        output::success(&format!(
            "Pinned Node.js version to {resolved_version} (resolved from {input_version})"
        ));
    } else {
        output::success(&format!("Pinned Node.js version to {resolved_version}"));
    }
}

/// Pin by writing the `.node-version` file.
///
/// Returns `true` when the pin was written, `false` when nothing changed
/// (already pinned, or the user cancelled).
async fn pin_node_version_file(
    cwd: &AbsolutePathBuf,
    input_version: &str,
    resolved_version: &str,
    was_alias: bool,
    force: bool,
) -> Result<bool, Error> {
    let node_version_path = cwd.join(NODE_VERSION_FILE);

    // Check if .node-version already exists
    if tokio::fs::try_exists(&node_version_path).await.unwrap_or(false) {
        let existing_content = tokio::fs::read_to_string(&node_version_path).await?;
        let existing_version = existing_content.trim();
        if !confirm_overwrite_pin(
            ".node-version already exists with version",
            existing_version,
            resolved_version,
            force,
        )? {
            return Ok(false);
        }
    }

    // Write the version to .node-version
    tokio::fs::write(&node_version_path, format!("{resolved_version}\n")).await?;

    print_pin_success(input_version, resolved_version, was_alias);
    println!("  Created {} in {}", NODE_VERSION_FILE, cwd.as_path().display());

    // If a devEngines.runtime range is declared and no longer satisfied, offer to
    // sync it in interactive terminals and warn otherwise (rfcs/dev-engines.md)
    check_dev_engines_sync(cwd, resolved_version, force, vp_shared::is_stdin_terminal()).await?;

    Ok(true)
}

/// Pin by writing `package.json#devEngines.runtime`.
///
/// Returns `true` when the pin was written, `false` when nothing changed
/// (already pinned, or the user cancelled).
async fn pin_dev_engines(
    cwd: &AbsolutePathBuf,
    input_version: &str,
    resolved_version: &str,
    was_alias: bool,
    force: bool,
) -> Result<bool, Error> {
    let package_json_path = cwd.join(PACKAGE_JSON_FILE);

    if let Some(existing_version) = read_dev_engines_node_version(cwd).await
        && !confirm_overwrite_pin(
            "devEngines.runtime already set to",
            &existing_version,
            resolved_version,
            force,
        )?
    {
        return Ok(false);
    }

    write_dev_engines_node_version(cwd, resolved_version).await?;

    print_pin_success(input_version, resolved_version, was_alias);
    println!("  Updated devEngines.runtime in {}", package_json_path.as_path().display());

    Ok(true)
}

/// Read the devEngines.runtime node entry version from the current directory's
/// package.json.
async fn read_dev_engines_node_version(cwd: &AbsolutePathBuf) -> Option<String> {
    let content = tokio::fs::read_to_string(cwd.join(PACKAGE_JSON_FILE)).await.ok()?;
    let pkg: vp_shared::PackageJson = serde_json::from_str(&content).ok()?;
    Some(pkg.dev_engines_runtime("node")?.version.clone()?.to_string())
}

/// After updating `.node-version`, check the declared devEngines.runtime range:
/// if the new version no longer satisfies it, offer to sync when `interactive`
/// (the caller passes stdin TTY-ness) and warn otherwise (rfcs/dev-engines.md).
/// Never syncs silently.
async fn check_dev_engines_sync(
    cwd: &AbsolutePathBuf,
    resolved_version: &str,
    force: bool,
    interactive: bool,
) -> Result<(), Error> {
    let Some(declared) = read_dev_engines_node_version(cwd).await else {
        return Ok(());
    };
    // An invalid declared range is reported by `vp env doctor`, not here
    let Ok(range) = node_semver::Range::parse(&declared) else {
        return Ok(());
    };
    let Ok(version) = node_semver::Version::parse(resolved_version) else {
        return Ok(());
    };
    if range.satisfies(&version) {
        return Ok(());
    }

    if interactive && !force {
        print!(
            "devEngines.runtime (\"{declared}\") is no longer satisfied. Update it to \
             {resolved_version}? (y/n): "
        );
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("y") {
            write_dev_engines_node_version(cwd, resolved_version).await?;
            output::success(&format!("Updated devEngines.runtime to {resolved_version}"));
            return Ok(());
        }
    }

    output::warn(&format!(
        "Node.js {resolved_version} does not satisfy devEngines.runtime \"{declared}\". Run \
         'vp env doctor' for details."
    ));
    Ok(())
}

/// Create a new devEngines.runtime node entry.
fn new_node_entry(version: &str) -> serde_json::Value {
    vp_shared::dev_engine_entry("node", version)
}

/// Write the node entry version into `package.json#devEngines.runtime`,
/// preserving formatting, key order, sibling runtime entries, and any existing
/// `onFail` value. An existing `engines.node` is never touched; a newly created
/// `devEngines` is placed right after `engines` when present.
async fn write_dev_engines_node_version(cwd: &AbsolutePathBuf, version: &str) -> Result<(), Error> {
    let package_json_path = cwd.join(PACKAGE_JSON_FILE);
    let content = tokio::fs::read_to_string(&package_json_path).await?;
    let updated = vp_shared::edit_json_object(&content, |obj| {
        set_dev_engines_runtime_node(obj, version);
    })
    .map_err(|e| Error::Other(format!("failed to update package.json: {e}").into()))?;
    tokio::fs::write(&package_json_path, updated).await?;
    Ok(())
}

/// Set the node entry version inside a parsed package.json object.
fn set_dev_engines_runtime_node(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    version: &str,
) {
    use serde_json::Value;

    let Some(dev_engines) = obj.get_mut("devEngines").and_then(Value::as_object_mut) else {
        // No (object-shaped) devEngines yet: create it next to engines
        vp_shared::insert_after(
            obj,
            "engines",
            "devEngines",
            serde_json::json!({ "runtime": new_node_entry(version) }),
        );
        return;
    };

    let Some(runtime) = dev_engines.get_mut("runtime") else {
        dev_engines.insert("runtime".into(), new_node_entry(version));
        return;
    };

    match runtime {
        // Single node entry: update its version in place (preserves onFail)
        Value::Object(entry) if entry.get("name").and_then(Value::as_str) == Some("node") => {
            entry.insert("version".into(), Value::String(version.to_string()));
        }
        // Single entry for another runtime: convert to array form and append node
        Value::Object(_) => {
            let existing = std::mem::take(runtime);
            *runtime = Value::Array(vec![existing, new_node_entry(version)]);
        }
        // Array form: update the existing node entry or append one
        Value::Array(entries) => {
            if let Some(entry) = entries
                .iter_mut()
                .filter_map(Value::as_object_mut)
                .find(|entry| entry.get("name").and_then(Value::as_str) == Some("node"))
            {
                entry.insert("version".into(), Value::String(version.to_string()));
            } else {
                entries.push(new_node_entry(version));
            }
        }
        // Malformed value: replace with a single node entry
        _ => {
            *runtime = new_node_entry(version);
        }
    }
}

/// Remove the node entry from `package.json#devEngines.runtime`.
///
/// Cleans up an emptied `runtime` array and an emptied `devEngines` object.
/// Returns `true` when an entry was removed.
async fn remove_dev_engines_runtime_node(cwd: &AbsolutePathBuf) -> Result<bool, Error> {
    let package_json_path = cwd.join(PACKAGE_JSON_FILE);
    if !tokio::fs::try_exists(&package_json_path).await.unwrap_or(false) {
        return Ok(false);
    }
    let content = tokio::fs::read_to_string(&package_json_path).await?;
    let mut removed = false;
    let updated = vp_shared::edit_json_object(&content, |obj| {
        use serde_json::Value;

        let Some(dev_engines) = obj.get_mut("devEngines").and_then(Value::as_object_mut) else {
            return;
        };
        match dev_engines.get_mut("runtime") {
            Some(Value::Object(entry))
                if entry.get("name").and_then(Value::as_str) == Some("node") =>
            {
                dev_engines.remove("runtime");
                removed = true;
            }
            Some(Value::Array(entries)) => {
                let before = entries.len();
                entries.retain(|entry| entry.get("name").and_then(Value::as_str) != Some("node"));
                removed = entries.len() != before;
                if entries.is_empty() {
                    dev_engines.remove("runtime");
                }
            }
            _ => {}
        }
        if removed && dev_engines.is_empty() {
            obj.remove("devEngines");
        }
    })
    .map_err(|e| Error::Other(format!("failed to update package.json: {e}").into()))?;

    if removed {
        tokio::fs::write(&package_json_path, updated).await?;
    }
    Ok(removed)
}

/// Resolve version for pinning.
///
/// Aliases (lts, latest) are resolved to exact versions.
/// Returns (resolved_version, was_alias).
async fn resolve_version_for_pin(
    version: &str,
    provider: &NodeProvider,
) -> Result<(String, bool), Error> {
    match version.to_lowercase().as_str() {
        "lts" => {
            let resolved = provider.resolve_latest_version().await?;
            Ok((resolved.to_string(), true))
        }
        "latest" => {
            let resolved = provider.resolve_absolute_latest_version().await?;
            Ok((resolved.to_string(), true))
        }
        _ => {
            // For exact versions, validate they exist
            if NodeProvider::is_exact_version(version) {
                // Validate the version exists by trying to resolve it
                provider.resolve_version(version).await?;
                Ok((version.to_string(), false))
            } else {
                // For ranges/partial versions, resolve to exact version
                let resolved = provider.resolve_version(version).await?;
                Ok((resolved.to_string(), true))
            }
        }
    }
}

/// Remove the Node.js pin from the current directory.
///
/// Removes the same source that `vp env pin` would write: `.node-version` when
/// present, otherwise the node entry from `package.json#devEngines.runtime`.
/// An explicit `target` overrides the selection.
pub async fn do_unpin(
    cwd: &AbsolutePathBuf,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    let node_version_path = cwd.join(NODE_VERSION_FILE);
    let node_version_exists = tokio::fs::try_exists(&node_version_path).await.unwrap_or(false);

    let target = target.unwrap_or(if node_version_exists {
        PinTarget::NodeVersion
    } else {
        PinTarget::DevEngines
    });

    match target {
        PinTarget::NodeVersion => {
            if !node_version_exists {
                println!("No {NODE_VERSION_FILE} file in current directory.");
                return Ok(ExitStatus::default());
            }

            tokio::fs::remove_file(&node_version_path).await?;

            // Invalidate resolve cache so the unpinned version falls back correctly
            crate::shim::invalidate_cache();

            output::success(&format!(
                "Removed {} from {}",
                NODE_VERSION_FILE,
                cwd.as_path().display()
            ));
        }
        PinTarget::DevEngines => {
            if remove_dev_engines_runtime_node(cwd).await? {
                // Invalidate resolve cache so the unpinned version falls back correctly
                crate::shim::invalidate_cache();

                output::success(&format!(
                    "Removed devEngines.runtime node entry from {}",
                    cwd.join(PACKAGE_JSON_FILE).as_path().display()
                ));
            } else {
                println!("No Node.js pin found in current directory.");
            }
        }
        PinTarget::PackageManager => {
            return Err(Error::Other(
                "--target package-manager requires package-manager scope".into(),
            ));
        }
    }

    Ok(ExitStatus::default())
}

pub async fn do_unpin_scope(
    cwd: &AbsolutePathBuf,
    scope: EnvScope,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    if matches!(scope, EnvScope::Node) && matches!(target, Some(PinTarget::PackageManager)) {
        return Err(Error::Other(
            "--target package-manager is incompatible with node scope".into(),
        ));
    }
    if scope.includes_package_managers()
        && !scope.includes_node()
        && matches!(target, Some(PinTarget::NodeVersion))
    {
        return Err(Error::Other(
            "--target node-version is incompatible with package-manager scope".into(),
        ));
    }
    if scope.includes_node() && !matches!(target, Some(PinTarget::PackageManager)) {
        do_unpin(cwd, target).await?;
    }
    if scope.includes_package_managers() {
        unpin_package_manager(cwd, scope, target).await?;
    }
    Ok(ExitStatus::default())
}

async fn pin_package_manager(
    cwd: &AbsolutePathBuf,
    package_manager: PackageManagerType,
    version: &str,
    hash: Option<&str>,
    no_install: bool,
    force: bool,
    target: Option<PinTarget>,
) -> Result<ExitStatus, Error> {
    if matches!(target, Some(PinTarget::NodeVersion)) {
        return Err(Error::Other("--target node-version cannot pin a package manager".into()));
    }
    let resolved = resolve_package_manager_version(package_manager, version).await?;
    package_manager::warn_if_target_differs(cwd, package_manager).await;
    let package_json_path = cwd.join(PACKAGE_JSON_FILE);
    let content = tokio::fs::read_to_string(&package_json_path).await?;
    let package_json: serde_json::Value = serde_json::from_str(&content)?;
    let use_top_level = matches!(target, Some(PinTarget::PackageManager))
        || (target.is_none() && package_json.get("packageManager").is_some());
    let shadowing_top_level =
        if use_top_level { None } else { existing_package_manager_pin(&content, true) };
    let shadowing_top_level = shadowing_top_level.filter(|(name, version)| {
        PackageManagerType::from_name(name) == Some(package_manager)
            && version.as_str() != resolved.as_str()
    });
    if let Some((name, version)) = existing_package_manager_pin(&content, use_top_level) {
        let existing = format!("{name}@{version}");
        let next = format!("{package_manager}@{resolved}");
        if existing != next
            && !confirm_overwrite_pin("Package manager already pinned to", &existing, &next, force)?
        {
            return Ok(ExitStatus::default());
        }
    }
    let mut changed = false;
    let updated = vp_shared::edit_json_object(&content, |obj| {
        if use_top_level {
            let prefix = format!("{package_manager}@{resolved}");
            let existing = obj.get("packageManager").and_then(serde_json::Value::as_str);
            let next = hash.map_or_else(
                || {
                    existing
                        .filter(|value| {
                            *value == prefix
                                || value
                                    .strip_prefix(&prefix)
                                    .is_some_and(|suffix| suffix.starts_with('+'))
                        })
                        .unwrap_or(&prefix)
                        .to_string()
                },
                |hash| format!("{prefix}+{hash}"),
            );
            if obj.get("packageManager").and_then(serde_json::Value::as_str) != Some(&next) {
                obj.insert("packageManager".into(), serde_json::Value::String(next));
                changed = true;
            }
        } else {
            set_dev_engines_package_manager(obj, package_manager, &resolved);
            changed = true;
        }
    })
    .map_err(|error| Error::Other(format!("failed to update package.json: {error}").into()))?;
    if !changed {
        println!("Already pinned to {package_manager}@{resolved}");
        return Ok(ExitStatus::default());
    }
    tokio::fs::write(&package_json_path, updated).await?;
    crate::shim::invalidate_cache();
    output::success(&format!("Pinned package manager to {package_manager}@{resolved}"));
    if let Some((name, version)) = shadowing_top_level {
        output::warn(&format!(
            "Top-level packageManager {name}@{version} remains effective; remove or update it to use devEngines.packageManager {package_manager}@{resolved}."
        ));
    }
    if no_install {
        output::note("Package manager will be downloaded on first use.");
    } else if let Err(error) = download_package_manager(package_manager, &resolved, hash).await {
        output::warn(&format!("Failed to download {package_manager} {resolved}: {error}"));
    }
    Ok(ExitStatus::default())
}

fn existing_package_manager_pin(content: &str, top_level: bool) -> Option<(String, String)> {
    if top_level {
        let package_json: serde_json::Value = serde_json::from_str(content).ok()?;
        let value = package_json.get("packageManager")?.as_str()?;
        let (name, version) = value.split_once('@')?;
        return Some((
            name.into(),
            version.split_once('+').map_or(version, |(version, _)| version).into(),
        ));
    }

    let package_json: vp_shared::PackageJson = serde_json::from_str(content).ok()?;
    let package_manager = package_json.dev_engines?.package_manager?;
    let entry = package_manager.entries().first()?;
    Some((entry.name.to_string(), entry.version.as_deref().unwrap_or("*").to_string()))
}

fn set_dev_engines_package_manager(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    package_manager: PackageManagerType,
    version: &str,
) {
    use serde_json::Value;

    let entry = vp_shared::dev_engine_entry(&package_manager.to_string(), version);
    let Some(dev_engines) = obj.get_mut("devEngines").and_then(Value::as_object_mut) else {
        vp_shared::insert_after(
            obj,
            "engines",
            "devEngines",
            serde_json::json!({ "packageManager": entry }),
        );
        return;
    };
    let Some(field) = dev_engines.get_mut("packageManager") else {
        dev_engines.insert("packageManager".into(), entry);
        return;
    };
    match field {
        Value::Object(value)
            if value.get("name").and_then(Value::as_str)
                == Some(package_manager.to_string().as_str()) =>
        {
            value.insert("version".into(), Value::String(version.into()));
        }
        Value::Object(_) => {
            let previous = std::mem::take(field);
            *field = Value::Array(vec![entry, previous]);
        }
        Value::Array(entries) => {
            let name = package_manager.to_string();
            let mut entry = entries
                .iter()
                .position(|value| value.get("name").and_then(Value::as_str) == Some(name.as_str()))
                .map(|index| entries.remove(index))
                .unwrap_or(entry);
            if let Some(value) = entry.as_object_mut() {
                value.insert("version".into(), Value::String(version.into()));
            }
            entries.insert(0, entry);
        }
        _ => *field = entry,
    }
}

async fn unpin_package_manager(
    cwd: &AbsolutePathBuf,
    scope: EnvScope,
    target: Option<PinTarget>,
) -> Result<(), Error> {
    if matches!(target, Some(PinTarget::NodeVersion)) {
        return Ok(());
    }
    let root = workspace_root(cwd)?.unwrap_or_else(|| cwd.clone());
    let package_json_path = root.join(PACKAGE_JSON_FILE);
    let Ok(content) = tokio::fs::read_to_string(&package_json_path).await else {
        println!("No package manager pin found in current directory.");
        return Ok(());
    };
    let effective = resolve_package_manager_from_package_json(&root)?;
    let expected = match scope {
        EnvScope::PackageManager(package_manager) => Some(package_manager),
        _ if matches!(target, Some(PinTarget::DevEngines)) => {
            let package_json: vp_shared::PackageJson = serde_json::from_str(&content)?;
            package_json.dev_engines.and_then(|dev_engines| dev_engines.package_manager).and_then(
                |field| {
                    field
                        .entries()
                        .iter()
                        .find_map(|entry| PackageManagerType::from_name(&entry.name))
                },
            )
        }
        _ => effective.as_ref().map(|resolution| resolution.package_manager_type),
    };
    let mut changed = false;
    let updated = vp_shared::edit_json_object(&content, |obj| {
        let remove_top_level = matches!(target, Some(PinTarget::PackageManager))
            || (target.is_none() && obj.get("packageManager").is_some());
        let top_level_matches = expected.is_none_or(|expected| {
            obj.get("packageManager")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| value.split_once('@'))
                .and_then(|(name, _)| PackageManagerType::from_name(name))
                == Some(expected)
        });
        if remove_top_level && top_level_matches {
            changed = obj.remove("packageManager").is_some();
        } else if !matches!(target, Some(PinTarget::PackageManager))
            && let Some(expected) = expected
        {
            changed = remove_dev_engines_package_manager(obj, expected);
        }
    })
    .map_err(|error| Error::Other(format!("failed to update package.json: {error}").into()))?;
    if changed {
        tokio::fs::write(&package_json_path, updated).await?;
        crate::shim::invalidate_cache();
        output::success("Removed package-manager pin");
    } else {
        println!("No package manager pin found in current directory.");
    }
    Ok(())
}

fn workspace_root(cwd: &AbsolutePathBuf) -> Result<Option<AbsolutePathBuf>, Error> {
    match vt_workspace::find_workspace_root(cwd) {
        Ok((workspace, _)) => Ok(Some(workspace.path.to_absolute_path_buf())),
        Err(vt_workspace::Error::PackageJsonNotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn remove_dev_engines_package_manager(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    expected: PackageManagerType,
) -> bool {
    let Some(dev_engines) = obj.get_mut("devEngines").and_then(serde_json::Value::as_object_mut)
    else {
        return false;
    };
    let Some(field) = dev_engines.get_mut("packageManager") else {
        return false;
    };
    let expected = expected.to_string();
    let changed = match field {
        serde_json::Value::Object(entry) => {
            entry.get("name").and_then(serde_json::Value::as_str) == Some(expected.as_str())
        }
        serde_json::Value::Array(entries) => {
            let before = entries.len();
            entries.retain(|entry| {
                entry.get("name").and_then(serde_json::Value::as_str) != Some(expected.as_str())
            });
            before != entries.len()
        }
        _ => false,
    };
    let remove_field = changed
        && match field {
            serde_json::Value::Object(_) => true,
            serde_json::Value::Array(entries) => entries.is_empty(),
            _ => false,
        };
    if remove_field {
        dev_engines.remove("packageManager");
    }
    changed
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use vp_shared::env_vars;
    use vt_path::AbsolutePathBuf;

    use super::*;

    /// Shared VP_HOME for tests that hit the real Node.js version index:
    /// pinning isolates them from concurrent scopes, and one shared root
    /// keeps the index cache warm across tests and runs.
    fn shared_vp_home() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("vp-global-cli-tests-vp-home");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn package_manager_pin_preserves_matching_integrity_suffix() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::write(
            cwd.join("package.json"),
            "{\n  \"packageManager\": \"pnpm@10.18.0+sha512.keep\"\n}\n",
        )
        .await
        .unwrap();

        pin_package_manager(&cwd, PackageManagerType::Pnpm, "10.18.0", None, true, true, None)
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(cwd.join("package.json")).await.unwrap();
        assert!(content.contains("pnpm@10.18.0+sha512.keep"));
    }

    #[tokio::test]
    async fn package_manager_pin_drops_stale_integrity_suffix() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::write(
            cwd.join("package.json"),
            "{\n  \"packageManager\": \"pnpm@10.17.0+sha512.stale\"\n}\n",
        )
        .await
        .unwrap();

        pin_package_manager(&cwd, PackageManagerType::Pnpm, "10.18.0", None, true, true, None)
            .await
            .unwrap();

        let content = tokio::fs::read_to_string(cwd.join("package.json")).await.unwrap();
        assert!(content.contains("pnpm@10.18.0"));
        assert!(!content.contains("sha512.stale"));
    }

    #[tokio::test]
    async fn package_manager_pin_uses_explicit_integrity_suffix() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::write(
            cwd.join("package.json"),
            "{\n  \"packageManager\": \"pnpm@10.17.0\"\n}\n",
        )
        .await
        .unwrap();

        pin_package_manager(
            &cwd,
            PackageManagerType::Pnpm,
            "10.18.0",
            Some("sha512.explicit"),
            true,
            true,
            None,
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(cwd.join("package.json")).await.unwrap();
        assert!(content.contains("pnpm@10.18.0+sha512.explicit"));
    }

    #[test]
    fn remove_last_package_manager_array_entry_removes_field() {
        let mut manifest = serde_json::json!({
            "devEngines": {
                "packageManager": [{ "name": "pnpm", "version": "10.18.0" }],
                "runtime": { "name": "node", "version": "22.0.0" }
            }
        });
        assert!(remove_dev_engines_package_manager(
            manifest.as_object_mut().unwrap(),
            PackageManagerType::Pnpm,
        ));
        assert!(manifest["devEngines"].get("packageManager").is_none());
        assert_eq!(manifest["devEngines"]["runtime"]["version"], "22.0.0");
    }

    #[tokio::test]
    async fn package_manager_unpin_target_does_not_remove_node_pin() {
        let temp_dir = TempDir::new().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        tokio::fs::write(cwd.join(".node-version"), "22.0.0\n").await.unwrap();
        tokio::fs::write(
            cwd.join("package.json"),
            "{\n  \"packageManager\": \"pnpm@10.18.0\"\n}\n",
        )
        .await
        .unwrap();

        do_unpin_scope(&cwd, EnvScope::All, Some(PinTarget::PackageManager)).await.unwrap();

        assert!(cwd.join(".node-version").as_path().exists());
        let manifest = tokio::fs::read_to_string(cwd.join("package.json")).await.unwrap();
        assert!(!manifest.contains("packageManager"));
    }

    #[tokio::test]
    async fn test_show_pinned_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Should not error when no .node-version exists
        let result = show_pinned(&temp_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_pinned_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        let result = show_pinned(&temp_path).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_inherited_version() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version in parent
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();

        // Create subdirectory
        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let result = find_inherited_version(&subdir).await.unwrap();
        assert!(result.is_some());
        let (version, _) = result.unwrap();
        assert_eq!(version, "20.18.0");
    }

    #[tokio::test]
    async fn test_find_inherited_version_from_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Ancestor pin declared via devEngines.runtime instead of .node-version
        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
        )
        .await
        .unwrap();

        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let (version, source) = find_inherited_version(&subdir).await.unwrap().unwrap();
        assert_eq!(version, "^24.0.0");
        assert!(source.ends_with("package.json (devEngines.runtime)"), "got: {source}");
    }

    #[tokio::test]
    async fn test_find_inherited_version_node_version_wins_over_dev_engines() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Same directory declares both: .node-version wins (resolution order)
        tokio::fs::write(temp_path.join(".node-version"), "20.18.0\n").await.unwrap();
        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
        )
        .await
        .unwrap();

        let subdir = temp_path.join("subdir");
        tokio::fs::create_dir(&subdir).await.unwrap();

        let (version, source) = find_inherited_version(&subdir).await.unwrap().unwrap();
        assert_eq!(version, "20.18.0");
        assert!(source.ends_with(".node-version"), "got: {source}");
    }

    #[tokio::test]
    async fn test_do_unpin() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Create .node-version
        let node_version_path = temp_path.join(".node-version");
        tokio::fs::write(&node_version_path, "20.18.0\n").await.unwrap();

        // Unpin (scoped VP_HOME isolates the resolve-cache invalidation)
        vp_shared::EnvConfig::scoped_async(|_| async {
            let result = do_unpin(&temp_path, None).await;
            assert!(result.is_ok());
        })
        .await;

        // File should be gone
        assert!(!tokio::fs::try_exists(&node_version_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_do_unpin_invalidates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        // Pin VP_HOME to the temp dir so invalidate_cache() targets our file
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, temp_path.as_path())],
            |_| async {
                // Create cache file manually
                let cache_dir = temp_path.join("cache");
                std::fs::create_dir_all(&cache_dir).unwrap();
                let cache_file = cache_dir.join("resolve_cache.json");
                std::fs::write(&cache_file, r#"{"version":2,"entries":{}}"#).unwrap();
                assert!(
                    std::fs::metadata(cache_file.as_path()).is_ok(),
                    "Cache file should exist before unpin"
                );

                // Create .node-version and unpin
                let node_version_path = temp_path.join(".node-version");
                tokio::fs::write(&node_version_path, "20.18.0\n").await.unwrap();
                let result = do_unpin(&temp_path, None).await;
                assert!(result.is_ok());

                // Cache file should be removed by invalidate_cache()
                assert!(
                    std::fs::metadata(cache_file.as_path()).is_err(),
                    "Cache file should be removed after unpin"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_do_pin_invalidates_cache() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let vp_home = shared_vp_home();

        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                // Create cache file manually
                let cache_dir = vp_home.join("cache");
                std::fs::create_dir_all(&cache_dir).unwrap();
                let cache_file = cache_dir.join("resolve_cache.json");
                std::fs::write(&cache_file, r#"{"version":2,"entries":{}}"#).unwrap();
                assert!(
                    std::fs::metadata(cache_file.as_path()).is_ok(),
                    "Cache file should exist before pin"
                );

                // Pin an exact version (no_install=true to skip download, force=true to skip prompt)
                let result = do_pin(&temp_path, "20.18.0", true, true, None).await;
                assert!(result.is_ok());

                // .node-version should be created
                let node_version_path = temp_path.join(".node-version");
                assert!(tokio::fs::try_exists(&node_version_path).await.unwrap());
                let content = tokio::fs::read_to_string(&node_version_path).await.unwrap();
                assert_eq!(content.trim(), "20.18.0");

                // Cache file should be removed by invalidate_cache()
                assert!(
                    std::fs::metadata(cache_file.as_path()).is_err(),
                    "Cache file should be removed after pin"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_do_unpin_no_file() {
        vp_shared::EnvConfig::scoped_async(|_| async {
            let temp_dir = TempDir::new().unwrap();
            let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

            // Should not error when no file exists
            let result = do_unpin(&temp_path, None).await;
            assert!(result.is_ok());
        })
        .await;
    }

    #[tokio::test]
    async fn test_do_pin_targets_dev_engines_when_package_json_exists() {
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                let temp_dir = TempDir::new().unwrap();
                let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

                // package.json without .node-version: the pin goes into devEngines.runtime
                tokio::fs::write(
            temp_path.join("package.json"),
            "{\n  \"name\": \"test\",\n  \"engines\": {\n    \"node\": \">=18.0.0\"\n  }\n}\n",
        )
        .await
        .unwrap();

                let result = do_pin(&temp_path, "20.18.0", true, true, None).await;
                assert!(result.is_ok());

                // .node-version is NOT created
                assert!(!tokio::fs::try_exists(temp_path.join(".node-version")).await.unwrap());

                let content =
                    tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
                let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
                let entry = &pkg["devEngines"]["runtime"];
                assert_eq!(entry["name"].as_str().unwrap(), "node");
                assert_eq!(entry["version"].as_str().unwrap(), "20.18.0");
                assert_eq!(entry["onFail"].as_str().unwrap(), "download");
                // existing engines.node is kept unchanged
                assert_eq!(pkg["engines"]["node"].as_str().unwrap(), ">=18.0.0");
                // devEngines is placed right after engines
                let keys: Vec<&str> = pkg.as_object().unwrap().keys().map(String::as_str).collect();
                assert_eq!(keys, ["name", "engines", "devEngines"]);
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_do_pin_keeps_node_version_file_target_when_it_exists() {
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                let temp_dir = TempDir::new().unwrap();
                let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

                tokio::fs::write(temp_path.join(".node-version"), "18.20.0\n").await.unwrap();
                tokio::fs::write(temp_path.join("package.json"), "{\n  \"name\": \"test\"\n}\n")
                    .await
                    .unwrap();

                // force=true skips the overwrite prompt
                let result = do_pin(&temp_path, "20.18.0", true, true, None).await;
                assert!(result.is_ok());

                // .node-version keeps winning for writes (compatibility-first)
                let content =
                    tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
                assert_eq!(content.trim(), "20.18.0");

                // package.json is untouched
                let pkg = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
                assert!(!pkg.contains("devEngines"));
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_do_pin_explicit_dev_engines_target_wins_over_node_version_file() {
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                let temp_dir = TempDir::new().unwrap();
                let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

                tokio::fs::write(temp_path.join(".node-version"), "18.20.0\n").await.unwrap();
                tokio::fs::write(temp_path.join("package.json"), "{\n  \"name\": \"test\"\n}\n")
                    .await
                    .unwrap();

                let result =
                    do_pin(&temp_path, "20.18.0", true, true, Some(PinTarget::DevEngines)).await;
                assert!(result.is_ok());

                // devEngines is written; .node-version stays untouched (a warning is printed)
                let content =
                    tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
                let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
                assert_eq!(pkg["devEngines"]["runtime"]["version"].as_str().unwrap(), "20.18.0");
                let node_version =
                    tokio::fs::read_to_string(temp_path.join(".node-version")).await.unwrap();
                assert_eq!(node_version.trim(), "18.20.0");
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_do_pin_dev_engines_target_requires_package_json() {
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                let temp_dir = TempDir::new().unwrap();
                let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

                let result =
                    do_pin(&temp_path, "20.18.0", true, true, Some(PinTarget::DevEngines)).await;
                assert!(result.is_err());
            },
        )
        .await;
    }

    #[test]
    fn test_set_dev_engines_runtime_node_updates_in_place_preserving_on_fail() {
        let content = r#"{
  "devEngines": {
    "runtime": {
      "name": "node",
      "version": "^22.0.0",
      "onFail": "error"
    }
  }
}
"#;
        let updated = vp_shared::edit_json_object(content, |obj| {
            set_dev_engines_runtime_node(obj, "24.1.0");
        })
        .unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let entry = &pkg["devEngines"]["runtime"];
        assert_eq!(entry["version"].as_str().unwrap(), "24.1.0");
        // the existing onFail is preserved
        assert_eq!(entry["onFail"].as_str().unwrap(), "error");
    }

    #[test]
    fn test_set_dev_engines_runtime_node_converts_other_runtime_to_array() {
        let content = r#"{"devEngines":{"runtime":{"name":"deno","version":"^2.0.0"}}}"#;
        let updated = vp_shared::edit_json_object(content, |obj| {
            set_dev_engines_runtime_node(obj, "24.1.0");
        })
        .unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let entries = pkg["devEngines"]["runtime"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        // the existing deno entry stays first
        assert_eq!(entries[0]["name"].as_str().unwrap(), "deno");
        assert_eq!(entries[1]["name"].as_str().unwrap(), "node");
        assert_eq!(entries[1]["version"].as_str().unwrap(), "24.1.0");
    }

    #[test]
    fn test_set_dev_engines_runtime_node_updates_array_entry() {
        let content = r#"{
  "devEngines": {
    "runtime": [
      {"name": "deno", "version": "^2.0.0"},
      {"name": "node", "version": "^22.0.0", "onFail": "warn"}
    ]
  }
}
"#;
        let updated = vp_shared::edit_json_object(content, |obj| {
            set_dev_engines_runtime_node(obj, "24.1.0");
        })
        .unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&updated).unwrap();
        let entries = pkg["devEngines"]["runtime"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1]["version"].as_str().unwrap(), "24.1.0");
        assert_eq!(entries[1]["onFail"].as_str().unwrap(), "warn");
    }

    #[tokio::test]
    async fn test_do_unpin_dev_engines_default_when_no_node_version_file() {
        vp_shared::EnvConfig::scoped_async(|_| async {
            let temp_dir = TempDir::new().unwrap();
            let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

            tokio::fs::write(
                temp_path.join("package.json"),
                r#"{
  "name": "test",
  "devEngines": {
    "runtime": {"name": "node", "version": "^24.0.0"}
  }
}
"#,
            )
            .await
            .unwrap();

            let result = do_unpin(&temp_path, None).await;
            assert!(result.is_ok());

            let content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
            let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
            // the emptied devEngines object is cleaned up entirely
            assert!(pkg.get("devEngines").is_none());
            assert_eq!(pkg["name"].as_str().unwrap(), "test");
        })
        .await;
    }

    #[tokio::test]
    async fn test_remove_dev_engines_runtime_node_keeps_other_entries() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{
  "devEngines": {
    "runtime": [
      {"name": "deno", "version": "^2.0.0"},
      {"name": "node", "version": "^24.0.0"}
    ],
    "packageManager": {"name": "pnpm", "version": "^11.0.0"}
  }
}
"#,
        )
        .await
        .unwrap();

        let removed = remove_dev_engines_runtime_node(&temp_path).await.unwrap();
        assert!(removed);

        let content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
        // other runtime entries and the packageManager entry are preserved
        let entries = pkg["devEngines"]["runtime"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["name"].as_str().unwrap(), "deno");
        assert_eq!(pkg["devEngines"]["packageManager"]["name"].as_str().unwrap(), "pnpm");
    }

    #[tokio::test]
    async fn test_check_dev_engines_sync_warns_without_tty() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
        )
        .await
        .unwrap();

        // 20.18.0 does not satisfy ^24.0.0: without a TTY this warns and never
        // rewrites the declared range
        check_dev_engines_sync(&temp_path, "20.18.0", false, false).await.unwrap();

        let content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(pkg["devEngines"]["runtime"]["version"].as_str().unwrap(), "^24.0.0");
    }

    #[tokio::test]
    async fn test_check_dev_engines_sync_force_skips_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        tokio::fs::write(
            temp_path.join("package.json"),
            r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#,
        )
        .await
        .unwrap();

        // --force never prompts, even in an interactive terminal: it warns and
        // leaves the declared range untouched
        check_dev_engines_sync(&temp_path, "20.18.0", true, true).await.unwrap();

        let content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        let pkg: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(pkg["devEngines"]["runtime"]["version"].as_str().unwrap(), "^24.0.0");
    }

    #[tokio::test]
    async fn test_check_dev_engines_sync_noop_when_satisfied() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

        let original = r#"{"devEngines":{"runtime":{"name":"node","version":"^24.0.0"}}}"#;
        tokio::fs::write(temp_path.join("package.json"), original).await.unwrap();

        // interactive=true must not prompt when the range is already satisfied
        check_dev_engines_sync(&temp_path, "24.2.0", false, true).await.unwrap();

        let content = tokio::fs::read_to_string(temp_path.join("package.json")).await.unwrap();
        assert_eq!(content, original);
    }

    #[tokio::test]
    async fn test_resolve_version_for_pin_partial_version() {
        let vp_home = shared_vp_home();
        vp_shared::EnvConfig::with_vars_async(
            [(env_vars::VP_HOME, vp_home.as_os_str())],
            |_| async {
                let provider = NodeProvider::new();

                // Partial version "20" should resolve to an exact version like "20.x.y"
                let (resolved, was_alias) = resolve_version_for_pin("20", &provider).await.unwrap();
                assert!(was_alias, "partial version should be treated as alias");

                // The resolved version should be a full semver version starting with "20."
                assert!(
                    resolved.starts_with("20."),
                    "expected resolved version to start with '20.', got: {resolved}"
                );

                // Should be a valid exact version (major.minor.patch)
                let parts: Vec<&str> = resolved.split('.').collect();
                assert_eq!(parts.len(), 3, "expected 3 version parts, got: {resolved}");
                assert!(
                    parts.iter().all(|p| p.parse::<u64>().is_ok()),
                    "all parts should be numeric"
                );
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_resolve_version_for_pin_exact_version() {
        let provider = NodeProvider::new();

        // Exact version should be returned as-is
        let (resolved, was_alias) = resolve_version_for_pin("20.18.0", &provider).await.unwrap();
        assert!(!was_alias, "exact version should not be treated as alias");
        assert_eq!(resolved, "20.18.0");
    }
}
