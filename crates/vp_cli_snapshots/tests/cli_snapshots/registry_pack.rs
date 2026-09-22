//! Share immutable package tarballs across nextest processes within one test run.
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

/// Filesystem stamps avoid re-reading and compressing the package bytes in every
/// worker. The directory is run-scoped, not a persistent cache across builds.
fn stamp(path: &Path) -> Result<serde_json::Value, String> {
    let metadata = path.metadata().map_err(|error| format!("{}: {error}", path.display()))?;
    let modified = metadata
        .modified()
        .and_then(|time| time.duration_since(UNIX_EPOCH).map_err(std::io::Error::other))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(serde_json::json!({
        "bytes": metadata.len(),
        "modified_ns": modified.as_nanos().to_string(),
    }))
}

fn collect_inputs(
    root: &Path,
    relative: &Path,
    inputs: &mut BTreeMap<String, serde_json::Value>,
    ancestors: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let path = root.join(relative);
    if !path.is_dir() {
        inputs.insert(relative.to_string_lossy().into_owned(), stamp(&path)?);
        return Ok(());
    }
    let canonical = dunce::canonicalize(&path).map_err(|error| error.to_string())?;
    if ancestors.contains(&canonical) {
        return Err(format!("cycle in package inputs at {}", path.display()));
    }
    ancestors.push(canonical);
    for entry in path.read_dir().map_err(|error| format!("{}: {error}", path.display()))? {
        let name = entry.map_err(|error| error.to_string())?.file_name();
        // Installed dependencies and Cargo output are not package inputs.
        if matches!(name.to_str(), Some("node_modules" | "target" | ".git")) {
            continue;
        }
        collect_inputs(root, &relative.join(name), inputs, ancestors)?;
    }
    ancestors.pop();
    Ok(())
}

fn build_stamp(repo: &Path) -> Result<serde_json::Value, String> {
    let repo = dunce::canonicalize(repo).map_err(|error| error.to_string())?;
    let mut inputs = BTreeMap::new();
    for path in
        ["package.json", "pnpm-workspace.yaml", "pnpm-lock.yaml", "packages/cli", "packages/core"]
    {
        collect_inputs(&repo, Path::new(path), &mut inputs, &mut Vec::new())?;
    }
    Ok(serde_json::json!({ "repo": repo, "inputs": inputs }))
}

fn archives(directory: &Path) -> Result<serde_json::Value, String> {
    let mut archives = BTreeMap::new();
    for entry in directory.read_dir().map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "tgz") {
            archives.insert(entry.file_name().to_string_lossy().into_owned(), stamp(&path)?);
        }
    }
    if archives.len() != 2 {
        return Err(format!(
            "expected two packed packages in {}, found {}",
            directory.display(),
            archives.len()
        ));
    }
    Ok(serde_json::json!(archives))
}

/// Only the lock holder prepares the packages. Publish the complete directory
/// atomically, so a failed pack can be retried without exposing partial tarballs.
/// A stale directory fails explicitly rather than testing a previous build.
pub fn get_or_prepare(
    root: &Path,
    repo: &Path,
    prepare: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("pack.lock"))
        .map_err(|error| error.to_string())?;
    File::lock(&lock).map_err(|error| error.to_string())?;
    let packages = root.join("packages");
    let build = build_stamp(repo)?;
    if packages.exists() {
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(packages.join("build.json")).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if manifest["build"] != build || manifest["archives"] != archives(&packages)? {
            return Err(format!(
                "prepared snapshot packages in {} are stale; use a fresh VP_SNAP_PACKAGES_DIR for this run",
                root.display()
            ));
        }
        return Ok(packages);
    }
    let staging = tempfile::tempdir_in(root).map_err(|error| error.to_string())?;
    prepare(staging.path())?;
    if build != build_stamp(repo)? {
        return Err(
            "checkout changed while packing snapshot packages; retry after the build finishes"
                .into(),
        );
    }
    let manifest = serde_json::json!({ "build": build, "archives": archives(staging.path())? });
    std::fs::write(staging.path().join("build.json"), serde_json::to_vec(&manifest).unwrap())
        .map_err(|error| error.to_string())?;
    std::fs::rename(staging.path(), &packages).map_err(|error| error.to_string())?;
    Ok(packages)
}
